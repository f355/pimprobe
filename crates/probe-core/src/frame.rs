// PIMProbe - touch probing for the Nestworks C500.
// Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
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
    async fn read_settings(&self) -> Result<BTreeMap<i32, f64>, Error> {
        self.inner.read_settings().await
    }
    async fn read_modes(&self) -> Result<Modes, Error> {
        self.inner.read_modes().await
    }
    async fn read_firmware_version(&self) -> Result<String, Error> {
        self.inner.read_firmware_version().await
    }
    async fn read_coordinates(&self) -> Result<State, Error> {
        Ok(self.frame.state(self.inner.read_coordinates().await?))
    }
    async fn write_setting(&self, key: i32, value: f64) -> Result<(), Error> {
        self.inner.write_setting(key, value).await
    }
    async fn write_modes(&self, modes: Modes, plane: Option<i32>) -> Result<(), Error> {
        self.inner.write_modes(modes, plane).await
    }
    async fn write_coordinates(
        &self,
        wcs: i32,
        mut axes: [Option<f64>; 4],
        rotation: Option<f64>,
    ) -> Result<(), Error> {
        if self.frame.angle.abs() >= 1e-9 {
            let state = self.state();
            if wcs != state.wcs {
                return Err(Error::Preflight(
                    "Select the measurement's work coordinates before setting zero".into(),
                ));
            }
            if rotation.is_some() {
                return Err(Error::InvalidConfig(
                    "Work-axis origin writes cannot change rotation".into(),
                ));
            }
            let mut origin = state
                .wcs_origin
                .ok_or_else(|| Error::Preflight("Work zero is unavailable".into()))?;
            for i in 0..4 {
                if let Some(value) = axes[i] {
                    origin[i] = value;
                }
            }
            rotate(&mut origin, self.frame.angle);
            let xy = axes[0].is_some() || axes[1].is_some();
            for i in 0..4 {
                if axes[i].is_some() || i < 2 && xy {
                    axes[i] = Some(quantize(origin[i]));
                }
            }
        }
        self.inner.write_coordinates(wcs, axes, rotation).await
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn partial_work_axis_origin_write_preserves_the_other_rotated_axis() {
        let mut start = MockController::new().state();
        start.wcs_origin = Some([-10., -20., 30., 40.]);
        start.coordinate_offset = [1., 2., 3., 4.];
        start.tool_length_offset = 8.;
        let machine = MockController::with_state(start);
        let axes = FrameController {
            inner: &machine,
            frame: Frame::new(&machine.state(), 90.),
        };
        axes.write_coordinates(54, [Some(-33.), None, None, None], None)
            .await
            .unwrap();
        let origin = machine.state().wcs_origin.unwrap();
        assert!((origin[0] + 10.).abs() < 0.001);
        assert!((origin[1] + 33.).abs() < 0.001);
        assert_eq!(origin[2..], [30., 40.]);
        assert_eq!(machine.state().coordinate_offset, [1., 2., 3., 4.]);
        assert_eq!(machine.state().tool_length_offset, 8.);
        let refreshed = axes.read_coordinates().await.unwrap();
        assert!((refreshed.wcs_origin.unwrap()[0] + 33.).abs() < 0.001);
    }
}
