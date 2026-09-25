// PIMProbe - touch probing for the Nestworks C500.
// Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::{
    device::Device,
    logs::LogStore,
    reviews::Reviews,
    settings::{Settings, SettingsError},
    updates::{UpdateError, UpdateManager},
};
use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, FromRequest, Query, Request, State, rejection::JsonRejection},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use pimprobe_core::{
    CancellationToken, Controller, Error, RepeatabilityEvent, RepeatabilityOptions,
    RepeatabilityReport, RepeatabilitySettings, RoutineConfig, TimingPolicy,
};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::{
    convert::Infallible,
    fs,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    time::Instant,
};
use tokio::sync::{Mutex, OwnedMutexGuard, mpsc};
use tokio_stream::Stream;

pub struct App {
    pub device: Device,
    settings: Mutex<Settings>,
    reviews: Mutex<Reviews>,
    action: Arc<Mutex<()>>,
    active: AtomicBool,
    recovery_failed: AtomicBool,
    shutdown: CancellationToken,
    ready_token: Option<String>,
    updates: UpdateManager,
    logs: Arc<LogStore>,
}

impl App {
    pub fn new(
        device: Device,
        settings: Settings,
        ready_token: Option<String>,
        logs: LogStore,
    ) -> Arc<Self> {
        Arc::new(Self {
            device,
            settings: Mutex::new(settings),
            reviews: Mutex::new(Reviews::default()),
            action: Arc::new(Mutex::new(())),
            active: AtomicBool::new(false),
            recovery_failed: AtomicBool::new(false),
            shutdown: CancellationToken::new(),
            ready_token,
            updates: UpdateManager::production(),
            logs: Arc::new(logs),
        })
    }

    fn acquire(&self) -> Result<OwnedMutexGuard<()>, ApiError> {
        if self.shutdown.is_cancelled() {
            return Err(ApiError::conflict("shutdown", "Service is stopping"));
        }
        let guard = self
            .action
            .clone()
            .try_lock_owned()
            .map_err(|_| ApiError::conflict("busy", "Another action is running"))?;
        if self.recovery_failed.load(Ordering::SeqCst) {
            return Err(ApiError::conflict(
                "recovery",
                "Waiting for confirmation that motion has stopped",
            ));
        }
        Ok(guard)
    }

    pub async fn shutdown(&self) {
        self.shutdown.cancel();
        // Wait for any run to finish its independent stop-confirmation path.
        let _guard = self.action.lock().await;
        self.device.shutdown().await;
    }

    async fn recover(&self, error: &Error) {
        if matches!(
            error,
            Error::Preflight(_)
                | Error::InvalidConfig(_)
                | Error::MotionBlocked
                | Error::CoarseNoContact
                | Error::UnexpectedContact {
                    retracted: true,
                    ..
                }
        ) {
            return;
        }
        if self.device.state().motion_blocked {
            return;
        }
        loop {
            if self.device.stop().await.is_ok() {
                self.recovery_failed.store(false, Ordering::SeqCst);
                return;
            }
            self.recovery_failed.store(true, Ordering::SeqCst);
            tokio::select! {
                _ = self.shutdown.cancelled() => return,
                _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {},
            }
        }
    }
}

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/api/v1/state", get(state))
        .route("/api/v1/settings", get(settings).patch(update_settings))
        .route("/api/v1/settings/schema", get(settings_schema))
        .route("/api/v1/probe-actuator", post(actuator))
        .route("/api/v1/wcs", post(wcs))
        .route("/api/v1/routine/review", post(review))
        .route("/api/v1/routine/run", post(run))
        .route("/api/v1/routine/zero", post(zero))
        .route("/api/v1/routine/return", post(return_start))
        .route("/api/v1/routine/measured", post(go_to_measured))
        .route("/api/v1/repeatability/run", post(repeatability))
        .route("/api/v1/logs/history", get(history))
        .route("/api/v1/logs/export", post(export_logs))
        .route("/api/v1/logs/clear", post(clear_logs))
        .route("/api/v1/updates/check", get(check_update))
        .route("/api/v1/updates/install", post(install_update))
        .route("/api/v1/updates/status", get(update_status))
        .route("/api/v1/mock/ready", get(ready))
        .layer(DefaultBodyLimit::max(8192))
        .with_state(app)
}

