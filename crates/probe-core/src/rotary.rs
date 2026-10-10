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

use crate::{
    runtime::{set_modes, Cancellable, Observed},
    *,
};
use std::{sync::atomic::AtomicBool, time::Duration};

mod level;
pub use level::LevelResult;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RotaryOperation {
    #[default]
    Axis,
    Horizontal,
    Vertical,
    VerticalNegative,
}
impl RotaryOperation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Axis => "Rotary axis calibration",
            Self::Horizontal => "Level horizontal surface",
            Self::Vertical => "Align vertical surface toward Y+",
            Self::VerticalNegative => "Align vertical surface toward Y-",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct RotaryConfig {
    pub operation: RotaryOperation,
    pub y_distance: f64,
    pub z_distance: f64,
    pub rod_diameter: f64,
    pub x_distance: f64,
    pub rotary_feed: f64,
    pub diameter: f64,
    pub retract: f64,
    pub positioning_feed: f64,
    pub coarse_feed: f64,
    pub fine_feed: f64,
}
impl Default for RotaryConfig {
    fn default() -> Self {
        Self {
            operation: RotaryOperation::Axis,
            y_distance: 10.0,
            z_distance: 10.0,
            rod_diameter: 10.0,
            x_distance: 30.0,
            rotary_feed: 5000.0,
            diameter: 2.0,
            retract: 0.5,
            positioning_feed: 1000.0,
            coarse_feed: 300.0,
            fine_feed: 50.0,
        }
    }
}
impl RotaryConfig {
    pub fn validate(&self) -> Result<(), Error> {
        let validate = |name, value: f64, lo: f64, hi: f64| {
            if !value.is_finite() || !(lo..=hi).contains(&value) {
                let requirement = if hi == f64::MAX {
                    format!("at least {lo}")
                } else {
                    format!("between {lo} and {hi}")
                };
                Err(Error::InvalidConfig(format!(
                    "{name} must be {requirement}."
                )))
            } else {
                Ok(())
            }
        };
        for (name, value, lo, hi) in [
            ("Rotary feed", self.rotary_feed, 1.0, f64::MAX),
            ("Probe ball diameter", self.diameter, 0.1, 20.0),
            ("Backoff", self.retract, 0.1, 10.0),
            ("Positioning feed", self.positioning_feed, 1.0, f64::MAX),
            ("Coarse feed", self.coarse_feed, 1.0, f64::MAX),
            ("Fine feed", self.fine_feed, 1.0, f64::MAX),
        ] {
            validate(name, value, lo, hi)?;
        }
        if self.operation == RotaryOperation::Axis {
            validate("Rod diameter", self.rod_diameter, 3., 100.)?;
            if !self.x_distance.is_finite() || !(1.0..=200.0).contains(&self.x_distance.abs()) {
                return Err(Error::InvalidConfig(
                    "X distance must be between 1 and 200 mm in either direction".into(),
                ));
            }
        } else {
            validate("Y distance", self.y_distance, 0.1, 100.)?;
            validate("Z distance", self.z_distance, 0.1, 100.)?;
        }
        Ok(())
    }
}

