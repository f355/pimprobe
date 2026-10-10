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

use crate::device::DeviceSnapshot;
use pimprobe_core::{Position, probe_z_offset};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DroPosition {
    pub machine_position: Position,
    pub work_position: Position,
}

#[derive(Debug, Default, Serialize)]
pub struct Coordinates {
    pub spindle: Option<DroPosition>,
    pub probe: Option<DroPosition>,
}

impl Coordinates {
    pub fn from_snapshot(snapshot: &DeviceSnapshot) -> Self {
        let Some(status) = snapshot.status.as_ref() else {
            return Self::default();
        };
        let mut machine = status.machine_position;
        machine[2] -= snapshot.tool_length_offset;
        let spindle = Some(DroPosition {
            machine_position: machine,
            work_position: status.work_position,
        });
        let probe = (|| {
            let x = *snapshot.settings.get(&33)?;
            let y = *snapshot.settings.get(&34)?;
            let z = -probe_z_offset(
                *snapshot.settings.get(&35)?,
                *snapshot.settings.get(&202)?,
                snapshot.tool_length_offset,
            );
            let (sin, cos) = snapshot
                .wcs_rotations
                .get(&status.wcs)
                .copied()
                .unwrap_or(0.)
                .to_radians()
                .sin_cos();
            let mut machine = status.machine_position;
            machine[0] += x;
            machine[1] += y;
            machine[2] += z;
            let mut work = status.work_position;
            work[0] += cos * x + sin * y;
            work[1] += -sin * x + cos * y;
            work[2] += z + snapshot.tool_length_offset;
            Some(DroPosition {
                machine_position: machine,
                work_position: work,
            })
        })();
        Self { spindle, probe }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::DeviceStatus;

    #[test]
    fn probe_coordinates_use_the_ets_reference_when_tool_compensation_is_cancelled() {
        let snapshot = DeviceSnapshot {
            status: Some(DeviceStatus {
                wcs: 54,
                machine_position: [0., 0., -30., 0.],
                work_position: [0., 0., 10., 0.],
                ..Default::default()
            }),
            settings: [(33, 0.), (34, 0.), (35, -50.), (202, -60.)].into(),
            tool_length_offset: 0.,
            ..Default::default()
        };
        let probe = Coordinates::from_snapshot(&snapshot).probe.unwrap();
        assert_eq!(probe.machine_position[2], -40.);
        assert_eq!(probe.work_position[2], 0.);
    }

    #[test]
    fn probe_and_tool_tips_use_their_own_offsets_in_machine_and_rotated_work_coordinates() {
        let snapshot = DeviceSnapshot {
            status: Some(DeviceStatus {
                wcs: 54,
                machine_position: [-100., -90., -80., 20.],
                work_position: [10., 20., 30., 40.],
                ..Default::default()
            }),
            settings: [(33, -50.), (34, -8.), (35, -25.), (202, 12.)].into(),
            wcs_rotations: [(54, 90.)].into(),
            tool_length_offset: 12.,
            ..Default::default()
        };
        let coordinates = Coordinates::from_snapshot(&snapshot);
        let tool = coordinates.spindle.unwrap();
        assert_eq!(tool.machine_position, [-100., -90., -92., 20.]);
        assert_eq!(tool.work_position, [10., 20., 30., 40.]);
        let probe = coordinates.probe.unwrap();
        assert_eq!(probe.machine_position, [-150., -98., -55., 20.]);
        for (actual, expected) in probe.work_position.into_iter().zip([2., 70., 67., 40.]) {
            assert!((actual - expected).abs() < 1e-9);
        }
        let changed_tool = DeviceSnapshot {
            tool_length_offset: 35.,
            settings: [(33, -50.), (34, -8.), (35, -25.), (202, 35.)].into(),
            ..snapshot
        };
        assert_eq!(
            Coordinates::from_snapshot(&changed_tool)
                .probe
                .unwrap()
                .machine_position,
            probe.machine_position
        );
    }

    #[test]
    fn probe_and_last_tool_read_zero_at_the_same_physical_surface() {
        let mut snapshot = DeviceSnapshot {
            status: Some(DeviceStatus {
                wcs: 54,
                machine_position: [-100., -90., -125., 0.],
                work_position: [0., 0., -37., 0.],
                ..Default::default()
            }),
            settings: [(33, 0.), (34, 0.), (35, -25.), (202, 12.)].into(),
            tool_length_offset: 12.,
            ..Default::default()
        };
        let probe = Coordinates::from_snapshot(&snapshot).probe.unwrap();
        assert_eq!(probe.work_position[2], 0.);
        let status = snapshot.status.as_mut().unwrap();
        status.machine_position[2] = -88.;
        status.work_position[2] = 0.;
        let tool = Coordinates::from_snapshot(&snapshot).spindle.unwrap();
        assert_eq!(tool.work_position[2], 0.);
        assert_eq!(tool.machine_position[2], probe.machine_position[2]);
    }

    #[test]
    fn available_frames_depend_on_status_and_calibration() {
        let mut snapshot = DeviceSnapshot::default();
        let missing = Coordinates::from_snapshot(&snapshot);
        assert!(missing.spindle.is_none());
        snapshot.status = Some(DeviceStatus::default());
        let partial = Coordinates::from_snapshot(&snapshot);
        assert!(partial.spindle.is_some());
        assert!(partial.probe.is_none());
    }
}
