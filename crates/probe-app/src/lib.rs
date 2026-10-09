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

pub mod device;
pub mod history;
pub mod host;
pub mod operations;
mod results;
mod reviews;
mod rotary;
pub mod settings;
pub use operations::{MeasurementResult, OperationEvent};
pub use rotary::RotaryReview;

use device::{DeviceSnapshot, ProbeDevice};
use history::{HistoryEntry, Records};
use host::HostActions;
use pimprobe_core::{
    CancellationToken, Error, RepeatabilityEvent, RepeatabilityOptions, RepeatabilityReport,
    RoutineConfig, TimingPolicy,
};
use reviews::Reviews;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use settings::{Settings, SettingsError};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tokio::sync::{Mutex, OwnedMutexGuard, mpsc};

pub struct ProbeApp {
    pub device: Arc<dyn ProbeDevice>,
    settings: Mutex<Settings>,
    reviews: Mutex<Reviews>,
    rotary_reviews: Mutex<Reviews<pimprobe_core::RotaryPlan, pimprobe_core::RotaryResult>>,
    action: Arc<Mutex<()>>,
    active: AtomicBool,
    recovery_failed: AtomicBool,
    repeatability_stop: Mutex<Option<CancellationToken>>,
    shutdown: CancellationToken,
    host: Arc<dyn HostActions>,
    logs: Arc<Records>,
}

impl ProbeApp {
    pub fn new(
        device: Arc<dyn ProbeDevice>,
        settings: Settings,
        host: Arc<dyn HostActions>,
        logs: Records,
    ) -> Arc<Self> {
        Arc::new(Self {
            device,
            settings: Mutex::new(settings),
            reviews: Mutex::new(Reviews::default()),
            rotary_reviews: Mutex::new(Reviews::default()),
            action: Arc::new(Mutex::new(())),
            active: AtomicBool::new(false),
            recovery_failed: AtomicBool::new(false),
            repeatability_stop: Mutex::new(None),
            shutdown: CancellationToken::new(),
            host,
            logs: Arc::new(logs),
        })
    }

    pub fn acquire(&self) -> Result<OperationGuard, AppError> {
        if self.shutdown.is_cancelled() {
            return Err(AppError::conflict("shutdown", "Service is stopping"));
        }
        let guard = self
            .action
            .clone()
            .try_lock_owned()
            .map_err(|_| AppError::conflict("busy", "Another action is running"))?;
        if self.recovery_failed.load(Ordering::SeqCst) {
            return Err(AppError::conflict(
                "recovery",
                "Waiting for confirmation that motion has stopped",
            ));
        }
        let machine = self
            .device
            .try_acquire()
            .map_err(|error| AppError::conflict("busy", error.to_string()))?;
        Ok(OperationGuard {
            _local: guard,
            _machine: machine,
        })
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
            Error::Stopped
                | Error::Preflight(_)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Invalid,
    Conflict,
    Storage,
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct AppError {
    pub kind: ErrorKind,
    pub code: &'static str,
    pub message: String,
}
impl AppError {
    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Conflict,
            code,
            message: message.into(),
        }
    }
    pub fn invalid(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Invalid,
            code,
            message: message.into(),
        }
    }
    pub fn storage(code: &'static str, error: impl std::fmt::Display) -> Self {
        Self {
            kind: ErrorKind::Storage,
            code,
            message: error.to_string(),
        }
    }
}
impl From<Error> for AppError {
    fn from(error: Error) -> Self {
        Self::conflict(error_code(&error), error.to_string())
    }
}

pub struct OperationGuard {
    _local: OwnedMutexGuard<()>,
    _machine: device::ActionGuard,
}
fn error_code(error: &Error) -> &'static str {
    match error {
        Error::Stopped => "stopped",
        Error::MotionBlocked => "controller_alarm",
        Error::CoarseNoContact | Error::NoContact => "no_contact",
        Error::UnexpectedContact { .. } => "unexpected_contact",
        _ => "failed",
    }
}

