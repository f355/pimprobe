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

//! Work-axis probing uses one coordinate transform for every controller.

use crate::*;
use tokio::sync::broadcast;

#[derive(Clone, Copy)]
pub(crate) struct Frame {
    pub angle: f64,
    g92: Position,
}
impl Frame {
    pub fn new(start: &State, angle: f64) -> Self {
        Self {
            angle,
            g92: start.coordinate_offset,
        }
    }
    pub fn state(self, mut state: State) -> State {
        if self.angle.abs() < 1e-9 {
            return state;
        }
        rotate(&mut state.position, -self.angle);
        rotate(&mut state.probe_offset, -self.angle);
        if let Some(mut origin) = state.wcs_origin {
            rotate(&mut origin, -self.angle);
            state.wcs_origin = Some(origin);
            state.work_position = std::array::from_fn(|i| {
                state.position[i]
                    - origin[i]
                    - state.coordinate_offset[i]
                    - if i == 2 { state.tool_length_offset } else { 0. }
            });
        }
        state.wcs_rotation = Some(0.);
        state
    }
    fn event(self, mut event: Event) -> Event {
        if self.angle.abs() < 1e-9 {
            return event;
        }
        if let Some(status) = &mut event.status {
            rotate(&mut status.position, -self.angle);
            for i in 0..2 {
                status.work_position[i] += self.g92[i];
            }
            rotate(&mut status.work_position, -self.angle);
            for i in 0..2 {
                status.work_position[i] -= self.g92[i];
            }
        }
        if let Some(contact) = &mut event.probe {
            rotate(&mut contact.position, -self.angle);
        }
        if let Some((_, origin)) = &mut event.wcs_origin {
            rotate(origin, -self.angle);
        }
        if let Some((_, rotation)) = &mut event.wcs_rotation {
            *rotation = 0.;
        }
        event
    }
    pub fn result(self, mut result: RoutineResult, start: &State, physical: bool) -> RoutineResult {
        if self.angle.abs() < 1e-9 {
            return result;
        }
        let convert = |point: [Option<f64>; 3], tip: bool| {
            if point[0].is_none() && point[1].is_none() {
                return point;
            }
            let mut full = start.position;
            if tip {
                full[0] += start.probe_offset[0];
                full[1] += start.probe_offset[1];
                full[2] -= start.probe_offset[2];
            }
            if physical {
                rotate(&mut full, -self.angle);
            }
            for i in 0..3 {
                if let Some(value) = point[i] {
                    full[i] = value;
                }
            }
            rotate(&mut full, if physical { self.angle } else { -self.angle });
            [Some(full[0]), Some(full[1]), point[2]]
        };
        result.machine_point = convert(result.machine_point, true);
        result.raw_point = convert(result.raw_point, false);
        result
    }
}
pub struct EventReceiver {
    receiver: broadcast::Receiver<Event>,
    frame: Option<Frame>,
}
impl From<broadcast::Receiver<Event>> for EventReceiver {
    fn from(receiver: broadcast::Receiver<Event>) -> Self {
        Self {
            receiver,
            frame: None,
        }
    }
}
impl EventReceiver {
    pub async fn recv(&mut self) -> Result<Event, broadcast::error::RecvError> {
        self.receiver.recv().await.map(|event| match self.frame {
            Some(frame) => frame.event(event),
            None => event,
        })
    }
    pub(crate) fn in_frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }
}

pub(crate) struct FrameController<'a, C: ?Sized> {
    pub inner: &'a C,
    pub frame: Frame,
}
#[async_trait]
impl<C: Controller + ?Sized> Controller for FrameController<'_, C> {
    fn state(&self) -> State {
        self.frame.state(self.inner.state())
    }
    fn subscribe(&self) -> EventReceiver {
        self.inner.subscribe().in_frame(self.frame)
    }
    async fn send(&self, command: &str) -> Result<(), Error> {
        if self.frame.angle.abs() < 1e-9 {
            return self.inner.send(command).await;
        }
        let mut words = command.split_whitespace();
        let code = words.next().unwrap_or("");
        if matches!(code, "G0" | "G1" | "G38.2" | "G38.3" | "G38.4" | "G38.5") {
            let mut delta = [0.; 4];
            let mut tail = Vec::new();
            let mut xy = false;
            for word in words.by_ref() {
                if word.starts_with('X') || word.starts_with('Y') {
                    delta[usize::from(word.starts_with('Y'))] = word[1..]
                        .parse()
                        .map_err(|_| Error::InvalidConfig("Invalid probing move".into()))?;
                    xy = true;
                } else {
                    tail.push(word.to_owned());
                }
            }
            if xy {
                if self.inner.state().modes.distance != 91 {
                    return Err(Error::InvalidConfig(
                        "Probing work-axis moves require incremental mode".into(),
                    ));
                }
                rotate(&mut delta, self.frame.angle);
                tail.insert(
                    0,
                    format!(
                        "X{} Y{}",
                        crate::script::number(delta[0]),
                        crate::script::number(delta[1])
                    ),
                );
                return self.inner.send(&format!("{code} {}", tail.join(" "))).await;
            }
        }
        if code == "G10" {
            let mut mode = 2;
            let mut p = 0;
            let mut axes = [None; 4];
            for word in words {
                let value = word[1..]
                    .parse::<f64>()
                    .map_err(|_| Error::InvalidConfig("Invalid probing work zero".into()))?;
                match &word[..1] {
                    "L" => mode = value as i32,
                    "P" => p = value as i32,
                    "X" => axes[0] = Some(value),
                    "Y" => axes[1] = Some(value),
                    "Z" => axes[2] = Some(value),
                    "A" => axes[3] = Some(value),
                    _ => {
                        return Err(Error::InvalidConfig(
                            "Unsupported probing work-zero command".into(),
                        ))
                    }
                }
            }
            let state = self.state();
            if p != state.wcs - 53 {
                return Err(Error::Preflight(
                    "Select the measurement's work coordinates before setting zero".into(),
                ));
            }
            let mut origin = state
                .wcs_origin
                .ok_or_else(|| Error::Preflight("Work zero is unavailable".into()))?;
            for i in 0..4 {
                if let Some(value) = axes[i] {
                    origin[i] = if mode == 20 {
                        state.position[i]
                            - state.coordinate_offset[i]
                            - if i == 2 { state.tool_length_offset } else { 0. }
                            - value
                    } else {
                        value
                    };
                }
            }
            rotate(&mut origin, self.frame.angle);
            let mut command = format!("G10 L2 P{p}");
            for i in 0..4 {
                if axes[i].is_some() || i < 2 && (axes[0].is_some() || axes[1].is_some()) {
                    command += &format!(
                        " {}{}",
                        ["X", "Y", "Z", "A"][i],
                        crate::script::number(origin[i])
                    );
                }
            }
            return self.inner.send(&command).await;
        }
        self.inner.send(command).await
    }
}
fn rotate(point: &mut Position, angle: f64) {
    let (sin, cos) = angle.to_radians().sin_cos();
    let [x, y, _, _] = *point;
    point[0] = cos * x - sin * y;
    point[1] = sin * x + cos * y;
}

impl RoutinePlan {
    pub(crate) fn frame(&self) -> Frame {
        Frame::new(
            &self.start,
            if self.config.family == "angle" {
                0.
            } else {
                self.start.wcs_rotation.unwrap_or(0.)
            },
        )
    }
    pub(crate) fn execution_plan(&self) -> Self {
        let mut plan = self.clone();
        plan.start = self.frame().state(plan.start);
        plan
    }
}
