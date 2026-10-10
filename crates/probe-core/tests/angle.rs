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
async fn angles_touch_twice_and_return_to_start() {
    for (feature, x, y, z, axis, direction) in [
        ("x-plus", 1, 0, false, Axis::X, 1.),
        ("x-minus", -1, 0, false, Axis::X, -1.),
        ("y-plus", 0, 1, false, Axis::Y, 1.),
        ("y-minus", 0, -1, false, Axis::Y, -1.),
        ("z-x", 0, 0, true, Axis::Z, -1.),
        ("z-y", 0, 0, true, Axis::Z, -1.),
    ] {
        let c = MockController::new();
        c.set_extended(true).unwrap();
        let start = c.state();
        c.set_geometry(MockGeometry::Plane {
            axis,
            coordinate: start.position[axis.index()] + direction * 2.,
        });
        let plan = review(
            start.clone(),
            RoutineConfig {
                family: "angle".into(),
                feature: feature.into(),
                x,
                y,
                z,
                ..RoutineConfig::default()
            },
        )
        .unwrap();
        let result = run(
            &c,
            &plan,
            TimingPolicy::default(),
            CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(c.state().position, start.position, "{feature}");
        assert_eq!(
            c.commands()
                .iter()
                .filter(|line| line.starts_with("G38.2"))
                .count(),
            2
        );
        assert!(result.returned);
        assert_eq!(result.angle.unwrap().degrees, 0.);
    }
}

#[tokio::test]
async fn slopes_use_machine_axes_and_restore_the_existing_work_rotation() {
    for (feature, x, y, z) in [
        ("x-plus", 1, 0, false),
        ("x-minus", -1, 0, false),
        ("y-plus", 0, 1, false),
        ("y-minus", 0, -1, false),
        ("z-x", 0, 0, true),
        ("z-y", 0, 0, true),
    ] {
        let mut state = MockController::new().state();
        state.probe_extended = true;
        state.wcs_rotation = Some(12.);
        let c = MockController::with_state(state.clone());
        let config = RoutineConfig {
            family: "angle".into(),
            feature: feature.into(),
            x,
            y,
            z,
            ..RoutineConfig::default()
        };
        c.configure(&config).unwrap();
        let plan = review(state.clone(), config).unwrap();
        let result = run(
            &c,
            &plan,
            TimingPolicy::default(),
            CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap();
        let angle = result.angle.as_ref().unwrap();
        assert!(
            (angle.degrees - 0.02_f64.atan().to_degrees()).abs() < 0.00001,
            "{feature}: {angle:?}"
        );
        assert_eq!(angle.rotation_supported, !z);
        assert_eq!(c.state().position, state.position);
        assert_eq!(c.state().modes, state.modes);
        assert_eq!(c.state().wcs_rotation, Some(12.));
        if !z {
            let origin = c.state().wcs_origin;
            let applied = rotate_angle_result(&c, &result).await.unwrap();
            assert!(applied.angle.unwrap().rotation_applied);
            assert_eq!(c.state().wcs_origin, origin);
            assert!((c.state().wcs_rotation.unwrap() - angle.degrees).abs() < 0.001);
        }
    }
}

#[tokio::test]
async fn stock_firmware_measures_angles_and_rejects_work_rotation() {
    let mut state = MockController::new().state();
    state.probe_extended = true;
    state.firmware_version = "1.0.35".into();
    state.wcs_rotation = None;
    let c = MockController::with_state(state.clone());
    let config = RoutineConfig {
        family: "angle".into(),
        feature: "y-plus".into(),
        y: 1,
        z: false,
        ..RoutineConfig::default()
    };
    c.configure(&config).unwrap();
    let plan = review(state, config).unwrap();
    let result = run(
        &c,
        &plan,
        TimingPolicy::default(),
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    assert!(!result.angle.as_ref().unwrap().rotation_supported);
    assert!(rotate_angle_result(&c, &result).await.is_err());
}