#[derive(Debug)]
struct ApiError(StatusCode, &'static str, String);
impl ApiError {
    fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self(StatusCode::CONFLICT, code, message.into())
    }
}
impl From<Error> for ApiError {
    fn from(error: Error) -> Self {
        Self::conflict(error_code(&error), error.to_string())
    }
}
impl From<UpdateError> for ApiError {
    fn from(error: UpdateError) -> Self {
        if matches!(error, UpdateError::MachineBusy) {
            Self::conflict("machine_busy", error.to_string())
        } else {
            Self(StatusCode::BAD_GATEWAY, "update", error.to_string())
        }
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Qt 6.9's QML XMLHttpRequest discards the status and body of non-2xx
        // loopback responses. Keep application failures visible to the operator.
        Json(json!({
            "error": true,
            "code": self.1,
            "message": self.2,
            "status": self.0.as_u16()
        }))
        .into_response()
    }
}

struct ApiJson<T>(T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    S: Send + Sync,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|error| ApiError(StatusCode::BAD_REQUEST, "request", error.body_text()))
    }
}
fn error_code(error: &Error) -> &'static str {
    match error {
        Error::MotionBlocked => "controller_alarm",
        Error::CoarseNoContact | Error::NoContact => "no_contact",
        Error::UnexpectedContact { .. } => "unexpected_contact",
        _ => "failed",
    }
}

fn log_error(error: std::io::Error) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, "log", error.to_string())
}

fn log_warning(result: std::io::Result<()>) {
    if let Err(error) = result {
        eprintln!("Probing log: {error}");
    }
}

fn software_info() -> Value {
    let root = Path::new("/userdata/pimprobe");
    let version = fs::read_to_string(root.join("VERSION"))
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").into());
    let commit = fs::read_to_string(root.join("COMMIT")).unwrap_or_default();
    json!({"version":version.trim(), "commit":commit.trim()})
}

fn routine_label(config: &RoutineConfig) -> String {
    if config.z {
        return "Z surface".into();
    }
    if config.family == "center" {
        return format!("{} center", config.feature.replace('-', " "));
    }
    let feature = match (config.x, config.y) {
        (0, y) => format!("Y{} edge", if y > 0 { "+" } else { "-" }),
        (x, 0) => format!("X{} edge", if x > 0 { "+" } else { "-" }),
        (x, y) => format!(
            "X{}/Y{} corner",
            if x > 0 { "+" } else { "-" },
            if y > 0 { "+" } else { "-" }
        ),
    };
    format!(
        "{} {feature}",
        if config.family == "inside" {
            "Inside"
        } else {
            "Outside"
        }
    )
}

async fn history(State(app): State<Arc<App>>) -> Result<Json<Vec<Value>>, ApiError> {
    let logs = app.logs.clone();
    let entries = tokio::task::spawn_blocking(move || logs.history())
        .await
        .map_err(|e| ApiError::conflict("log", e.to_string()))?
        .map_err(log_error)?;
    Ok(Json(entries))
}

fn usb_mount(mounts: &str) -> Option<PathBuf> {
    let mut candidates = mounts.lines().filter_map(|line| {
        let mut fields = line.split_whitespace();
        let source = fields.next()?;
        let target = fields.next()?.replace("\\040", " ");
        (source.starts_with("/dev/")
            && (target == "/mnt/udisk" || target.starts_with("/run/media/")))
        .then_some(PathBuf::from(target))
    });
    candidates
        .clone()
        .find(|path| path == Path::new("/mnt/udisk"))
        .or_else(|| candidates.next())
}

