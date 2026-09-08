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
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

fn settings() -> RepeatabilitySettings {
    RepeatabilitySettings {
        diameter: 2.0,
        retract: 0.5,
        positioning_feed: 1000.0,
        coarse_feed: 300.0,
        fine_feed: 50.0,
    }
}
fn machine() -> MockController {
    machine_with_surface_offset(0.0)
}
fn machine_with_surface_offset(offset: f64) -> MockController {
    let mut state = MockController::new().state();
    state.probe_extended = true;
    state.work_position = [70.0, 20.0, 15.0, 0.0];
    let mock = MockController::with_state(state.clone());
    let mut corner = state.position;
    for (i, coordinate) in corner.iter_mut().enumerate().take(2) {
        *coordinate =
            state.position[i] - state.work_position[i] - state.probe_offset[i] + 1.0 + offset;
    }
    let floor = state.position[2] - state.work_position[2] + 57.75 + state.probe_offset[2];
    mock.set_geometry(MockGeometry::Corner {
        origin: corner,
        directions: [-1, -1],
        travel: [0.0, 0.0],
        top: floor + 20.0,
        floor,
        internal: true,
    });
    mock
}

#[test]
fn repeatability_can_start_at_machine_zero() {
    let initial = machine();
    let mut state = initial.state();
    for i in 0..3 {
        state.work_position[i] -= state.position[i];
        state.position[i] = 0.0;
    }
    let machine = MockController::with_state(state);
    check_repeatability(&machine, &RepeatabilityOptions::default(), settings()).unwrap();
}

#[test]
fn statistics_use_sample_deviation_and_handle_single_readings() {
    let stats = AxisStatistics::from_readings(vec![5.0, 1.0, 4.0, 2.0]).unwrap();
    assert_eq!(stats.count, 4);
    assert_eq!(stats.mean, 3.0);
    assert_eq!(stats.median, 3.0);
    assert_eq!(stats.range, 4.0);
    assert!((stats.stddev.unwrap() - (10.0_f64 / 3.0).sqrt()).abs() < 1e-12);
    let single = AxisStatistics::from_readings(vec![-0.004]).unwrap();
    assert_eq!(single.mean, -0.004);
    assert_eq!(single.stddev, None);
    assert_eq!(
        AxisStatistics::from_readings(vec![3.0, 9.0, 5.0])
            .unwrap()
            .median,
        5.0
    );
}

#[tokio::test]
async fn failure_keeps_the_completed_axes_and_stops_before_next_repetition() {
    let machine = machine();
    let options = RepeatabilityOptions::default();
    let mut report = RepeatabilityReport::default();
    let result = run_repeatability(
        &machine,
        &options,
        settings(),
        &mut report,
        CancellationToken::new(),
        |event| {
            if matches!(event, RepeatabilityEvent::Measurement { axis: Axis::X, .. }) {
                machine.set_geometry(MockGeometry::Empty);
            }
        },
    )
    .await;
    assert_eq!(result, Err(Error::CoarseNoContact));
    assert_eq!(report.measurements.len(), 1);
    assert!(report.measurements[0][0].unwrap().abs() < 0.001);
    assert_eq!(report.measurements[0][1], None);
    assert_eq!(report.statistics[0].as_ref().unwrap().count, 1);
}

#[tokio::test]
async fn cancellation_after_a_reading_stops_further_commands() {
    let machine = machine();
    let cancel = CancellationToken::new();
    let captured = Mutex::new(Vec::new());
    let mut report = RepeatabilityReport::default();
    let result = run_repeatability(
        &machine,
        &RepeatabilityOptions::default(),
        settings(),
        &mut report,
        cancel.clone(),
        |event| {
            if matches!(event, RepeatabilityEvent::Measurement { .. }) {
                *captured.lock().unwrap() = machine.commands();
                cancel.cancel();
            }
        },
    )
    .await;
    assert_eq!(result, Err(Error::Cancelled));
    assert_eq!(machine.commands(), *captured.lock().unwrap());
    assert_eq!(report.measurements.len(), 1);
}

#[tokio::test]
async fn cancellation_before_start_sends_nothing() {
    let machine = machine();
    let cancel = CancellationToken::new();
    cancel.cancel();
    let result = run_repeatability(
        &machine,
        &RepeatabilityOptions::default(),
        settings(),
        &mut RepeatabilityReport::default(),
        cancel,
        |_| {},
    )
    .await;
    assert_eq!(result, Err(Error::Cancelled));
    assert!(machine.commands().is_empty());
}

