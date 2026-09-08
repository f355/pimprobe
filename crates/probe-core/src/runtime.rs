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

use crate::script::{trim_command, Line, Values};
use crate::*;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RoutineResult {
    pub point: [Option<f64>; 3],
    #[serde(default, rename = "machinePoint")]
    pub machine_point: [Option<f64>; 3],
    /// Separations between opposing compensated surfaces, in millimeters.
    #[serde(default)]
    pub spans: [Option<f64>; 3],
    pub wcs: i32,
    pub zeroed: bool,
    #[serde(default)]
    pub returned: bool,
    pub axes: Vec<String>,
    #[serde(skip)]
    pub motion_started: bool,
    #[serde(skip)]
    pub settled: bool,
}
pub async fn query_modes<C: Controller + ?Sized>(c: &C) -> Result<Modes, Error> {
    let mut rx = c.subscribe();
    tokio::time::timeout(Duration::from_secs(3), async {
        c.send("$G").await?;
        loop {
            if let Some(m) = receive(&mut rx).await?.modes {
                if !m.valid() {
                    return Err(Error::Preflight("incomplete parser modes".into()));
                }
                return Ok(m);
            }
        }
    })
    .await
    .map_err(|_| Error::Timeout)?
}
pub(crate) async fn set_modes<C: Controller + ?Sized>(c: &C, m: Modes) -> Result<(), Error> {
    if !m.valid() {
        return Err(Error::Preflight("invalid modes".into()));
    }
    if c.state().motion_blocked {
        return Err(Error::MotionBlocked);
    }
    tokio::time::timeout(Duration::from_secs(3), c.send(&m.command()))
        .await
        .map_err(|_| Error::Timeout)??;
    if query_modes(c).await? != m {
        return Err(Error::Controller("parser modes not confirmed".into()));
    }
    Ok(())
}
pub fn surface_machine_coordinate(
    contact: &Contact,
    axis: Axis,
    direction: i32,
    diameter: f64,
    offset: Position,
) -> Result<f64, Error> {
    if !contact.success
        || !finite(contact.position)
        || !finite(offset)
        || !positive(diameter)
        || !matches!(direction, -1 | 1)
    {
        return Err(Error::Compensation("invalid contact".into()));
    }
    let i = axis.index();
    if axis == Axis::Z {
        let tool = contact
            .tool_length
            .filter(|v| v.is_finite())
            .ok_or_else(|| Error::Compensation("probe report lacks Z tool value".into()))?;
        Ok(contact.position[i] + tool - offset[i])
    } else {
        Ok(contact.position[i] + offset[i] + f64::from(direction) * diameter / 2.0)
    }
}
pub(crate) struct Observed<'a, C: ?Sized, F> {
    pub(crate) inner: &'a C,
    pub(crate) observe: &'a F,
    pub(crate) scripted: &'a AtomicBool,
}