async fn export_logs(State(app): State<Arc<App>>) -> Result<Json<Value>, ApiError> {
    let _guard = app.acquire()?;
    let mounts = fs::read_to_string("/proc/mounts").map_err(log_error)?;
    let destination = usb_mount(&mounts)
        .ok_or_else(|| ApiError::conflict("usb", "Insert a mounted USB drive before exporting"))?;
    let usb_root = destination.clone();
    let logs = app.logs.clone();
    let folder = tokio::task::spawn_blocking(move || logs.export_to(&destination))
        .await
        .map_err(|e| ApiError::conflict("log", e.to_string()))?
        .map_err(log_error)?;
    let relative = folder
        .strip_prefix(&usb_root)
        .map_err(|error| ApiError::conflict("log", error.to_string()))?;
    Ok(Json(json!({"path":folder,"relativePath":relative})))
}

async fn clear_logs(State(app): State<Arc<App>>) -> Result<Json<Value>, ApiError> {
    let _guard = app.acquire()?;
    let logs = app.logs.clone();
    tokio::task::spawn_blocking(move || logs.clear())
        .await
        .map_err(|e| ApiError::conflict("log", e.to_string()))?
        .map_err(log_error)?;
    Ok(Json(json!({"cleared":true})))
}

async fn state(State(app): State<Arc<App>>) -> Json<Value> {
    let mut snapshot = app.device.snapshot();
    snapshot["contactActive"] = json!(app.active.load(Ordering::SeqCst));
    snapshot["recoveryFailed"] = json!(app.recovery_failed.load(Ordering::SeqCst));
    Json(snapshot)
}
async fn settings(State(app): State<Arc<App>>) -> Json<Map<String, Value>> {
    Json(app.settings.lock().await.snapshot())
}
async fn settings_schema() -> Json<Map<String, Value>> {
    Json(crate::settings::schema())
}
async fn update_settings(
    State(app): State<Arc<App>>,
    ApiJson(patch): ApiJson<Map<String, Value>>,
) -> Result<Json<Map<String, Value>>, ApiError> {
    let mut store = app.settings.lock().await;
    store.update(patch).map_err(|e| {
        let status = if matches!(e, SettingsError::Invalid(_)) {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };
        ApiError(status, "settings", e.to_string())
    })?;
    Ok(Json(store.snapshot()))
}
async fn ready(State(app): State<Arc<App>>) -> Response {
    match &app.ready_token {
        Some(token) => token.clone().into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Deserialize)]
struct UpdateQuery {
    #[serde(default)]
    development: bool,
}

