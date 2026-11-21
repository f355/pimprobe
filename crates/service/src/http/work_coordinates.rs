// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use super::{ApiError, ApiJson, App};
use crate::device::Device;
use axum::{Json, extract::State};
use pimprobe_core::{Controller, Position};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Presets {
    active: Option<i32>,
    available: bool,
    fourth_axis: bool,
    origins: BTreeMap<i32, Position>,
    positions: BTreeMap<i32, Position>,
    rotations: BTreeMap<i32, f64>,
}

fn rotate(mut point: Position, angle: f64) -> Position {
    let (sin, cos) = angle.to_radians().sin_cos();
    let [x, y, ..] = point;
    point[0] = cos * x - sin * y;
    point[1] = sin * x + cos * y;
    point
}

fn work(machine: Position, origin: Position, angle: f64, g92: Position, tlo: f64) -> Position {
    let p = rotate(std::array::from_fn(|i| machine[i] - origin[i]), -angle);
    std::array::from_fn(|i| p[i] - g92[i] - if i == 2 { tlo } else { 0. })
}

fn presets(device: &Device) -> Presets {
    let snapshot = device.snapshot();
    let status = snapshot.status.as_ref().filter(|s| s.complete);
    let fourth_axis = match device {
        Device::Machine(controller) => {
            controller.snapshot().status.and_then(|s| s.fourth_axis) == Some(true)
        }
        Device::Mock(_) => true,
    };
    Presets {
        active: status.map(|s| s.wcs),
        available: snapshot.connected
            && snapshot.status_fresh
            && !snapshot.actuator_pending
            && status.is_some_and(|s| s.mode == "Ready" && !s.motion_blocked),
        fourth_axis,
        positions: snapshot
            .wcs_origins
            .iter()
            .filter_map(|(&wcs, &origin)| {
                Some((
                    wcs,
                    work(
                        status?.machine_position,
                        origin,
                        snapshot.wcs_rotations.get(&wcs).copied().unwrap_or(0.),
                        snapshot.coordinate_offset,
                        snapshot.tool_length_offset,
                    ),
                ))
            })
            .collect(),
        origins: snapshot.wcs_origins,
        rotations: snapshot.wcs_rotations,
    }
}

pub(super) async fn get(State(app): State<Arc<App>>) -> Result<Json<Presets>, ApiError> {
    let current = presets(&app.device);
    // The plugin can accept a connection before CNC_Lab starts its serial queue.
    if current.available
        && (54..=59).any(|wcs| !current.origins.contains_key(&wcs))
        && let Ok(_guard) = app.probe.acquire()
    {
        app.device.read_coordinates().await.map_err(write_error)?;
        return Ok(Json(presets(&app.device)));
    }
    Ok(Json(current))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Edit {
    wcs: i32,
    axis: Option<usize>,
    value: f64,
    #[serde(default)]
    at_position: bool,
}

pub(super) async fn edit(
    State(app): State<Arc<App>>,
    ApiJson(edit): ApiJson<Edit>,
) -> Result<Json<Presets>, ApiError> {
    let invalid = |message| ApiError(axum::http::StatusCode::BAD_REQUEST, "wcs", message);
    let bound = match edit.axis {
        Some(0..=2) => 100000.,
        Some(3) => 1e9,
        _ => 360.,
    };
    if !(54..=59).contains(&edit.wcs)
        || edit.axis.is_some_and(|axis| axis >= 4)
        || !edit.value.is_finite()
        || edit.value.abs() > bound
        || (edit.axis.is_none() && edit.at_position)
    {
        return Err(invalid("Invalid work coordinate value".into()));
    }
    let guard = app.probe.acquire()?;
    tokio::spawn(async move {
        let _guard = guard;
        let current = presets(&app.device);
        if !current.available {
            return Err(ApiError::conflict("wcs", "Wait until the machine is idle."));
        }
        let rotation = current.rotations.get(&edit.wcs).copied();
        let mut axes = [None; 4];
        let mut angle = None;
        if let Some(axis) = edit.axis {
            if axis == 3 && !current.fourth_axis {
                return Err(ApiError::conflict(
                    "wcs",
                    "The fourth axis is not connected.",
                ));
            }
            if edit.at_position {
                let origin = current.origins.get(&edit.wcs).ok_or_else(|| {
                    ApiError::conflict("wcs", "The machine has not reported this origin.")
                })?;
                let mut shift = [0.; 4];
                shift[axis] = current.positions[&edit.wcs][axis] - edit.value;
                let shift = rotate(shift, rotation.unwrap_or(0.));
                for i in 0..4 {
                    if i == axis || (axis < 2 && i < 2 && rotation.unwrap_or(0.) != 0.) {
                        axes[i] = Some(origin[i] + shift[i]);
                    }
                }
            } else {
                axes[axis] = Some(edit.value);
            }
        } else {
            if rotation.is_none() {
                return Err(ApiError::conflict(
                    "wcs",
                    "This firmware does not support work coordinate rotation.",
                ));
            }
            angle = Some(edit.value);
        }
        // Axis values from the editor are always mm/degrees, regardless of the active units.
        let modes = app.device.read_modes().await.map_err(write_error)?;
        if modes.units == 20 {
            app.device
                .write_modes(pimprobe_core::Modes { units: 21, ..modes }, None)
                .await
                .map_err(write_error)?;
        }
        let result = app.device.write_coordinates(edit.wcs, axes, angle).await;
        let restored = if modes.units == 20 {
            app.device.write_modes(modes, None).await
        } else {
            Ok(())
        };
        result.and(restored).map_err(write_error)?;
        Ok(Json(presets(&app.device)))
    })
    .await
    .map_err(|error| ApiError::conflict("wcs", error.to_string()))?
}

fn write_error(error: pimprobe_core::Error) -> ApiError {
    ApiError::conflict("wcs", error.to_string())
}
