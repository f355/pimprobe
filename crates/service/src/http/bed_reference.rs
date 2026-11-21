// SPDX-License-Identifier: GPL-2.0-or-later
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

use super::{ApiError, ApiJson, App};
use crate::device::Device;
use axum::{Json, extract::State};
use pimprobe_controller::SocketController;
use pimprobe_core::{
    Axis, ContactConfig, Controller, Error, EventReceiver, Modes, Position, async_trait,
    bed_reference::{self, NOMINAL_CONTACT, POSITIONING_FEED, Progress, Reference},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;

const COMMAND: Duration = Duration::from_secs(5);
const MOVE: Duration = Duration::from_secs(45);

#[derive(Default)]
pub(super) struct Bed {
    path: PathBuf,
    state: Mutex<RunState>,
}

#[derive(Default)]
struct RunState {
    cancel: Option<CancellationToken>,
    message: &'static str,
    readings: [Option<f64>; 3],
    pending: Option<Pending>,
}

struct Pending {
    reference: Reference,
    session: u64,
    position: Position,
}

impl Bed {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            ..Self::default()
        }
    }

    pub fn stop(&self) {
        if let Some(cancel) = &self.state.lock().unwrap().cancel {
            cancel.cancel();
        }
    }

    fn notify(&self, progress: Progress) {
        let mut state = self.state.lock().unwrap();
        match progress {
            Progress::Message(message) => state.message = message,
            Progress::Contact(axis, value) => state.readings[axis.index()] = Some(value),
        }
    }

    fn saved(&self) -> io::Result<Option<Reference>> {
        match fs::read(&self.path) {
            Ok(data) => serde_json::from_slice(&data)
                .map(Some)
                .map_err(io::Error::other),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn save(&self, reference: Reference) -> io::Result<()> {
        let parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer(&mut file, &reference)?;
        file.as_file().sync_all()?;
        file.persist(&self.path).map_err(|error| error.error)?;
        fs::File::open(parent)?.sync_all()
    }

    fn view(&self, device: &Device) -> Result<Value, ApiError> {
        let saved = self.saved().map_err(storage_error)?;
        let state = self.state.lock().unwrap();
        let reason = machine(device)
            .and_then(ready)
            .err()
            .map(|error| error.to_string());
        Ok(
            json!({"saved":saved, "measurement":state.pending.as_ref().map(|p| p.reference), "busy":state.cancel.is_some(),
            "message":state.message, "coordinates":state.readings, "reason":reason}),
        )
    }
}

pub(super) fn supported(device: &Device) -> bool {
    machine(device).is_ok()
}

fn machine(device: &Device) -> Result<&SocketController, Error> {
    if let Device::Machine(controller) = device
        && controller
            .snapshot()
            .status
            .is_some_and(|s| s.reference_known.is_some())
    {
        return Ok(controller);
    }
    Err(Error::Preflight(
        "This firmware does not support bed referencing.".into(),
    ))
}

fn ready(controller: &SocketController) -> Result<(), Error> {
    let state = controller.state();
    if !state.connected || !state.ready || !state.spindle_stopped || state.motion_blocked {
        return Err(Error::Preflight(
            "Wait until the machine is idle with the spindle stopped.".into(),
        ));
    }
    let status = controller.snapshot().status.unwrap_or_default();
    match (status.fourth_axis, status.vise) {
        (Some(false), Some(false)) => Ok(()),
        (Some(true), _) | (_, Some(true)) => Err(Error::Preflight(
            "Remove and disconnect the fourth axis and electric vise. They cover the bed corner."
                .into(),
        )),
        _ => Err(Error::Preflight(
            "Waiting for the fourth axis and vise connection status.".into(),
        )),
    }
}

pub(super) async fn get(State(app): State<Arc<App>>) -> Result<Json<Value>, ApiError> {
    Ok(Json(app.bed.view(&app.device)?))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Action {
    Record,
    Reference,
    Save,
    Stop,
}

#[derive(Deserialize)]
pub(super) struct Request {
    action: Action,
    #[serde(default)]
    confirmed_clear: bool,
}

pub(super) async fn action(
    State(app): State<Arc<App>>,
    ApiJson(request): ApiJson<Request>,
) -> Result<Json<Value>, ApiError> {
    machine(&app.device).map_err(failure)?;
    if matches!(request.action, Action::Stop) {
        app.bed.stop();
        return Ok(Json(app.bed.view(&app.device)?));
    }
    let guard = app.probe.acquire()?;
    tokio::spawn(async move {
        let _guard = guard;
        let controller = machine(&app.device).map_err(failure)?;
        ready(controller).map_err(failure)?;
        if matches!(request.action, Action::Save) {
            let mut state = app.bed.state.lock().unwrap();
            let pending = state.pending.as_ref().ok_or_else(|| {
                ApiError::conflict("bed", "Measure the bed corner before saving.")
            })?;
            if controller.snapshot().session_id != pending.session
                || controller
                    .state()
                    .position
                    .iter()
                    .zip(pending.position)
                    .any(|(a, b)| (a - b).abs() > 0.002)
            {
                return Err(ApiError::conflict(
                    "bed",
                    "The machine moved. Measure the bed corner again before saving.",
                ));
            }
            app.bed.save(pending.reference).map_err(storage_error)?;
            state.pending = None;
        } else {
            if !request.confirmed_clear {
                return Err(ApiError::conflict(
                    "bed",
                    "Confirm that the bed is clear and the spindle is empty.",
                ));
            }
            let saved = app.bed.saved().map_err(storage_error)?;
            let restore = matches!(request.action, Action::Reference);
            if restore && saved.is_none() {
                return Err(ApiError::conflict("bed", "Record the bed reference first."));
            }
            let cancel = CancellationToken::new();
            *app.bed.state.lock().unwrap() = RunState {
                cancel: Some(cancel.clone()),
                ..RunState::default()
            };
            let result = run(&app, controller, saved, restore, cancel).await;
            app.bed.state.lock().unwrap().cancel = None;
            result.map_err(failure)?;
        }
        Ok(Json(app.bed.view(&app.device)?))
    })
    .await
    .map_err(|error| ApiError::conflict("bed", error.to_string()))?
}

async fn run(
    app: &App,
    controller: &SocketController,
    saved: Option<Reference>,
    restore: bool,
    cancel: CancellationToken,
) -> Result<(), Error> {
    let values = app
        .probe
        .settings()
        .await
        .map_err(|error| Error::Controller(error.message))?;
    let number = |key: &str| values[key].as_f64().unwrap_or(f64::NAN);
    let contact = ContactConfig {
        axis: Axis::Z,
        direction: -1,
        coarse_travel: 16.,
        retract_distance: number("retractDistance"),
        coarse_feed: number("coarseFeed"),
        fine_feed: number("fineFeed"),
        retract_feed: POSITIONING_FEED,
    };
    pimprobe_core::plan_contact(contact)?;
    controller.command("$TOOLS", COMMAND).await?;
    if controller.snapshot().tool_present == Some(true) {
        return Err(Error::Preflight(
            "Remove the tool from the spindle before bed referencing.".into(),
        ));
    }
    let modes = controller.read_modes().await?;
    let plane = controller.state().plane;
    // Refresh rotations before interpreting any incremental displacement.
    controller.read_coordinates().await?;
    let session = controller.snapshot().session_id;
    let adapter = BedMotion(controller);
    let expected = saved.map_or(NOMINAL_CONTACT, |r| r.raw_point);
    let motion = async {
        adapter.set_probe(false).await?;
        app.bed
            .notify(Progress::Message("Referencing from limit switches."));
        controller.command("$H", Duration::from_secs(240)).await?;
        adapter
            .wait(|s| s.homed && s.reference_known == Some(true))
            .await?;
        ready(controller)?;
        controller.command("G21 G90 G94", COMMAND).await?;
        let status = controller.snapshot().status.unwrap_or_default();
        let open_lid = match (status.magazine, status.magazine_right, status.magazine_lid) {
            (Some(true), Some(false), Some(0)) => true,
            (Some(false), _, _)
            | (Some(true), Some(true), _)
            | (Some(true), Some(false), Some(1)) => false,
            _ => {
                return Err(Error::Preflight(
                    "Wait for the magazine orientation and lid status.".into(),
                ));
            }
        };
        if open_lid {
            app.bed
                .notify(Progress::Message("Opening the magazine lid."));
            controller.command("M117", MOVE).await?;
            adapter.wait(|s| s.magazine_lid == Some(1)).await?;
        }
        let measured =
            bed_reference::measure(&adapter, expected, contact, |p| app.bed.notify(p)).await?;
        if open_lid {
            app.bed
                .notify(Progress::Message("Closing the magazine lid."));
            controller.command("M118", MOVE).await?;
            adapter.wait(|s| s.magazine_lid == Some(0)).await?;
        }
        Ok(measured)
    };
    let outcome = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(Error::Cancelled),
        result = motion => result,
    };
    if outcome.is_err() && !controller.state().motion_blocked {
        app.probe.confirm_stopped().await?;
    }
    if controller.snapshot().session_id != session {
        return Err(Error::Disconnected);
    }
    if controller.state().ready && !controller.state().motion_blocked {
        controller
            .write_modes(modes, (17..=19).contains(&plane).then_some(plane))
            .await?;
    }
    let measured = outcome?;
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    if restore {
        ready(controller)?;
        let saved = saved.unwrap();
        let current = controller.state().position;
        let goal: [f64; 3] =
            std::array::from_fn(|i| current[i] + saved.raw_point[i] - measured.raw_point[i]);
        app.bed
            .notify(Progress::Message("Saving machine reference."));
        controller
            .command(
                &format!("$REFERENCE={:.6},{:.6},{:.6}", goal[0], goal[1], goal[2]),
                COMMAND,
            )
            .await?;
        adapter
            .wait(|s| {
                s.reference_known == Some(true)
                    && s.m_pos[..3]
                        .iter()
                        .zip(goal)
                        .all(|(actual, target)| (actual - target).abs() < 0.004)
            })
            .await?;
    } else {
        app.bed.state.lock().unwrap().pending = Some(Pending {
            reference: measured,
            session,
            position: controller.state().position,
        });
    }
    Ok(())
}

struct BedMotion<'a>(&'a SocketController);

