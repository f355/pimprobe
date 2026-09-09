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

use pimprobe_core::*;
use serde::Deserialize;
use std::{path::Path, sync::Mutex};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    result: RoutineResult,
}
fn config(name: &str) -> RoutineConfig {
    let parts: Vec<_> = name.split('_').collect();
    let mut c = RoutineConfig {
        family: parts[0].into(),
        coarse_feed: 30.0,
        fine_feed: 10.0,
        diameter: 4.0,
        ..RoutineConfig::default()
    };
    if c.family == "center" {
        c.feature = parts[1].into();
        c.x = if c.feature.starts_with("y-") || c.feature == "z" {
            0
        } else {
            1
        };
        c.y = if c.feature.starts_with("x-") || c.feature == "z" {
            0
        } else {
            1
        };
        c.z = c.feature == "z";
        c.x_search_distance = 20.0;
        c.y_search_distance = 24.0;
    } else {
        c.x = parts[1].parse().unwrap();
        c.y = parts[2].parse().unwrap();
        c.z = c.x == 0 && c.y == 0;
    }
    c
}
fn mock() -> MockController {
    let mut s = MockController::new().state();
    s.probe_offset = [-55.872, -5.362, -64.724, 0.0];
    s.probe_extended = true;
    MockController::with_state(s)
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}

#[test]
fn external_search_starts_at_the_entered_distance() {
    for feature in ["center_block", "center_boss"] {
        let mut cfg = config(feature);
        cfg.x_search_distance = 20.0;
        cfg.y_search_distance = 15.0;
        let plan = review(mock().state(), cfg).unwrap();
        let program = plan.program();
        for command in ["G38.3 X-20 F1000", "G38.3 Y-15 F1000"] {
            assert!(program.iter().any(|s| s == command), "{program:?}");
        }
    }
}

