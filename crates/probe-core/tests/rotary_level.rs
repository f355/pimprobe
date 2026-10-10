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

struct ParsedController {
    inner: MockController,
    events: tokio::sync::broadcast::Sender<Event>,
    relay: tokio::task::JoinHandle<()>,
    ignore_writes: bool,
}
impl ParsedController {
    fn new(inner: MockController, ignore_writes: bool) -> Self {
        let mut input = inner.subscribe();
        let (events, _) = tokio::sync::broadcast::channel(256);
        let output = events.clone();
        let relay = tokio::spawn(async move {
            while let Ok(event) = input.recv().await {
                if !event.acknowledged {
                    let _ = output.send(event);
                }
            }
        });
        Self {
            inner,
            events,
            relay,
            ignore_writes,
        }
    }
}
impl Drop for ParsedController {
    fn drop(&mut self) {
        self.relay.abort();
    }
}
#[async_trait]
impl Controller for ParsedController {
    fn state(&self) -> State {
        self.inner.state()
    }
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Event> {
        self.events.subscribe()
    }
    async fn send(&self, command: &str) -> Result<(), Error> {
        if self.ignore_writes && command.starts_with("G10 ") {
            return Ok(());
        }
        self.inner.send(command).await
    }
}

#[tokio::test]
async fn rotary_writes_and_restoration_use_readback_without_acknowledgments() {
    for version in ["1.0.35-a", "1.0.35+c0.test"] {
        let (inner, config) = fixture(RotaryOperation::Horizontal, 8., version);
        let machine = ParsedController::new(inner, false);
        let before = machine.state();
        let plan = review_rotary(before.clone(), config).unwrap();
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
        assert_eq!(machine.state().modes, before.modes);
        assert_eq!(machine.state().plane, before.plane);
        assert_eq!(machine.state().wcs_rotation, before.wcs_rotation);
        zero_rotary(&machine, &result).await.unwrap();
        assert_eq!(machine.state().modes, before.modes);
        assert!(
            (machine.state().wcs_origin.unwrap()[3] + before.coordinate_offset[3]
                - result.level.unwrap().center[3])
                .abs()
                < 0.001
        );
    }
}