pub(crate) struct Cancellable<'a, C: ?Sized> {
    pub(crate) inner: &'a C,
    pub(crate) cancel: &'a CancellationToken,
}
#[async_trait]
impl<C: Controller + ?Sized> Controller for Cancellable<'_, C> {
    fn state(&self) -> State {
        self.inner.state()
    }
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Event> {
        self.inner.subscribe()
    }
    async fn send(&self, command: &str) -> Result<(), Error> {
        tokio::select! {
            biased;
            _=self.cancel.cancelled()=>Err(Error::Cancelled),
            result=self.inner.send(command)=>result,
        }
    }
}
#[async_trait]
impl<C: Controller + ?Sized, F: Fn(Progress) + Send + Sync> Controller for Observed<'_, C, F> {
    fn state(&self) -> State {
        self.inner.state()
    }
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Event> {
        self.inner.subscribe()
    }
    async fn send(&self, command: &str) -> Result<(), Error> {
        if command != "$G" && !self.scripted.swap(false, Ordering::Relaxed) {
            (self.observe)(Progress {
                kind: "script".into(),
                command: String::new(),
                message: trim_command(command),
            });
        }
        self.inner.send(command).await?;
        if command != "$G" {
            (self.observe)(Progress {
                kind: "command".into(),
                command: command.into(),
                message: "Submitted".into(),
            });
        }
        Ok(())
    }
}
struct Stream<'a> {
    lines: Vec<Line>,
    cursor: usize,
    scripted: &'a AtomicBool,
}
impl Stream<'_> {
    fn line<F: Fn(Progress)>(&mut self, refs: &Values, observe: &F) -> Result<(), Error> {
        let message = self.lines[self.cursor].render(Some(refs))?;
        observe(Progress {
            kind: "script".into(),
            message,
            command: String::new(),
        });
        self.cursor += 1;
        Ok(())
    }
    fn command<F: Fn(Progress)>(
        &mut self,
        step: isize,
        refs: &Values,
        observe: &F,
    ) -> Result<(), Error> {
        while self.cursor < self.lines.len() && self.lines[self.cursor].step == step {
            let command = self.lines[self.cursor].is_command();
            self.line(refs, observe)?;
            if command {
                self.scripted.store(true, Ordering::Relaxed);
                break;
            }
        }
        Ok(())
    }
    fn declarations<F: Fn(Progress)>(
        &mut self,
        step: isize,
        refs: &Values,
        observe: &F,
    ) -> Result<(), Error> {
        while self.cursor < self.lines.len()
            && self.lines[self.cursor].step == step
            && self.lines[self.cursor].is_declaration()
        {
            // A missing tool field is diagnostic data, not permission to skip
            // the final release. Surface conversion will reject it afterward.
            if self.lines[self.cursor].render(Some(refs)).is_err() {
                observe(Progress {
                    kind: "script".into(),
                    command: String::new(),
                    message: "; Probe report tool value unavailable".into(),
                });
                self.cursor += 1;
            } else {
                self.line(refs, observe)?;
            }
        }
        Ok(())
    }
    fn rest<F: Fn(Progress)>(
        &mut self,
        step: isize,
        refs: &Values,
        observe: &F,
    ) -> Result<(), Error> {
        while self.cursor < self.lines.len() && self.lines[self.cursor].step == step {
            self.line(refs, observe)?;
        }
        Ok(())
    }
}
pub async fn run<C: Controller + ?Sized>(
    c: &C,
    p: &RoutinePlan,
    t: TimingPolicy,
    cancel: CancellationToken,
    observe: impl Fn(Progress) + Send + Sync,
) -> Result<RoutineResult, Error> {
    let controller = Cancellable {
        inner: c,
        cancel: &cancel,
    };
    tokio::select! {
        biased;
        _=cancel.cancelled()=>Err(Error::Cancelled),
        result=run_inner(&controller,p,t,&observe)=>result,
    }
}
async fn run_inner<C: Controller + ?Sized, F: Fn(Progress) + Send + Sync>(
    c: &C,
    p: &RoutinePlan,
    t: TimingPolicy,
    observe: &F,
) -> Result<RoutineResult, Error> {
    // Reject caller-mutated plans before any command is submitted.
    if review(p.start.clone(), p.config.clone())? != *p {
        return Err(Error::Preflight("plan modified since review".into()));
    }
    p.check_state(&c.state(), p.start.position)?;
    let scripted = AtomicBool::new(false);
    let c = Observed {
        inner: c,
        observe,
        scripted: &scripted,
    };
    let mut result = RoutineResult {
        wcs: p.config.wcs,
        axes: p.axes().iter().map(ToString::to_string).collect(),
        ..RoutineResult::default()
    };
    let mut stream = Stream {
        lines: p.script(),
        cursor: 0,
        scripted: &scripted,
    };
    let mut refs = Values::new();
    for a in [Axis::X, Axis::Y, Axis::Z] {
        refs.insert(format!("start_{}", a.name()), p.start.position[a.index()]);
    }
    stream.command(-1, &refs, observe)?;
    set_modes(&c, Modes::PROBING).await?;
    let mut position = p.start.position;
    let mut conversion_error = None;
    let execution: Result<(), Error> = async {
        for (index, step) in p.steps.iter().enumerate() {
            p.check_state(&c.state(), position)?;
            match step {
                RoutineStep::Center { axis } => {
                    let n = axis.name();
                    let low = refs[&format!("surface_{n}_low")];
                    let high = refs[&format!("surface_{n}_high")];
                    if high <= low {
                        return Err(Error::Compensation(format!(
                            "opposing {axis} contacts have non-positive width"
                        )));
                    }
                    let center = (low + high) / 2.0;
                    result.spans[axis.index()] = Some(high - low);
                    refs.insert(format!("surface_{n}"), center);
                    result.point[axis.index()] = Some(
                        center
                            - (p.start.position[axis.index()]
                                - p.start.work_position[axis.index()]),
                    );
                }
                RoutineStep::Contact {
                    axis,
                    config,
                    limit,
                    measurement,
                } => {
                    let mut config = *config;
                    config.coarse_travel =
                        quantize((limit - position[axis.index()]) * f64::from(config.direction));
                    if config.coarse_travel <= 0.0 {
                        return Err(Error::CoarseNoContact);
                    }
                    preview_contact(&c.state(), config, t)?;
                    let mut fine = None;
                    for stage in plan_contact(config)?.stages {
                        let latest = c.state();
                        if latest.wcs != p.start.wcs || !within(position, latest.position, 0.05) {
                            return Err(Error::Position("contact stage state changed".into()));
                        }
                        stream.command(index as isize, &refs, observe)?;
                        result.motion_started = true;
                        let (contact, stopped) = execute(
                            &c,
                            &stage,
                            t,
                            matches!(
                                stage.kind,
                                StageKind::CoarseRetract | StageKind::FinalRetract
                            ),
                        )
                        .await?;
                        position = stopped;
                        if stage.kind == StageKind::FineProbe {
                            let contact = contact.ok_or(Error::NoContact)?;
                            refs.insert(
                                format!("contact_{measurement}"),
                                contact.position[axis.index()],
                            );
                            if let Some(tool) = contact.tool_length.filter(|v| v.is_finite()) {
                                refs.insert("probe_report_tool_value".into(), tool);
                            }
                            fine = Some(contact);
                        }
                        if stage.kind == StageKind::FinalRetract {
                            refs.insert(
                                format!("{}_after_backoff", axis.name()),
                                position[axis.index()],
                            );
                        }
                        stream.declarations(index as isize, &refs, observe)?;
                    }
                    let contact = fine.ok_or(Error::NoContact)?;
                    match surface_machine_coordinate(
                        &contact,
                        *axis,
                        config.direction,
                        p.config.diameter,
                        p.start.probe_offset,
                    ) {
                        Ok(surface) => {
                            refs.insert(format!("surface_{measurement}"), surface);
                            if measurement == axis.name() {
                                result.point[axis.index()] = Some(
                                    surface
                                        - (p.start.position[axis.index()]
                                            - p.start.work_position[axis.index()]),
                                );
                            }
                        }
                        Err(e) if *axis == Axis::Z => {
                            conversion_error = Some(e);
                            continue;
                        }
                        Err(e) => return Err(e),
                    }
                }
                RoutineStep::Move { targets } => {
                    let mut delta = [0.0; 4];
                    for MoveTarget {
                        axis,
                        reference: target,
                        offset,
                    } in targets
                    {
                        let target = if target == &format!("current_{}", axis.name()) {
                            position[axis.index()]
                        } else {
                            *refs.get(target).ok_or_else(|| {
                                Error::Compensation("missing motion target".into())
                            })?
                        };
                        delta[axis.index()] = quantize(target + offset - position[axis.index()]);
                    }
                    stream.command(index as isize, &refs, observe)?;
                    if delta.iter().any(|v| *v != 0.0) {
                        result.motion_started = true;
                        position = run_position_stage(
                            &c,
                            positioning_stage(delta, p.config.positioning_feed),
                            p.config.retract,
                            t,
                        )
                        .await?
                        .position;
                    }
                }
            }
            stream.rest(index as isize, &refs, observe)?;
        }
        p.check_state(&c.state(), position)?;
        if let Some(e) = conversion_error {
            return Err(e);
        }
        result.machine_point = std::array::from_fn(|i| {
            result.point[i].map(|v| v + p.start.position[i] - p.start.work_position[i])
        });
        if p.config.zero {
            stream.command(p.steps.len() as isize, &refs, observe)?;
            write_zero(&c, p, &result, [0.0; 3], t).await?;
            result.zeroed = true;
        }
        Ok(())
    }
    .await;
    if let Err(mut cause) = execution {
        let mut settled = matches!(
            &cause,
            Error::CoarseNoContact
                | Error::Compensation(_)
                | Error::UnexpectedContact {
                    retracted: true,
                    ..
                }
        );
        if matches!(cause, Error::CoarseNoContact) && p.config.z {
            let state = c.state();
            let return_result = async {
                p.check_state(&state, state.position)?;
                if (0..4).any(|i| i != 2 && (state.position[i] - position[i]).abs() > 0.05) {
                    return Err(Error::Position("cross-axis move on coarse miss".into()));
                }
                let delta = quantize(p.start.position[2] - state.position[2]);
                if delta != 0.0 {
                    run_position_stage(
                        &c,
                        positioning_stage([0.0, 0.0, delta, 0.0], p.config.positioning_feed),
                        p.config.retract,
                        t,
                    )
                    .await?;
                }
                Ok(())
            }
            .await;
            if let Err(e) = return_result {
                settled = false;
                cause = Error::Recovery {
                    cause: Box::new(cause),
                    recovery: Box::new(e),
                };
            }
        }
        if settled && !c.state().motion_blocked {
            if let Err(recovery) = set_modes(&c, p.start.modes).await {
                return Err(Error::Recovery {
                    cause: Box::new(cause),
                    recovery: Box::new(recovery),
                });
            }
        }
        return Err(cause);
    }
    stream.command(p.steps.len() as isize + 1, &refs, observe)?;
    set_modes(&c, p.start.modes).await?;
    result.settled = true;
    observe(Progress {
        kind: "stage".into(),
        message: "Routine complete".into(),
        command: String::new(),
    });
    Ok(result)
}
async fn write_zero<C: Controller + ?Sized>(
    c: &C,
    p: &RoutinePlan,
    result: &RoutineResult,
    offsets: [f64; 3],
    t: TimingPolicy,
) -> Result<(), Error> {
    let mut rx = c.subscribe();
    let state = c.state();
    p.check_state(&state, state.position)?;
    let mut desired = state.work_position;
    let mut words = Vec::new();
    for a in p.axes() {
        let i = a.index();
        let point = result.point[i]
            .filter(|v| v.is_finite())
            .ok_or_else(|| Error::Compensation(format!("missing {a} result")))?;
        let surface = point + p.start.position[i] - p.start.work_position[i] + offsets[i];
        desired[i] = quantize(state.position[i] - surface);
        words.push(format!("{a}{:.3}", desired[i]));
    }
    let duration = t
        .response_margin
        .checked_add(Duration::from_secs(3))
        .ok_or_else(|| Error::InvalidConfig("zero timeout overflow".into()))?;
    tokio::time::timeout(duration, async {
        c.send(&format!(
            "G10 L20 P{} {}",
            p.config.wcs - 53,
            words.join(" ")
        ))
        .await?;
        loop {
            if let Some(s) = receive(&mut rx).await?.status {
                if s.wcs != p.config.wcs || !within(s.position, state.position, 0.05) {
                    return Err(Error::Position(
                        "machine moved or WCS changed while zeroing".into(),
                    ));
                }
                if s.ready && within(s.work_position, desired, 0.05) {
                    return Ok(());
                }
            }
        }
    })
    .await
    .map_err(|_| Error::Timeout)?
}
pub async fn zero_result<C: Controller + ?Sized>(
    c: &C,
    p: &RoutinePlan,
    result: &RoutineResult,
    offsets: [f64; 3],
    t: TimingPolicy,
    cancel: CancellationToken,
) -> Result<RoutineResult, Error> {
    let controller = Cancellable {
        inner: c,
        cancel: &cancel,
    };
    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(Error::Cancelled),
        result = zero_inner(&controller, p, result, offsets, t) => result,
    }
}

