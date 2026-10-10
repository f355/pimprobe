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

use crate::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AngleMeasurement {
    pub plane: String,
    pub degrees: f64,
    pub difference: f64,
    pub spacing: f64,
    pub touches: [Position; 2],
    pub rotation_supported: bool,
    pub rotation_applied: bool,
}

impl RoutineConfig {
    /// Measuring axis, travel along the face, and search direction.
    pub fn angle_axes(&self) -> (Axis, Axis, i32) {
        match self.feature.as_str() {
            "x-plus" => (Axis::X, Axis::Y, 1),
            "x-minus" => (Axis::X, Axis::Y, -1),
            "y-plus" => (Axis::Y, Axis::X, 1),
            "y-minus" => (Axis::Y, Axis::X, -1),
            "z-x" => (Axis::Z, Axis::X, -1),
            "z-y" => (Axis::Z, Axis::Y, -1),
            _ => unreachable!("validated angle operation"),
        }
    }
}

impl AngleMeasurement {
    pub fn from_touches(
        config: &RoutineConfig,
        touches: [Position; 2],
        community: bool,
    ) -> Result<Self, Error> {
        let (measured, along, _) = config.angle_axes();
        let spacing = touches[1][along.index()] - touches[0][along.index()];
        let difference = touches[1][measured.index()] - touches[0][measured.index()];
        if !touches.into_iter().all(finite) || spacing.abs() < 0.1 {
            return Err(Error::Compensation(
                "The two touch points are too close to measure an angle.".into(),
            ));
        }
        Ok(Self {
            plane: if measured == Axis::Z {
                format!("{along}/Z")
            } else {
                "X/Y".into()
            },
            degrees: (difference / spacing).atan().to_degrees()
                * if measured == Axis::X { -1. } else { 1. },
            difference,
            spacing,
            touches,
            rotation_supported: community && measured != Axis::Z,
            rotation_applied: false,
        })
    }
}

pub async fn rotate_angle_result<C: Controller + ?Sized>(
    c: &C,
    result: &RoutineResult,
) -> Result<RoutineResult, Error> {
    preflight_machine(&c.state())?;
    let angle = result
        .angle
        .as_ref()
        .filter(|angle| angle.plane == "X/Y")
        .ok_or_else(|| Error::Preflight("This measurement has no X/Y angle.".into()))?;
    crate::rotary::write_rotary_rotation(c, result.wcs, angle.degrees).await?;
    let mut updated = result.clone();
    updated.angle.as_mut().unwrap().rotation_applied = true;
    Ok(updated)
}