#[test]
fn internal_search_limits_use_each_axis_distance() {
    for feature in ["center_pocket", "center_hole"] {
        let mut cfg = config(feature);
        cfg.x_search_distance = 20.0;
        cfg.y_search_distance = 15.0;
        let plan = review(mock().state(), cfg).unwrap();
        let limits: Vec<_> = plan
            .steps
            .iter()
            .filter_map(|step| match step {
                RoutineStep::Contact { axis, limit, .. } => {
                    Some((*axis, limit - plan.start.position[axis.index()]))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            limits,
            vec![
                (Axis::X, -20.0),
                (Axis::X, 20.0),
                (Axis::Y, -15.0),
                (Axis::Y, 15.0)
            ]
        );
    }
}

#[test]
fn center_internal_features_finish_at_probing_height() {
    for feature in [
        "center_hole",
        "center_pocket",
        "center_x-valley",
        "center_y-valley",
    ] {
        let mut state = mock().state();
        state.position[2] = -22.6;
        let plan =
            review(state, config(feature)).unwrap_or_else(|error| panic!("{feature}: {error}"));
        assert!(
            !plan.program().iter().any(|line| line == "G1 Z40 F1000"),
            "{feature}"
        );
    }
}

#[test]
fn travel_error_identifies_the_exhausted_direction() {
    let mut state = mock().state();
    state.position[1] = -18.6;
    let error = review(state, config("center_hole"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("Y+"), "{error}");
    assert!(error.contains("G53 Y"), "{error}");
    assert!(error.contains("Move toward Y-"), "{error}");
}

#[test]
fn preflight_errors_explain_what_needs_attention() {
    let base = mock().state();
    let cases: Vec<(State, &str)> = vec![
        (
            State {
                connected: false,
                ..base.clone()
            },
            "Controller is disconnected",
        ),
        (
            State {
                ready: false,
                ..base.clone()
            },
            "Machine is not ready",
        ),
        (
            State {
                spindle_stopped: false,
                ..base.clone()
            },
            "Stop the spindle before probing",
        ),
        (
            State {
                probe_triggered: true,
                ..base.clone()
            },
            "The probe is already touching something",
        ),
    ];
    for (state, expected) in cases {
        let error = review(state, RoutineConfig::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
    }
}

#[tokio::test]
async fn internal_side_probing_stays_at_the_starting_height() {
    let controller = mock();
    let cfg = config("inside_1_1");
    controller.configure(&cfg).unwrap();
    let plan = review(controller.state(), cfg).unwrap();
    let height = plan.start.position[2];
    run(
        &controller,
        &plan,
        TimingPolicy::default(),
        CancellationToken::new(),
        |p| {
            if p.command.starts_with("G38.3 X") && p.command.ends_with("F30.000") {
                near(controller.state().position[2], height);
            }
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn opposing_searches_stop_at_bounds_measured_from_the_original_start() {
    for center_offset in [-5.0, 5.0] {
        let controller = mock();
        let cfg = config("center_x-valley");
        let start = controller.state().position;
        controller.set_geometry(MockGeometry::Center {
            center: [start[0] + center_offset, start[1]],
            half: [24.0, 3.0],
            enabled: [true, false],
            round: false,
            internal: true,
            top: start[2] + 2.0,
            floor: start[2] - 10.0,
        });
        let plan = review(controller.state(), cfg).unwrap();
        let error = run(
            &controller,
            &plan,
            TimingPolicy::default(),
            CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap_err();
        assert_eq!(error, Error::CoarseNoContact);
        near(
            controller.state().position[0],
            start[0] + center_offset.signum() * 20.0,
        );
        near(controller.state().position[2], start[2]);
        if center_offset > 0.0 {
            assert!(controller
                .commands()
                .iter()
                .any(|s| s == "G38.3 X38.500 F30.000"));
        }
    }
}

#[tokio::test]
async fn external_search_without_contact_stops_at_the_starting_axis_coordinate() {
    for family in ["outside", "center"] {
        let controller = mock();
        let cfg = if family == "outside" {
            config("outside_1_0")
        } else {
            config("center_x-ridge")
        };
        let plan = review(controller.state(), cfg).unwrap();
        let error = run(
            &controller,
            &plan,
            TimingPolicy::default(),
            CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap_err();
        assert_eq!(error, Error::CoarseNoContact);
        near(controller.state().position[0], plan.start.position[0]);
        near(
            controller.state().position[2],
            plan.start.position[2] - plan.config.depth,
        );
    }
}

#[tokio::test]
async fn inside_corner_finishes_at_its_starting_xy_without_lifting() {
    let controller = mock();
    let cfg = RoutineConfig {
        family: "inside".into(),
        x: 1,
        y: -1,
        z: false,
        ..Default::default()
    };
    controller.configure(&cfg).unwrap();
    let plan = review(controller.state(), cfg).unwrap();
    run(
        &controller,
        &plan,
        TimingPolicy::default(),
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    let end = controller.state().position;
    near(end[0], plan.start.position[0]);
    near(end[1], plan.start.position[1]);
    near(end[2], plan.start.position[2]);
    assert!(!controller
        .commands()
        .iter()
        .any(|command| command == "G1 Z40.000 F1000.000"));
}

#[tokio::test]
async fn inside_result_can_lift_then_move_to_the_measured_point() {
    let controller = mock();
    let cfg = config("inside_1_-1");
    controller.configure(&cfg).unwrap();
    let plan = review(controller.state(), cfg).unwrap();
    let result = run(
        &controller,
        &plan,
        TimingPolicy::default(),
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    let updated = go_to_measured(
        &controller,
        &plan,
        &result,
        25.0,
        TimingPolicy::default(),
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    let end = controller.state().position;
    near(end[2], plan.start.position[2] + 25.0);
    for (i, position) in end.iter().enumerate().take(2) {
        near(
            position + plan.start.probe_offset[i],
            result.machine_point[i].unwrap(),
        );
    }
    assert!(updated.positioned);
}

#[tokio::test]
async fn inside_result_reports_an_obstructed_move_in_operator_terms() {
    let controller = mock();
    let cfg = config("inside_1_-1");
    controller.configure(&cfg).unwrap();
    let plan = review(controller.state(), cfg).unwrap();
    let result = run(
        &controller,
        &plan,
        TimingPolicy::default(),
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    let start = controller.state().position;
    let target_x = result.machine_point[0].unwrap() - plan.start.probe_offset[0];
    controller.set_geometry(MockGeometry::Plane {
        axis: Axis::X,
        coordinate: (start[0] + target_x) / 2.0,
    });
    let error = go_to_measured(
        &controller,
        &plan,
        &result,
        25.0,
        TimingPolicy::default(),
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(error.contains("Probe touched while moving X/Y"), "{error}");
    assert!(error.contains("Clear the path"), "{error}");
    assert_eq!(controller.state().modes, plan.start.modes);
}

#[tokio::test]
async fn result_can_return_before_or_after_zero_using_machine_coordinates() {
    for zero_first in [false, true] {
        let controller = mock();
        let cfg = RoutineConfig {
            family: "outside".into(),
            x: 1,
            y: -1,
            z: false,
            ..Default::default()
        };
        controller.configure(&cfg).unwrap();
        let plan = review(controller.state(), cfg).unwrap();
        let mut result = run(
            &controller,
            &plan,
            TimingPolicy::default(),
            CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap();
        if zero_first {
            result = zero_result(
                &controller,
                &plan,
                &result,
                [1.0, -2.0, 0.0],
                TimingPolicy::default(),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        }
        let count = controller.commands().len();
        let log = Mutex::new(Vec::new());
        result = return_to_start(
            &controller,
            &plan,
            &result,
            TimingPolicy::default(),
            CancellationToken::new(),
            |p| {
                if p.kind == "script" {
                    log.lock().unwrap().push(p.message);
                }
            },
        )
        .await
        .unwrap();
        assert!(result.returned);
        assert!(controller
            .state()
            .position
            .iter()
            .zip(plan.start.position)
            .all(|(a, b)| (a - b).abs() < 0.002));
        let commands = controller.commands();
        let moves = commands[count..]
            .iter()
            .filter(|s| s.starts_with("G38.3"))
            .collect::<Vec<_>>();
        assert_eq!(moves.len(), 1);
        assert!(moves[0].contains(" X") && moves[0].contains(" Y"));
        assert!(log
            .lock()
            .unwrap()
            .iter()
            .any(|s| s.starts_with("G38.3 X") && s.contains(" Y")));
        if !zero_first {
            result = zero_result(
                &controller,
                &plan,
                &result,
                [1.0, -2.0, 0.0],
                TimingPolicy::default(),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        }
        assert!(result.zeroed);
    }
}

#[tokio::test]
async fn external_traverses_and_finish_use_starting_z() {
    for feature in [
        "outside_1_1",
        "outside_-1_0",
        "center_boss",
        "center_block",
        "center_x-ridge",
        "center_y-ridge",
    ] {
        let mut state = mock().state();
        state.position[2] = -5.0;
        state.work_position[2] += 55.0;
        let controller = MockController::with_state(state);
        let mut cfg = config(feature);
        cfg.safe_z_offset = 25.0;
        controller.configure(&cfg).unwrap();
        let plan = review(controller.state(), cfg).unwrap();
        let clearance = plan.start.position[2];
        let traverses = Mutex::new(0);
        run(
            &controller,
            &plan,
            TimingPolicy::default(),
            CancellationToken::new(),
            |p| {
                if (p.command.starts_with("G38.3 X") || p.command.starts_with("G38.3 Y"))
                    && p.command.ends_with("F1000.000")
                {
                    near(controller.state().position[2], clearance);
                    *traverses.lock().unwrap() += 1;
                }
            },
        )
        .await
        .unwrap();
        assert!(*traverses.lock().unwrap() >= 2);
        near(controller.state().position[2], clearance);
    }
}

#[tokio::test]
async fn contact_during_return_releases_along_the_diagonal_before_failing() {
    let controller = mock();
    let cfg = RoutineConfig {
        family: "outside".into(),
        x: 1,
        y: -1,
        z: false,
        ..Default::default()
    };
    controller.configure(&cfg).unwrap();
    let plan = review(controller.state(), cfg).unwrap();
    let result = run(
        &controller,
        &plan,
        TimingPolicy::default(),
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    let from = controller.state().position;
    controller.set_geometry(MockGeometry::Plane {
        axis: Axis::X,
        coordinate: (from[0] + plan.start.position[0]) / 2.0,
    });
    let error = return_to_start(
        &controller,
        &plan,
        &result,
        TimingPolicy::default(),
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        Error::UnexpectedContact {
            retracted: true,
            ..
        }
    ));
    let end = controller.state().position;
    near(end[2], from[2]);
    let direction = [
        plan.start.position[0] - from[0],
        plan.start.position[1] - from[1],
    ];
    let length = direction[0].hypot(direction[1]);
    for i in 0..2 {
        let expected = from[i] + direction[i] * (0.5 - plan.config.retract / length);
        assert!((end[i] - expected).abs() < 0.002);
    }
    assert_eq!(controller.state().modes, plan.start.modes);
}

#[tokio::test]
async fn geometry_measurements_end_positions_and_script_parity() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/routines");
    let mut paths = std::fs::read_dir(directory)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        let name = path.file_stem().unwrap().to_str().unwrap();
        let expected: Fixture = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let cfg = config(name);
        let controller = mock();
        controller.configure(&cfg).unwrap();
        let initial = controller.state();
        let plan = review(initial.clone(), cfg.clone()).unwrap();
        let script = Mutex::new(Vec::<String>::new());
        let commands = Mutex::new(Vec::<String>::new());
        let result = run(
            &controller,
            &plan,
            TimingPolicy::default(),
            CancellationToken::new(),
            |p| {
                if p.kind == "script" {
                    script.lock().unwrap().push(p.message);
                }
                if !p.command.is_empty() {
                    commands.lock().unwrap().push(p.command);
                }
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{name}: {e}; {:?}", controller.commands()));
        assert!(result.settled, "{name}");
        assert_eq!(result.zeroed, expected.result.zeroed, "{name}");
        assert_eq!(result.wcs, expected.result.wcs);
        assert_eq!(result.axes, expected.result.axes);
        for i in 0..3 {
            match (result.point[i], expected.result.point[i]) {
                (Some(a), Some(b)) => near(a, b),
                (None, None) => {}
                _ => panic!("{name}: mismatched measurement"),
            }
        }
        let final_state = controller.state();
        for i in 0..2 {
            let expected = if cfg.family == "inside" {
                initial.position[i]
            } else {
                result.machine_point[i]
                    .map(|v| v - initial.probe_offset[i])
                    .unwrap_or(initial.position[i])
            };
            assert!(
                (final_state.position[i] - expected).abs() < 0.001,
                "{name}: axis {i}"
            );
        }
        near(final_state.position[2], initial.position[2]);
        near(final_state.position[3], initial.position[3]);
        assert_eq!(final_state.modes, initial.modes);
        let runtime = script.lock().unwrap().clone();
        let symbolic = plan.program();
        assert_eq!(runtime.len(), symbolic.len(), "{name}");
        let runtime_commands = runtime
            .iter()
            .filter(|line| !line.starts_with(';'))
            .map(|line| canonical_command(line))
            .collect::<Vec<_>>();
        let expected_commands = commands
            .lock()
            .unwrap()
            .iter()
            .map(|line| canonical_command(line))
            .collect::<Vec<_>>();
        assert_eq!(
            runtime_commands, expected_commands,
            "resolved script diverged from motion: {name}"
        );
        for (actual, preview) in runtime.iter().zip(&symbolic) {
            if preview.contains("#<") {
                if preview.starts_with("; #<") {
                    let (a, v) = actual.split_once(" := ").unwrap();
                    assert!(preview.starts_with(a));
                    assert!(v.parse::<f64>().unwrap().is_finite());
                } else {
                    assert!(!actual.contains(['#', '[', ']']), "{actual}");
                }
            } else {
                assert_eq!(actual, preview, "{name}");
            }
        }
        if !cfg.zero {
            let updated = zero_result(
                &controller,
                &plan,
                &result,
                [0.0; 3],
                TimingPolicy::default(),
                CancellationToken::new(),
            )
            .await
            .unwrap();
            assert!(updated.zeroed);
            assert_eq!(updated.point, result.point);
            assert_eq!(controller.state().position, final_state.position);
            assert_eq!(controller.state().modes, final_state.modes);
            for i in 0..3 {
                if let Some(v) = result.point[i] {
                    let desired = (final_state.position[i]
                        - (v + initial.position[i] - initial.work_position[i]))
                        * 1000.0;
                    near(
                        controller.state().work_position[i],
                        desired.round() / 1000.0,
                    );
                } else {
                    near(
                        controller.state().work_position[i],
                        final_state.work_position[i],
                    );
                }
            }
            assert!(zero_result(
                &controller,
                &plan,
                &updated,
                [0.0; 3],
                TimingPolicy::default(),
                CancellationToken::new()
            )
            .await
            .is_err());
        }
    }
}

fn canonical_command(line: &str) -> String {
    line.split(';')
        .next()
        .unwrap()
        .split_whitespace()
        .map(|word| {
            if word.starts_with(['X', 'Y', 'Z', 'F']) {
                format!("{}{:.3}", &word[..1], word[1..].parse::<f64>().unwrap())
            } else {
                word.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[tokio::test]
async fn inside_corners_start_y_at_starting_x_and_probing_height() {
    for x in [-1, 1] {
        for y in [-1, 1] {
            {
                let cfg = RoutineConfig {
                    family: "inside".into(),
                    x,
                    y,
                    z: false,
                    ..RoutineConfig::default()
                };
                let controller = mock();
                controller.configure(&cfg).unwrap();
                let initial = controller.state();
                let plan = review(initial.clone(), cfg.clone()).unwrap();
                let at_y = Mutex::new(None);
                let y_coarse = format!(
                    "G38.3 Y{:.3} F{:.3}",
                    f64::from(y) * cfg.y_search_distance,
                    cfg.coarse_feed
                );
                run(
                    &controller,
                    &plan,
                    TimingPolicy::default(),
                    CancellationToken::new(),
                    |p| {
                        if p.command == y_coarse {
                            *at_y.lock().unwrap() = Some(controller.state().position);
                        }
                    },
                )
                .await
                .unwrap();
                let position = at_y.lock().unwrap().expect("Y coarse touch");
                near(position[0], initial.position[0]);
                near(position[2], initial.position[2] - cfg.side_depth());
                let commands = controller.commands();
                let y_index = commands.iter().position(|c| c == &y_coarse).unwrap();
                assert_eq!(
                    commands[y_index - 1],
                    format!("G38.3 X{:.3} F1000.000", -f64::from(x) * 3.5)
                );
            }
        }
    }
}

#[tokio::test]
async fn center_spans_are_compensated_and_survive_work_zero_offsets() {
    for feature in [
        "boss", "block", "hole", "pocket", "x-ridge", "x-valley", "y-ridge", "y-valley",
    ] {
        let cfg = config(&format!("center_{feature}"));
        let controller = mock();
        controller.configure(&cfg).unwrap();
        let plan = review(controller.state(), cfg.clone()).unwrap();
        let result = run(
            &controller,
            &plan,
            TimingPolicy::default(),
            CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap();
        let serialized = serde_json::to_value(&result).unwrap();
        let spans = serialized["spans"]
            .as_array()
            .expect("measured spans in result");
        assert!(spans[2].is_null());
        for (i, enabled) in [cfg.x != 0, cfg.y != 0].into_iter().enumerate() {
            if !enabled {
                assert!(spans[i].is_null());
                continue;
            }
            let value = spans[i].as_f64().unwrap();
            if i == 0 && matches!(feature, "hole" | "boss") {
                // The first pass starts 1.5 mm away from the circle's Y center.
                assert!(value < 14.0);
            } else {
                let expected = if i == 0 || matches!(feature, "hole" | "boss") {
                    14.0
                } else {
                    16.8
                };
                assert!(
                    (value - expected).abs() < 0.002,
                    "{feature}: {value} != {expected}"
                );
            }
        }
        let updated = zero_result(
            &controller,
            &plan,
            &result,
            [1.0, -2.0, 0.0],
            TimingPolicy::default(),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(
            serde_json::to_value(updated).unwrap()["spans"],
            serialized["spans"]
        );
    }
}

#[tokio::test]
async fn late_zero_converts_inches_and_restores_modes_without_motion() {
    let controller = mock();
    let cfg = RoutineConfig::default();
    controller.configure(&cfg).unwrap();
    let plan = review(controller.state(), cfg).unwrap();
    let result = run(
        &controller,
        &plan,
        TimingPolicy::default(),
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    controller.send("G20 G93 G90").await.unwrap();
    let before = controller.state();
    let updated = zero_result(
        &controller,
        &plan,
        &result,
        [0.0; 3],
        TimingPolicy::default(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(updated.zeroed);
    assert_eq!(controller.state().position, before.position);
    assert_eq!(controller.state().modes, before.modes);
}

#[tokio::test]
async fn modes_changed_after_review_are_intentionally_overridden() {
    let controller = mock();
    let cfg = RoutineConfig::default();
    controller.configure(&cfg).unwrap();
    let plan = review(controller.state(), cfg).unwrap();
    controller.send("G20 G93 G90").await.unwrap();
    assert!(run(
        &controller,
        &plan,
        TimingPolicy::default(),
        CancellationToken::new(),
        |_| {}
    )
    .await
    .is_ok());
    assert_eq!(controller.state().modes, plan.start.modes);
}

#[tokio::test]
async fn missing_z_compensation_returns_to_safe_height_without_zero() {
    let controller = mock();
    controller.set_tool_value(None);
    let cfg = RoutineConfig {
        zero: true,
        ..RoutineConfig::default()
    };
    controller.configure(&cfg).unwrap();
    let plan = review(controller.state(), cfg).unwrap();
    let err = run(
        &controller,
        &plan,
        TimingPolicy::default(),
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::Compensation(_)));
    assert_eq!(controller.state().position, plan.start.position);
    assert!(!controller.commands().iter().any(|s| s.starts_with("G10")));
}

#[tokio::test]
async fn coarse_miss_returns_z_and_restores_modes_without_zero() {
    let controller = mock();
    let cfg = RoutineConfig {
        zero: true,
        ..RoutineConfig::default()
    };
    let plan = review(controller.state(), cfg).unwrap();
    let log = Mutex::new(Vec::new());
    let error = run(
        &controller,
        &plan,
        TimingPolicy::default(),
        CancellationToken::new(),
        |p| {
            if p.kind == "script" {
                log.lock().unwrap().push(p.message)
            }
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, Error::CoarseNoContact);
    assert_eq!(controller.state().position, plan.start.position);
    assert_eq!(controller.state().modes, plan.start.modes);
    assert!(log
        .lock()
        .unwrap()
        .iter()
        .any(|s| s.starts_with("G38.3 Z-5")));
    assert!(!controller.commands().iter().any(|s| s.starts_with("G10")));
}

#[tokio::test]
async fn guarded_contact_reverses_no_further_than_traversed_segment() {
    let controller = mock();
    controller.send(&Modes::PROBING.command()).await.unwrap();
    controller.set_geometry(MockGeometry::Plane {
        axis: Axis::X,
        coordinate: -120.2,
    });
    let err = run_guarded_move(
        &controller,
        GuardedMoveConfig {
            axis: Axis::X,
            distance: -10.0,
            feed: 1000.0,
            retract_distance: 0.5,
        },
        TimingPolicy::default(),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        Error::UnexpectedContact {
            retracted: true,
            ..
        }
    ));
    near(controller.state().position[0], -120.0);
}

#[test]
fn preflight_configuration_and_envelope_errors_are_fail_closed() {
    let original = mock().state();
    let cfg = RoutineConfig::default();
    for mutate in [
        |s: &mut State| s.connected = false,
        |s: &mut State| s.ready = false,
        |s: &mut State| s.spindle_stopped = false,
        |s: &mut State| s.probe_extended = false,
        |s: &mut State| s.probe_triggered = true,
        |s: &mut State| s.probe_offset_known = false,
        |s: &mut State| s.travel_limits_known = false,
        |s: &mut State| s.position[0] = f64::NAN,
        |s: &mut State| s.modes.units = 0,
        |s: &mut State| s.work_position[0] = f64::INFINITY,
        |s: &mut State| s.wcs = 55,
        |s: &mut State| s.position[2] = -0.6,
    ] {
        let mut s = original.clone();
        mutate(&mut s);
        assert!(review(s, cfg.clone()).is_err());
    }
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY, 10001.0] {
        let mut c = cfg.clone();
        c.positioning_feed = value;
        assert!(review(original.clone(), c).is_err());
    }
    let mut c = cfg.clone();
    c.family = "unknown".into();
    assert!(review(original.clone(), c).is_err());
    let mut c = cfg;
    c.family = "center".into();
    c.feature = "boss".into();
    assert!(review(original, c).is_err());
}
