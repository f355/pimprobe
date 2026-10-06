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

use async_trait::async_trait;
use pimprobe_core::{Error, RepeatabilityController, RoutineConfig};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Held for a whole action, including stopping after cancellation.
/// The host releases its machine command lock when this value is dropped.
pub type ActionGuard = Box<dyn Send>;

#[async_trait]
pub trait ProbeDevice: RepeatabilityController + 'static {
    fn session_id(&self) -> u64;
    fn is_mock(&self) -> bool {
        false
    }
    fn snapshot(&self) -> DeviceSnapshot;
    fn configure(&self, _config: &RoutineConfig) -> Result<(), Error> {
        Ok(())
    }
    fn configure_repeatability(&self, _diameter: f64) -> Result<(), Error> {
        Ok(())
    }
    fn configure_rotary(&self, _config: &pimprobe_core::RotaryConfig) -> Result<(), Error> {
        Ok(())
    }
    /// Standalone applications can rely on ProbeApp's single-action lock.
    /// Hosts sharing a controller with other components supply their own guard.
    fn try_acquire(&self) -> Result<ActionGuard, Error> {
        Ok(Box::new(()))
    }
    async fn select_wcs(&self, wcs: i32) -> Result<(), Error>;
    async fn stop(&self) -> Result<(), Error>;
    async fn shutdown(&self) {}
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSnapshot {
    #[serde(default)]
    pub firmware_version: String,
    #[serde(default)]
    pub plane: i32,
    #[serde(default)]
    pub wcs_rotations: BTreeMap<i32, f64>,
    #[serde(default)]
    pub wcs_origins: BTreeMap<i32, pimprobe_core::Position>,
    #[serde(default)]
    pub coordinate_offset: pimprobe_core::Position,
    #[serde(default)]
    pub tool_length_offset: f64,
    pub connected: bool,
    #[serde(default)]
    pub session_id: u64,
    #[serde(default)]
    pub connection_error: String,
    pub status: Option<DeviceStatus>,
    #[serde(default)]
    pub settings: BTreeMap<i32, f64>,
    #[serde(default)]
    pub last_probe: Option<pimprobe_core::Contact>,
    #[serde(default)]
    pub last_error: Option<ControllerError>,
    #[serde(default)]
    pub modes: pimprobe_core::Modes,
    #[serde(default)]
    pub status_fresh: bool,
    pub actuator_pending: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ControllerError {
    pub code: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatus {
    #[serde(default)]
    pub complete: bool,
    #[serde(default)]
    pub homed: bool,
    pub mode: String,
    pub machine_position: pimprobe_core::Position,
    pub work_position: pimprobe_core::Position,
    pub wcs: i32,
    pub tool: i32,
    pub spindle_mode: i32,
    pub probe_actuator: i32,
    pub probe_actuator_known: bool,
    pub probe_trigger_known: bool,
    pub probe_triggered: bool,
    pub door_open: bool,
    pub motion_blocked: bool,
}
