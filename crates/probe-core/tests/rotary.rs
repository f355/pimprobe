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

use pimprobe_core::*;

#[tokio::test]
async fn zero_and_alignment_keep_the_axis_on_y_zero_in_either_order() {
    for zero_first in [true, false] {
        let mut state = MockController::new().state();
        state.wcs_rotation = Some(2.);
        state.coordinate_offset = [1., -2., 0.5, 0.];
        state.tool_length_offset = 7.25;
        let machine = MockController::with_state(state);
        let before = machine.state();
        let first = rotary_station(-100., -80., -40., -76., -84.).unwrap();
        let second = RotaryStation {
            center: [-50., -80. + 50. * 1_f64.to_radians().tan(), -44.],
            ..first.clone()
        };
        let mut result = RotaryResult::from_stations([first, second], 54, true).unwrap();
        if zero_first {
            zero_rotary(&machine, &result).await.unwrap();
            result.zeroed = true;
            align_rotary(&machine, &result).await.unwrap();
        } else {
            align_rotary(&machine, &result).await.unwrap();
            zero_rotary(&machine, &result).await.unwrap();
        }
        let after = machine.state();
        let origin = after.wcs_origin.unwrap();
        assert_eq!(origin[0], before.wcs_origin.unwrap()[0]);
        let angle = after.wcs_rotation.unwrap().to_radians();
        for station in &result.stations {
            let y = -(station.center[0] - origin[0]) * angle.sin()
                + (station.center[1] - origin[1]) * angle.cos()
                - after.coordinate_offset[1];
            assert!(y.abs() < 0.001, "station {station:?}, Y={y}");
            let z = station.center[2]
                - origin[2]
                - after.coordinate_offset[2]
                - after.tool_length_offset;
            assert!(z.abs() < 0.001, "Z={z}");
        }
    }
}

#[tokio::test]
async fn captured_modes_plane_and_rotation_survive_a_missed_touch() {
    let mut state = MockController::new().state();
    state.probe_extended = true;
    state.modes = Modes {
        units: 20,
        distance: 90,
        feed: 93,
    };
    state.plane = 18;
    state.wcs_rotation = Some(2.75);
    let machine = MockController::with_state(state.clone());
    let snapshot = query_rotary_state(&machine).await.unwrap();
    let plan = review_rotary(snapshot, RotaryConfig::default()).unwrap();
    let mut result = RotaryResult::default();
    let error = run_rotary(
        &machine,
        &plan,
        &mut result,
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap_err();
    assert_eq!(error, Error::CoarseNoContact);
    assert_eq!(machine.state().modes, state.modes);
    assert_eq!(machine.state().plane, state.plane);
    assert_eq!(machine.state().wcs_rotation, state.wcs_rotation);
    assert!(machine
        .commands()
        .iter()
        .any(|command| command == "G10 L2 P1 R0.000000"));
}

#[tokio::test]
async fn zero_uses_first_station_and_preserves_x_and_parser_modes() {
    let mut state = MockController::new().state();
    state.modes = Modes {
        units: 20,
        distance: 91,
        feed: 93,
    };
    let machine = MockController::with_state(state.clone());
    let result = RotaryResult::from_stations(
        [
            rotary_station(-100., -80., -40., -76., -84.).unwrap(),
            rotary_station(-50., -80., -40., -76., -84.).unwrap(),
        ],
        54,
        true,
    )
    .unwrap();
    zero_rotary(&machine, &result).await.unwrap();
    let after = machine.state();
    assert_eq!(
        after.position[0] - after.work_position[0],
        state.position[0] - state.work_position[0]
    );
    assert!((after.position[1] - after.work_position[1] + 80.).abs() < 0.001);
    assert!((after.position[2] - after.work_position[2] + 44.).abs() < 0.001);
    assert_eq!(after.modes, state.modes);
    let mut stock = state;
    stock.firmware_version = "1.0.35-b".into();
    assert!(align_rotary(&MockController::with_state(stock), &result)
        .await
        .is_err());
}

#[test]
fn fork_detection_distinguishes_community_and_bank_suffixes() {
    for version in ["1.0.35", "1.0.35-a", "1.0.35-b", ""] {
        assert!(!rotary_supported(version));
    }
    for version in ["1.0.35+c0", "1.0.35+c0.abcdef01-b", "1.0.35+c7-a"] {
        assert!(rotary_supported(version));
    }
}

#[test]
fn rotated_surfaces_remove_chucking_offset() {
    let result = rotary_station(-100.0, -80.0, -40.0, -76.0, -84.0).unwrap();
    assert_eq!(result.center, [-100.0, -80.0, -44.0]);
    let tilted = RotaryResult::from_stations(
        [
            result.clone(),
            RotaryStation {
                center: [-50.0, -79.5, -44.2],
                ..result
            },
        ],
        54,
        true,
    )
    .unwrap();
    assert!((tilted.xy_angle - (0.5f64 / 50.0).atan().to_degrees()).abs() < 1e-9);
    assert!((tilted.xz_angle - (-0.2f64 / 50.0).atan().to_degrees()).abs() < 1e-9);
}

#[tokio::test]
async fn eccentric_rod_is_measured_with_both_tracking_paths() {
    for (version, headroom) in [
        ("1.0.35-a", 1.5),
        ("1.0.35-a", 5.0),
        ("1.0.35+c0.abcdef01-b", 1.5),
        ("1.0.35+c0.abcdef01-b", 5.0),
    ] {
        let mut state = MockController::new().state();
        state.probe_extended = true;
        state.position = [-120.0, -110.0, -55.0, 12.0];
        state.firmware_version = version.into();
        let machine = MockController::with_state(state.clone());
        let config = RotaryConfig::default();
        machine.set_geometry(MockGeometry::Rotary {
            center: [
                state.position[0] + state.probe_offset[0],
                state.position[1] + state.probe_offset[1],
                state.position[2] - state.probe_offset[2] - headroom - config.rod_diameter / 2.0,
            ],
            slope: [0.003, -0.002],
            eccentric: [-0.2, 0.3],
            a_start: state.position[3],
            radius: config.rod_diameter / 2.0,
            ball_radius: config.diameter / 2.0,
            offset: state.probe_offset,
        });
        let plan = review_rotary(state.clone(), config).unwrap();
        let mut result = RotaryResult::default();
        run_rotary(
            &machine,
            &plan,
            &mut result,
            CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(result.stations.len(), 2);
        for station in &result.stations {
            let dx = station.center[0] - (state.position[0] + state.probe_offset[0]);
            assert!(
                (station.center[1] - (state.position[1] + state.probe_offset[1] + dx * 0.003))
                    .abs()
                    < 0.01,
                "{station:?}"
            );
            let expected_z = state.position[2]
                - state.probe_offset[2]
                - headroom
                - plan.config.rod_diameter / 2.0
                - dx * 0.002;
            assert!((station.center[2] - expected_z).abs() < 0.01, "{station:?}");
        }
        assert!((result.xy_angle - 0.003f64.atan().to_degrees()).abs() < 0.02);
        assert!((result.xz_angle - (-0.002f64).atan().to_degrees()).abs() < 0.02);
        assert!((machine.state().position[3] - state.position[3]).abs() < 0.001);
        assert_eq!(machine.state().modes, state.modes);
        assert_eq!(
            machine.commands().iter().any(|c| c.starts_with("G19 G")),
            rotary_supported(version)
        );
    }
}