impl BedMotion<'_> {
    async fn wait(
        &self,
        condition: impl Fn(&pimprobe_controller::parser::MachineStatus) -> bool + Send,
    ) -> Result<(), Error> {
        let session = self.0.snapshot().session_id;
        timeout(MOVE, async {
            loop {
                let snapshot = self.0.snapshot();
                if !snapshot.connected || snapshot.session_id != session {
                    return Err(Error::Disconnected);
                }
                if let Some(status) = snapshot.status.filter(|_| snapshot.status_fresh) {
                    if status.motion_blocked {
                        return Err(Error::MotionBlocked);
                    }
                    if status.complete && status.mode == "Ready" && condition(&status) {
                        return Ok(());
                    }
                }
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
}

#[async_trait]
impl bed_reference::Positioning for BedMotion<'_> {
    const BLENDS_POSITIONING: bool = false;

    async fn position(&self, source: &str) -> Result<(), Error> {
        // Each serial move settles before the next starts; omit the blend controls.
        for line in source.lines().filter(|line| line.contains(" G53 ")) {
            let mut target = [None; 3];
            for word in line.split_whitespace() {
                if let Some(axis) = ["X", "Y", "Z"]
                    .iter()
                    .position(|axis| word.starts_with(axis))
                {
                    target[axis] = Some(
                        word[1..]
                            .parse::<f64>()
                            .map_err(|error| Error::InvalidConfig(error.to_string()))?,
                    );
                }
            }
            self.0.command(line, MOVE).await?;
            self.wait(|s| {
                target.iter().zip(s.m_pos).all(|(target, actual)| {
                    target.is_none_or(|target| (actual - target).abs() < 0.005)
                })
            })
            .await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Controller for BedMotion<'_> {
    fn state(&self) -> pimprobe_core::State {
        self.0.state()
    }
    fn subscribe(&self) -> EventReceiver {
        self.0.subscribe().into()
    }

    async fn send(&self, command: &str) -> Result<(), Error> {
        if command.contains(" G53 ") {
            return bed_reference::Positioning::position(self, command).await;
        }
        let angle = self.0.state().wcs_rotation.unwrap_or(0.);
        if angle == 0. || !(command.starts_with("G38.") || command.starts_with("G1 ")) {
            return self.0.send(command).await;
        }
        // Undo the selected work rotation for incremental XY moves, without changing the preset.
        let mut delta = [0.; 2];
        let mut words = Vec::new();
        for word in command.split_whitespace() {
            if let Some(axis) = ["X", "Y"].iter().position(|axis| word.starts_with(axis)) {
                delta[axis] = word[1..]
                    .parse::<f64>()
                    .map_err(|error| Error::InvalidConfig(error.to_string()))?;
            } else {
                words.push(word);
            }
        }
        let [x, y] = delta;
        if x == 0. && y == 0. {
            return self.0.send(command).await;
        }
        let (sin, cos) = angle.to_radians().sin_cos();
        self.0
            .send(&format!(
                "{} X{:.6} Y{:.6}",
                words.join(" "),
                cos * x + sin * y,
                -sin * x + cos * y
            ))
            .await
    }

    async fn set_probe(&self, extended: bool) -> Result<(), Error> {
        let target = i32::from(extended);
        if !self
            .0
            .snapshot()
            .status
            .is_some_and(|s| s.probe_actuator_known && s.probe_actuator == target)
        {
            self.0
                .command(if extended { "M122" } else { "M121" }, MOVE)
                .await?;
        }
        self.wait(|s| s.probe_actuator_known && s.probe_actuator == target)
            .await
    }

    async fn read_settings(&self) -> Result<BTreeMap<i32, f64>, Error> {
        self.0.read_settings().await
    }
    async fn read_modes(&self) -> Result<Modes, Error> {
        self.0.read_modes().await
    }
    async fn read_firmware_version(&self) -> Result<String, Error> {
        self.0.read_firmware_version().await
    }
    async fn read_coordinates(&self) -> Result<pimprobe_core::State, Error> {
        self.0.read_coordinates().await
    }
    async fn write_setting(&self, key: i32, value: f64) -> Result<(), Error> {
        self.0.write_setting(key, value).await
    }
    async fn write_modes(&self, modes: Modes, plane: Option<i32>) -> Result<(), Error> {
        self.0.write_modes(modes, plane).await
    }
    async fn write_coordinates(
        &self,
        wcs: i32,
        axes: [Option<f64>; 4],
        rotation: Option<f64>,
    ) -> Result<(), Error> {
        self.0.write_coordinates(wcs, axes, rotation).await
    }
}

fn failure(error: Error) -> ApiError {
    ApiError::conflict("bed", error.to_string())
}

fn storage_error(error: io::Error) -> ApiError {
    ApiError(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "bed",
        format!("Could not read or save the bed reference: {error}"),
    )
}
