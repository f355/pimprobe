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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryResult {
    pub entry: HistoryEntry,
    pub result: MeasurementResult,
    pub can_apply: bool,
}

pub enum HistoryAction {
    Wcs(i32),
    Zero([f64; 3]),
    Rotation,
}

impl ProbeApp {
    pub async fn open_history_result(&self, id: &str) -> Result<HistoryResult, AppError> {
        let entry = self
            .history()
            .await?
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| AppError::conflict("result", "Probing history entry not found"))?;
        let value = entry
            .result
            .clone()
            .ok_or_else(|| AppError::conflict("result", "This attempt has no measurement"))?;
        let mut result = match entry.category.as_str() {
            "routine" => MeasurementResult::Routine(
                serde_json::from_value(value).map_err(|e| AppError::storage("result", e))?,
            ),
            "rotary" => MeasurementResult::Rotary(
                serde_json::from_value(value).map_err(|e| AppError::storage("result", e))?,
            ),
            _ => {
                return Err(AppError::invalid(
                    "result",
                    "This history entry has no work-zero result",
                ));
            }
        };
        let state = self.device.state();
        // Reopening starts a fresh work-zero action in the active WCS.
        let wcs = if (54..=59).contains(&state.wcs) {
            state.wcs
        } else {
            entry.config["wcs"].as_i64().unwrap_or(54) as i32
        };
        let complete = match &mut result {
            MeasurementResult::Routine(routine) => {
                routine.wcs = wcs;
                routine.zeroed = false;
                routine.point = std::array::from_fn(|i| {
                    routine.machine_point[i]
                        .map(|v| v - (state.position[i] - state.work_position[i]))
                });
                !routine.axes.is_empty()
                    && routine.axes.iter().all(|axis| {
                        ["X", "Y", "Z"]
                            .iter()
                            .position(|a| a == axis)
                            .is_some_and(|i| routine.machine_point[i].is_some_and(f64::is_finite))
                    })
            }
            MeasurementResult::Rotary(rotary) => {
                rotary.wcs = wcs;
                rotary.zeroed = false;
                rotary.rotation_applied = false;
                rotary
                    .level
                    .as_ref()
                    .map_or(rotary.stations.len() == 2, |level| level.touches.len() == 2)
            }
            MeasurementResult::Repeatability(_) => unreachable!(),
        };
        let opened = HistoryResult {
            can_apply: entry.status == "success" && complete,
            entry,
            result,
        };
        *self.history_result.lock().await = Some(opened.clone());
        Ok(opened)
    }

    pub async fn apply_history_result(
        self: &Arc<Self>,
        id: String,
        action: HistoryAction,
    ) -> Result<MeasurementResult, AppError> {
        if let HistoryAction::Wcs(wcs) = action
            && !(54..=59).contains(&wcs)
        {
            return Err(AppError::invalid("wcs", "Invalid WCS"));
        }
        let guard = self.acquire()?;
        let app = self.clone();
        tokio::spawn(async move {
            let _guard = guard;
            let mut opened = app
                .history_result
                .lock()
                .await
                .as_ref()
                .filter(|opened| opened.entry.id == id)
                .cloned()
                .ok_or_else(|| AppError::conflict("result", "Open the history result first"))?;
            if !opened.can_apply {
                return Err(AppError::conflict(
                    "result",
                    "This attempt has no completed measurement",
                ));
            }
            let name = match action {
                HistoryAction::Wcs(wcs) => {
                    app.device.select_wcs(wcs).await?;
                    let state = app.device.state();
                    match &mut opened.result {
                        MeasurementResult::Routine(result) => {
                            if result.wcs != wcs {
                                result.zeroed = false;
                            }
                            result.wcs = wcs;
                            result.point = std::array::from_fn(|i| {
                                result.machine_point[i]
                                    .map(|v| v - (state.position[i] - state.work_position[i]))
                            });
                        }
                        MeasurementResult::Rotary(result) => {
                            if result.wcs != wcs {
                                result.zeroed = false;
                                result.rotation_applied = false;
                            }
                            result.wcs = wcs;
                        }
                        MeasurementResult::Repeatability(_) => unreachable!(),
                    }
                    "result_wcs"
                }
                HistoryAction::Zero(offsets) => {
                    match &mut opened.result {
                        MeasurementResult::Routine(result) => {
                            *result = pimprobe_core::zero_recorded_result(
                                app.device.as_ref(),
                                result,
                                offsets,
                            )
                            .await?;
                        }
                        MeasurementResult::Rotary(result) => {
                            pimprobe_core::zero_rotary(app.device.as_ref(), result).await?;
                            result.zeroed = true;
                        }
                        MeasurementResult::Repeatability(_) => unreachable!(),
                    }
                    "work_zero"
                }
                HistoryAction::Rotation => {
                    let MeasurementResult::Rotary(result) = &mut opened.result else {
                        return Err(AppError::invalid(
                            "result",
                            "This measurement has no rotary alignment",
                        ));
                    };
                    pimprobe_core::align_rotary(app.device.as_ref(), result).await?;
                    result.rotation_applied = true;
                    "apply_xy_alignment"
                }
            };
            let mut data = json!({"result":opened.result});
            if let HistoryAction::Zero(offsets) = action {
                data["offsets"] = json!(offsets);
            }
            let result = opened.result.clone();
            *app.history_result.lock().await = Some(opened);
            app.logs.action(&id, name, data).map_err(|error| {
                AppError::storage(
                    "log",
                    format!("Action completed, but its history could not be saved: {error}"),
                )
            })?;
            Ok(result)
        })
        .await
        .map_err(|error| AppError::conflict("result", error.to_string()))?
    }
}
