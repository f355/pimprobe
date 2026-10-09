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
use std::{collections::BTreeMap, sync::Mutex};
use tokio::sync::broadcast;

/// Stock geometry expressed as the probe-ball-center collision envelope.
#[derive(Debug, Clone)]
pub enum MockGeometry {
    RotatingPlane {
        pivot: [f64; 2],
        distance: f64,
        angle: f64,
        a_start: f64,
        vertical: bool,
        ball_radius: f64,
        offset: Position,
    },
    Rotary {
        center: [f64; 3],
        slope: [f64; 2],
        eccentric: [f64; 2],
        a_start: f64,
        radius: f64,
        ball_radius: f64,
        offset: Position,
    },
    Empty,
    Plane {
        axis: Axis,
        coordinate: f64,
    },
    Corner {
        origin: Position,
        directions: [i32; 2],
        travel: [f64; 2],
        top: f64,
        floor: f64,
        internal: bool,
    },
    Center {
        center: [f64; 2],
        half: [f64; 2],
        enabled: [bool; 2],
        round: bool,
        internal: bool,
        top: f64,
        floor: f64,
    },
}
impl MockGeometry {
    pub fn intersect(&self, axis: Axis, position: Position, delta: f64) -> Option<f64> {
        let i = axis.index();
        match self {
            Self::RotatingPlane {
                pivot,
                distance,
                angle,
                a_start,
                vertical,
                ball_radius,
                offset,
            } => {
                if axis == Axis::X {
                    return None;
                }
                let theta = (angle + position[3] - a_start).to_radians();
                let normal = if *vertical {
                    [-theta.cos(), -theta.sin()]
                } else {
                    [-theta.sin(), theta.cos()]
                };
                let ball = [
                    position[1] + offset[1],
                    position[2] - offset[2] + ball_radius,
                ];
                let j = i - 1;
                if normal[j].abs() < 1e-9 {
                    return None;
                }
                let signed = (0..2)
                    .map(|k| normal[k] * (ball[k] - pivot[k]))
                    .sum::<f64>();
                Some(position[i] + (distance + ball_radius - signed) / normal[j])
            }
            Self::Rotary {
                center,
                slope,
                eccentric,
                a_start,
                radius,
                ball_radius,
                offset,
            } => {
                if axis == Axis::X {
                    return None;
                }
                let dx = position[0] + offset[0] - center[0];
                let a = (position[3] - a_start).to_radians();
                let cy = center[1] + slope[0] * dx + eccentric[0] * a.cos()
                    - eccentric[1] * a.sin()
                    - offset[1];
                let cz = center[2]
                    + slope[1] * dx
                    + eccentric[0] * a.sin()
                    + eccentric[1] * a.cos()
                    + offset[2]
                    - ball_radius;
                let (coordinate, other) = if axis == Axis::Y {
                    (cy, position[2] - cz)
                } else {
                    (cz, position[1] - cy)
                };
                let square = (radius + ball_radius).powi(2) - other.powi(2);
                (square >= 0.0).then(|| coordinate - delta.signum() * square.sqrt())
            }
            Self::Empty => None,
            Self::Plane {
                axis: a,
                coordinate,
            } => {
                if *a == axis {
                    Some(*coordinate)
                } else {
                    None
                }
            }
            Self::Corner {
                origin,
                directions,
                travel,
                top,
                floor,
                internal,
            } => {
                if i == 2 {
                    if *internal {
                        return Some(*floor);
                    }
                    let inside = (0..2).all(|j| {
                        directions[j] == 0
                            || f64::from(directions[j])
                                * (position[j]
                                    - (origin[j] - f64::from(directions[j]) * travel[j] * 0.4))
                                >= -0.001
                    });
                    return inside.then_some(*top);
                }
                (directions[i] != 0 && position[2] < *top).then_some(
                    origin[i]
                        + if *internal { 1.0 } else { -1.0 }
                            * f64::from(directions[i])
                            * travel[i]
                            * 0.4,
                )
            }
            Self::Center {
                center,
                half,
                enabled,
                round,
                internal,
                top,
                floor,
            } => {
                if i == 2 {
                    if *internal {
                        return Some(*floor);
                    }
                    let inside = if *round {
                        (position[0] - center[0]).hypot(position[1] - center[1]) < half[0]
                    } else {
                        (0..2).all(|j| !enabled[j] || (position[j] - center[j]).abs() <= half[j])
                    };
                    return inside.then_some(*top);
                }
                if !enabled[i] || position[2] >= *top {
                    return None;
                }
                let mut distance = half[i];
                if *round {
                    let other = 1 - i;
                    let squared = distance * distance - (position[other] - center[other]).powi(2);
                    if squared <= 0.0 {
                        return None;
                    }
                    distance = squared.sqrt();
                }
                Some(center[i] + delta.signum() * if *internal { 1.0 } else { -1.0 } * distance)
            }
        }
    }
}
struct Inner {
    state: State,
    geometry: MockGeometry,
    offsets: BTreeMap<i32, Position>,
    commands: Vec<String>,
}
pub struct MockController {
    inner: Mutex<Inner>,
    events: broadcast::Sender<Event>,
    command_delay: std::time::Duration,
}
impl Default for MockController {
    fn default() -> Self {
        Self::new()
    }
}
impl MockController {
    pub fn set_rotary_geometry(&self, config: &RotaryConfig, eccentric: [f64; 2], slope: [f64; 2]) {
        let s = self.state();
        self.set_geometry(MockGeometry::Rotary {
            center: [
                s.position[0] + s.probe_offset[0],
                s.position[1] + s.probe_offset[1],
                s.position[2] - s.probe_offset[2] - 3.0 - config.rod_diameter / 2.0,
            ],
            slope,
            eccentric,
            a_start: s.position[3],
            radius: config.rod_diameter / 2.0,
            ball_radius: config.diameter / 2.0,
            offset: s.probe_offset,
        });
    }
    pub fn new() -> Self {
        Self::with_state(State {
            homed: true,
            modes: Modes {
                units: 21,
                distance: 90,
                feed: 94,
            },
            connected: true,
            ready: true,
            motion_blocked: false,
            spindle_stopped: true,
            probe_extended: false,
            probe_trigger_known: false,
            probe_triggered: false,
            probe_offset_known: true,
            travel_limits_known: true,
            travel_limits: [240.0, 235.0, 125.0, 0.0],
            position: [-120.0, -110.0, -60.0, 0.0],
            wcs: 54,
            work_position: [35.0, 20.0, -8.0, 0.0],
            probe_offset: [-55.872, -5.362, -49.98, 0.0],
            tool: 3,
            firmware_version: "1.0.35+c0.mock".into(),
            plane: 17,
            wcs_rotation: Some(0.0),
            wcs_origin: None,
            coordinate_offset: [0.0; 4],
            tool_length_offset: 0.0,
        })
    }
    pub fn with_state(mut state: State) -> Self {
        if state.wcs_origin.is_none() {
            state.wcs_origin = Some(Self::origin_from_work(&state));
        }
        let (events, _) = broadcast::channel(256);
        Self {
            inner: Mutex::new(Inner {
                state,
                geometry: MockGeometry::Empty,
                offsets: BTreeMap::new(),
                commands: Vec::new(),
            }),
            events,
            command_delay: std::time::Duration::ZERO,
        }
    }
    fn origin_from_work(state: &State) -> Position {
        let (sin, cos) = state.wcs_rotation.unwrap_or(0.).to_radians().sin_cos();
        let x = state.work_position[0] + state.coordinate_offset[0];
        let y = state.work_position[1] + state.coordinate_offset[1];
        [
            state.position[0] - cos * x + sin * y,
            state.position[1] - sin * x - cos * y,
            state.position[2]
                - state.work_position[2]
                - state.coordinate_offset[2]
                - state.tool_length_offset,
            state.position[3] - state.work_position[3] - state.coordinate_offset[3],
        ]
    }
    /// Pace preview submission without holding a lock or leaving simulated motion
    /// pending when the caller cancels the send future.
    pub fn with_delay(mut self, delay: std::time::Duration) -> Self {
        self.command_delay = delay;
        self
    }
    pub fn state(&self) -> State {
        self.inner.lock().unwrap().state.clone()
    }
    pub fn snapshot(&self) -> State {
        self.state()
    }
    pub fn commands(&self) -> Vec<String> {
        self.inner.lock().unwrap().commands.clone()
    }
    pub fn set_geometry(&self, geometry: MockGeometry) {
        self.inner.lock().unwrap().geometry = geometry;
    }
    pub fn configure(&self, c: &RoutineConfig) -> Result<(), Error> {
        c.validate()?;
        let mut inner = self.inner.lock().unwrap();
        let s = inner.state.position;
        inner.geometry = if c.z {
            MockGeometry::Plane {
                axis: Axis::Z,
                coordinate: s[2] - c.depth * 0.4,
            }
        } else if c.family == "center" {
            let internal = c.center_internal();
            let round = matches!(c.feature.as_str(), "boss" | "hole");
            let radius = c.diameter / 2.0 * if internal { -1.0 } else { 1.0 };
            MockGeometry::Center {
                center: [s[0] + 1.0, s[1] - 1.5],
                half: [
                    c.x_search_distance * 0.35 + radius,
                    if round {
                        c.x_search_distance
                    } else {
                        c.y_search_distance
                    } * 0.35
                        + radius,
                ],
                enabled: [c.x != 0, c.y != 0],
                round,
                internal,
                top: s[2] + if internal { 2.0 } else { -c.depth * 0.5 },
                floor: s[2] - c.side_depth() - c.depth * 1.2,
            }
        } else {
            MockGeometry::Corner {
                origin: s,
                directions: [c.x, c.y],
                travel: [c.x_search_distance, c.y_search_distance],
                top: s[2]
                    + if c.family == "inside" {
                        2.0
                    } else {
                        -c.depth * 0.5
                    },
                floor: s[2] - c.side_depth() - c.depth,
                internal: c.family == "inside",
            }
        };
        Ok(())
    }
    pub fn set_extended(&self, extended: bool) -> Result<(), Error> {
        let mut inner = self.inner.lock().unwrap();
        if !inner.state.ready || inner.state.motion_blocked {
            return Err(Error::Preflight("mock not ready for actuator".into()));
        }
        inner.state.probe_extended = extended;
        self.publish(&inner.state);
        Ok(())
    }
    pub fn select_wcs(&self, wcs: i32) -> Result<(), Error> {
        if !(54..=59).contains(&wcs) {
            return Err(Error::InvalidConfig("invalid WCS".into()));
        }
        let mut inner = self.inner.lock().unwrap();
        if !inner.state.ready {
            return Err(Error::Preflight("mock moving".into()));
        }
        let old = inner.state.wcs;
        let offset =
            std::array::from_fn(|i| inner.state.position[i] - inner.state.work_position[i]);
        inner.offsets.insert(old, offset);
        let new = *inner.offsets.entry(wcs).or_insert([0.0; 4]);
        inner.state.wcs = wcs;
        inner.state.wcs_origin = Some(new);
        for (i, off) in new.iter().enumerate() {
            inner.state.work_position[i] = inner.state.position[i] - off;
        }
        self.publish(&inner.state);
        Ok(())
    }
    fn publish(&self, s: &State) {
        self.publish_mode(s, if s.ready { "Ready" } else { "Moving" });
    }
    fn publish_mode(&self, s: &State, mode: &str) {
        let _ = self.events.send(Event {
            status: Some(MotionStatus {
                mode: mode.into(),
                homed: s.homed,
                ready: s.ready,
                motion_blocked: s.motion_blocked,
                position: s.position,
                probe_actuator: if s.probe_extended { 1 } else { 0 },
                probe_actuator_known: true,
                wcs: s.wcs,
                work_position: s.work_position,
            }),
            ..Event::default()
        });
    }
}
#[async_trait]
impl Controller for MockController {
    fn state(&self) -> State {
        self.state()
    }
    fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }
    async fn send(&self, command: &str) -> Result<(), Error> {
        if !self.command_delay.is_zero() {
            tokio::time::sleep(self.command_delay).await;
        }
        let mut inner = self.inner.lock().unwrap();
        if !inner.state.connected {
            return Err(Error::Disconnected);
        }
        inner.commands.push(command.into());
        if command == "$G" {
            let _ = self.events.send(Event {
                modes: Some(inner.state.modes),
                plane: Some(inner.state.plane),
                ..Event::default()
            });
            return Ok(());
        }
        if command == "$V" {
            let _ = self.events.send(Event {
                firmware_version: Some(inner.state.firmware_version.clone()),
                ..Event::default()
            });
            return Ok(());
        }
        if command == "$#" {
            let _ = self.events.send(Event {
                wcs_origin: Some((inner.state.wcs, inner.state.wcs_origin.unwrap())),
                ..Event::default()
            });
            let _ = self.events.send(Event {
                wcs_rotation: Some((inner.state.wcs, inner.state.wcs_rotation.unwrap_or(0.0))),
                coordinate_offset: Some(inner.state.coordinate_offset),
                tool_length_offset: Some(inner.state.tool_length_offset),
                ..Event::default()
            });
            let _ = self.events.send(Event {
                probe: Some(Contact {
                    position: inner.state.position,
                    success: false,
                }),
                ..Event::default()
            });
            return Ok(());
        }
        if matches!(command, "G17" | "G18" | "G19") {
            inner.state.plane = command[1..].parse().unwrap();
            let _ = self.events.send(Event {
                acknowledged: true,
                ..Event::default()
            });
            return Ok(());
        }
        let mut words: Vec<_> = command.split_whitespace().collect();
        if words.first() == Some(&"G19") && words.len() > 1 {
            inner.state.plane = 19;
            words.remove(0);
        }
        let a_delta = words
            .iter()
            .find_map(|w| w.strip_prefix('A').and_then(|v| v.parse::<f64>().ok()))
            .unwrap_or(0.0);
        if words.len() == 3 && matches!(words[0], "G20" | "G21") {
            let parse = |w: &str| {
                w.strip_prefix('G')
                    .and_then(|v| v.parse::<i32>().ok())
                    .ok_or_else(|| Error::Controller("invalid modal word".into()))
            };
            let modes = Modes {
                units: parse(words[0])?,
                feed: parse(words[1])?,
                distance: parse(words[2])?,
            };
            if !modes.valid() {
                return Err(Error::Controller("invalid modes".into()));
            }
            inner.state.modes = modes;
            return Ok(());
        }
        if inner.state.motion_blocked {
            return Err(Error::MotionBlocked);
        }
        if matches!(command, "M122" | "M121") {
            inner.state.probe_extended = command == "M122";
            self.publish(&inner.state);
            return Ok(());
        }
        if command == "$H" {
            inner.state.ready = false;
            inner.state.homed = false;
            self.publish_mode(&inner.state, "Homing");
            let origin: Position =
                std::array::from_fn(|i| inner.state.position[i] - inner.state.work_position[i]);
            inner.state.position = [0.0; 4];
            inner.state.work_position =
                std::array::from_fn(|i| inner.state.position[i] - origin[i]);
            inner.state.homed = true;
            inner.state.ready = true;
            self.publish(&inner.state);
            return Ok(());
        }
        let mut axes = Vec::new();
        for w in &words {
            let axis = match w.as_bytes().first() {
                Some(b'X') => Axis::X,
                Some(b'Y') => Axis::Y,
                Some(b'Z') => Axis::Z,
                _ => continue,
            };
            let value = w[1..]
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| Error::Controller("invalid numeric word".into()))?;
            axes.push((axis, value));
        }
        if words.starts_with(&["G90", "G53", "G0"]) && !axes.is_empty() {
            inner.state.modes.distance = 90;
            inner.state.ready = false;
            self.publish(&inner.state);
            for (axis, value) in axes {
                let i = axis.index();
                inner.state.work_position[i] += value - inner.state.position[i];
                inner.state.position[i] = value;
            }
            inner.state.ready = true;
            self.publish(&inner.state);
            return Ok(());
        }
        if words.first() == Some(&"G10") {
            if words.get(1) == Some(&"L2") {
                if let Some(angle) = words
                    .iter()
                    .find_map(|w| w.strip_prefix('R').and_then(|v| v.parse::<f64>().ok()))
                {
                    inner.state.wcs_rotation = Some(angle);
                }
                let mut origin = inner.state.wcs_origin.unwrap();
                for (axis, value) in axes {
                    origin[axis.index()] = value;
                }
                if words.iter().any(|word| word.starts_with('A')) {
                    origin[3] = a_delta;
                }
                inner.state.wcs_origin = Some(origin);
                let angle = inner.state.wcs_rotation.unwrap_or(0.).to_radians();
                let x = inner.state.position[0] - origin[0];
                let y = inner.state.position[1] - origin[1];
                inner.state.work_position = [
                    angle.cos() * x + angle.sin() * y - inner.state.coordinate_offset[0],
                    angle.cos() * y - angle.sin() * x - inner.state.coordinate_offset[1],
                    inner.state.position[2]
                        - origin[2]
                        - inner.state.coordinate_offset[2]
                        - inner.state.tool_length_offset,
                    inner.state.position[3] - origin[3] - inner.state.coordinate_offset[3],
                ];
                self.publish(&inner.state);
                let _ = self.events.send(Event {
                    acknowledged: true,
                    ..Event::default()
                });
                return Ok(());
            }
            if words.get(1) != Some(&"L20")
                || words.get(2).copied() != Some(format!("P{}", inner.state.wcs - 53).as_str())
                || inner.state.modes.units != 21
                || axes.is_empty()
            {
                return Err(Error::Controller("invalid work-zero command".into()));
            }
            for (a, v) in axes {
                inner.state.work_position[a.index()] = v;
            }
            inner.state.wcs_origin = Some(Self::origin_from_work(&inner.state));
            self.publish(&inner.state);
            return Ok(());
        }
        if (axes.is_empty() && a_delta == 0.)
            || !matches!(
                words.first().copied(),
                Some("G38.2" | "G38.3" | "G1" | "G0" | "G2" | "G3")
            )
            || inner.state.modes != Modes::PROBING
        {
            return Err(Error::Controller(format!(
                "unsupported mock command: {command}"
            )));
        }
        let start = inner.state.position;
        let mut fraction = 1.0_f64;
        let probe = matches!(words[0], "G38.2" | "G38.3");
        let mut hit = false;
        inner.state.ready = false;
        self.publish(&inner.state);
        if probe {
            for &(axis, delta) in &axes {
                let i = axis.index();
                if let Some(plane) = inner.geometry.intersect(axis, start, delta) {
                    let t = (plane - start[i]) / delta;
                    if t > 0.0001 && t <= fraction {
                        fraction = t;
                        hit = true;
                    }
                }
            }
        }
        for (axis, delta) in axes {
            let i = axis.index();
            inner.state.position[i] = start[i] + delta * fraction;
            inner.state.work_position[i] += delta * fraction;
        }
        inner.state.position[3] += a_delta * fraction;
        inner.state.work_position[3] += a_delta * fraction;
        if probe {
            let _ = self.events.send(Event {
                probe: Some(Contact {
                    position: if words[0] == "G38.3" && hit {
                        [0.0; 4]
                    } else {
                        inner.state.position
                    },
                    success: hit,
                }),
                ..Event::default()
            });
            if words[0] == "G38.2" && !hit {
                inner.state.motion_blocked = true;
            }
        }
        inner.state.ready = !inner.state.motion_blocked;
        self.publish(&inner.state);
        Ok(())
    }
}
