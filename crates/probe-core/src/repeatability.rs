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

use crate::{
    runtime::{set_modes, Cancellable, Observed},
    *,
};
use std::{sync::atomic::AtomicBool, time::Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepeatabilityOptions {
    pub axes: [bool; 3],
    pub repetitions: usize,
    pub home: bool,
    pub retract: bool,
}
impl Default for RepeatabilityOptions {
    fn default() -> Self {
        Self {
            axes: [true; 3],
            repetitions: 5,
            home: false,
            retract: true,
        }
    }
}
impl RepeatabilityOptions {
    pub fn validate(&self) -> Result<(), Error> {
        if !self.axes.iter().any(|v| *v) || !(1..=100).contains(&self.repetitions) {
            return Err(Error::InvalidConfig(
                "select at least one axis and 1..100 repetitions".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RepeatabilitySettings {
    pub diameter: f64,
    pub retract: f64,
    pub positioning_feed: f64,
    pub coarse_feed: f64,
    pub fine_feed: f64,
}
impl RepeatabilitySettings {
    pub fn validate(self) -> Result<(), Error> {
        for (value, parameter) in [
            (self.diameter, Parameter::Diameter),
            (self.retract, Parameter::Retract),
            (self.positioning_feed, Parameter::PositioningFeed),
            (self.coarse_feed, Parameter::ProbeFeed),
            (self.fine_feed, Parameter::ProbeFeed),
        ] {
            if !parameter.range().contains(value) {
                return Err(Error::InvalidConfig(
                    "invalid repeatability probing settings".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisStatistics {
    pub count: usize,
    pub mean: f64,
    pub median: f64,
    pub stddev: Option<f64>,
    pub range: f64,
}
impl AxisStatistics {
    pub fn from_readings(mut values: Vec<f64>) -> Option<Self> {
        if values.is_empty() || values.iter().any(|v| !v.is_finite()) {
            return None;
        }
        values.sort_by(f64::total_cmp);
        let count = values.len();
        let mean = values.iter().sum::<f64>() / count as f64;
        Some(Self {
            count,
            mean,
            median: (values[(count - 1) / 2] + values[count / 2]) / 2.0,
            stddev: (count > 1).then(|| {
                (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (count - 1) as f64).sqrt()
            }),
            range: values[count - 1] - values[0],
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepeatabilityReport {
    pub measurements: Vec<[Option<f64>; 3]>,
    pub statistics: [Option<AxisStatistics>; 3],
}
impl RepeatabilityReport {
    fn record(&mut self, repetition: usize, axis: Axis, value: f64) {
        self.measurements.resize(repetition + 1, [None; 3]);
        self.measurements[repetition][axis.index()] = Some(value);
        self.statistics = std::array::from_fn(|i| {
            AxisStatistics::from_readings(
                self.measurements.iter().filter_map(|row| row[i]).collect(),
            )
        });
    }
}

pub enum RepeatabilityEvent {
    Progress(Progress),
    Measurement {
        repetition: usize,
        axis: Axis,
        value: f64,
        statistics: [Option<AxisStatistics>; 3],
    },
}

#[async_trait]
pub trait RepeatabilityController: Controller {
    /// Submit actuator movement. The routine waits for the reported end state.
    async fn set_probe(&self, extended: bool) -> Result<(), Error>;
    /// Firmware $202, also carried in the fourth field of TOOL reports.
    fn probe_reference_z(&self) -> Result<f64, Error>;
}

#[async_trait]
impl RepeatabilityController for MockController {
    async fn set_probe(&self, extended: bool) -> Result<(), Error> {
        self.send(if extended { "M122" } else { "M121" }).await
    }
    fn probe_reference_z(&self) -> Result<f64, Error> {
        MockController::probe_reference_z(self)
            .ok_or_else(|| Error::Preflight("unknown probe Z reference".into()))
    }
}

#[async_trait]
impl<C: RepeatabilityController + ?Sized> RepeatabilityController for Cancellable<'_, C> {
    async fn set_probe(&self, extended: bool) -> Result<(), Error> {
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => Err(Error::Cancelled),
            result = self.inner.set_probe(extended) => result,
        }
    }
    fn probe_reference_z(&self) -> Result<f64, Error> {
        self.inner.probe_reference_z()
    }
}

pub fn check_repeatability<C: RepeatabilityController + ?Sized>(
    c: &C,
    options: &RepeatabilityOptions,
    settings: RepeatabilitySettings,
) -> Result<(), Error> {
    options.validate()?;
    settings.validate()?;
    let state = c.state();
    preflight_machine(&state)?;
    if !state.homed
        || state.wcs != 54
        || !finite(state.work_position)
        || !finite(state.probe_offset)
    {
        return Err(Error::Preflight(
            "home the machine and select the zeroed G54 first".into(),
        ));
    }
    let target = measurement_start(&state, c.probe_reference_z()?);
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let i = axis.index();
        check_path(
            &state,
            axis,
            state.position[i].min(target[i]),
            state.position[i].max(target[i]),
        )?;
        if options.axes[i] {
            let mut at_target = state.clone();
            at_target.position = target;
            // Contact planning is for the extended probe at the measuring point.
            at_target.probe_extended = true;
            preview_contact(
                &at_target,
                contact_settings(axis, settings),
                TimingPolicy::default(),
            )?;
        }
    }
    Ok(())
}

fn measurement_start(s: &State, reference_z: f64) -> Position {
    let mut target = s.position;
    for (i, coordinate) in target.iter_mut().enumerate().take(2) {
        *coordinate = s.position[i] - s.work_position[i] + 15.0 - s.probe_offset[i];
    }
    target[2] = s.position[2] - s.work_position[2] + 5.0 - reference_z + s.probe_offset[2];
    target
}

fn contact_settings(axis: Axis, settings: RepeatabilitySettings) -> ContactConfig {
    ContactConfig {
        axis,
        direction: -1,
        // Allow half a millimeter beyond the zeroed surface.
        coarse_travel: if axis == Axis::Z { 5.5 } else { 15.5 },
        retract_distance: settings.retract,
        coarse_feed: settings.coarse_feed,
        fine_feed: settings.fine_feed,
        retract_feed: settings.positioning_feed,
    }
}

async fn actuate<C: RepeatabilityController + ?Sized>(c: &C, extended: bool) -> Result<(), Error> {
    let already_at_target = c.state().probe_extended == extended;
    let mut events = c.subscribe();
    tokio::time::timeout(Duration::from_secs(120), async {
        c.set_probe(extended).await?;
        if already_at_target {
            return Ok(());
        }
        loop {
            if let Some(status) = receive(&mut events).await?.status {
                if status.ready
                    && status.probe_actuator_known
                    && status.probe_actuator == i32::from(extended)
                {
                    return Ok(());
                }
            }
        }
    })
    .await
    .map_err(|_| Error::Timeout)?
}

async fn home<C: Controller + ?Sized>(c: &C) -> Result<(), Error> {
    for (command, coordinates) in [
        ("G90 G53 G0 Z0", &[(2, 0.0)][..]),
        ("G90 G53 G0 X-20 Y-5", &[(0, -20.0), (1, -5.0)][..]),
    ] {
        let state = c.state();
        preflight_machine(&state)?;
        let mut target = state.position;
        for &(axis, coordinate) in coordinates {
            target[axis] = coordinate;
        }
        let mut events = c.subscribe();
        tokio::time::timeout(Duration::from_secs(120), async {
            c.send(command).await?;
            loop {
                if let Some(status) = receive(&mut events).await?.status {
                    if status.wcs != state.wcs {
                        return Err(Error::Position("WCS changed before homing".into()));
                    }
                    if status.ready && within(status.position, target, 0.05) {
                        return Ok(());
                    }
                }
            }
        })
        .await
        .map_err(|_| Error::Timeout)??;
    }
    let mut events = c.subscribe();
    tokio::time::timeout(Duration::from_secs(600), async {
        c.send("$H").await?;
        let mut moving = false;
        loop {
            if let Some(status) = receive(&mut events).await?.status {
                moving |= !status.ready;
                if moving && status.ready && c.state().homed {
                    return Ok(());
                }
            }
        }
    })
    .await
    .map_err(|_| Error::Timeout)?
}

async fn move_to<C: Controller + ?Sized>(
    c: &C,
    target: Position,
    settings: RepeatabilitySettings,
) -> Result<(), Error> {
    let start = c.state().position;
    let delta = std::array::from_fn(|i| {
        // Status coordinates are rounded to 0.001 mm; ignore one reporting increment.
        if i < 3 && quantize(target[i] - start[i]).abs() > 0.001 {
            quantize(target[i] - start[i])
        } else {
            0.0
        }
    });
    if delta.iter().any(|v| *v != 0.0) {
        let stage = if c.state().probe_extended {
            positioning_stage(delta, settings.positioning_feed)
        } else {
            Stage::movement(StageKind::PositionMove, delta, settings.positioning_feed)
        };
        run_position_stage(c, stage, settings.retract, TimingPolicy::default()).await?;
    }
    Ok(())
}

/// Measurements are retained in `report` even when a later movement fails.
pub async fn run_repeatability<C: RepeatabilityController + ?Sized>(
    c: &C,
    options: &RepeatabilityOptions,
    settings: RepeatabilitySettings,
    report: &mut RepeatabilityReport,
    cancel: CancellationToken,
    observe: impl Fn(RepeatabilityEvent) + Send + Sync,
) -> Result<(), Error> {
    check_repeatability(c, options, settings)?;
    let c = Cancellable {
        inner: c,
        cancel: &cancel,
    };
    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(Error::Cancelled),
        result = repeat_inner(&c, options, settings, report, &observe) => result,
    }
}

async fn repeat_inner<C: RepeatabilityController + ?Sized>(
    machine: &C,
    options: &RepeatabilityOptions,
    settings: RepeatabilitySettings,
    report: &mut RepeatabilityReport,
    observe: &(impl Fn(RepeatabilityEvent) + Send + Sync),
) -> Result<(), Error> {
    let start = machine.state();
    let reference_z = machine.probe_reference_z()?;
    let target = measurement_start(&start, reference_z);
    let origin: Position = std::array::from_fn(|i| start.position[i] - start.work_position[i]);
    let progress = |p| observe(RepeatabilityEvent::Progress(p));
    let scripted = AtomicBool::new(false);
    let c = Observed {
        inner: machine,
        observe: &progress,
        scripted: &scripted,
    };
    let comment = |message: String| {
        progress(Progress {
            kind: "script".into(),
            command: String::new(),
            message: format!("; {message}"),
        })
    };
    let modes = query_modes(&c).await?;
    set_modes(&c, Modes::PROBING).await?;
    let execution = async {
        for repetition in 0..options.repetitions {
            comment(format!(
                "Repetition {} of {}",
                repetition + 1,
                options.repetitions
            ));
            if c.state().probe_extended && (options.home || (repetition > 0 && options.retract)) {
                let mut lateral = c.state().position;
                lateral[0] = target[0];
                lateral[1] = target[1];
                comment("Move probe tip to G54 X15 Y15".into());
                move_to(&c, lateral, settings).await?;
                comment("Retract probe".into());
                actuate(machine, false).await?;
            }
            if options.home {
                comment("Home machine".into());
                home(&c).await?;
                set_modes(&c, Modes::PROBING).await?;
                comment("Extend probe".into());
                actuate(machine, true).await?;
            }
            let state = c.state();
            let current_origin =
                std::array::from_fn(|i| state.position[i] - state.work_position[i]);
            if state.wcs != 54
                || !within(origin, current_origin, 0.01)
                || !within(start.probe_offset, state.probe_offset, 0.001)
                || (machine.probe_reference_z()? - reference_z).abs() > 0.001
            {
                return Err(Error::Preflight(
                    "G54 or probe calibration changed during the check".into(),
                ));
            }
            preflight_machine(&c.state())?;
            let mut lateral = c.state().position;
            lateral[0] = target[0];
            lateral[1] = target[1];
            comment("Move probe tip to G54 X15 Y15".into());
            move_to(&c, lateral, settings).await?;
            comment("Move probe tip to G54 Z5".into());
            move_to(&c, target, settings).await?;
            if !c.state().probe_extended {
                comment("Extend probe".into());
                actuate(machine, true).await?;
            }
            preflight(&c.state())?;
            for axis in [Axis::X, Axis::Y, Axis::Z] {
                if !options.axes[axis.index()] {
                    continue;
                }
                comment(format!("Measure {axis}-"));
                let mut converted = None;
                run_contact_with_reading(
                    &c,
                    contact_settings(axis, settings),
                    TimingPolicy::default(),
                    |contact| {
                        let coordinate = surface_machine_coordinate(
                            contact,
                            axis,
                            -1,
                            settings.diameter,
                            start.probe_offset,
                        )
                        .map(|surface| surface - origin[axis.index()]);
                        if let Ok(value) = coordinate {
                            report.record(repetition, axis, value);
                            observe(RepeatabilityEvent::Measurement {
                                repetition: repetition + 1,
                                axis,
                                value,
                                statistics: report.statistics.clone(),
                            });
                        }
                        converted = Some(coordinate);
                    },
                )
                .await?;
                converted.ok_or(Error::NoContact)??;
                comment("Return to measurement start".into());
                move_to(&c, target, settings).await?;
            }
        }
        Ok(())
    }
    .await;
    if execution.is_ok()
        || matches!(
            execution,
            Err(Error::CoarseNoContact
                | Error::UnexpectedContact {
                    retracted: true,
                    ..
                })
        )
    {
        if let Err(recovery) = set_modes(&c, modes).await {
            return Err(match execution {
                Ok(()) => recovery,
                Err(cause) => Error::Recovery {
                    cause: Box::new(cause),
                    recovery: Box::new(recovery),
                },
            });
        }
    }
    execution
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn positioning_waits_for_a_delayed_completion_report() {
        let mut state = MockController::new().state();
        state.modes = Modes::PROBING;
        let c = MockController::with_state(state).with_delay(Duration::from_secs(3));
        c.set_extended(true).unwrap();
        let mut target = c.state().position;
        target[2] += 4.519;
        move_to(
            &c,
            target,
            RepeatabilitySettings {
                diameter: 2.0,
                retract: 0.5,
                positioning_feed: 1000.0,
                coarse_feed: 300.0,
                fine_feed: 50.0,
            },
        )
        .await
        .unwrap();
        assert!(within(c.state().position, target, 0.001));
    }

    #[tokio::test]
    async fn positioning_accepts_one_reporting_increment_at_the_target() {
        let c = MockController::new();
        let mut target = c.state().position;
        target[1] += 0.001;
        move_to(
            &c,
            target,
            RepeatabilitySettings {
                diameter: 2.0,
                retract: 0.5,
                positioning_feed: 1000.0,
                coarse_feed: 300.0,
                fine_feed: 50.0,
            },
        )
        .await
        .unwrap();
        assert!(c.commands().is_empty());
    }
}
