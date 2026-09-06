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

mod mock;
mod motion;
mod parameters;
mod plan;
pub use parameters::*;
mod runtime;
mod script;
pub use mock::*;
pub use tokio_util::sync::CancellationToken;

pub use async_trait::async_trait;
pub use motion::*;
pub use plan::*;
pub use runtime::*;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

pub type Position = [f64; 4];

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("cancelled; machine stop must be confirmed by the owner")]
    Cancelled,
    #[error("preflight: {0}")]
    Preflight(String),
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("controller: {0}")]
    Controller(String),
    #[error("controller disconnected")]
    Disconnected,
    #[error("controller events lost")]
    EventLagged,
    #[error("motion blocked")]
    MotionBlocked,
    #[error("position: {0}")]
    Position(String),
    #[error("coarse search made no contact")]
    CoarseNoContact,
    #[error("fine search made no contact")]
    NoContact,
    #[error("unexpected contact; released={retracted}")]
    UnexpectedContact { position: Position, retracted: bool },
    #[error("deadline exceeded")]
    Timeout,
    #[error("compensation: {0}")]
    Compensation(String),
    #[error("{cause}; recovery: {recovery}")]
    Recovery {
        cause: Box<Error>,
        recovery: Box<Error>,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modes {
    pub units: i32,
    pub distance: i32,
    pub feed: i32,
}
impl Modes {
    pub const PROBING: Self = Self {
        units: 21,
        distance: 91,
        feed: 94,
    };
    pub fn valid(self) -> bool {
        matches!(self.units, 20 | 21)
            && matches!(self.distance, 90 | 91)
            && matches!(self.feed, 93 | 94)
    }
    pub fn command(self) -> String {
        format!("G{} G{} G{}", self.units, self.feed, self.distance)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub modes: Modes,
    pub connected: bool,
    pub ready: bool,
    pub spindle_stopped: bool,
    pub motion_blocked: bool,
    pub probe_extended: bool,
    pub probe_trigger_known: bool,
    pub probe_triggered: bool,
    pub probe_offset_known: bool,
    pub travel_limits_known: bool,
    pub travel_limits: Position,
    pub position: Position,
    pub wcs: i32,
    pub work_position: Position,
    pub probe_offset: Position,
    pub tool: i32,
}
pub type ControllerState = State;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub position: Position,
    pub success: bool,
    pub tool_length: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MotionStatus {
    pub ready: bool,
    pub motion_blocked: bool,
    pub position: Position,
    pub probe_actuator: i32,
    pub probe_actuator_known: bool,
    pub wcs: i32,
    pub work_position: Position,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub modes: Option<Modes>,
    pub acknowledged: bool,
    pub controller_error: Option<i32>,
    pub probe: Option<Contact>,
    pub status: Option<MotionStatus>,
}

/// The caller owns exclusive machine command access for the entire run.
/// Subscribe before sending; publish moving status before a probe report and
/// settled status after it. Losing events is a fatal synchronization error.
#[async_trait]
pub trait Controller: Send + Sync {
    fn state(&self) -> State;
    fn subscribe(&self) -> broadcast::Receiver<Event>;
    async fn send(&self, command: &str) -> Result<(), Error>;
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    pub kind: String,
    pub command: String,
    pub message: String,
}

pub(crate) fn positive(v: f64) -> bool {
    v.is_finite() && v > 0.0
}
pub(crate) fn quantize(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}
pub(crate) fn finite(p: Position) -> bool {
    p.iter().all(|v| v.is_finite())
}
pub(crate) fn within(a: Position, b: Position, t: f64) -> bool {
    finite(a) && finite(b) && a.iter().zip(b).all(|(a, b)| (a - b).abs() <= t)
}
pub(crate) fn preflight(s: &State) -> Result<(), Error> {
    if s.motion_blocked {
        return Err(Error::MotionBlocked);
    }
    for (ok, why) in [
        (s.connected, "disconnected"),
        (s.ready, "not ready"),
        (s.spindle_stopped, "spindle running"),
        (s.probe_extended, "probe not extended"),
        (!s.probe_triggered, "probe already triggered"),
        (s.probe_offset_known, "unknown probe offset"),
        (s.travel_limits_known, "unknown travel limits"),
        (finite(s.position), "invalid position"),
    ] {
        if !ok {
            return Err(Error::Preflight(why.into()));
        }
    }
    Ok(())
}
pub(crate) fn check_path(s: &State, a: Axis, lo: f64, hi: f64) -> Result<(), Error> {
    let limit = s.travel_limits[a.index()];
    if !s.travel_limits_known
        || !positive(limit)
        || !lo.is_finite()
        || !hi.is_finite()
        || lo > hi
        || lo < -limit + 0.5
        || hi > -0.5
    {
        return Err(Error::Preflight(format!(
            "{a} path {lo:.3}..{hi:.3} exceeds machine travel"
        )));
    }
    Ok(())
}