fn log_error(error: std::io::Error) -> AppError {
    AppError::storage("log", error)
}

fn log_warning(result: std::io::Result<()>) {
    if let Err(error) = result {
        eprintln!("Probing log: {error}");
    }
}

fn persisted(event: OperationEvent, saved: std::io::Result<()>) -> OperationEvent {
    let Err(error) = saved else { return event };
    match event {
        OperationEvent::Result { result } => OperationEvent::Error {
            code: "log".into(),
            message: format!("Measurement completed, but its history could not be saved: {error}"),
            result: Some(result),
        },
        OperationEvent::Error {
            code,
            message,
            result,
        } => OperationEvent::Error {
            code,
            message: format!("{message}; history could not be saved: {error}"),
            result,
        },
        other => other,
    }
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

#[derive(Debug, Serialize)]
pub struct Review {
    pub id: String,
    pub program: Vec<String>,
    pub simulated: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Token {
    pub id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZeroRequest {
    pub id: String,
    #[serde(default)]
    pub offsets: [f64; 3],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultWcsRequest {
    pub id: String,
    pub wcs: i32,
}

#[derive(Clone, Copy)]
pub enum Motion {
    Run,
    Return,
    Measured(f64),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    #[serde(flatten)]
    pub device: DeviceSnapshot,
    pub contact_active: bool,
    pub recovery_failed: bool,
}

/// Dropping the receiving side cancels the operation. Its task keeps machine
/// ownership until it confirms that motion stopped.
pub struct Operation {
    receiver: mpsc::Receiver<OperationEvent>,
    cancel: CancellationToken,
}
impl Operation {
    pub fn poll_recv(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<OperationEvent>> {
        self.receiver.poll_recv(cx)
    }
    pub async fn recv(&mut self) -> Option<OperationEvent> {
        self.receiver.recv().await
    }
    pub fn cancel(&self) {
        self.cancel.cancel();
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

fn queue_progress(
    sender: &mpsc::Sender<OperationEvent>,
    cancel: &CancellationToken,
    event: OperationEvent,
) {
    // Reserve a slot for the terminal result when the client falls behind.
    if sender.capacity() <= 1 || sender.try_send(event).is_err() {
        cancel.cancel();
    }
}

impl ProbeApp {
    pub fn state(&self) -> AppState {
        AppState {
            device: self.device.snapshot(),
            contact_active: self.active.load(Ordering::SeqCst),
            recovery_failed: self.recovery_failed.load(Ordering::SeqCst),
        }
    }
    pub async fn settings(&self) -> Map<String, Value> {
        self.settings.lock().await.snapshot()
    }
    pub async fn update_settings(
        &self,
        patch: Map<String, Value>,
    ) -> Result<Map<String, Value>, AppError> {
        let feeds: Vec<_> = patch
            .iter()
            .filter(|(key, _)| {
                matches!(
                    key.as_str(),
                    "positioningFeed" | "coarseFeed" | "fineFeed" | "rotaryFeed"
                )
            })
            .map(|(key, value)| (key.as_str(), value.as_f64().unwrap_or(f64::NAN)))
            .collect();
        if !feeds.is_empty() {
            self.validate_feeds(&feeds).await?;
        }
        let mut settings = self.settings.lock().await;
        settings.update(patch).map_err(|e| match e {
            SettingsError::Invalid(_) => AppError::invalid("settings", e.to_string()),
            _ => AppError::storage("settings", e),
        })?;
        Ok(settings.snapshot())
    }
    pub async fn history(&self) -> Result<Vec<HistoryEntry>, AppError> {
        let logs = self.logs.clone();
        tokio::task::spawn_blocking(move || logs.history())
            .await
            .map_err(|e| AppError::conflict("log", e.to_string()))?
            .map_err(log_error)
    }
    pub async fn clear_logs(&self) -> Result<(), AppError> {
        let _guard = self.acquire()?;
        let logs = self.logs.clone();
        tokio::task::spawn_blocking(move || logs.clear())
            .await
            .map_err(|e| AppError::conflict("log", e.to_string()))?
            .map_err(log_error)
    }
    pub async fn export_logs(&self) -> Result<host::ExportResult, AppError> {
        let _guard = self.acquire()?;
        self.host.export_logs().await
    }
    pub async fn set_extended(self: &Arc<Self>, extended: bool) -> Result<(), AppError> {
        let guard = self.acquire()?;
        let app = self.clone();
        tokio::spawn(async move {
            let _guard = guard;
            app.device.set_probe(extended).await
        })
        .await
        .map_err(|e| AppError::conflict("actuator", e.to_string()))??;
        Ok(())
    }
    pub async fn select_wcs(self: &Arc<Self>, wcs: i32) -> Result<(), AppError> {
        if !(54..=59).contains(&wcs) {
            return Err(AppError::invalid("wcs", "Invalid WCS"));
        }
        let guard = self.acquire()?;
        let app = self.clone();
        tokio::spawn(async move {
            let _guard = guard;
            app.device.select_wcs(wcs).await
        })
        .await
        .map_err(|e| AppError::conflict("wcs", e.to_string()))??;
        Ok(())
    }

    pub async fn review(self: &Arc<Self>, config: RoutineConfig) -> Result<Review, AppError> {
        let app = self;
        let _guard = app.acquire()?;
        let session = app.device.session_id();
        app.validate_feeds(&[
            ("positioningFeed", config.positioning_feed),
            ("coarseFeed", config.coarse_feed),
            ("fineFeed", config.fine_feed),
        ])
        .await?;
        app.device.configure(&config)?;
        let modes = pimprobe_core::query_modes(app.device.as_ref()).await?;
        let mut state = app.device.state();
        state.modes = modes;
        let plan = pimprobe_core::review(state, config)?;
        let program = plan.program();
        if app.device.session_id() != session {
            return Err(AppError::conflict(
                "review",
                "Controller reconnected; review the routine again",
            ));
        }
        let id = app.reviews.lock().await.put(plan, session);
        Ok(Review {
            id,
            program,
            simulated: app.device.is_mock(),
        })
    }

    pub async fn stop_repeatability(&self) {
        if let Some(stop) = self.repeatability_stop.lock().await.as_ref() {
            stop.cancel();
        }
    }

    pub async fn repeatability(
        self: &Arc<Self>,
        options: RepeatabilityOptions,
    ) -> Result<Operation, AppError> {
        let app = self.clone();
        let guard = app.acquire()?;
        let values = app.settings.lock().await.snapshot();
        let settings = app.settings.lock().await.values().repeatability();
        app.validate_feeds(&[
            ("positioningFeed", settings.positioning_feed),
            ("coarseFeed", settings.coarse_feed),
            ("fineFeed", settings.fine_feed),
        ])
        .await?;
        options.validate()?;
        settings.validate()?;
        pimprobe_core::check_repeatability(app.device.as_ref(), &options, settings)?;
        app.device.configure_repeatability(settings.diameter)?;
        let run_id = uuid::Uuid::new_v4().to_string();
        app.logs
            .start(
                &run_id,
                "Probe repeatability",
                "repeatability",
                json!({"options":options,"settings":values}),
                json!({"controller":app.device.state(),"software":app.host.software_info()}),
            )
            .map_err(log_error)?;
        let cancel = app.shutdown.child_token();
        let run_cancel = cancel.clone();
        let stop = CancellationToken::new();
        *app.repeatability_stop.lock().await = Some(stop.clone());
        let (sender, receiver) = mpsc::channel(256);
        app.active.store(true, Ordering::SeqCst);
        tokio::spawn(async move {
            let _guard = guard;
            let mut report = RepeatabilityReport::default();
            let progress_id = run_id.clone();
            let progress_logs = app.logs.clone();
            let result = pimprobe_core::run_repeatability(
            app.device.as_ref(), &options, settings, &mut report, run_cancel.clone(), stop, |event| {
                let event = match event {
                    RepeatabilityEvent::Progress(progress) => OperationEvent::Progress { elapsed_ms:None, progress },
                    RepeatabilityEvent::Measurement { repetition, axis, value, statistics } =>
                        OperationEvent::Measurement { repetition, axis, value, statistics },
                };
                let encoded = json!(event);
                let name = if encoded["progress"]["kind"] == "contact" {
                    "contact"
                } else {
                    encoded["type"].as_str().unwrap_or("progress")
                };
                let data = if name == "contact" {
                    json!({"measurement":encoded["progress"]["measurement"],"contact":encoded["progress"]["contact"]})
                } else {
                    encoded.clone()
                };
                log_warning(progress_logs.trace(&progress_id, name, data));
                queue_progress(&sender, &run_cancel, event);
            },
        ).await;
            let event = match result {
                Ok(()) => {
                    let saved = app
                        .logs
                        .finish(&run_id, "success", Some(json!(report)), None);
                    persisted(
                        OperationEvent::Result {
                            result: MeasurementResult::Repeatability(report),
                        },
                        saved,
                    )
                }
                Err(error) => {
                    app.recover(&error).await;
                    let saved = app.logs.finish(
                        &run_id,
                        if matches!(error, Error::Stopped) {
                            "stopped"
                        } else if matches!(error, Error::Cancelled) {
                            "cancelled"
                        } else {
                            "failed"
                        },
                        Some(json!(report)),
                        if matches!(error, Error::Stopped) {
                            None
                        } else {
                            Some(error.to_string())
                        },
                    );
                    persisted(
                        OperationEvent::Error {
                            code: error_code(&error).into(),
                            message: error.to_string(),
                            result: Some(MeasurementResult::Repeatability(report)),
                        },
                        saved,
                    )
                }
            };
            log_warning(
                app.logs
                    .trace(&run_id, "end_state", json!(app.device.state())),
            );
            *app.repeatability_stop.lock().await = None;
            app.active.store(false, Ordering::SeqCst);
            let _ = sender.try_send(event);
        });
        Ok(Operation { receiver, cancel })
    }

    pub async fn start_motion(
        self: &Arc<Self>,
        token: Token,
        motion: Motion,
    ) -> Result<Operation, AppError> {
        let app = self.clone();
        let guard = app.acquire()?;
        let session = app.device.session_id();
        let (plan, completed) = match motion {
            Motion::Run => (
                app.reviews
                    .lock()
                    .await
                    .take(&token.id, session)
                    .ok_or_else(|| {
                        AppError::conflict("review", "Review expired; review the routine again")
                    })?,
                None,
            ),
            Motion::Return | Motion::Measured(_) => {
                let (plan, result) = app
                    .reviews
                    .lock()
                    .await
                    .take_completed(&token.id, session)
                    .ok_or_else(|| AppError::conflict("result", "No completed result available"))?;
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
                    return Err(AppError::conflict(
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
                    json!({"controller":plan.start,"software":app.host.software_info()}),
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
                let event = OperationEvent::Progress {
                    elapsed_ms: Some(started.elapsed().as_millis() as u64),
                    progress: progress.clone(),
                };
                let encoded = json!(event);
                let name = if encoded["progress"]["kind"] == "contact" {
                    "contact"
                } else {
                    "progress"
                };
                let data = if name == "contact" {
                    json!({"elapsedMs":encoded["elapsedMs"],"measurement":progress.measurement,"contact":progress.contact})
                } else {
                    encoded.clone()
                };
                log_warning(progress_logs.trace(&progress_id, name, data));
                queue_progress(&progress_sender, &progress_cancel, event);
            };
            let result = match (motion, completed) {
                (Motion::Run, None) => {
                    pimprobe_core::run(
                        app.device.as_ref(),
                        &plan,
                        TimingPolicy::default(),
                        run_cancel,
                        observe,
                    )
                    .await
                }
                (Motion::Return, Some(result)) => {
                    pimprobe_core::return_to_start(
                        app.device.as_ref(),
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
                        app.device.as_ref(),
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
                    let saved = match motion {
                        Motion::Run => {
                            app.logs
                                .finish(&run_id, "success", Some(json!(result)), None)
                        }
                        Motion::Return => {
                            app.logs
                                .action(&run_id, "return_to_start", json!({"result":result}))
                        }
                        Motion::Measured(clearance) => app.logs.action(
                            &run_id,
                            "go_to_measured",
                            json!({"safeZOffset":clearance,"result":result}),
                        ),
                    };
                    app.reviews
                        .lock()
                        .await
                        .finish(token.id, session, plan, result.clone());
                    persisted(
                        OperationEvent::Result {
                            result: MeasurementResult::Routine(result),
                        },
                        saved,
                    )
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
                            pimprobe_core::restore_modes(app.device.as_ref(), plan.start.modes)
                                .await
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
                    OperationEvent::Error {
                        code: error_code(&error).into(),
                        message: error.to_string(),
                        result: None,
                    }
                }
            };
            log_warning(
                app.logs
                    .trace(&run_id, "end_state", json!(app.device.state())),
            );
            app.active.store(false, Ordering::SeqCst);
            let _ = sender.try_send(event);
        });
        Ok(Operation { receiver, cancel })
    }

    pub async fn zero(
        self: &Arc<Self>,
        token: ZeroRequest,
    ) -> Result<pimprobe_core::RoutineResult, AppError> {
        let app = self.clone();
        let guard = app.acquire()?;
        let session = app.device.session_id();
        let (plan, result) = app
            .reviews
            .lock()
            .await
            .take_completed(&token.id, session)
            .ok_or_else(|| AppError::conflict("zero", "No unzeroed result available"))?;
        if result.zeroed {
            app.reviews
                .lock()
                .await
                .finish(token.id, session, plan, result);
            return Err(AppError::conflict("zero", "Work zero already set"));
        }
        let result = tokio::spawn(async move {
            let _guard = guard;
            let updated = pimprobe_core::zero_result(
                app.device.as_ref(),
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
                let saved = app.logs.action(
                    &token.id,
                    "work_zero",
                    json!({"offsets":token.offsets,"result":result}),
                );
                app.reviews
                    .lock()
                    .await
                    .finish(token.id, session, plan, result.clone());
                saved.map_err(|e| {
                    AppError::storage(
                        "log",
                        format!("Work zero was set, but its history could not be saved: {e}"),
                    )
                })?;
            }
            updated.map_err(AppError::from)
        })
        .await
        .map_err(|e| AppError::conflict("zero", e.to_string()))??;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn slow_reader_leaves_room_for_terminal_event() {
        let (sender, mut receiver) = mpsc::channel(4);
        let cancel = CancellationToken::new();
        for _ in 0..10 {
            queue_progress(
                &sender,
                &cancel,
                OperationEvent::Progress {
                    elapsed_ms: None,
                    progress: Default::default(),
                },
            );
        }
        assert!(cancel.is_cancelled());
        sender
            .try_send(OperationEvent::Error {
                code: "cancelled".into(),
                message: "Reader stopped consuming progress".into(),
                result: None,
            })
            .unwrap();
        drop(sender);
        let mut events = Vec::new();
        while let Some(event) = receiver.recv().await {
            events.push(event);
        }
        assert_eq!(events.len(), 4);
        assert!(matches!(events.last(), Some(OperationEvent::Error { .. })));
    }
}