async fn check_update(
    State(app): State<Arc<App>>,
    Query(query): Query<UpdateQuery>,
) -> Result<Json<crate::updates::CheckResult>, ApiError> {
    Ok(Json(app.updates.check(query.development).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InstallUpdateRequest {
    token: String,
}

async fn install_update(
    State(app): State<Arc<App>>,
    ApiJson(body): ApiJson<InstallUpdateRequest>,
) -> Result<Json<crate::updates::InstallStarted>, ApiError> {
    let _guard = app.acquire()?;
    let state = app.device.state();
    if !state.connected || !state.ready || !state.spindle_stopped {
        return Err(ApiError::conflict(
            "machine_busy",
            "The machine must be idle with the spindle stopped before updating",
        ));
    }
    Ok(Json(
        app.updates
            .install(&body.token, || {
                let state = app.device.state();
                state.connected && state.ready && state.spindle_stopped
            })
            .await?,
    ))
}

#[derive(Deserialize)]
struct UpdateStatusQuery {
    id: String,
}

async fn update_status(
    State(app): State<Arc<App>>,
    Query(query): Query<UpdateStatusQuery>,
) -> Result<Json<crate::updates::InstallStatus>, ApiError> {
    Ok(Json(app.updates.status(&query.id).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActuatorRequest {
    extended: bool,
}
async fn actuator(
    State(app): State<Arc<App>>,
    ApiJson(body): ApiJson<ActuatorRequest>,
) -> Result<StatusCode, ApiError> {
    let guard = app.acquire()?;
    tokio::spawn(async move {
        let _guard = guard;
        app.device.set_extended(body.extended).await
    })
    .await
    .map_err(|e| ApiError::conflict("actuator", e.to_string()))??;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WcsRequest {
    wcs: i32,
}
async fn wcs(
    State(app): State<Arc<App>>,
    ApiJson(body): ApiJson<WcsRequest>,
) -> Result<StatusCode, ApiError> {
    if !(54..=59).contains(&body.wcs) {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "wcs",
            "Invalid WCS".into(),
        ));
    }
    let guard = app.acquire()?;
    tokio::spawn(async move {
        let _guard = guard;
        app.device.select_wcs(body.wcs).await
    })
    .await
    .map_err(|e| ApiError::conflict("wcs", e.to_string()))??;
    Ok(StatusCode::NO_CONTENT)
}

async fn review(
    State(app): State<Arc<App>>,
    ApiJson(config): ApiJson<RoutineConfig>,
) -> Result<Json<Value>, ApiError> {
    let _guard = app.acquire()?;
    let session = app.device.session_id();
    app.device.configure(&config)?;
    let modes = pimprobe_core::query_modes(&app.device).await?;
    let mut state = app.device.state();
    state.modes = modes;
    let plan = pimprobe_core::review(state, config)?;
    let program = plan.program();
    if app.device.session_id() != session {
        return Err(ApiError::conflict(
            "review",
            "Controller reconnected; review the routine again",
        ));
    }
    let id = app.reviews.lock().await.put(plan, session);
    Ok(Json(
        json!({"id":id,"program":program,"simulated":app.device.is_mock()}),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Token {
    id: String,
}

struct RunStream {
    receiver: mpsc::Receiver<Result<Bytes, Infallible>>,
    cancel: CancellationToken,
}
impl Stream for RunStream {
    type Item = Result<Bytes, Infallible>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}
impl Drop for RunStream {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
fn encoded(value: Value) -> Result<Bytes, Infallible> {
    Ok(Bytes::from(value.to_string() + "\n"))
}

fn queue_progress(
    sender: &mpsc::Sender<Result<Bytes, Infallible>>,
    cancel: &CancellationToken,
    event: Value,
) {
    // Keep one slot for the terminal event, even if the client stops reading.
    if sender.capacity() <= 1 || sender.try_send(encoded(event)).is_err() {
        cancel.cancel();
    }
}

async fn repeatability(
    State(app): State<Arc<App>>,
    ApiJson(options): ApiJson<RepeatabilityOptions>,
) -> Result<Response, ApiError> {
    let guard = app.acquire()?;
    let values = app.settings.lock().await.snapshot();
    let settings = RepeatabilitySettings {
        diameter: values["probeDiameter"].as_f64().unwrap(),
        retract: values["retractDistance"].as_f64().unwrap(),
        positioning_feed: values["positioningFeed"].as_f64().unwrap(),
        coarse_feed: values["coarseFeed"].as_f64().unwrap(),
        fine_feed: values["fineFeed"].as_f64().unwrap(),
    };
    options.validate()?;
    settings.validate()?;
    app.device.refresh_probe_reference().await?;
    pimprobe_core::check_repeatability(&app.device, &options, settings)?;
    app.device.configure_repeatability(settings.diameter)?;
    let run_id = uuid::Uuid::new_v4().to_string();
    app.logs
        .start(
            &run_id,
            "Probe repeatability",
            "repeatability",
            json!({"options":options,"settings":values}),
            json!({"controller":app.device.state(),"software":software_info()}),
        )
        .map_err(log_error)?;
    let cancel = app.shutdown.child_token();
    let run_cancel = cancel.clone();
    let (sender, receiver) = mpsc::channel(256);
    app.active.store(true, Ordering::SeqCst);
    tokio::spawn(async move {
        let _guard = guard;
        let mut report = RepeatabilityReport::default();
        let progress_id = run_id.clone();
        let progress_logs = app.logs.clone();
        let result = pimprobe_core::run_repeatability(
            &app.device, &options, settings, &mut report, run_cancel.clone(), |event| {
                let event = match event {
                    RepeatabilityEvent::Progress(progress) => json!({"type":"progress", "progress":progress}),
                    RepeatabilityEvent::Measurement { repetition, axis, value, statistics } =>
                        json!({"type":"measurement", "repetition":repetition, "axis":axis, "value":value, "statistics":statistics}),
                };
                let name = if event["progress"]["kind"] == "contact" {
                    "contact"
                } else {
                    event["type"].as_str().unwrap_or("progress")
                };
                let data = if name == "contact" {
                    json!({"measurement":event["progress"]["measurement"],"contact":event["progress"]["contact"]})
                } else {
                    event.clone()
                };
                log_warning(progress_logs.trace(&progress_id, name, data));
                queue_progress(&sender, &run_cancel, event);
            },
        ).await;
        let event = match result {
            Ok(()) => {
                log_warning(
                    app.logs
                        .finish(&run_id, "success", Some(json!(report)), None),
                );
                json!({"type":"result", "result":report})
            }
            Err(error) => {
                app.recover(&error).await;
                log_warning(app.logs.finish(
                    &run_id,
                    if matches!(error, Error::Cancelled) {
                        "cancelled"
                    } else {
                        "failed"
                    },
                    Some(json!(report)),
                    Some(error.to_string()),
                ));
                json!({"type":"error", "code":error_code(&error), "message":error.to_string(), "result":report})
            }
        };
        log_warning(
            app.logs
                .trace(&run_id, "end_state", json!(app.device.state())),
        );
        app.active.store(false, Ordering::SeqCst);
        let _ = sender.try_send(encoded(event));
    });
    Ok((
        [
            (header::CONTENT_TYPE, "application/x-ndjson"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        Body::from_stream(RunStream { receiver, cancel }),
    )
        .into_response())
}

async fn run(
    State(app): State<Arc<App>>,
    ApiJson(token): ApiJson<Token>,
) -> Result<Response, ApiError> {
    start_motion(app, token, Motion::Run).await
}

async fn return_start(
    State(app): State<Arc<App>>,
    ApiJson(token): ApiJson<Token>,
) -> Result<Response, ApiError> {
    start_motion(app, token, Motion::Return).await
}

#[derive(Clone, Copy)]
enum Motion {
    Run,
    Return,
    Measured(f64),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MeasuredRequest {
    id: String,
    safe_z_offset: f64,
}

async fn go_to_measured(
    State(app): State<Arc<App>>,
    ApiJson(request): ApiJson<MeasuredRequest>,
) -> Result<Response, ApiError> {
    start_motion(
        app,
        Token { id: request.id },
        Motion::Measured(request.safe_z_offset),
    )
    .await
}

async fn start_motion(app: Arc<App>, token: Token, motion: Motion) -> Result<Response, ApiError> {
    let guard = app.acquire()?;
    let session = app.device.session_id();
    let (plan, completed) = match motion {
        Motion::Run => (
            app.reviews
                .lock()
                .await
                .take(&token.id, session)
                .ok_or_else(|| {
                    ApiError::conflict("review", "Review expired; review the routine again")
                })?,
            None,
        ),
        Motion::Return | Motion::Measured(_) => {
            let (plan, result) = app
                .reviews
                .lock()
                .await
                .take_completed(&token.id, session)
                .ok_or_else(|| ApiError::conflict("result", "No completed result available"))?;
            let already_done = match motion {
                Motion::Return => result.returned,
                Motion::Measured(_) => result.positioned,
                Motion::Run => false,
            };
            if already_done {
                app.reviews
                    .lock()
                    .await
                    .finish(token.id, session, plan, result);
                return Err(ApiError::conflict(
                    "result",
                    "Positioning action already completed",
                ));
            }
            (plan, Some(result))
        }
    };
    if matches!(motion, Motion::Run) {
        app.logs
            .start(
                &token.id,
                &routine_label(&plan.config),
                "routine",
                json!(plan.config),
                json!({"controller":plan.start,"software":software_info()}),
            )
            .map_err(log_error)?;
    } else {
        log_warning(app.logs.trace(
            &token.id,
            "action_start",
            json!({"action":match motion { Motion::Return => "return_to_start", Motion::Measured(_) => "go_to_measured", Motion::Run => unreachable!() }}),
        ));
    }
    let cancel = app.shutdown.child_token();
    let run_cancel = cancel.clone();
    let (sender, receiver) = mpsc::channel(256);
    app.active.store(true, Ordering::SeqCst);
    tokio::spawn(async move {
        let _guard = guard;
        let started = Instant::now();
        let run_id = token.id.clone();
        let progress_sender = sender.clone();
        let progress_cancel = run_cancel.clone();
        let progress_logs = app.logs.clone();
        let progress_id = run_id.clone();
        let observe = move |progress: pimprobe_core::Progress| {
            let event = json!({"type":"progress","elapsedMs":started.elapsed().as_millis(),"progress":progress});
            let name = if event["progress"]["kind"] == "contact" {
                "contact"
            } else {
                "progress"
            };
            let data = if name == "contact" {
                json!({"elapsedMs":event["elapsedMs"],"measurement":progress.measurement,"contact":progress.contact})
            } else {
                event.clone()
            };
            log_warning(progress_logs.trace(&progress_id, name, data));
            queue_progress(&progress_sender, &progress_cancel, event);
        };
        let result = match (motion, completed) {
            (Motion::Run, None) => {
                pimprobe_core::run(
                    &app.device,
                    &plan,
                    TimingPolicy::default(),
                    run_cancel,
                    observe,
                )
                .await
            }
            (Motion::Return, Some(result)) => {
                pimprobe_core::return_to_start(
                    &app.device,
                    &plan,
                    &result,
                    TimingPolicy::default(),
                    run_cancel,
                    observe,
                )
                .await
            }
            (Motion::Measured(clearance), Some(result)) => {
                pimprobe_core::go_to_measured(
                    &app.device,
                    &plan,
                    &result,
                    clearance,
                    TimingPolicy::default(),
                    run_cancel,
                    observe,
                )
                .await
            }
            _ => unreachable!(),
        };
        let event = match result {
            Ok(result) => {
                match motion {
                    Motion::Run => {
                        log_warning(
                            app.logs
                                .finish(&run_id, "success", Some(json!(result)), None),
                        )
                    }
                    Motion::Return => log_warning(app.logs.action(
                        &run_id,
                        "return_to_start",
                        json!({"result":result}),
                    )),
                    Motion::Measured(clearance) => log_warning(app.logs.action(
                        &run_id,
                        "go_to_measured",
                        json!({"safeZOffset":clearance,"result":result}),
                    )),
                }
                app.reviews
                    .lock()
                    .await
                    .finish(token.id, session, plan, result.clone());
                json!({"type":"result","result":result})
            }
            Err(mut error) => {
                app.recover(&error).await;
                let state = app.device.state();
                if matches!(motion, Motion::Measured(_))
                    && state.connected
                    && state.ready
                    && !state.motion_blocked
                    && state.modes != plan.start.modes
                    && let Err(recovery) =
                        pimprobe_core::restore_modes(&app.device, plan.start.modes).await
                {
                    error = Error::Recovery {
                        cause: Box::new(error),
                        recovery: Box::new(recovery),
                    };
                }
                if matches!(motion, Motion::Run) {
                    log_warning(app.logs.finish(
                        &run_id,
                        if matches!(error, Error::Cancelled) {
                            "cancelled"
                        } else {
                            "failed"
                        },
                        None,
                        Some(error.to_string()),
                    ));
                } else {
                    log_warning(app.logs.action(
                        &run_id,
                        "action_failed",
                        json!({"message":error.to_string()}),
                    ));
                }
                json!({"type":"error","code":error_code(&error),"message":error.to_string()})
            }
        };
        log_warning(
            app.logs
                .trace(&run_id, "end_state", json!(app.device.state())),
        );
        app.active.store(false, Ordering::SeqCst);
        let _ = sender.try_send(encoded(event));
    });
    Ok((
        [
            (header::CONTENT_TYPE, "application/x-ndjson"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        Body::from_stream(RunStream { receiver, cancel }),
    )
        .into_response())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ZeroRequest {
    id: String,
    #[serde(default)]
    offsets: [f64; 3],
}

async fn zero(
    State(app): State<Arc<App>>,
    ApiJson(token): ApiJson<ZeroRequest>,
) -> Result<Json<Value>, ApiError> {
    let guard = app.acquire()?;
    let session = app.device.session_id();
    let (plan, result) = app
        .reviews
        .lock()
        .await
        .take_completed(&token.id, session)
        .ok_or_else(|| ApiError::conflict("zero", "No unzeroed result available"))?;
    if result.zeroed {
        app.reviews
            .lock()
            .await
            .finish(token.id, session, plan, result);
        return Err(ApiError::conflict("zero", "Work zero already set"));
    }
    let result = tokio::spawn(async move {
        let _guard = guard;
        let updated = pimprobe_core::zero_result(
            &app.device,
            &plan,
            &result,
            token.offsets,
            TimingPolicy::default(),
            app.shutdown.child_token(),
        )
        .await;
        if let Err(error) = &updated {
            app.recover(error).await;
            log_warning(app.logs.action(
                &token.id,
                "work_zero_failed",
                json!({"offsets":token.offsets,"message":error.to_string()}),
            ));
        } else if let Ok(result) = &updated {
            log_warning(app.logs.action(
                &token.id,
                "work_zero",
                json!({"offsets":token.offsets,"result":result}),
            ));
            app.reviews
                .lock()
                .await
                .finish(token.id, session, plan, result.clone());
        }
        updated
    })
    .await
    .map_err(|e| ApiError::conflict("zero", e.to_string()))??;
    Ok(Json(json!(result)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_mount_prefers_the_machine_mount_and_requires_a_block_device() {
        let mounts = "tmpfs /mnt/udisk tmpfs rw 0 0\n/dev/sdb1 /run/media/backup vfat rw 0 0\n/dev/sda1 /mnt/udisk vfat rw 0 0\n";
        assert_eq!(usb_mount(mounts), Some(PathBuf::from("/mnt/udisk")));
        assert_eq!(
            usb_mount("tmpfs /mnt/udisk tmpfs rw 0 0\n/dev/sdb1 /run/media/backup vfat rw 0 0\n"),
            Some(PathBuf::from("/run/media/backup"))
        );
        assert_eq!(usb_mount("tmpfs /mnt/udisk tmpfs rw 0 0\n"), None);
    }

    #[tokio::test]
    async fn slow_reader_always_has_room_for_terminal_event() {
        let (sender, mut receiver) = mpsc::channel(4);
        let cancel = CancellationToken::new();
        for _ in 0..10 {
            queue_progress(&sender, &cancel, json!({"type":"progress"}));
        }
        assert!(cancel.is_cancelled());
        sender.try_send(encoded(json!({"type":"error"}))).unwrap();
        drop(sender);
        let mut events = Vec::new();
        while let Some(event) = receiver.recv().await {
            events.push(event.unwrap());
        }
        assert_eq!(events.len(), 4);
        assert_eq!(
            serde_json::from_slice::<Value>(events.last().unwrap()).unwrap()["type"],
            "error"
        );
    }
}
