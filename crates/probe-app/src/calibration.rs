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
use pimprobe_core::{Calibration, CalibrationOptions};
use std::time::Duration;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CalibrationRequest {
    pub id: String,
    pub options: CalibrationOptions,
    #[serde(default)]
    pub save: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedProbeCalibration {
    pub diameter: f64,
    pub offsets: [f64; 3],
}

impl ProbeApp {
    pub async fn calibrate(
        self: &Arc<Self>,
        request: CalibrationRequest,
    ) -> Result<Calibration, AppError> {
        let guard = if request.save {
            Some(self.acquire()?)
        } else {
            None
        };
        let app = self.clone();
        tokio::spawn(async move {
            let _guard = guard;
            let session = app.device.session_id();
            let (mut plan, result) = app
                .reviews
                .lock()
                .await
                .completed(&request.id, session)
                .ok_or_else(|| AppError::conflict("result", "No completed result available"))?;
            let state = app.device.state();
            let diameter = app.settings().await["probeDiameter"].as_f64().unwrap();
            let mut calibration =
                pimprobe_core::calibrate_probe(&plan, &result, &state, diameter, &request.options)?;
            if !request.save {
                return Ok(calibration);
            }
            if !request.options.ball && !request.options.offsets.contains(&true) {
                return Err(AppError::invalid(
                    "calibration",
                    "Select a calibration value to save.",
                ));
            }
            let before = SavedProbeCalibration {
                diameter,
                offsets: calibration.current_offsets,
            };
            let after = SavedProbeCalibration {
                diameter: calibration.diameter.unwrap_or(diameter),
                offsets: std::array::from_fn(|i| {
                    calibration.offsets[i].unwrap_or(before.offsets[i])
                }),
            };
            calibration.warning = app
                .save_probe_calibration(before, after, "Probe calibration")
                .await?;
            if let Some(diameter) = calibration.diameter {
                plan.config.diameter = diameter;
            }
            plan.start.probe_offset = app.device.state().probe_offset;
            plan.start.probe_z_calibration = app.device.state().probe_z_calibration;
            // Raw contacts and the measurement frame remain unchanged so another
            // calibration uses the same evidence rather than accumulating corrections.
            app.reviews.lock().await.finish(
                request.id.clone(),
                session,
                plan,
                calibration.result.clone(),
            );
            if let Err(error) = app.logs.action(
                &request.id,
                "probe_calibration",
                json!({"options":request.options,"result":calibration.result}),
            ) {
                calibration.warning = Some(format!(
                    "Calibration saved, but its history could not be saved: {error}"
                ));
            }
            Ok(calibration)
        })
        .await
        .map_err(|e| AppError::conflict("calibration", e.to_string()))?
    }

    pub async fn restore_probe_calibration(
        self: &Arc<Self>,
        token: Token,
    ) -> Result<ActionResult<SavedProbeCalibration>, AppError> {
        let guard = self.acquire()?;
        let app = self.clone();
        tokio::spawn(async move {
            let _guard = guard;
            let entry = app
                .history()
                .await?
                .into_iter()
                .find(|entry| entry.id == token.id && entry.category == "calibration")
                .ok_or_else(|| {
                    AppError::invalid("calibration", "Select a saved probe calibration.")
                })?;
            let saved: SavedProbeCalibration =
                serde_json::from_value(entry.config["before"].clone()).map_err(|_| {
                    AppError::invalid(
                        "calibration",
                        "This history entry has no saved calibration.",
                    )
                })?;
            let state = app.device.state();
            let before = SavedProbeCalibration {
                diameter: app.settings().await["probeDiameter"].as_f64().unwrap(),
                offsets: [
                    state.probe_offset[0],
                    state.probe_offset[1],
                    state.probe_z_calibration.ok_or_else(|| {
                        AppError::conflict(
                            "calibration",
                            "The saved probe Z offset is unavailable.",
                        )
                    })?,
                ],
            };
            let warning = app
                .save_probe_calibration(before, saved.clone(), "Restore probe calibration")
                .await?;
            Ok(ActionResult {
                result: saved,
                storage_warning: warning,
            })
        })
        .await
        .map_err(|e| AppError::conflict("calibration", e.to_string()))?
    }

    async fn save_probe_calibration(
        &self,
        before: SavedProbeCalibration,
        after: SavedProbeCalibration,
        label: &str,
    ) -> Result<Option<String>, AppError> {
        if !pimprobe_core::Parameter::Diameter
            .range()
            .contains(after.diameter)
            || after
                .offsets
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 1000.)
        {
            return Err(AppError::invalid(
                "calibration",
                "The saved probe calibration is invalid.",
            ));
        }
        let state = self.device.state();
        if !state.connected || !state.ready || state.motion_blocked {
            return Err(AppError::conflict(
                "busy",
                "Stop the machine and clear any alarm before saving calibration.",
            ));
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.logs
            .start(
                &id,
                label,
                "calibration",
                json!({"before":before,"after":after}),
                json!({}),
            )
            .map_err(|error| {
                AppError::storage(
                    "log",
                    format!(
                        "Could not save the previous calibration. No values were changed: {error}"
                    ),
                )
            })?;
        let saved: Result<(), AppError> = async {
            self.write_probe_offsets(std::array::from_fn(|i| {
                (before.offsets[i] != after.offsets[i]).then_some(after.offsets[i])
            }))
            .await?;
            if before.diameter != after.diameter {
                self.update_settings(
                    json!({"probeDiameter":after.diameter})
                        .as_object()
                        .unwrap()
                        .clone(),
                )
                .await?;
            }
            Ok(())
        }
        .await;
        // Completed measurements carry the calibration used for their contacts.
        *self.reviews.lock().await = Reviews::default();
        *self.rotary_reviews.lock().await = Reviews::default();
        *self.history_result.lock().await = None;
        let recorded = self.logs.finish(
            &id,
            if saved.is_ok() { "success" } else { "failed" },
            None,
            saved.as_ref().err().map(ToString::to_string),
        );
        if let Err(error) = saved {
            log_warning(recorded);
            return Err(AppError::conflict(
                "calibration_incomplete",
                format!(
                    "Calibration was not fully saved. Restore the previous calibration from Probe history before using the probe. {error}"
                ),
            ));
        }
        Ok(recorded.err().map(|error| {
            format!("Calibration saved, but its history could not be completed: {error}")
        }))
    }

    async fn write_probe_offsets(&self, offsets: [Option<f64>; 3]) -> Result<(), AppError> {
        if offsets.iter().all(Option::is_none) {
            return Ok(());
        }
        for (i, value) in offsets.iter().enumerate() {
            let Some(value) = value else {
                continue;
            };
            let mut events = self.device.subscribe();
            self.device.send(&format!("${}={value:.6}", 33 + i)).await?;
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    let event = events.recv().await.map_err(|_| Error::Disconnected)?;
                    if let Some(code) = event.controller_error {
                        return Err(Error::Controller(format!(
                            "The machine could not save probe calibration (error {code})."
                        )));
                    }
                    if event.acknowledged {
                        return Ok(());
                    }
                }
            })
            .await
            .map_err(|_| Error::Timeout)??;
        }
        // Standalone transports send commands without awaiting a reply. Read the
        // saved settings once so the next operation uses the new calibration.
        let mut events = self.device.subscribe();
        self.device.send("$$").await?;
        tokio::time::timeout(Duration::from_secs(5), async {
            let mut seen = offsets.map(|v| v.is_none());
            loop {
                let event = events.recv().await.map_err(|_| Error::Disconnected)?;
                if let Some(code) = event.controller_error {
                    return Err(Error::Controller(format!(
                        "The machine could not read probe calibration (error {code})."
                    )));
                }
                if let Some((key, _)) = event.setting {
                    if (33..=35).contains(&key) {
                        seen[(key - 33) as usize] = true;
                    }
                    if seen.iter().all(|v| *v) {
                        return Ok(());
                    }
                }
            }
        })
        .await
        .map_err(|_| Error::Timeout)??;
        Ok(())
    }
}
