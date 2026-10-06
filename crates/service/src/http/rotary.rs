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

pub(super) async fn review(
    State(app): State<Arc<App>>,
    ApiJson(config): ApiJson<RotaryConfig>,
) -> Result<Json<Value>, ApiError> {
    let _guard = app.acquire()?;
    config.validate()?;
    let session = app.device.session_id();
    app.device.configure_rotary(&config);
    let state = pimprobe_core::query_rotary_state(&app.device).await?;
    let plan = pimprobe_core::review_rotary(state, config)?;
    let program = plan.program();
    if session != app.device.session_id() {
        return Err(ApiError::conflict(
            "review",
            "Controller reconnected; review again",
        ));
    }
    let id = app.rotary_reviews.lock().await.put(plan, session);
    Ok(Json(
        json!({"id":id,"program":program,"simulated":app.device.is_mock()}),
    ))
}

pub(super) async fn run(
    State(app): State<Arc<App>>,
    ApiJson(token): ApiJson<Token>,
) -> Result<Response, ApiError> {
    let guard = app.acquire()?;
    let session = app.device.session_id();
    let plan = app
        .rotary_reviews
        .lock()
        .await
        .take(&token.id, session)
        .ok_or_else(|| {
            ApiError::conflict("review", "Review expired; review the calibration again")
        })?;
    let mut config = json!(plan.config);
    config["wcs"] = json!(plan.start.wcs);
    app.logs
        .start(
            &token.id,
            plan.config.operation.label(),
            "rotary",
            config,
            json!({"controller":plan.start,"software":software_info()}),
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
        let outcome = pimprobe_core::run_rotary(&app.device, &plan, &mut result,
            run_cancel.clone(), |progress| {
                let event = json!({"type":"progress","elapsedMs":started.elapsed().as_millis(),"progress":progress});
                log_warning(logs.trace(&token.id, "progress", event.clone()));
                queue_progress(&sender, &run_cancel, event);
            }).await;
        let event = match outcome {
            Ok(()) => {
                log_warning(
                    app.logs
                        .finish(&token.id, "success", Some(json!(result)), None),
                );
                app.rotary_reviews.lock().await.finish(
                    token.id.clone(),
                    session,
                    plan,
                    result.clone(),
                );
                json!({"type":"result","result":result})
            }
            Err(mut error) => {
                app.recover(&error).await;
                if app.device.state().connected
                    && app.device.state().ready
                    && !app.device.state().motion_blocked
                {
                    match pimprobe_core::restore_rotary_state(&app.device, &plan).await {
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
                log_warning(app.logs.finish(
                    &token.id,
                    if matches!(error, Error::Cancelled) {
                        "cancelled"
                    } else {
                        "failed"
                    },
                    Some(json!(result)),
                    Some(error.to_string()),
                ));
                json!({"type":"error","code":error_code(&error),"message":error.to_string(),"result":result})
            }
        };
        log_warning(
            app.logs
                .trace(&token.id, "end_state", json!(app.device.state())),
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

pub(super) async fn zero(
    State(app): State<Arc<App>>,
    ApiJson(token): ApiJson<Token>,
) -> Result<Json<Value>, ApiError> {
    apply(app, token, false).await
}
pub(super) async fn rotation(
    State(app): State<Arc<App>>,
    ApiJson(token): ApiJson<Token>,
) -> Result<Json<Value>, ApiError> {
    apply(app, token, true).await
}
async fn apply(app: Arc<App>, token: Token, rotate: bool) -> Result<Json<Value>, ApiError> {
    let guard = app.acquire()?;
    tokio::spawn(async move {
        let _guard = guard;
        let session = app.device.session_id();
        let (plan, mut result) = app
            .rotary_reviews
            .lock()
            .await
            .take_completed(&token.id, session)
            .ok_or_else(|| ApiError::conflict("result", "No completed calibration available"))?;
        let action = if rotate {
            "apply_xy_alignment"
        } else {
            "work_zero"
        };
        let outcome = if rotate {
            pimprobe_core::align_rotary(&app.device, &result).await
        } else {
            pimprobe_core::zero_rotary(&app.device, &result).await
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
        Ok(Json(json!(result)))
    })
    .await
    .map_err(|error| ApiError::conflict("result", error.to_string()))?
}
