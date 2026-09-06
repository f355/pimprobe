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

use pimprobe_controller::SocketController;
use pimprobe_core::{Controller, Error, Event, MockController, RoutineConfig, State, async_trait};
use serde_json::{Value, json};
use tokio::sync::broadcast;

pub enum Device {
    Machine(SocketController),
    Mock(Box<MockController>),
}

impl Device {
    pub fn session_id(&self) -> u64 {
        match self {
            Self::Machine(c) => c.snapshot().session_id,
            Self::Mock(_) => 0,
        }
    }
    pub fn is_mock(&self) -> bool {
        matches!(self, Self::Mock(_))
    }

    pub fn configure(&self, config: &RoutineConfig) -> Result<(), Error> {
        if let Self::Mock(mock) = self {
            mock.configure(config)?;
        }
        Ok(())
    }

    pub fn snapshot(&self) -> Value {
        match self {
            Self::Machine(controller) => {
                let mut snapshot = serde_json::to_value(controller.snapshot())
                    .expect("finite controller snapshot");
                if let Some(code) = snapshot["lastError"].as_i64() {
                    snapshot["lastError"] = json!({"code":code});
                }
                // Machine actions require fresh telemetry.
                snapshot["connected"] = json!(controller.state().connected);
                snapshot
            }
            Self::Mock(mock) => {
                let state = mock.state();
                json!({
                    "connected":state.connected,
                    "status":{
                        "mode":if state.ready {"Ready"} else {"Run"},
                        "machinePosition":state.position,"workPosition":state.work_position,
                        "wcs":state.wcs,"tool":state.tool,"spindleMode":if state.spindle_stopped {5} else {3},
                        "probeActuator":if state.probe_extended {1} else {0},"probeActuatorKnown":true,
                        "probeTriggerKnown":state.probe_trigger_known,"probeTriggered":state.probe_triggered,
                        "doorOpen":false,"motionBlocked":state.motion_blocked
                    },
                    "settings":{"33":state.probe_offset[0],"34":state.probe_offset[1],"35":state.probe_offset[2]},
                    "actuatorPending":false
                })
            }
        }
    }

    pub async fn set_extended(&self, extended: bool) -> Result<(), Error> {
        match self {
            Self::Machine(controller) => controller.set_probe_extended(extended).await,
            Self::Mock(mock) => mock.set_extended(extended),
        }
    }

    pub async fn select_wcs(&self, wcs: i32) -> Result<(), Error> {
        match self {
            Self::Machine(controller) => controller.select_wcs(wcs).await,
            Self::Mock(mock) => mock.select_wcs(wcs),
        }
    }

    pub async fn stop(&self) -> Result<(), Error> {
        match self {
            Self::Machine(controller) => controller.stop().await,
            // The mock completes each submitted move synchronously.
            Self::Mock(mock) if mock.state().ready => Ok(()),
            Self::Mock(_) => Err(Error::Controller("mock did not settle".into())),
        }
    }

    pub async fn shutdown(&self) {
        if let Self::Machine(controller) = self {
            controller.shutdown().await;
        }
    }
}

#[async_trait]
impl Controller for Device {
    fn state(&self) -> State {
        match self {
            Self::Machine(c) => c.state(),
            Self::Mock(c) => c.state(),
        }
    }
    fn subscribe(&self) -> broadcast::Receiver<Event> {
        match self {
            Self::Machine(c) => c.subscribe(),
            Self::Mock(c) => c.subscribe(),
        }
    }
    async fn send(&self, command: &str) -> Result<(), Error> {
        match self {
            Self::Machine(c) => c.send(command).await,
            Self::Mock(c) => c.send(command).await,
        }
    }
}
