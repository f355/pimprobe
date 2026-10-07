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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use super::*;
use pimprobe_core::{RotaryConfig, RotaryResult};

#[derive(Debug, Serialize)]
pub struct RotaryReview {
    pub id: String,
    pub program: String,
    pub simulated: bool,
}

impl ProbeApp {
    pub async fn review_rotary(
        self: &Arc<Self>,
        config: RotaryConfig,
    ) -> Result<RotaryReview, AppError> {
        let _guard = self.acquire()?;
        config.validate()?;
        self.validate_feeds(&[
            ("positioningFeed", config.positioning_feed),
            ("coarseFeed", config.coarse_feed),
            ("fineFeed", config.fine_feed),
            ("rotaryFeed", config.rotary_feed),
        ])
        .await?;
        let session = self.device.session_id();
        self.device.configure_rotary(&config)?;
        let state = pimprobe_core::query_rotary_state(self.device.as_ref()).await?;
        let plan = pimprobe_core::review_rotary(state, config)?;
        let program = plan.program();
        if session != self.device.session_id() {
            return Err(AppError::conflict(
                "review",
                "Controller reconnected; review again",
            ));
        }
        let id = self.rotary_reviews.lock().await.put(plan, session);
        Ok(RotaryReview {
            id,
            program,
            simulated: self.device.is_mock(),
        })
    }

    pub async fn run_rotary(self: &Arc<Self>, token: Token) -> Result<Operation, AppError> {
        let app = self.clone();
        let guard = app.acquire()?;
        let session = app.device.session_id();
        let plan = app
            .rotary_reviews
            .lock()
            .await
            .take(&token.id, session)
            .ok_or_else(|| {
                AppError::conflict("review", "Review expired; review the calibration again")
            })?;
        let mut config = json!(plan.config);
        config["wcs"] = json!(plan.start.wcs);
        app.logs
            .start(
                &token.id,
                plan.config.operation.label(),
                "rotary",
                config,
                json!({"controller":plan.start,"software":app.host.software_info()}),
            )
            .map_err(log_error)?;
        let cancel = app.shutdown.child_token();
        let run_cancel = cancel.clone();
        let (sender, receiver) = mpsc::channel(256);
        app.active.store(true, Ordering::SeqCst);
        tokio::spawn(async move {
            let _guard = guard;
            let mut result = RotaryResult::default();
            let logs = app.logs.clone();
            let started = Instant::now();
            let outcome = pimprobe_core::run_rotary(
                app.device.as_ref(),
                &plan,
                &mut result,
                run_cancel.clone(),
                |progress| {
                    let event = OperationEvent::Progress {
                        elapsed_ms: Some(started.elapsed().as_millis() as u64),
                        progress,
                    };
                    log_warning(logs.trace(&token.id, "progress", json!(event)));
                    queue_progress(&sender, &run_cancel, event);
                },
            )
            .await;
            let event = match outcome {
                Ok(()) => {
                    let saved = app
                        .logs
                        .finish(&token.id, "success", Some(json!(result)), None);
                    app.rotary_reviews.lock().await.finish(
                        token.id.clone(),
                        session,
                        plan,
                        result.clone(),
                    );
                    persisted(
                        OperationEvent::Result {
                            result: MeasurementResult::Rotary(result),
                        },
                        saved,
                    )
                }
                Err(mut error) => {
                    app.recover(&error).await;
                    let state = app.device.state();
                    if !matches!(error, Error::Preflight(_) | Error::InvalidConfig(_))
                        && state.connected
                        && state.ready
                        && !state.motion_blocked
                    {
                        match pimprobe_core::restore_rotary_state(app.device.as_ref(), &plan).await
                        {
                            Err(recovery) => {
                                error = Error::Recovery {
                                    cause: Box::new(error),
                                    recovery: Box::new(recovery),
                                }
                            }
                            Ok(()) => {
                                if let Error::Recovery { cause, .. } = error {
                                    error = *cause;
                                }
                            }
                        }
                    }
                    let saved = app.logs.finish(
                        &token.id,
                        if matches!(error, Error::Cancelled) {
                            "cancelled"
                        } else {
                            "failed"
                        },
                        Some(json!(result)),
                        Some(error.to_string()),
                    );
                    persisted(
                        OperationEvent::Error {
                            code: error_code(&error).into(),
                            message: error.to_string(),
                            result: Some(MeasurementResult::Rotary(result)),
                        },
                        saved,
                    )
                }
            };
            log_warning(
                app.logs
                    .trace(&token.id, "end_state", json!(app.device.state())),
            );
            app.active.store(false, Ordering::SeqCst);
            let _ = sender.try_send(event);
        });
        Ok(Operation { receiver, cancel })
    }

    pub async fn apply_rotary(
        self: &Arc<Self>,
        token: Token,
        rotate: bool,
    ) -> Result<RotaryResult, AppError> {
        let guard = self.acquire()?;
        let app = self.clone();
        tokio::spawn(async move {
            let _guard = guard;
            let session = app.device.session_id();
            let (plan, mut result) = app
                .rotary_reviews
                .lock()
                .await
                .take_completed(&token.id, session)
                .ok_or_else(|| {
                    AppError::conflict("result", "No completed calibration available")
                })?;
            let action = if rotate {
                "apply_xy_alignment"
            } else {
                "work_zero"
            };
            let outcome = if rotate {
                pimprobe_core::align_rotary(app.device.as_ref(), &result).await
            } else {
                pimprobe_core::zero_rotary(app.device.as_ref(), &result).await
            };
            if let Err(error) = outcome {
                log_warning(app.logs.action(
                    &token.id,
                    "action_failed",
                    json!({"action":action,"message":error.to_string()}),
                ));
                if matches!(
                    error,
                    Error::Preflight(_) | Error::InvalidConfig(_) | Error::MotionBlocked
                ) {
                    app.rotary_reviews
                        .lock()
                        .await
                        .finish(token.id, session, plan, result);
                }
                return Err(error.into());
            }
            if rotate {
                result.rotation_applied = true;
            } else {
                result.zeroed = true;
            }
            log_warning(app.logs.action(&token.id, action, json!({"result":result})));
            app.rotary_reviews
                .lock()
                .await
                .finish(token.id, session, plan, result.clone());
            Ok(result)
        })
        .await
        .map_err(|error| AppError::conflict("result", error.to_string()))?
    }
}
