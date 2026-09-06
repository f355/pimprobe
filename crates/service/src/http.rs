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
    reviews::Reviews,
    settings::{Settings, SettingsError},
};
use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use pimprobe_core::{CancellationToken, Controller, Error, RoutineConfig, TimingPolicy};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::{
    convert::Infallible,
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
}

impl App {
    pub fn new(device: Device, settings: Settings, ready_token: Option<String>) -> Arc<Self> {
        Arc::new(Self {
            device,
            settings: Mutex::new(settings),
            reviews: Mutex::new(Reviews::default()),
            action: Arc::new(Mutex::new(())),
            active: AtomicBool::new(false),
            recovery_failed: AtomicBool::new(false),
            shutdown: CancellationToken::new(),
            ready_token,
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
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"code":self.1,"message":self.2}))).into_response()
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
    Json(patch): Json<Map<String, Value>>,
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
#[serde(deny_unknown_fields)]
struct ActuatorRequest {
    extended: bool,
}
async fn actuator(
    State(app): State<Arc<App>>,
    Json(body): Json<ActuatorRequest>,
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
    Json(body): Json<WcsRequest>,
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
    Json(config): Json<RoutineConfig>,
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

async fn run(State(app): State<Arc<App>>, Json(token): Json<Token>) -> Result<Response, ApiError> {
    start_motion(app, token, false).await
}

async fn return_start(
    State(app): State<Arc<App>>,
    Json(token): Json<Token>,
) -> Result<Response, ApiError> {
    start_motion(app, token, true).await
}

async fn start_motion(app: Arc<App>, token: Token, returning: bool) -> Result<Response, ApiError> {
    let guard = app.acquire()?;
    let session = app.device.session_id();
    let (plan, completed) = if returning {
        let (plan, result) = app
            .reviews
            .lock()
            .await
            .take_completed(&token.id, session)
            .ok_or_else(|| ApiError::conflict("return", "No completed result available"))?;
        if result.returned {
            app.reviews
                .lock()
                .await
                .finish(token.id, session, plan, result);
            return Err(ApiError::conflict(
                "return",
                "Already returned to starting position",
            ));
        }
        (plan, Some(result))
    } else {
        (
            app.reviews
                .lock()
                .await
                .take(&token.id, session)
                .ok_or_else(|| {
                    ApiError::conflict("review", "Review expired; review the routine again")
                })?,
            None,
        )
    };
    let cancel = app.shutdown.child_token();
    let run_cancel = cancel.clone();
    let (sender, receiver) = mpsc::channel(256);
    app.active.store(true, Ordering::SeqCst);
    tokio::spawn(async move {
        let _guard = guard;
        let started = Instant::now();
        let progress_sender = sender.clone();
        let progress_cancel = run_cancel.clone();
        let observe = move |progress| {
            let event = json!({"type":"progress","elapsedMs":started.elapsed().as_millis(),"progress":progress});
            queue_progress(&progress_sender, &progress_cancel, event);
        };
        let result = if let Some(result) = completed {
            pimprobe_core::return_to_start(
                &app.device,
                &plan,
                &result,
                TimingPolicy::default(),
                run_cancel,
                observe,
            )
            .await
        } else {
            pimprobe_core::run(
                &app.device,
                &plan,
                TimingPolicy::default(),
                run_cancel,
                observe,
            )
            .await
        };
        let event = match result {
            Ok(result) => {
                app.reviews
                    .lock()
                    .await
                    .finish(token.id, session, plan, result.clone());
                json!({"type":"result","result":result})
            }
            Err(error) => {
                app.recover(&error).await;
                json!({"type":"error","code":error_code(&error),"message":error.to_string()})
            }
        };
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
    Json(token): Json<ZeroRequest>,
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
        } else if let Ok(result) = &updated {
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