#[tokio::test]
async fn coordinate_readback_waits_for_its_trailing_probe_report() {
    struct Readback {
        inner: MockController,
        events: tokio::sync::broadcast::Sender<Event>,
        requested: tokio::sync::Notify,
    }
    #[async_trait]
    impl Controller for Readback {
        fn state(&self) -> State {
            self.inner.state()
        }
        fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Event> {
            self.events.subscribe()
        }
        async fn send(&self, command: &str) -> Result<(), Error> {
            self.inner.send(command).await?;
            if command == "$#" {
                self.events
                    .send(Event {
                        wcs_rotation: Some((54, 0.0)),
                        ..Event::default()
                    })
                    .unwrap();
                self.requested.notify_one();
            }
            Ok(())
        }
    }
    let (inner, _) = fixture(RotaryOperation::Horizontal, 8., "1.0.35+c0.test");
    let c = std::sync::Arc::new(Readback {
        inner,
        events: tokio::sync::broadcast::channel(16).0,
        requested: tokio::sync::Notify::new(),
    });
    let worker = c.clone();
    let task = tokio::spawn(async move { write_rotary_rotation(worker.as_ref(), 54, 0.).await });
    c.requested.notified().await;
    tokio::task::yield_now().await;
    assert!(
        !task.is_finished(),
        "coordinate readback returned before the rest of $#"
    );
    c.events
        .send(Event {
            probe: Some(Contact {
                position: c.state().position,
                success: false,
            }),
            ..Event::default()
        })
        .unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn rotary_rotation_rejects_a_write_that_did_not_take_effect() {
    let (inner, _) = fixture(RotaryOperation::Horizontal, 8., "1.0.35+c0.test");
    let machine = ParsedController::new(inner, true);
    let error = write_rotary_rotation(&machine, 54, 0.).await.unwrap_err();
    assert!(matches!(error, Error::Controller(_)), "{error}");
    assert!(error
        .to_string()
        .contains("did not confirm the new work coordinate rotation"));
    assert_eq!(machine.state().wcs_rotation, Some(2.5));
}

fn fixture(operation: RotaryOperation, tilt: f64, version: &str) -> (MockController, RotaryConfig) {
    let mut state = MockController::new().state();
    state.probe_extended = true;
    state.position[3] = 17.;
    state.modes = Modes {
        units: 20,
        feed: 93,
        distance: 90,
    };
    state.firmware_version = version.into();
    state.wcs_rotation = rotary_supported(version).then_some(2.5);
    state.coordinate_offset = [1., -2., 0.5, 3.];
    state.tool_length_offset = 7.25;
    let machine = MockController::with_state(state.clone());
    let config = RotaryConfig {
        operation,
        y_distance: 12.,
        z_distance: 12.,
        ..RotaryConfig::default()
    };
    let y = state.position[1] + state.probe_offset[1];
    let z = state.position[2] - state.probe_offset[2];
    let vertical = operation != RotaryOperation::Horizontal;
    let mirrored = operation == RotaryOperation::VerticalNegative;
    machine.set_geometry(MockGeometry::RotatingPlane {
        pivot: if vertical {
            [
                y + if mirrored { -8. } else { 8. },
                z + 6. + config.diameter / 2.,
            ]
        } else {
            [y + 6., z - 8.]
        },
        distance: 5. * tilt.to_radians().cos(),
        angle: if mirrored {
            180. - tilt
        } else if vertical {
            -tilt
        } else {
            tilt
        },
        a_start: state.position[3],
        vertical,
        ball_radius: config.diameter / 2.,
        offset: state.probe_offset,
    });
    (machine, config)
}

#[tokio::test]
async fn levels_both_surfaces_and_preserves_other_work_axes() {
    for version in ["1.0.35-a", "1.0.35+c0.example-b"] {
        for operation in [
            RotaryOperation::Horizontal,
            RotaryOperation::Vertical,
            RotaryOperation::VerticalNegative,
        ] {
            for tilt in [-8., 8.] {
                let (machine, config) = fixture(operation, tilt, version);
                let before = machine.state();
                let plan = review_rotary(before.clone(), config).unwrap();
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
                let level = result.level.as_ref().unwrap();
                let expected = tilt
                    * if operation == RotaryOperation::Horizontal {
                        -1.
                    } else {
                        1.
                    };
                assert!((level.correction - expected).abs() < 0.02, "{level:?}");
                assert!(level.residual.abs() < 0.02, "{level:?}");
                let i = if operation == RotaryOperation::Horizontal {
                    1
                } else {
                    2
                };
                let spacing = 12.;
                for points in [level.initial_touches, level.touches] {
                    assert!(
                        (points[0][i]
                            - before.position[i]
                            - if i == 1 {
                                before.probe_offset[1]
                            } else {
                                -before.probe_offset[2] + plan.config.diameter / 2.
                            })
                        .abs()
                            < 0.001
                    );
                    assert!((points[1][i] - points[0][i] - spacing).abs() < 0.001);
                }
                let after = machine.state();
                assert_eq!(after.modes, before.modes);
                assert_eq!(after.plane, before.plane);
                assert_eq!(after.wcs_rotation, before.wcs_rotation);
                for axis in 0..3 {
                    assert!((after.position[axis] - before.position[axis]).abs() < 0.002);
                }
                assert!((after.position[3] - before.position[3] - expected).abs() < 0.02);
                let commands = machine.commands();
                let spacing_axis = if i == 1 { 'Y' } else { 'Z' };
                for program in [commands.join("\n"), plan.program()] {
                    let moves: Vec<f64> = program
                        .lines()
                        .filter(|line| line.starts_with("G38.3 "))
                        .filter_map(|line| {
                            line.split_whitespace()
                                .find_map(|word| word.strip_prefix(spacing_axis)?.parse().ok())
                        })
                        .collect();
                    assert_eq!(moves, [spacing, -spacing, spacing, -spacing]);
                }
                let turn = commands
                    .iter()
                    .position(|cmd| {
                        cmd.split_whitespace().next() == Some("G1")
                            && cmd.split_whitespace().any(|word| word.starts_with('A'))
                    })
                    .unwrap();
                let return_move = &commands[turn - 1];
                assert!(return_move.starts_with("G38.3"));
                assert!(return_move.contains(&format!(
                    "{}{}",
                    if i == 1 { "Y" } else { "Z" },
                    -spacing
                )));
                zero_rotary(&machine, &result).await.unwrap();
                let saved = machine.state();
                let origin = saved.wcs_origin.unwrap();
                assert_eq!(origin[0], before.wcs_origin.unwrap()[0]);
                assert!((origin[3] + saved.coordinate_offset[3] - level.center[3]).abs() < 0.001);
                if operation == RotaryOperation::Horizontal {
                    assert_eq!(origin[1], before.wcs_origin.unwrap()[1]);
                    assert!(
                        (origin[2] + saved.coordinate_offset[2] - level.center[2]).abs() < 0.001
                    );
                } else {
                    assert_eq!(origin[2], before.wcs_origin.unwrap()[2]);
                    let angle = saved.wcs_rotation.unwrap_or(0.).to_radians();
                    let y = angle.cos() * (level.center[1] - origin[1])
                        - angle.sin() * (level.center[0] - origin[0])
                        - saved.coordinate_offset[1];
                    assert!(y.abs() < 0.001);
                }
                assert_eq!(saved.modes, before.modes);
            }
        }
    }
}

#[tokio::test]
async fn missed_or_excessively_tilted_surface_does_not_turn_the_chuck() {
    for missing in [true, false] {
        let (machine, mut config) = fixture(RotaryOperation::Horizontal, 30., "1.0.35+c1.test");
        config.z_distance = 20.;
        config.y_distance = 4.;
        if missing {
            machine.set_geometry(MockGeometry::Empty);
        }
        let before = machine.state();
        let plan = review_rotary(before.clone(), config).unwrap();
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
        if missing {
            assert!(matches!(error, Error::CoarseNoContact), "{error:?}");
        } else {
            assert!(error.to_string().contains("within 20 degrees"), "{error}");
        }
        assert_eq!(machine.state().position[3], before.position[3]);
        assert_eq!(machine.state().modes, before.modes);
        assert_eq!(machine.state().wcs_rotation, before.wcs_rotation);
    }
}

#[test]
fn review_checks_search_and_second_touch_travel() {
    let (machine, config) = fixture(RotaryOperation::Horizontal, 8., "1.0.35-a");
    let mut state = machine.state();
    state.position[1] = -2.;
    assert!(review_rotary(state, config).is_err());
    let (machine, config) = fixture(RotaryOperation::Vertical, 8., "1.0.35-a");
    let mut state = machine.state();
    state.position[2] = -2.;
    assert!(review_rotary(state, config).is_err());
}

#[tokio::test]
async fn contact_while_ascending_along_a_wall_aborts_before_rotation() {
    let (machine, mut config) = fixture(RotaryOperation::Vertical, -15., "1.0.35-a");
    config.z_distance = 30.;
    let before = machine.state();
    let plan = review_rotary(before.clone(), config).unwrap();
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
    assert!(matches!(error, Error::UnexpectedContact { .. }), "{error}");
    assert_eq!(machine.state().position[3], before.position[3]);
}

#[tokio::test]
async fn vertical_zero_rejects_an_xy_frame_that_requires_changing_x_origin() {
    let (machine, config) = fixture(RotaryOperation::Vertical, 8., "1.0.35+c0.test");
    let plan = review_rotary(machine.state(), config).unwrap();
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
    write_rotary_rotation(&machine, 54, 90.).await.unwrap();
    let before = machine.state();
    let error = zero_rotary(&machine, &result).await.unwrap_err();
    assert!(matches!(error, Error::Preflight(_)), "{error}");
    assert_eq!(machine.state().wcs_origin, before.wcs_origin);
    assert_eq!(machine.state().modes, before.modes);
}