pub async fn return_to_start<C: Controller + ?Sized>(
    c: &C,
    p: &RoutinePlan,
    result: &RoutineResult,
    t: TimingPolicy,
    cancel: CancellationToken,
    observe: impl Fn(Progress) + Send + Sync,
) -> Result<RoutineResult, Error> {
    let controller = Cancellable {
        inner: c,
        cancel: &cancel,
    };
    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(Error::Cancelled),
        result = return_inner(&controller, p, result, t, &observe) => result,
    }
}

async fn return_inner<C: Controller + ?Sized, F: Fn(Progress) + Send + Sync>(
    c: &C,
    p: &RoutinePlan,
    result: &RoutineResult,
    t: TimingPolicy,
    observe: &F,
) -> Result<RoutineResult, Error> {
    if !result.settled
        || result.returned
        || result.wcs != p.config.wcs
        || result.axes != p.axes().iter().map(ToString::to_string).collect::<Vec<_>>()
    {
        return Err(Error::Preflight("no matching result to return from".into()));
    }
    let state = c.state();
    let mut check = p.clone();
    if result.zeroed {
        // Setting work zero changes the offset, not the saved machine position.
        check.start.work_position = std::array::from_fn(|i| {
            p.start.position[i] - (state.position[i] - state.work_position[i])
        });
    }
    check.check_state(&state, state.position)?;
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let i = axis.index();
        check_path(
            &state,
            axis,
            state.position[i].min(p.start.position[i]),
            state.position[i].max(p.start.position[i]),
        )?;
    }
    let scripted = AtomicBool::new(false);
    let c = Observed {
        inner: c,
        observe,
        scripted: &scripted,
    };
    let modes = query_modes(&c).await?;
    set_modes(&c, Modes::PROBING).await?;
    let movement = async {
        let mut position = state.position;
        for (axes, comment) in [
            (
                vec![Axis::X, Axis::Y],
                "; Return X/Y to the starting position, expecting no contact",
            ),
            (vec![Axis::Z], "; Return Z to the starting height"),
        ] {
            check.check_state(&c.state(), position)?;
            let mut delta = [0.0; 4];
            for axis in axes {
                let i = axis.index();
                delta[i] = quantize(p.start.position[i] - position[i]);
            }
            if delta.iter().any(|v| *v != 0.0) {
                observe(Progress {
                    kind: "script".into(),
                    message: comment.into(),
                    command: String::new(),
                });
                position = run_position_stage(
                    &c,
                    positioning_stage(delta, p.config.positioning_feed),
                    p.config.retract,
                    t,
                )
                .await?
                .position;
            }
        }
        Ok::<_, Error>(())
    }
    .await;
    if movement.is_ok()
        || matches!(
            movement,
            Err(Error::UnexpectedContact {
                retracted: true,
                ..
            })
        )
    {
        if let Err(recovery) = set_modes(&c, modes).await {
            return Err(match movement {
                Ok(()) => recovery,
                Err(cause) => Error::Recovery {
                    cause: Box::new(cause),
                    recovery: Box::new(recovery),
                },
            });
        }
    }
    movement?;
    let mut updated = result.clone();
    updated.returned = true;
    Ok(updated)
}