#[tokio::test]
async fn unselected_axes_stay_unmeasured_and_tip_returns_to_fifteen_fifteen_five() {
    for axes in [
        [true, false, false],
        [false, true, false],
        [false, false, true],
        [true, true, true],
    ] {
        let machine = machine();
        let initial = machine.state();
        let options = RepeatabilityOptions {
            axes,
            repetitions: 2,
            home: false,
            retract: false,
        };
        let mut report = RepeatabilityReport::default();
        run_repeatability(
            &machine,
            &options,
            settings(),
            &mut report,
            CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap();
        for (i, selected) in axes.iter().enumerate() {
            assert_eq!(report.measurements[0][i].is_some(), *selected);
            if *selected {
                assert!(report.measurements[0][i].unwrap().abs() < 0.001);
            }
        }
        let state = machine.state();
        for i in 0..2 {
            assert!((state.work_position[i] + state.probe_offset[i] - 15.0).abs() < 0.001);
        }
        assert!((state.work_position[2] - 57.75 - state.probe_offset[2] - 5.0).abs() < 0.001);
        assert_eq!(state.modes, initial.modes);
    }
}

enum Behaviour {
    FailBackoff,
    HomeRetracts,
    HomeReadyOnly,
    SlowActuationAndHoming,
}
struct ControllerFixture {
    inner: MockController,
    behaviour: Behaviour,
    fine_completed: AtomicBool,
}
#[async_trait]
impl Controller for ControllerFixture {
    fn state(&self) -> State {
        self.inner.state()
    }
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Event> {
        self.inner.subscribe()
    }
    async fn send(&self, command: &str) -> Result<(), Error> {
        if command == "G90 G53 G0 X-20 Y-5" {
            assert!(self.inner.state().ready);
            assert_eq!(self.inner.state().position[2], 0.0);
        }
        if command == "$H" {
            assert_eq!(&self.inner.state().position[..3], &[-20.0, -5.0, 0.0]);
            let commands = self.inner.commands();
            assert_eq!(
                &commands[commands.len() - 2..],
                &["G90 G53 G0 Z0", "G90 G53 G0 X-20 Y-5"]
            );
        }
        if command == "$H" && matches!(self.behaviour, Behaviour::SlowActuationAndHoming) {
            tokio::time::sleep(std::time::Duration::from_secs(240)).await;
        }
        if matches!(self.behaviour, Behaviour::FailBackoff)
            && command.starts_with("G1 ")
            && self.fine_completed.load(Ordering::Relaxed)
        {
            return Err(Error::Controller("backoff failed".into()));
        }
        if command == "$H" && matches!(self.behaviour, Behaviour::HomeReadyOnly) {
            return self.inner.set_extended(self.inner.state().probe_extended);
        }
        self.inner.send(command).await?;
        if command.starts_with("G38.2 ") {
            self.fine_completed.store(true, Ordering::Relaxed);
        }
        if command == "$H" && matches!(self.behaviour, Behaviour::HomeRetracts) {
            self.inner.set_extended(false)?;
        }
        Ok(())
    }
}
#[async_trait]
impl RepeatabilityController for ControllerFixture {
    async fn set_probe(&self, extended: bool) -> Result<(), Error> {
        let state = self.inner.state();
        let extending_at_home = extended && state.position[..3] == [-1.0, -1.0, 0.0];
        if !extending_at_home {
            for i in 0..2 {
                assert!((state.work_position[i] + state.probe_offset[i] - 15.0).abs() < 0.001);
            }
            if extended {
                assert!(
                    (state.work_position[2] - 57.75 - state.probe_offset[2] - 5.0).abs() < 0.001
                );
            }
        }
        if matches!(self.behaviour, Behaviour::SlowActuationAndHoming) {
            tokio::time::sleep(std::time::Duration::from_secs(45)).await;
        }
        self.inner.set_probe(extended).await
    }
    fn probe_reference_z(&self) -> Result<f64, Error> {
        RepeatabilityController::probe_reference_z(&self.inner)
    }
}

#[tokio::test]
async fn measurements_preserve_surface_coordinates() {
    let machine = machine_with_surface_offset(0.2);
    let mut report = RepeatabilityReport::default();
    let readings = Mutex::new(Vec::new());
    run_repeatability(
        &machine,
        &RepeatabilityOptions {
            axes: [true, true, false],
            repetitions: 2,
            retract: false,
            home: false,
        },
        settings(),
        &mut report,
        CancellationToken::new(),
        |event| {
            if let RepeatabilityEvent::Measurement {
                value,
                repetition,
                axis,
                statistics,
            } = event
            {
                let summary = statistics[axis.index()].as_ref().unwrap();
                assert_eq!(summary.count, repetition);
                let expected_mean = if repetition == 1 { 0.2 } else { 0.22 };
                assert!((summary.mean - expected_mean).abs() < 0.001);
                readings.lock().unwrap().push(value);
                if repetition == 1 && axis == Axis::Y {
                    let state = machine.state();
                    let mut corner = state.position;
                    for (i, coordinate) in corner.iter_mut().enumerate().take(2) {
                        *coordinate =
                            state.position[i] - state.work_position[i] - state.probe_offset[i]
                                + 1.24;
                    }
                    let floor =
                        state.position[2] - state.work_position[2] + 57.75 + state.probe_offset[2];
                    machine.set_geometry(MockGeometry::Corner {
                        origin: corner,
                        directions: [-1, -1],
                        travel: [0.0, 0.0],
                        top: floor + 20.0,
                        floor,
                        internal: true,
                    });
                }
            }
        },
    )
    .await
    .unwrap();
    for (value, expected) in readings
        .into_inner()
        .unwrap()
        .iter()
        .zip([0.2, 0.2, 0.24, 0.24])
    {
        assert!((value - expected).abs() < 0.001, "{value}");
    }
    for i in 0..2 {
        assert!((report.measurements[0][i].unwrap() - 0.2).abs() < 0.001);
        assert!((report.measurements[1][i].unwrap() - 0.24).abs() < 0.001);
        assert!((report.statistics[i].as_ref().unwrap().mean - 0.22).abs() < 0.001);
    }
}

#[tokio::test(start_paused = true)]
async fn slow_actuation_and_homing_complete() {
    let machine = fixture(Behaviour::SlowActuationAndHoming);
    let mut report = RepeatabilityReport::default();
    run_repeatability(
        &machine,
        &RepeatabilityOptions {
            repetitions: 1,
            home: true,
            ..Default::default()
        },
        settings(),
        &mut report,
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(report.measurements.len(), 1);
}
fn fixture(behaviour: Behaviour) -> ControllerFixture {
    ControllerFixture {
        inner: machine(),
        behaviour,
        fine_completed: AtomicBool::new(false),
    }
}

#[tokio::test]
async fn fine_reading_survives_a_failed_backoff() {
    let machine = fixture(Behaviour::FailBackoff);
    let mut report = RepeatabilityReport::default();
    let error = run_repeatability(
        &machine,
        &RepeatabilityOptions::default(),
        settings(),
        &mut report,
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap_err();
    assert_eq!(error, Error::Controller("backoff failed".into()));
    assert!(report.measurements[0][0].unwrap().abs() < 0.001);
    assert_eq!(report.statistics[0].as_ref().unwrap().count, 1);
}

#[tokio::test]
async fn homing_implies_retraction_at_the_measuring_point() {
    let machine = fixture(Behaviour::HomeRetracts);
    let options = RepeatabilityOptions {
        repetitions: 2,
        home: true,
        retract: false,
        ..Default::default()
    };
    let mut report = RepeatabilityReport::default();
    run_repeatability(
        &machine,
        &options,
        settings(),
        &mut report,
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(
        machine
            .inner
            .commands()
            .iter()
            .filter(|c| *c == "M122")
            .count(),
        2
    );
    assert_eq!(
        machine
            .inner
            .commands()
            .iter()
            .filter(|c| *c == "M121")
            .count(),
        2
    );
    assert_eq!(report.measurements.len(), 2);
    let commands = machine.inner.commands();
    for cycle in commands.split(|command| command == "$H").skip(1) {
        let extension = cycle.iter().position(|command| command == "M122").unwrap();
        let positioning = cycle
            .iter()
            .position(|command| command.starts_with("G38.3 ") || command.starts_with("G1 "))
            .unwrap();
        assert!(
            extension < positioning,
            "extend before positioning after homing: {cycle:?}"
        );
        assert!(cycle[positioning].starts_with("G38.3 "));
    }
}

#[tokio::test(start_paused = true)]
async fn homing_needs_a_moving_to_ready_transition() {
    let machine = fixture(Behaviour::HomeReadyOnly);
    let options = RepeatabilityOptions {
        repetitions: 1,
        home: true,
        ..Default::default()
    };
    let mut report = RepeatabilityReport::default();
    let result = run_repeatability(
        &machine,
        &options,
        settings(),
        &mut report,
        CancellationToken::new(),
        |_| {},
    )
    .await;
    assert_eq!(result, Err(Error::Timeout));
    assert!(report.measurements.is_empty());
}

#[tokio::test]
async fn retracted_start_extends_at_the_measuring_point_before_contact() {
    let machine = fixture(Behaviour::HomeRetracts);
    machine.inner.set_extended(false).unwrap();
    let options = RepeatabilityOptions {
        repetitions: 1,
        retract: false,
        ..Default::default()
    };
    run_repeatability(
        &machine,
        &options,
        settings(),
        &mut RepeatabilityReport::default(),
        CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    let commands = machine.inner.commands();
    let extension = commands.iter().position(|c| c == "M122").unwrap();
    let movement = commands.iter().position(|c| c.starts_with("G38")).unwrap();
    assert!(commands.iter().position(|c| c.starts_with("G1 ")).unwrap() < extension);
    assert!(extension < movement);
}
