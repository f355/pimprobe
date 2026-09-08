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

use crate::*;
use std::{fmt, time::Duration};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis {
    X,
    Y,
    Z,
}
impl Axis {
    pub fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
            Self::Z => "z",
        }
    }
}
impl fmt::Display for Axis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StageKind {
    CoarseProbe,
    CoarseRetract,
    FineProbe,
    FinalRetract,
    GuardedMove,
    ContactRelease,
    PositionMove,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stage {
    pub kind: StageKind,
    pub command: String,
    pub delta: Position,
    pub distance: f64,
    pub feed: f64,
    pub no_error: bool,
}
impl Stage {
    pub(crate) fn new(kind: StageKind, axis: Axis, delta: f64, feed: f64) -> Self {
        let mut displacement = [0.0; 4];
        displacement[axis.index()] = delta;
        Self::movement(kind, displacement, feed)
    }
    pub(crate) fn movement(kind: StageKind, delta: Position, feed: f64) -> Self {
        let delta = delta.map(quantize);
        let feed = quantize(feed);
        let code = match kind {
            StageKind::CoarseProbe | StageKind::GuardedMove => "G38.3",
            StageKind::FineProbe => "G38.2",
            _ => "G1",
        };
        let words = [Axis::X, Axis::Y, Axis::Z]
            .into_iter()
            .filter(|a| delta[a.index()] != 0.0)
            .map(|a| format!("{a}{:.3}", delta[a.index()]))
            .collect::<Vec<_>>();
        let command = format!("{code} {} F{feed:.3}", words.join(" "));
        Self {
            kind,
            command,
            delta,
            distance: delta[0].hypot(delta[1]).hypot(delta[2]),
            feed,
            no_error: matches!(kind, StageKind::CoarseProbe | StageKind::GuardedMove),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactConfig {
    pub axis: Axis,
    pub direction: i32,
    pub coarse_travel: f64,
    pub retract_distance: f64,
    pub coarse_feed: f64,
    pub fine_feed: f64,
    pub retract_feed: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContactPlan {
    pub stages: Vec<Stage>,
}
pub fn plan_contact(c: ContactConfig) -> Result<ContactPlan, Error> {
    if !matches!(c.direction, -1 | 1)
        || [
            c.coarse_travel,
            c.retract_distance,
            c.coarse_feed,
            c.fine_feed,
            c.retract_feed,
        ]
        .iter()
        .any(|v| !positive(quantize(*v)))
        || quantize(c.retract_distance) < 0.1
    {
        return Err(Error::InvalidConfig(
            "invalid contact travel, direction, retract or feed".into(),
        ));
    }
    let d = c.direction as f64;
    let retract = quantize(c.retract_distance);
    Ok(ContactPlan {
        stages: vec![
            Stage::new(
                StageKind::CoarseProbe,
                c.axis,
                d * quantize(c.coarse_travel),
                c.coarse_feed,
            ),
            Stage::new(
                StageKind::CoarseRetract,
                c.axis,
                -d * retract,
                c.retract_feed,
            ),
            Stage::new(
                StageKind::FineProbe,
                c.axis,
                d * (retract + 0.5),
                c.fine_feed,
            ),
            Stage::new(
                StageKind::FinalRetract,
                c.axis,
                -d * retract,
                c.retract_feed,
            ),
        ],
    })
}

#[derive(Debug, Clone, Copy)]
pub struct TimingPolicy {
    pub response_margin: Duration,
}
impl Default for TimingPolicy {
    fn default() -> Self {
        Self {
            response_margin: Duration::from_secs(5),
        }
    }
}
impl TimingPolicy {
    pub fn deadline(self, s: &Stage) -> Result<Duration, Error> {
        let feed = s.feed;
        if !positive(feed) || !positive(s.distance) {
            return Err(Error::InvalidConfig(
                "invalid timing feed or distance".into(),
            ));
        }
        // Safety mode can cap a programmed feed at 1000 mm/min.
        Duration::try_from_secs_f64(s.distance / feed.min(1000.0) * 60.0)
            .ok()
            .and_then(|d| d.checked_add(self.response_margin))
            .ok_or_else(|| Error::InvalidConfig("deadline overflow".into()))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactPreview {
    pub stages: Vec<Stage>,
    pub warnings: Vec<String>,
    pub path_min: Position,
    pub path_max: Position,
}
pub(crate) fn extent(plan: &ContactPlan) -> (Position, Position) {
    let (mut lo, mut hi, mut min, mut max) =
        ([0.0_f64; 4], [0.0_f64; 4], [0.0_f64; 4], [0.0_f64; 4]);
    for s in &plan.stages {
        for i in 0..3 {
            if matches!(s.kind, StageKind::CoarseProbe | StageKind::FineProbe) {
                lo[i] += s.delta[i].min(0.0);
                hi[i] += s.delta[i].max(0.0);
            } else {
                lo[i] += s.delta[i];
                hi[i] += s.delta[i];
            }
            min[i] = min[i].min(lo[i]);
            max[i] = max[i].max(hi[i]);
        }
    }
    (min, max)
}
pub fn preview_contact(
    s: &State,
    c: ContactConfig,
    t: TimingPolicy,
) -> Result<ContactPreview, Error> {
    preflight(s)?;
    let plan = plan_contact(c)?;
    for stage in &plan.stages {
        t.deadline(stage)?;
    }
    let (mut lo, mut hi) = extent(&plan);
    for i in 0..4 {
        lo[i] += s.position[i];
        hi[i] += s.position[i];
    }
    check_path(s, c.axis, lo[c.axis.index()], hi[c.axis.index()])?;
    let mut warnings = vec!["Fixed machine obstructions are not modeled".into()];
    if !s.probe_trigger_known {
        warnings.push("Controller does not report idle probe input".into());
    }
    Ok(ContactPreview {
        stages: plan.stages,
        warnings,
        path_min: lo,
        path_max: hi,
    })
}
pub(crate) async fn receive(rx: &mut broadcast::Receiver<Event>) -> Result<Event, Error> {
    let e = rx.recv().await.map_err(|e| match e {
        broadcast::error::RecvError::Closed => Error::Disconnected,
        broadcast::error::RecvError::Lagged(_) => Error::EventLagged,
    })?;
    if let Some(n) = e.controller_error {
        if n == -1 {
            return Err(Error::Disconnected);
        }
        return Err(Error::Controller(format!("error:{n}")));
    }
    if e.status.as_ref().is_some_and(|s| s.motion_blocked) {
        return Err(Error::MotionBlocked);
    }
    Ok(e)
}
pub(crate) fn segment(s: &Stage, start: Position, p: Position) -> Result<(), Error> {
    if !finite(p) || !finite(start) {
        return Err(Error::Position("non-finite report".into()));
    }
    let distance = s.distance;
    let along = (0..3)
        .map(|i| (p[i] - start[i]) * s.delta[i] / distance)
        .sum::<f64>();
    if along < -0.05
        || along > distance + 0.05
        || (0..4).any(|i| (p[i] - start[i] - along * s.delta[i] / distance).abs() > 0.05)
    {
        return Err(Error::Position("report outside commanded segment".into()));
    }
    Ok(())
}
pub(crate) async fn execute<C: Controller + ?Sized>(
    c: &C,
    s: &Stage,
    t: TimingPolicy,
    release: bool,
) -> Result<(Option<Contact>, Position), Error> {
    let mut rx = c.subscribe();
    let mut state = c.state();
    if release {
        state.probe_triggered = false;
    }
    if s.kind == StageKind::PositionMove {
        preflight_machine(&state)?;
    } else {
        preflight(&state)?;
    }
    let i = s.delta.iter().position(|v| *v != 0.0).unwrap();
    let start = state.position;
    let end = std::array::from_fn(|j| start[j] + s.delta[j]);
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let j = axis.index();
        if s.delta[j] != 0.0 {
            check_path(&state, axis, start[j].min(end[j]), start[j].max(end[j]))?;
        }
    }
    let deadline = t.deadline(s)?;
    let mut inconsistent_ready = false;
    tokio::time::timeout(deadline, async {
        c.send(&s.command).await?;
        let probe = matches!(
            s.kind,
            StageKind::CoarseProbe | StageKind::FineProbe | StageKind::GuardedMove
        );
        let (mut moving, mut failed, mut hit) = (false, false, None::<Contact>);
        loop {
            let e = receive(&mut rx).await?;
            if let Some(status) = &e.status {
                if !finite(status.position) || status.wcs != state.wcs {
                    return Err(Error::Position("invalid position or changed WCS".into()));
                }
                if !status.ready {
                    moving = true;
                }
            }
            if let Some(contact) = e.probe {
                if probe && moving {
                    if contact.success {
                        if !s.no_error {
                            segment(s, start, contact.position)?;
                        }
                        hit = Some(contact);
                    } else {
                        failed = true;
                    }
                }
            }
            let Some(status) = e.status else { continue };
            if !status.ready {
                continue;
            }
            if !probe {
                if within(status.position, end, 0.05) {
                    return Ok((None, status.position));
                }
                inconsistent_ready = true;
                continue;
            }
            if !moving {
                continue;
            }
            if s.kind == StageKind::GuardedMove {
                segment(s, start, status.position)?;
                if hit.is_some() {
                    return Err(Error::UnexpectedContact {
                        position: status.position,
                        retracted: false,
                    });
                }
                if !failed || !within(end, status.position, 0.05) {
                    return Err(Error::Position(
                        "guarded move missing no-contact report or stopped short".into(),
                    ));
                }
                return Ok((None, status.position));
            }
            if failed || hit.is_none() {
                segment(s, start, status.position)?;
                if s.no_error && !within(end, status.position, 0.05) {
                    return Err(Error::Position("coarse miss stopped short".into()));
                }
                return Err(if s.no_error {
                    Error::CoarseNoContact
                } else {
                    Error::NoContact
                });
            }
            let mut hit = hit.take().unwrap();
            if s.no_error {
                segment(s, start, status.position)?;
                hit.position = status.position;
            } else {
                for j in 0..4 {
                    let d = (status.position[j] - hit.position[j])
                        * if j == i { s.delta[i].signum() } else { 1.0 };
                    if (j == i && !(-0.05..=0.10).contains(&d)) || (j != i && d.abs() > 0.05) {
                        return Err(Error::Position("stop inconsistent with trigger".into()));
                    }
                }
            }
            return Ok((Some(hit), status.position));
        }
    })
    .await
    .map_err(|_| {
        if inconsistent_ready {
            Error::Position("move did not reach expected endpoint".into())
        } else {
            Error::Timeout
        }
    })?
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactResult {
    pub contact: Contact,
    pub final_position: Position,
    pub motion_started: bool,
}
pub async fn run_contact<C: Controller + ?Sized>(
    c: &C,
    config: ContactConfig,
    t: TimingPolicy,
) -> Result<ContactResult, Error> {
    run_contact_with_reading(c, config, t, |_| {}).await
}

/// Publish the confirmed fine touch before the final backoff.
pub async fn run_contact_with_reading<C: Controller + ?Sized>(
    c: &C,
    config: ContactConfig,
    t: TimingPolicy,
    mut reading: impl FnMut(&Contact),
) -> Result<ContactResult, Error> {
    preview_contact(&c.state(), config, t)?;
    let wcs = c.state().wcs;
    let mut hit = None;
    let mut p = c.state().position;
    for s in plan_contact(config)?.stages {
        if c.state().wcs != wcs || !within(p, c.state().position, 0.05) {
            return Err(Error::Position("stage start changed".into()));
        }
        let (contact, pos) = execute(
            c,
            &s,
            t,
            matches!(s.kind, StageKind::CoarseRetract | StageKind::FinalRetract),
        )
        .await?;
        p = pos;
        if s.kind == StageKind::FineProbe {
            if let Some(contact) = &contact {
                reading(contact);
            }
            hit = contact;
        }
    }
    Ok(ContactResult {
        contact: hit.ok_or(Error::NoContact)?,
        final_position: p,
        motion_started: true,
    })
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GuardedMoveConfig {
    pub axis: Axis,
    pub distance: f64,
    pub feed: f64,
    pub retract_distance: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardedMoveResult {
    pub position: Position,
    pub motion_started: bool,
    pub retracted: bool,
}
pub fn plan_guarded_move(config: GuardedMoveConfig) -> Result<Stage, Error> {
    if !positive(quantize(config.distance).abs())
        || !positive(quantize(config.feed))
        || !positive(config.retract_distance)
        || config.retract_distance < 0.1
    {
        return Err(Error::InvalidConfig("invalid guarded move".into()));
    }
    Ok(Stage::new(
        StageKind::GuardedMove,
        config.axis,
        config.distance,
        config.feed,
    ))
}
pub async fn run_guarded_move<C: Controller + ?Sized>(
    c: &C,
    config: GuardedMoveConfig,
    t: TimingPolicy,
) -> Result<GuardedMoveResult, Error> {
    let stage = plan_guarded_move(config)?;
    run_position_stage(c, stage, config.retract_distance, t).await
}

pub(crate) fn positioning_stage(delta: Position, feed: f64) -> Stage {
    let lift = delta[0] == 0.0 && delta[1] == 0.0 && delta[2] > 0.0;
    Stage::movement(
        if lift {
            StageKind::PositionMove
        } else {
            StageKind::GuardedMove
        },
        delta,
        feed,
    )
}

pub(crate) async fn run_position_stage<C: Controller + ?Sized>(
    c: &C,
    stage: Stage,
    retract_distance: f64,
    t: TimingPolicy,
) -> Result<GuardedMoveResult, Error> {
    let start = c.state();
    match execute(c, &stage, t, false).await {
        Ok((_, position)) => Ok(GuardedMoveResult {
            position,
            motion_started: true,
            retracted: false,
        }),
        Err(Error::UnexpectedContact { position, .. }) => {
            let latest = c.state();
            if latest.wcs != start.wcs || !within(latest.position, position, 0.05) {
                return Err(Error::Position(
                    "state changed before contact release".into(),
                ));
            }
            let distance = quantize(
                retract_distance.min(
                    (position[0] - start.position[0])
                        .hypot(position[1] - start.position[1])
                        .hypot(position[2] - start.position[2]),
                ),
            );
            if distance <= 0.05 {
                return Err(Error::UnexpectedContact {
                    position,
                    retracted: false,
                });
            }
            let release = Stage::movement(
                StageKind::ContactRelease,
                stage.delta.map(|v| -v / stage.distance * distance),
                stage.feed,
            );
            match execute(c, &release, t, true).await {
                Ok((_, position)) => Err(Error::UnexpectedContact {
                    position,
                    retracted: true,
                }),
                Err(recovery) => Err(Error::Recovery {
                    cause: Box::new(Error::UnexpectedContact {
                        position,
                        retracted: false,
                    }),
                    recovery: Box::new(recovery),
                }),
            }
        }
        Err(e) => Err(e),
    }
}