async fn zero_inner<C: Controller + ?Sized>(
    c: &C,
    p: &RoutinePlan,
    result: &RoutineResult,
    offsets: [f64; 3],
    t: TimingPolicy,
) -> Result<RoutineResult, Error> {
    if offsets.iter().any(|v| !v.is_finite() || v.abs() > 1000.0) {
        return Err(Error::InvalidConfig(
            "offsets must be finite and within -1000..1000 mm".into(),
        ));
    }
    if !result.settled
        || result.zeroed
        || result.wcs != p.config.wcs
        || result.axes != p.axes().iter().map(ToString::to_string).collect::<Vec<_>>()
    {
        return Err(Error::Preflight("no matching unzeroed result".into()));
    }
    let state = c.state();
    p.check_state(&state, state.position)?;
    let modes = query_modes(c).await?;
    let metric = Modes { units: 21, ..modes };
    if metric != modes {
        set_modes(c, metric).await?;
    }
    let result_zero = write_zero(c, p, result, offsets, t).await;
    if metric != modes && !c.state().motion_blocked {
        if let Err(recovery) = set_modes(c, modes).await {
            return Err(match result_zero {
                Ok(()) => recovery,
                Err(cause) => Error::Recovery {
                    cause: Box::new(cause),
                    recovery: Box::new(recovery),
                },
            });
        }
    }
    result_zero?;
    let mut updated = result.clone();
    updated.zeroed = true;
    Ok(updated)
}
