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

use super::*;
use pimprobe_core::{RotaryResult, RoutineResult, State};

pub(super) fn rebase_start(start: &mut State, state: &State) {
    start.wcs = state.wcs;
    start.work_position =
        std::array::from_fn(|i| start.position[i] - (state.position[i] - state.work_position[i]));
    start.wcs_origin = state.wcs_origin;
    start.wcs_rotation = state.wcs_rotation;
}

impl ProbeApp {
    pub async fn rotate_result(
        self: &Arc<Self>,
        request: Token,
    ) -> Result<RoutineResult, AppError> {
        let guard = self.acquire()?;
        let app = self.clone();
        tokio::spawn(async move {
            let _guard = guard;
            let session = app.device.session_id();
            let (mut plan, result) = app
                .reviews
                .lock()
                .await
                .take_completed(&request.id, session)
                .ok_or_else(|| AppError::conflict("result", "No completed result available"))?;
            let updated = pimprobe_core::rotate_angle_result(app.device.as_ref(), &result).await;
            let saved = updated.as_ref().unwrap_or(&result).clone();
            if updated.is_ok() {
                rebase_start(&mut plan.start, &app.device.state());
                log_warning(app.logs.action(
                    &request.id,
                    "apply_xy_alignment",
                    json!({"result":saved}),
                ));
            }
            app.reviews
                .lock()
                .await
                .finish(request.id, session, plan, saved);
            updated.map_err(Into::into)
        })
        .await
        .map_err(|error| AppError::conflict("rotation", error.to_string()))?
    }

    pub async fn select_result_wcs(
        self: &Arc<Self>,
        request: ResultWcsRequest,
    ) -> Result<RoutineResult, AppError> {
        if !(54..=59).contains(&request.wcs) {
            return Err(AppError::invalid(
                "wcs",
                "Select a work coordinate system from G54 to G59.",
            ));
        }
        let guard = self.acquire()?;
        let app = self.clone();
        tokio::spawn(async move {
            let _guard = guard;
            let session = app.device.session_id();
            let (mut plan, mut result) = app
                .reviews
                .lock()
                .await
                .take_completed(&request.id, session)
                .ok_or_else(|| AppError::conflict("result", "No completed result available"))?;
            if let Err(error) = app.device.select_wcs(request.wcs).await {
                app.reviews
                    .lock()
                    .await
                    .finish(request.id, session, plan, result);
                return Err(error.into());
            }
            let state = app.device.state();
            if result.wcs != state.wcs {
                result.zeroed = false;
                if let Some(angle) = &mut result.angle {
                    angle.rotation_applied = false;
                }
            }
            result.wcs = state.wcs;
            result.point = std::array::from_fn(|i| {
                result.machine_point[i].map(|v| v - (state.position[i] - state.work_position[i]))
            });
            plan.config.wcs = state.wcs;
            rebase_start(&mut plan.start, &state);
            log_warning(
                app.logs
                    .action(&request.id, "result_wcs", json!({"result":result})),
            );
            app.reviews
                .lock()
                .await
                .finish(request.id, session, plan, result.clone());
            Ok(result)
        })
        .await
        .map_err(|error| AppError::conflict("wcs", error.to_string()))?
    }

    pub async fn select_rotary_result_wcs(
        self: &Arc<Self>,
        request: ResultWcsRequest,
    ) -> Result<RotaryResult, AppError> {
        if !(54..=59).contains(&request.wcs) {
            return Err(AppError::invalid(
                "wcs",
                "Select a work coordinate system from G54 to G59.",
            ));
        }
        let guard = self.acquire()?;
        let app = self.clone();
        tokio::spawn(async move {
            let _guard = guard;
            let session = app.device.session_id();
            let (mut plan, mut result) = app
                .rotary_reviews
                .lock()
                .await
                .take_completed(&request.id, session)
                .ok_or_else(|| {
                    AppError::conflict("result", "No completed calibration available")
                })?;
            if let Err(error) = app.device.select_wcs(request.wcs).await {
                app.rotary_reviews
                    .lock()
                    .await
                    .finish(request.id, session, plan, result);
                return Err(error.into());
            }
            let state = app.device.state();
            if result.wcs != state.wcs {
                result.zeroed = false;
                result.rotation_applied = false;
            }
            result.wcs = state.wcs;
            rebase_start(&mut plan.start, &state);
            log_warning(
                app.logs
                    .action(&request.id, "result_wcs", json!({"result":result})),
            );
            app.rotary_reviews
                .lock()
                .await
                .finish(request.id, session, plan, result.clone());
            Ok(result)
        })
        .await
        .map_err(|error| AppError::conflict("wcs", error.to_string()))?
    }
}