pub fn rotary_supported(version: &str) -> bool {
    version.split_once('+').is_some_and(|(base, suffix)| {
        !suffix.is_empty()
            && base.split('.').count() == 3
            && base.split('.').all(|part| part.parse::<u32>().is_ok())
    })
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotaryStation {
    pub center: [f64; 3],
    pub crest: [f64; 2],
    pub sides: [f64; 2],
    pub radius: f64,
}

pub fn rotary_station(
    x: f64,
    crest_y: f64,
    crest_z: f64,
    right: f64,
    left: f64,
) -> Result<RotaryStation, Error> {
    let radius = (right - left) / 2.0;
    if ![x, crest_y, crest_z, right, left]
        .iter()
        .all(|v| v.is_finite())
        || radius <= 0.0
    {
        return Err(Error::Compensation(
            "The side measurements give a zero or negative radius. Check the rod diameter and probe calibration.".into(),
        ));
    }
    Ok(RotaryStation {
        center: [x, (right + left) / 2.0, crest_z - radius],
        crest: [crest_y, crest_z],
        sides: [left, right],
        radius,
    })
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotaryResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<LevelResult>,
    pub stations: Vec<RotaryStation>,
    pub xy_angle: f64,
    pub xz_angle: f64,
    pub wcs: i32,
    pub rotation_supported: bool,
    pub zeroed: bool,
    pub rotation_applied: bool,
}
impl RotaryResult {
    pub fn from_stations(
        stations: [RotaryStation; 2],
        wcs: i32,
        rotation_supported: bool,
    ) -> Result<Self, Error> {
        let d: [f64; 3] = std::array::from_fn(|i| stations[1].center[i] - stations[0].center[i]);
        if !d.iter().all(|v| v.is_finite()) || d[0].abs() < 1.0 {
            return Err(Error::Compensation(
                "Measure the axis center at two different X positions.".into(),
            ));
        }
        Ok(Self {
            xy_angle: (d[1] / d[0]).atan().to_degrees(),
            xz_angle: (d[2] / d[0]).atan().to_degrees(),
            stations: stations.into(),
            wcs,
            rotation_supported,
            ..Self::default()
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RotaryPlan {
    pub start: State,
    pub config: RotaryConfig,
}
pub fn review_rotary(start: State, config: RotaryConfig) -> Result<RotaryPlan, Error> {
    config.validate()?;
    preflight(&start)?;
    if !start.homed || !start.modes.valid() {
        return Err(Error::Preflight(
            "Home the machine before calibrating the rotary axis.".into(),
        ));
    }
    if !matches!(start.plane, 17..=19) {
        return Err(Error::Preflight(
            "The machine did not report the active G-code plane. Reconnect and try again.".into(),
        ));
    }
    if rotary_supported(&start.firmware_version) && start.wcs_rotation.is_none() {
        return Err(Error::Preflight(
            "The machine did not report the work coordinate rotation. Reconnect and try again."
                .into(),
        ));
    }
    if config.operation != RotaryOperation::Axis {
        level::check_review(&start, &config)?;
        return Ok(RotaryPlan { start, config });
    }
    check_path(
        &start,
        Axis::X,
        start.position[0].min(start.position[0] + config.x_distance),
        start.position[0].max(start.position[0] + config.x_distance),
    )?;
    Ok(RotaryPlan { start, config })
}

pub async fn query_rotary_state<C: Controller + ?Sized>(c: &C) -> Result<State, Error> {
    query_modes(c).await?;
    let mut rx = c.subscribe();
    tokio::time::timeout(Duration::from_secs(3), async {
        c.send("$V").await?;
        loop {
            if receive(&mut rx).await?.firmware_version.is_some() {
                break;
            }
        }
        query_coordinates(c).await
    })
    .await
    .map_err(|_| Error::Timeout)?
}

/// Read the active work origin, G92 offset and tool-length compensation.
pub async fn query_coordinates<C: Controller + ?Sized>(c: &C) -> Result<State, Error> {
    let mut rx = c.subscribe();
    let wcs = c.state().wcs;
    let mut received = [
        false,
        !rotary_supported(&c.state().firmware_version),
        false,
        false,
        false,
    ];
    tokio::time::timeout(Duration::from_secs(3), async {
        c.send("$#").await?;
        while !received.into_iter().all(|value| value) {
            let event = receive(&mut rx).await?;
            received[0] |= event
                .wcs_origin
                .is_some_and(|(selected, _)| selected == wcs);
            received[1] |= event
                .wcs_rotation
                .is_some_and(|(selected, _)| selected == wcs);
            received[2] |= event.coordinate_offset.is_some();
            received[3] |= event.tool_length_offset.is_some();
            received[4] |= event.probe.is_some();
        }
        Ok(c.state())
    })
    .await
    .map_err(|_| Error::Preflight("The machine did not report its work coordinates.".into()))?
}

fn comment(observe: &impl Fn(Progress), text: impl Into<String>) {
    observe(Progress {
        kind: "script".into(),
        message: format!("; {}", text.into()),
        ..Progress::default()
    });
}
fn tip(s: &State) -> Position {
    [
        s.position[0] + s.probe_offset[0],
        s.position[1] + s.probe_offset[1],
        s.position[2] - s.probe_offset[2],
        s.position[3],
    ]
}
fn machine_tip(s: &State, p: Position) -> Position {
    [
        p[0] - s.probe_offset[0],
        p[1] - s.probe_offset[1],
        p[2] + s.probe_offset[2],
        p[3],
    ]
}

async fn move_tip<C: Controller + ?Sized>(
    c: &C,
    p: Position,
    config: &RotaryConfig,
) -> Result<(), Error> {
    let state = c.state();
    let target = machine_tip(&state, p);
    let delta = std::array::from_fn(|i| quantize(target[i] - state.position[i]));
    if delta[..3].iter().all(|v| v.abs() <= 0.001) {
        return Ok(());
    }
    run_position_stage(
        c,
        positioning_stage(delta, config.positioning_feed),
        config.retract,
        TimingPolicy::default(),
    )
    .await?;
    Ok(())
}
async fn touch<C: Controller + ?Sized>(
    c: &C,
    axis: Axis,
    direction: i32,
    travel: f64,
    config: &RotaryConfig,
    observe: &impl Fn(Progress),
) -> Result<f64, Error> {
    let result = run_contact(
        c,
        ContactConfig {
            axis,
            direction,
            coarse_travel: travel,
            retract_distance: config.retract,
            coarse_feed: config.coarse_feed,
            fine_feed: config.fine_feed,
            retract_feed: config.positioning_feed,
        },
        TimingPolicy::default(),
    )
    .await?;
    observe(Progress {
        kind: "contact".into(),
        measurement: Some(format!("rotary {axis}")),
        contact: Some(result.contact.clone()),
        ..Progress::default()
    });
    surface_machine_coordinate(
        &result.contact,
        axis,
        direction,
        config.diameter,
        c.state().probe_offset,
    )
}

fn orbit(center: [f64; 2], apex: Position, degrees: f64, ball_radius: f64) -> Position {
    let angle = degrees.to_radians();
    let y = apex[1] - center[0];
    let z = apex[2] + ball_radius - center[1];
    [
        apex[0],
        center[0] + y * angle.cos() - z * angle.sin(),
        center[1] + y * angle.sin() + z * angle.cos() - ball_radius,
        apex[3] + degrees,
    ]
}
async fn track<C: Controller + ?Sized>(
    c: &C,
    center: [f64; 2],
    apex: Position,
    from: f64,
    to: f64,
    config: &RotaryConfig,
    native: bool,
) -> Result<(), Error> {
    let count = ((to - from).abs() / 5.0).ceil() as usize;
    let state = c.state();
    let mut angles = vec![from.to_radians(), to.to_radians()];
    let y = apex[1] - center[0];
    let z = apex[2] + config.diameter / 2.0 - center[1];
    let low = from.min(to).to_radians();
    let high = from.max(to).to_radians();
    // Include the extrema between endpoints, not just the arc's chord.
    for critical in [(-z).atan2(y), y.atan2(z)] {
        for half_turn in -2..=2 {
            let angle = critical + f64::from(half_turn) * std::f64::consts::PI;
            if (low..=high).contains(&angle) {
                angles.push(angle);
            }
        }
    }
    for angle in angles {
        let target = machine_tip(
            &state,
            orbit(center, apex, angle.to_degrees(), config.diameter / 2.0),
        );
        for axis in [Axis::Y, Axis::Z] {
            let j = axis.index();
            check_path(&state, axis, target[j], target[j])?;
        }
    }
    let steps = if native { 1 } else { count };
    for i in 1..=steps {
        let state = c.state();
        let target = machine_tip(
            &state,
            orbit(
                center,
                apex,
                from + (to - from) * i as f64 / steps as f64,
                config.diameter / 2.0,
            ),
        );
        let delta = std::array::from_fn(|j| quantize(target[j] - state.position[j]));
        let mut stage = Stage::movement(
            if native {
                StageKind::ArcMove
            } else {
                StageKind::GuardedMove
            },
            delta,
            config.positioning_feed,
        );
        if native {
            let start = tip(&state);
            let radius = (start[1] - center[0]).hypot(start[2] + config.diameter / 2.0 - center[1]);
            let length = radius * ((to - from).to_radians()).abs();
            stage.distance = length.hypot(to - from);
            let minutes =
                (length / config.positioning_feed).max((to - from).abs() / config.rotary_feed);
            stage.feed = stage.distance / minutes;
            stage.command = format!(
                "G19 {} Y{:.3} Z{:.3} J{:.3} K{:.3} A{:.3} F{:.3}",
                if to > from { "G3" } else { "G2" },
                delta[1],
                delta[2],
                center[0] - start[1],
                center[1] - start[2] - config.diameter / 2.0,
                delta[3],
                stage.feed
            );
            execute(c, &stage, TimingPolicy::default(), false).await?;
        } else {
            // GRBL's path length includes A in degrees alongside XYZ in mm.
            let linear = delta[1].hypot(delta[2]);
            let minutes =
                (linear / config.positioning_feed).max(delta[3].abs() / config.rotary_feed);
            stage.feed = stage.distance / minutes;
            stage.command = format!(
                "G38.3 Y{:.3} Z{:.3} A{:.3} F{:.3}",
                delta[1], delta[2], delta[3], stage.feed
            );
            run_position_stage(c, stage, config.retract, TimingPolicy::default()).await?;
        }
    }
    Ok(())
}

async fn station<C: Controller + ?Sized>(
    c: &C,
    config: &RotaryConfig,
    clearance_z: f64,
    a: f64,
    native: bool,
    observe: &impl Fn(Progress),
) -> Result<RotaryStation, Error> {
    let search = config.rod_diameter * 2.0;
    comment(
        observe,
        "Locate the rod surface for the initial ridge height",
    );
    let rough_top = touch(c, Axis::Z, -1, search, config, observe).await?;
    let original = tip(&c.state());
    let headroom = clearance_z - rough_top;
    let reach = config.rod_diameter / 2.0 + config.diameter / 2.0 + headroom.max(config.retract);
    let mut sides = [0.0; 2];
    for (index, sign) in [-1.0, 1.0].into_iter().enumerate() {
        comment(observe, format!("Initial Y ridge, side {}", index + 1));
        let mut p = tip(&c.state());
        p[2] = clearance_z;
        move_tip(c, p, config).await?;
        p[1] = original[1] + sign * reach;
        move_tip(c, p, config).await?;
        // Keep the initial chord above the equator so the shaft clears the rod.
        p[2] = rough_top - config.rod_diameter * 0.3 - config.diameter / 2.0;
        move_tip(c, p, config).await?;
        sides[index] = touch(c, Axis::Y, -sign as i32, search, config, observe).await?;
    }
    let crest_y = (sides[0] + sides[1]) / 2.0;
    let mut p = tip(&c.state());
    p[2] = clearance_z;
    move_tip(c, p, config).await?;
    p[1] = crest_y;
    move_tip(c, p, config).await?;
    comment(observe, "Measure the true crest");
    let crest_z = touch(c, Axis::Z, -1, search, config, observe).await?;
    let mut center = [crest_y, crest_z - config.rod_diameter / 2.0];
    let mut measured = None;
    for pass in 0..4 {
        comment(observe, format!("Rotated crest pair {}", pass + 1));
        if clearance_z <= crest_z {
            return Err(Error::Preflight(
                "Position the probe above the top of the rod before calibration.".into(),
            ));
        }
        let apex = [original[0], crest_y, clearance_z, a];
        let first_from = if pass == 0 { 0.0 } else { -90.0 };
        // Apply the refined clearance orbit at the side, without stopping at
        // the crest between measurement pairs.
        move_tip(
            c,
            orbit(center, apex, first_from, config.diameter / 2.0),
            config,
        )
        .await?;
        for (index, (from, angle)) in [(first_from, 90.0), (90.0, -90.0)].into_iter().enumerate() {
            track(c, center, apex, from, angle, config, native).await?;
            let side_start = tip(&c.state());
            sides[index] = touch(
                c,
                Axis::Y,
                if angle > 0.0 { 1 } else { -1 },
                search,
                config,
                observe,
            )
            .await?;
            move_tip(c, side_start, config).await?;
        }
        let reading = rotary_station(original[0], crest_y, crest_z, sides[1], sides[0])?;
        let refined = [reading.center[1], reading.center[2]];
        let error = (refined[0] - center[0]).hypot(refined[1] - center[1]);
        measured = Some(reading);
        if pass >= 1 && error < 0.003 {
            track(c, center, apex, -90.0, 0.0, config, native).await?;
            break;
        }
        if pass == 3 && error >= 0.003 {
            track(c, center, apex, -90.0, 0.0, config, native).await?;
            return Err(Error::Compensation(
                "Repeated measurements did not agree on the rotary center. Check the rod diameter and starting height.".into(),
            ));
        }
        center = refined;
    }
    Ok(measured.unwrap())
}

pub async fn run_rotary<C: Controller + ?Sized>(
    machine: &C,
    plan: &RotaryPlan,
    report: &mut RotaryResult,
    cancel: CancellationToken,
    observe: impl Fn(Progress) + Send + Sync,
) -> Result<(), Error> {
    preflight(&machine.state())?;
    if review_rotary(plan.start.clone(), plan.config.clone())? != *plan
        || !within(machine.state().position, plan.start.position, 0.05)
        || machine.state().wcs != plan.start.wcs
        || !within(machine.state().probe_offset, plan.start.probe_offset, 0.001)
    {
        return Err(Error::Preflight(
            "The machine position or probe calibration changed. Start the operation again.".into(),
        ));
    }
    let guarded = Cancellable {
        inner: machine,
        cancel: &cancel,
    };
    let scripted = AtomicBool::new(false);
    let c = Observed {
        inner: &guarded,
        observe: &observe,
        scripted: &scripted,
    };
    tokio::select! {
        biased;
        _=cancel.cancelled()=>Err(Error::Cancelled),
        result=run_inner(&c,plan,report,&observe)=>result,
    }
}
async fn run_inner<C: Controller + ?Sized>(
    c: &C,
    plan: &RotaryPlan,
    report: &mut RotaryResult,
    observe: &(impl Fn(Progress) + Send + Sync),
) -> Result<(), Error> {
    let config = &plan.config;
    let native = rotary_supported(&plan.start.firmware_version);
    report.wcs = plan.start.wcs;
    report.rotation_supported = native;
    let start = tip(&plan.start);
    let execution = async {
        set_modes(c, Modes::PROBING).await?;
        if native {
            write_rotary_rotation(c, plan.start.wcs, 0.0).await?;
        }
        if config.operation != RotaryOperation::Axis {
            report.level = Some(level::run(c, plan, observe).await?);
            return Ok(());
        }
        for index in 0..2 {
            comment(observe, format!("Station {}", index + 1));
            if index == 1 {
                let mut p = tip(&c.state());
                p[2] = start[2];
                move_tip(c, p, config).await?;
                p[0] = start[0] + config.x_distance;
                move_tip(c, p, config).await?;
            }
            let value = station(c, config, start[2], start[3], native, observe).await?;
            report.stations.push(value);
        }
        *report = RotaryResult::from_stations(
            report.stations.clone().try_into().unwrap(),
            plan.start.wcs,
            native,
        )?;
        let mut p = tip(&c.state());
        p[2] = start[2];
        move_tip(c, p, config).await?;
        comment(observe, "Return X to the first station");
        p[0] = start[0];
        move_tip(c, p, config).await?;
        Ok(())
    }
    .await;
    if !c.state().motion_blocked
        && !matches!(execution, Err(Error::Cancelled | Error::Disconnected))
    {
        if let Err(recovery) = restore_rotary_state(c, plan).await {
            return Err(match execution {
                Err(cause) => Error::Recovery {
                    cause: Box::new(cause),
                    recovery: Box::new(recovery),
                },
                Ok(()) => recovery,
            });
        }
    }
    execution
}

pub async fn restore_rotary_state<C: Controller + ?Sized>(
    c: &C,
    plan: &RotaryPlan,
) -> Result<(), Error> {
    preflight_machine(&c.state())?;
    if rotary_supported(&plan.start.firmware_version) {
        write_rotary_rotation(c, plan.start.wcs, plan.start.wcs_rotation.unwrap()).await?;
    }
    c.send(&format!("G{}", plan.start.plane)).await?;
    query_modes(c).await?;
    if c.state().plane != plan.start.plane {
        return Err(Error::Controller(
            "The machine did not confirm the requested G-code plane.".into(),
        ));
    }
    set_modes(c, plan.start.modes).await
}

pub(crate) async fn write_coordinate_data<C: Controller + ?Sized>(
    c: &C,
    wcs: i32,
    axes: &[(usize, f64)],
    rotation: Option<f64>,
) -> Result<(), Error> {
    let mut command = format!("G10 L2 P{}", wcs - 53);
    for &(axis, value) in axes {
        command.push_str(&format!(" {}{value:.3}", ["X", "Y", "Z", "A"][axis]));
    }
    if let Some(angle) = rotation {
        command.push_str(&format!(" R{angle:.6}"));
    }
    let mut rx = c.subscribe();
    tokio::time::timeout(Duration::from_secs(3), async {
        c.send(&command).await?;
        c.send("$#").await?;
        let mut origin_received = axes.is_empty();
        let mut rotation_received = rotation.is_none();
        let mut probe_received = false;
        while !origin_received || !rotation_received || !probe_received {
            let event = receive(&mut rx).await?;
            // $# ends with the stored probe result. Consume it before motion
            // can mistake that historical result for its own contact.
            probe_received |= event.probe.is_some();
            if let Some((_, origin)) = event
                .wcs_origin
                .filter(|(selected, _)| *selected == wcs && !axes.is_empty())
            {
                if axes
                    .iter()
                    .any(|&(axis, value)| (origin[axis] - value).abs() > 0.0015)
                {
                    return Err(Error::Controller(
                        "The machine did not confirm the new work zero.".into(),
                    ));
                }
                origin_received = true;
            }
            if let Some(((_, actual), expected)) = event
                .wcs_rotation
                .filter(|(selected, _)| *selected == wcs)
                .zip(rotation)
            {
                let difference = (actual - expected + 180.0).rem_euclid(360.0) - 180.0;
                if difference.abs() > 0.0015 {
                    return Err(Error::Controller(
                        "The machine did not confirm the new work coordinate rotation.".into(),
                    ));
                }
                rotation_received = true;
            }
        }
        Ok(())
    })
    .await
    .map_err(|_| Error::Timeout)?
}
pub async fn write_rotary_rotation<C: Controller + ?Sized>(
    c: &C,
    wcs: i32,
    angle: f64,
) -> Result<(), Error> {
    if !rotary_supported(&c.state().firmware_version)
        || !(54..=59).contains(&wcs)
        || !angle.is_finite()
    {
        return Err(Error::Preflight(
            "This firmware does not support work coordinate rotation.".into(),
        ));
    }
    write_coordinate_data(c, wcs, &[], Some(angle)).await
}
pub async fn zero_rotary<C: Controller + ?Sized>(
    c: &C,
    result: &RotaryResult,
) -> Result<(), Error> {
    preflight_machine(&c.state())?;
    if let Some(level) = &result.level {
        if c.state().wcs != result.wcs {
            return Err(Error::Preflight(
                "Select the measured work coordinate system before setting work zero.".into(),
            ));
        }
        return level::zero(c, result.wcs, level).await;
    }
    if c.state().wcs != result.wcs {
        return Err(Error::Preflight(format!(
            "Select G{} before setting work zero.",
            result.wcs
        )));
    }
    if result.stations.len() != 2 {
        return Err(Error::Preflight(
            "No completed rotary calibration is available.".into(),
        ));
    }
    let state = query_coordinates(c).await?;
    write_rotary_zero(c, result, &state, state.wcs_rotation.unwrap_or(0.0), false).await
}

async fn write_rotary_zero<C: Controller + ?Sized>(
    c: &C,
    result: &RotaryResult,
    state: &State,
    angle: f64,
    rotate: bool,
) -> Result<(), Error> {
    let center = result.stations[0].center;
    let y = y_origin_for_point(state, center, angle)?;
    let z = center[2] - state.coordinate_offset[2];
    let modes = query_modes(c).await?;
    set_modes(c, Modes { units: 21, ..modes }).await?;
    let write =
        write_coordinate_data(c, result.wcs, &[(1, y), (2, z)], rotate.then_some(angle)).await;
    if !c.state().motion_blocked {
        set_modes(c, modes).await?;
    }
    write
}

fn y_origin_for_point(state: &State, point: [f64; 3], angle: f64) -> Result<f64, Error> {
    let (sin, cos) = angle.to_radians().sin_cos();
    if cos.abs() < 1e-6 {
        return Err(Error::Preflight("Cannot set Y zero without changing X zero at this rotation. Remove the rotation or select another work coordinate system.".into()));
    }
    let origin = state
        .wcs_origin
        .ok_or_else(|| Error::Preflight("Could not read the work zero from the machine.".into()))?;
    Ok(point[1] - sin / cos * (point[0] - origin[0]) - state.coordinate_offset[1] / cos)
}

pub async fn align_rotary<C: Controller + ?Sized>(
    c: &C,
    result: &RotaryResult,
) -> Result<(), Error> {
    preflight_machine(&c.state())?;
    if result.stations.len() != 2 || c.state().wcs != result.wcs {
        return Err(Error::Preflight(
            "Select the measured work coordinate system before setting X/Y rotation.".into(),
        ));
    }
    if !rotary_supported(&c.state().firmware_version) {
        return Err(Error::Preflight(
            "This firmware does not support work coordinate rotation.".into(),
        ));
    }
    if result.zeroed {
        let state = query_coordinates(c).await?;
        write_rotary_zero(c, result, &state, result.xy_angle, true).await
    } else {
        write_rotary_rotation(c, result.wcs, result.xy_angle).await
    }
}

impl RotaryPlan {
    pub fn program(&self) -> String {
        if self.config.operation != RotaryOperation::Axis {
            return level::program(self);
        }
        let c = &self.config;
        let mut lines = vec![
            "; Calibrate rotary Y/Z axis at two X stations".into(),
            format!("; Captured clearance Z = {} (G53)", self.start.position[2]),
            "; Dynamic coordinates below are G53 carriage positions calculated from touches".into(),
            "; current_y/current_z are captured after each completed move".into(),
            "G21 G94 G91".into(),
        ];
        if rotary_supported(&self.start.firmware_version) {
            lines.push(format!(
                "G10 L2 P{} R0 ; measure in machine axes",
                self.start.wcs - 53
            ));
        }
        for index in 0..2 {
            lines.push(format!("; Station {}", index + 1));
            if index == 1 {
                lines.push(format!(
                    "G1 Z[{} - #<current_z>] F{}",
                    self.start.position[2], c.positioning_feed
                ));
                lines.push(format!("G38.3 X{} F{}", c.x_distance, c.positioning_feed));
            }
            lines.push("; Locate the rod surface for the initial ridge height".into());
            preview_touch(&mut lines, Axis::Z, -1, c);
            lines.push("; #<rough_top_z> := contact_z".into());
            lines.push("; #<station_start_y> := Y before initial ridge".into());
            for (index, side) in ["left", "right"].into_iter().enumerate() {
                lines.push(format!("; Initial Y ridge, side {}", index + 1));
                lines.push(format!(
                    "G1 Z[{} - #<current_z>] F{}",
                    self.start.position[2], c.positioning_feed
                ));
                let reach = c.rod_diameter / 2.0 + c.diameter / 2.0;
                lines.push(format!(
                    "G38.3 Y[#<station_start_y> {} ({} + MAX({}, {} - #<rough_top_z>)) - #<current_y>] F{}",
                    if index == 0 { "-" } else { "+" },
                    reach, c.retract, self.start.position[2],
                    c.positioning_feed
                ));
                lines.push(format!(
                    "G38.3 Z[#<rough_top_z> - {} - #<current_z>] F{}",
                    c.rod_diameter * 0.3 + c.diameter / 2.0,
                    c.positioning_feed
                ));
                preview_touch(&mut lines, Axis::Y, if side == "left" { 1 } else { -1 }, c);
                lines.push(format!("; #<initial_{side}_y> := contact_y"));
            }
            lines.push("; #<crest_y> := (initial_left_y + initial_right_y) / 2".into());
            lines.push("; Raise and move above initial Y center".into());
            lines.push(format!(
                "G1 Z[{} - #<current_z>] F{}",
                self.start.position[2], c.positioning_feed
            ));
            lines.push(format!(
                "G38.3 Y[#<crest_y> - #<current_y>] F{}",
                c.positioning_feed
            ));
            lines.push("; Measure the true crest".into());
            preview_touch(&mut lines, Axis::Z, -1, c);
            lines.push("; #<crest_z> := contact_z".into());
            lines.push(format!(
                "; Initial axis estimate is the crest minus {} mm in Z",
                c.rod_diameter / 2.0
            ));
            lines.push(
                "; Refine rotated crest pairs until side heights converge (2 to 4 pairs)".into(),
            );
            lines.push("; First rotated crest pair".into());
            lines.push(format!(
                "G1 Z[{} - #<current_z>] F{}",
                self.start.position[2], c.positioning_feed
            ));
            for (from, angle) in [(0_i32, 90_i32), (90, -90)] {
                let rotation = angle - from;
                lines.push(
                    "; Tracking deltas and feed use the estimated axis, ball radius and starting Z"
                        .into(),
                );
                if rotary_supported(&self.start.firmware_version) {
                    lines.push(format!("G19 {} Y#<side_delta_y> Z#<side_delta_z> J#<axis_j> K#<axis_k> A{rotation} F#<tracking_feed>",if rotation>0 {"G3"} else {"G2"}));
                } else {
                    lines.push(format!(
                        "; {} coordinated 5-degree segments",
                        rotation.abs() / 5
                    ));
                    for _ in 0..rotation.abs() / 5 {
                        lines.push(format!(
                            "G38.3 Y#<segment_y> Z#<segment_z> A{} F#<tracking_feed>",
                            rotation.signum() * 5
                        ));
                    }
                }
                lines.push("; #<side_start_y> := Y after tracking".into());
                preview_touch(&mut lines, Axis::Y, if angle > 0 { 1 } else { -1 }, c);
                lines.push("; Back off to clearance orbit".into());
                lines.push(format!(
                    "G38.3 Y[#<side_start_y> - #<current_y>] F{}",
                    c.positioning_feed
                ));
            }
            lines.push(
                "; If another pair is needed, adjust the clearance orbit at this side".into(),
            );
            lines.push(format!(
                "G38.3 Y#<refinement_delta_y> Z#<refinement_delta_z> F{}",
                c.positioning_feed
            ));
            lines.push("; Cross directly to the first side for the next pair".into());
            if rotary_supported(&self.start.firmware_version) {
                lines.push("G19 G3 Y#<side_delta_y> Z#<side_delta_z> J#<axis_j> K#<axis_k> A180 F#<tracking_feed>".into());
            } else {
                for _ in 0..36 {
                    lines.push("G38.3 Y#<segment_y> Z#<segment_z> A5 F#<tracking_feed>".into());
                }
            }
            lines.push(
                "; Repeat both side touches and the intervening half-turn with the refined axis"
                    .into(),
            );
            lines.push("; After the final pair, track back to the crest".into());
            if rotary_supported(&self.start.firmware_version) {
                lines.push("G19 G3 Y#<return_y> Z#<return_z> J#<return_j> K#<return_k> A90 F#<tracking_feed>".into());
            } else {
                for _ in 0..18 {
                    lines.push(
                        "G38.3 Y#<return_segment_y> Z#<return_segment_z> A5 F#<tracking_feed>"
                            .into(),
                    );
                }
            }
        }
        lines.push(format!(
            "G1 Z[{} - #<current_z>] F{}",
            self.start.position[2], c.positioning_feed
        ));
        lines.push("; Return X to the first station".into());
        lines.push(format!(
            "G38.3 X[{} - #<current_x>] F{}",
            self.start.position[0], c.positioning_feed
        ));
        if rotary_supported(&self.start.firmware_version) {
            lines.push(format!(
                "G10 L2 P{} R{} ; restore work rotation",
                self.start.wcs - 53,
                self.start.wcs_rotation.unwrap_or(0.0)
            ));
        }
        lines.push(format!(
            "G{} ; restore arc plane",
            if matches!(self.start.plane, 17..=19) {
                self.start.plane
            } else {
                17
            }
        ));
        lines.push(format!(
            "{} ; restore captured modes",
            self.start.modes.command()
        ));
        lines.join("\n")
    }
}
fn preview_touch(lines: &mut Vec<String>, axis: Axis, direction: i32, c: &RotaryConfig) {
    lines.push(format!(
        "G38.3 {axis}{} F{} ; coarse",
        f64::from(direction) * c.rod_diameter * 2.0,
        c.coarse_feed
    ));
    lines.push(format!(
        "G1 {axis}{} F{}",
        -f64::from(direction) * c.retract,
        c.positioning_feed
    ));
    lines.push(format!(
        "G38.2 {axis}{} F{} ; fine reading",
        f64::from(direction) * (c.retract + 0.5),
        c.fine_feed
    ));
    lines.push(format!(
        "; #<contact_{}> := {axis} at the fine touch",
        axis.name()
    ));
    lines.push(format!(
        "G1 {axis}{} F{}",
        -f64::from(direction) * c.retract,
        c.positioning_feed
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn native_tracking_rotates_the_ball_center_and_checks_interior_extrema() {
        let mut state = MockController::new().state();
        state.probe_extended = true;
        state.modes = Modes::PROBING;
        let c = MockController::with_state(state.clone());
        let config = RotaryConfig::default();
        let apex = tip(&state);
        let center = [apex[1] - 0.25, apex[2] + 1.0 - 9.0];
        track(&c, center, apex, 0., 90., &config, true)
            .await
            .unwrap();
        let expected = machine_tip(&state, orbit(center, apex, 90., 1.));
        assert!(within(c.state().position, expected, 0.001));
        let command = c.commands().pop().unwrap();
        assert!(command.contains("J-0.250 K-9.000 A90.000"), "{command}");

        state.position[2] = -0.01;
        let c = MockController::with_state(state.clone());
        let apex = tip(&state);
        let center = [apex[1] - 1., apex[2] + 1. - 10.];
        let error = track(&c, center, apex, 0., 90., &config, true)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("Z+"), "{error}");
    }
}
