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

pub mod frame;
pub mod parser;

use parser::{MachineStatus, Record};
use pimprobe_core::{Contact, Controller, Error, Event, Modes, MotionStatus, Position, State};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tokio::{
    net::{unix::OwnedWriteHalf, UnixStream},
    sync::{broadcast, watch, Mutex},
    time::{sleep, timeout},
};

const FRESHNESS: Duration = Duration::from_secs(2);
const IO_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub connected: bool,
    pub session_id: u64,
    pub connection_error: String,
    pub status: Option<MachineStatus>,
    pub settings: BTreeMap<i32, f64>,
    pub last_probe: Option<Contact>,
    pub last_error: Option<i32>,
    pub modes: Modes,
    pub status_fresh: bool,
    pub actuator_pending: bool,
}
#[derive(Default)]
struct Stored {
    snapshot: Snapshot,
    status_at: Option<Instant>,
    actuator_target: i32,
    actuator_transition: bool,
}
struct Inner {
    stored: RwLock<Stored>,
    writer: Mutex<Option<OwnedWriteHalf>>,
    events: broadcast::Sender<Event>,
    action: Mutex<()>,
    travel_limits: Position,
}

struct FrameWrite<'a> {
    inner: &'a Inner,
    socket: Option<OwnedWriteHalf>,
}
impl Drop for FrameWrite<'_> {
    fn drop(&mut self) {
        if self.socket.is_some() {
            invalidate(self.inner, "controller frame write interrupted");
        }
    }
}

/// Cloneable handle. Dropping all handles stops the reconnect task.
#[derive(Clone)]
pub struct SocketController {
    inner: Arc<Inner>,
    stop: Arc<watch::Sender<bool>>,
    done: watch::Receiver<bool>,
}

fn failure(e: impl std::fmt::Display) -> Error {
    Error::Controller(e.to_string())
}

impl SocketController {
    pub async fn connect(path: impl AsRef<Path>, travel_limits: Position) -> Result<Self, Error> {
        let path = Self::path(path)?;
        let socket = timeout(IO_TIMEOUT, UnixStream::connect(&path))
            .await
            .map_err(failure)?
            .map_err(failure)?;
        Ok(Self::spawn(path, travel_limits, Some(socket)))
    }
    pub async fn start(path: impl AsRef<Path>, travel_limits: Position) -> Result<Self, Error> {
        Ok(Self::spawn(Self::path(path)?, travel_limits, None))
    }
    fn path(path: impl AsRef<Path>) -> Result<PathBuf, Error> {
        if path.as_ref().as_os_str().is_empty() {
            return Err(failure("controller socket path is required"));
        }
        Ok(path.as_ref().to_owned())
    }
    fn spawn(path: PathBuf, travel_limits: Position, socket: Option<UnixStream>) -> Self {
        let (events, _) = broadcast::channel(512);
        let inner = Arc::new(Inner {
            stored: RwLock::new(Stored::default()),
            writer: Mutex::new(None),
            events,
            action: Mutex::new(()),
            travel_limits,
        });
        let (stop, mut stopped) = watch::channel(false);
        let (finished, done) = watch::channel(false);
        let worker = inner.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = stopped.changed() => (),
                _ = reconnect(worker.clone(), path, socket) => (),
            }
            disconnect(&worker, "controller stopped").await;
            let _ = finished.send(true);
        });
        Self {
            inner,
            stop: Arc::new(stop),
            done,
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        let stored = self.inner.stored.read().unwrap();
        let mut s = stored.snapshot.clone();
        s.status_fresh =
            s.connected && stored.status_at.is_some_and(|at| at.elapsed() <= FRESHNESS);
        s
    }
    pub fn state(&self) -> State {
        let s = self.snapshot();
        let mut state = State {
            connected: s.connected && s.status_fresh,
            modes: s.modes,
            travel_limits: self.inner.travel_limits,
            ..Default::default()
        };
        if !state.connected {
            return state;
        }
        if let Some(status) = s.status {
            state.homed = status.homed;
            state.ready = status.complete && status.mode == "Ready";
            state.spindle_stopped = status.spindle_mode == 5;
            state.motion_blocked = status.motion_blocked;
            state.probe_extended =
                status.probe_actuator_known && status.probe_actuator == 1 && !s.actuator_pending;
            state.probe_trigger_known = status.probe_trigger_known;
            state.probe_triggered = status.probe_triggered;
            state.probe_offset_known = [33, 34, 35]
                .iter()
                .all(|k| s.settings.get(k).is_some_and(|v| v.is_finite()));
            state.travel_limits_known = self.inner.travel_limits[..3]
                .iter()
                .all(|v| v.is_finite() && *v > 0.0);
            state.position = status.m_pos;
            state.work_position = status.w_pos;
            state.wcs = status.wcs;
            state.tool = status.tool;
            state.probe_offset = [
                *s.settings.get(&33).unwrap_or(&0.0),
                *s.settings.get(&34).unwrap_or(&0.0),
                *s.settings.get(&35).unwrap_or(&0.0),
                0.0,
            ];
        }
        state
    }
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.inner.events.subscribe()
    }
    pub async fn send(&self, command: &str) -> Result<(), Error> {
        let command = command.strip_suffix('\n').unwrap_or(command);
        if command.is_empty() || command.contains(['\n', '\r', '\0']) {
            return Err(failure("expected one nonempty command"));
        }
        self.write(b'Q', format!("{command}\n").as_bytes()).await
    }
    pub async fn send_realtime(&self, bytes: &[u8]) -> Result<(), Error> {
        self.write(b'R', bytes).await
    }
    async fn write(&self, kind: u8, data: &[u8]) -> Result<(), Error> {
        if data.len() > frame::MAX_FRAME_SIZE {
            return Err(failure("oversize bridge frame"));
        }
        let mut writer = self.inner.writer.lock().await;
        // Remove the socket until the whole frame succeeds; cancellation poisons it.
        let mut frame_write = FrameWrite {
            inner: &self.inner,
            socket: Some(writer.take().ok_or(Error::Disconnected)?),
        };
        let result = timeout(
            IO_TIMEOUT,
            frame::write_frame(frame_write.socket.as_mut().unwrap(), kind, data),
        )
        .await
        .map_err(failure)
        .and_then(|r| r.map_err(failure));
        if result.is_ok() {
            *writer = frame_write.socket.take();
        }
        result
    }
    pub async fn shutdown(&self) {
        let _ = self.stop.send(true);
        let mut done = self.done.clone();
        let finished = *done.borrow_and_update();
        if !finished {
            let _ = done.changed().await;
        }
    }
    /// Stop motion without retracting the actuator or clearing an alarm.
    pub async fn stop(&self) -> Result<(), Error> {
        let mut events = self.subscribe();
        // The vendor parses hex notation into a framed realtime message.
        self.write(b'Q', b"0x19").await?;
        timeout(IO_TIMEOUT, async {
            let mut settled: Option<(Position, tokio::time::Instant)> = None;
            let mut last_status = tokio::time::Instant::now();
            loop {
                tokio::select! {
                    event = events.recv() => {
                        let event = event.map_err(failure)?;
                        if event.controller_error == Some(-1) { return Err(Error::Disconnected); }
                        let Some(status) = event.status else { continue };
                        if last_status.elapsed() > Duration::from_secs(1) { settled = None; }
                        last_status = tokio::time::Instant::now();
                        if !status.ready && !status.motion_blocked { settled = None; continue; }
                        if !status.position.iter().all(|v| v.is_finite()) { return Err(failure("non-finite stopped position")); }
                        if !settled.is_some_and(|(pos,_)| pos.iter().zip(status.position).all(|(a,b)| (a-b).abs() <= 0.005)) {
                            settled = Some((status.position, tokio::time::Instant::now()));
                        } else if settled.is_some_and(|(_, at)| at.elapsed() >= Duration::from_millis(500)) {
                            return Ok(());
                        }
                    }
                }
            }
        }).await.map_err(|_| Error::Timeout)?
    }
    pub async fn set_probe_extended(&self, extended: bool) -> Result<(), Error> {
        let _action = self
            .inner
            .action
            .try_lock()
            .map_err(|_| failure("controller action busy"))?;
        let snapshot = self.snapshot();
        let status = snapshot
            .status
            .ok_or_else(|| failure("machine status unavailable"))?;
        if !snapshot.connected
            || !snapshot.status_fresh
            || snapshot.actuator_pending
            || status.motion_blocked
            || status.mode != "Ready"
            || !status.complete
            || !status.probe_actuator_known
        {
            return Err(failure("probe actuator unavailable"));
        }
        let target = i32::from(extended);
        if status.probe_actuator == target {
            return Ok(());
        }
        if extended && status.probe_actuator != 0 {
            return Err(failure("probe moving or intermediate"));
        }
        {
            let mut stored = self.inner.stored.write().unwrap();
            stored.snapshot.actuator_pending = true;
            stored.actuator_target = target;
            stored.actuator_transition = matches!(status.probe_actuator, 2 | 3);
        }
        let result = self.send(if extended { "M122" } else { "M121" }).await;
        if result.is_err() {
            self.inner.stored.write().unwrap().snapshot.actuator_pending = false;
        }
        result
    }
    pub async fn select_wcs(&self, wcs: i32) -> Result<(), Error> {
        if !(54..=59).contains(&wcs) {
            return Err(failure("invalid WCS"));
        }
        let _action = self
            .inner
            .action
            .try_lock()
            .map_err(|_| failure("controller action busy"))?;
        let mut events = self.subscribe();
        let initial = self.state();
        if !initial.connected || !initial.ready || initial.motion_blocked {
            return Err(failure("controller not ready"));
        }
        if initial.wcs == wcs {
            return Ok(());
        }
        self.send(&format!("G{wcs}")).await?;
        timeout(IO_TIMEOUT, async {
            loop {
                let event = events.recv().await.map_err(failure)?;
                if let Some(code) = event.controller_error {
                    return Err(failure(format!("controller error:{code}")));
                }
                if let Some(status) = event.status {
                    if status.motion_blocked {
                        return Err(Error::MotionBlocked);
                    }
                    if !status
                        .position
                        .iter()
                        .zip(initial.position)
                        .all(|(a, b)| a.is_finite() && (a - b).abs() <= 0.05)
                    {
                        return Err(failure("position changed during WCS selection"));
                    }
                    if status.ready && status.wcs == wcs {
                        return Ok(());
                    }
                }
            }
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
}
#[async_trait::async_trait]
impl Controller for SocketController {
    fn state(&self) -> State {
        SocketController::state(self)
    }
    fn subscribe(&self) -> broadcast::Receiver<Event> {
        SocketController::subscribe(self)
    }
    async fn send(&self, command: &str) -> Result<(), Error> {
        SocketController::send(self, command).await
    }
}

fn invalidate(inner: &Inner, message: &str) {
    let mut stored = inner.stored.write().unwrap();
    *stored = Stored {
        snapshot: Snapshot {
            session_id: stored.snapshot.session_id,
            connection_error: message.into(),
            ..Default::default()
        },
        ..Default::default()
    };
    // Publish disconnects as errors to terminate in-flight waits.
    let _ = inner.events.send(Event {
        controller_error: Some(-1),
        ..Default::default()
    });
}
async fn disconnect(inner: &Inner, message: &str) {
    *inner.writer.lock().await = None;
    invalidate(inner, message);
}
async fn reconnect(inner: Arc<Inner>, path: PathBuf, mut socket: Option<UnixStream>) {
    loop {
        let connection = match socket.take() {
            Some(s) => Ok(s),
            None => match timeout(IO_TIMEOUT, UnixStream::connect(&path)).await {
                Ok(r) => r,
                Err(e) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, e)),
            },
        };
        let result = match connection {
            Ok(socket) => session(&inner, socket).await,
            Err(e) => Err(failure(e)),
        };
        disconnect(
            &inner,
            &result.err().map(|e| e.to_string()).unwrap_or_default(),
        )
        .await;
        sleep(Duration::from_millis(500)).await;
    }
}
async fn session(inner: &Inner, socket: UnixStream) -> Result<(), Error> {
    let (mut reader, mut writer) = socket.into_split();
    timeout(IO_TIMEOUT, frame::write_frame(&mut writer, b'Q', b"$P\n"))
        .await
        .map_err(failure)?
        .map_err(failure)?;
    {
        let mut slot = inner.writer.lock().await;
        let mut stored = inner.stored.write().unwrap();
        let session_id = stored
            .snapshot
            .session_id
            .checked_add(1)
            .ok_or_else(|| failure("controller session counter exhausted"))?;
        *stored = Stored {
            snapshot: Snapshot {
                connected: true,
                session_id,
                ..Default::default()
            },
            ..Default::default()
        };
        *slot = Some(writer);
    }
    loop {
        // The firmware goes quiet while homing; keep the session until reports resume.
        let read_timeout = if inner
            .stored
            .read()
            .unwrap()
            .snapshot
            .status
            .as_ref()
            .is_some_and(|s| s.mode == "Homing")
        {
            Duration::from_secs(600)
        } else {
            IO_TIMEOUT
        };
        let (kind, data) = timeout(read_timeout, frame::read_frame(&mut reader))
            .await
            .map_err(failure)?
            .map_err(failure)?;
        if !inner.stored.read().unwrap().snapshot.connected {
            return Err(Error::Disconnected);
        }
        if kind != b'D' {
            return Err(failure("unexpected controller bridge frame type"));
        }
        let text = std::str::from_utf8(&data).map_err(failure)?;
        for line in text.split(['\r', '\n']) {
            if let Some(record) = parser::parse_line(line).map_err(failure)? {
                apply(inner, record);
            }
        }
    }
}
fn apply(inner: &Inner, record: Record) {
    let mut stored = inner.stored.write().unwrap();
    if !stored.snapshot.connected {
        return;
    }
    let mut event = Event::default();
    match record {
        Record::Ack => event.acknowledged = true,
        Record::Error(code) => {
            stored.snapshot.last_error = Some(code);
            event.controller_error = Some(code);
        }
        Record::Modes(modes) => {
            stored.snapshot.modes = modes;
            event.modes = Some(modes);
        }
        Record::Setting(key, value) => {
            stored.snapshot.settings.insert(key, value);
            event.setting = Some((key, value));
        }
        Record::Probe(probe) => {
            stored.snapshot.last_probe = Some(probe.clone());
            event.probe = Some(probe);
        }
        Record::Status(mut status) => {
            if stored
                .status_at
                .is_some_and(|at| at.elapsed() < Duration::from_secs(2))
            {
                if let Some(previous) = &stored.snapshot.status {
                    status.inherit_modal_fields(previous);
                }
            }
            if stored.snapshot.actuator_pending && status.probe_actuator_known {
                if matches!(status.probe_actuator, 2 | 3) {
                    stored.actuator_transition = true;
                } else if stored.actuator_transition
                    && status.probe_actuator == stored.actuator_target
                {
                    stored.snapshot.actuator_pending = false;
                    stored.actuator_transition = false;
                }
            }
            event.status = Some(MotionStatus {
                ready: status.complete && status.mode == "Ready",
                motion_blocked: status.motion_blocked,
                position: status.m_pos,
                probe_actuator: status.probe_actuator,
                probe_actuator_known: status.probe_actuator_known,
                wcs: status.wcs,
                work_position: status.w_pos,
            });
            stored.snapshot.status = Some(status);
            stored.status_at = Some(Instant::now());
        }
    }
    let _ = inner.events.send(event);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tokio::net::UnixListener;

    struct SocketPath(PathBuf);
    impl Drop for SocketPath {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    fn listener() -> (SocketPath, UnixListener) {
        static ID: AtomicU64 = AtomicU64::new(0);
        let path = SocketPath(std::env::temp_dir().join(format!(
            "pimprobe-{}-{}.sock",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        )));
        let listener = UnixListener::bind(&path.0).unwrap();
        (path, listener)
    }
    async fn data(socket: &mut UnixStream, text: &str) {
        frame::write_frame(socket, b'D', text.as_bytes())
            .await
            .unwrap();
    }
    async fn wait_for(c: &SocketController, condition: impl Fn(Snapshot) -> bool) {
        timeout(Duration::from_secs(2), async {
            while !condition(c.snapshot()) {
                sleep(Duration::from_millis(2)).await;
            }
        })
        .await
        .unwrap();
    }
    async fn ready() -> (SocketPath, UnixListener, SocketController, UnixStream) {
        let (path, listener) = listener();
        let c = SocketController::connect(&path.0, [100.0; 4])
            .await
            .unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        assert_eq!(
            frame::read_frame(&mut socket).await.unwrap(),
            (b'Q', b"$P\n".to_vec())
        );
        data(
            &mut socket,
            "$33=1\r\n$34=2\n$35=3\n<Ready|MPos:-5,-5,-5,0|WPos:0,0,0,0|T:2|PM:0|Pn:|M:5|G:54>\n",
        )
        .await;
        wait_for(&c, |s| s.status_fresh).await;
        (path, listener, c, socket)
    }
    #[tokio::test]
    async fn fanout_actuator_and_sequencing() {
        let (_path, _listener, c, mut socket) = ready().await;
        assert!(c.state().probe_offset_known);
        let mut a = c.subscribe();
        let mut b = c.subscribe();
        c.set_probe_extended(true).await.unwrap();
        assert_eq!(
            frame::read_frame(&mut socket).await.unwrap(),
            (b'Q', b"M122\n".to_vec())
        );
        data(&mut socket, "ok\n").await;
        assert!(a.recv().await.unwrap().acknowledged);
        assert!(b.recv().await.unwrap().acknowledged);
        data(&mut socket, "<Ready|MPos:-5,-5,-5,0|PM:1>\n").await;
        a.recv().await.unwrap();
        assert!(c.snapshot().actuator_pending);
        assert!(!c.state().probe_extended);
        assert!(c.set_probe_extended(true).await.is_err());
        data(
            &mut socket,
            "<Ready|MPos:-5,-5,-5,0|PM:3>\n<Ready|MPos:-5,-5,-5,0|PM:1>\n",
        )
        .await;
        wait_for(&c, |s| !s.actuator_pending).await;
        assert!(c.state().probe_extended);
        c.shutdown().await;
    }
    #[tokio::test]
    async fn wcs_requires_status_and_propagates_errors() {
        let (_path, _listener, c, mut socket) = ready().await;
        let copy = c.clone();
        let task = tokio::spawn(async move { copy.select_wcs(55).await });
        assert_eq!(
            frame::read_frame(&mut socket).await.unwrap(),
            (b'Q', b"G55\n".to_vec())
        );
        data(&mut socket, "ok\n").await;
        sleep(Duration::from_millis(20)).await;
        assert!(!task.is_finished());
        data(
            &mut socket,
            "<Ready|MPos:-5,-5,-5,0|WPos:0,0,0,0|T:2|M:5|G:55>\n",
        )
        .await;
        task.await.unwrap().unwrap();
        let copy = c.clone();
        let task = tokio::spawn(async move { copy.select_wcs(56).await });
        frame::read_frame(&mut socket).await.unwrap();
        data(&mut socket, "error:9\n").await;
        assert!(task.await.unwrap().is_err());
        c.shutdown().await;
    }
    #[tokio::test]
    async fn homing_silence_keeps_the_controller_session() {
        let (_path, _listener, c, mut socket) = ready().await;
        let session = c.snapshot().session_id;
        data(&mut socket, "<Homing|MPos:-5,-5,-5,0|PM:1>\n").await;
        wait_for(&c, |s| {
            s.status.as_ref().is_some_and(|s| s.mode == "Homing")
        })
        .await;
        sleep(Duration::from_secs(4)).await;
        assert!(c.snapshot().connected);
        assert!(!c.snapshot().status_fresh);
        data(
            &mut socket,
            "<Ready|MPos:0,0,0,0|WPos:5,5,5,0|PM:0|T:0|M:5|G:54>\n",
        )
        .await;
        wait_for(&c, |s| s.status_fresh).await;
        assert_eq!(c.snapshot().session_id, session);
        c.shutdown().await;
    }
    #[tokio::test]
    async fn disconnect_invalidates_and_reconnects() {
        let (_path, listener, c, mut socket) = ready().await;
        let first_session = c.snapshot().session_id;
        assert_eq!(first_session, 1);
        assert_eq!(
            serde_json::to_value(c.snapshot()).unwrap()["sessionId"],
            first_session
        );
        let mut events = c.subscribe();
        frame::write_frame(&mut socket, b'Q', b"unexpected")
            .await
            .unwrap();
        assert_eq!(events.recv().await.unwrap().controller_error, Some(-1));
        assert!(!c.state().connected);
        assert!(c.snapshot().settings.is_empty());
        assert_eq!(c.snapshot().session_id, first_session);
        assert!(c.send("G54").await.is_err());
        let (mut socket, _) = timeout(Duration::from_secs(2), listener.accept())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            frame::read_frame(&mut socket).await.unwrap(),
            (b'Q', b"$P\n".to_vec())
        );
        data(&mut socket, "<Ready|MPos:-5,-5,-5,0|PM:1>\n").await;
        wait_for(&c, |s| s.status_fresh).await;
        assert_eq!(c.snapshot().session_id, first_session + 1);
        assert!(!c.state().probe_offset_known);
        c.inner.stored.write().unwrap().status_at = Some(Instant::now() - Duration::from_secs(3));
        assert!(!c.state().connected);
        assert!(c.set_probe_extended(false).await.is_err());
        c.shutdown().await;
        assert_eq!(c.snapshot().session_id, first_session + 1);
    }
    #[tokio::test]
    async fn stop_uses_framed_realtime_and_confirms_alarm_without_extra_motion() {
        let (_path, _listener, c, mut socket) = ready().await;
        let copy = c.clone();
        let task = tokio::spawn(async move { copy.stop().await });
        assert_eq!(
            frame::read_frame(&mut socket).await.unwrap(),
            (b'Q', b"0x19".to_vec())
        );
        data(&mut socket, "<Run|MPos:-5,-5,-5,0>\n").await;
        sleep(Duration::from_millis(20)).await;
        assert!(!task.is_finished());
        data(&mut socket, "<Alarm:1|MPos:-5,-5,-5,0>\n").await;
        sleep(Duration::from_millis(550)).await;
        assert!(
            !task.is_finished(),
            "one old alarm report cannot confirm settling"
        );
        data(&mut socket, "<Alarm:1|MPos:-5,-5,-5,0>\n").await;
        task.await.unwrap().unwrap();
        assert!(
            timeout(Duration::from_millis(20), frame::read_frame(&mut socket))
                .await
                .is_err()
        );
        c.shutdown().await;
    }
    #[tokio::test]
    async fn disconnected_start_is_available_and_commands_are_validated() {
        let (path, listener) = listener();
        drop(listener);
        let c = SocketController::start(&path.0, [100.0; 4]).await.unwrap();
        assert!(!c.state().connected);
        assert!(c.send("G54\nG55").await.is_err());
        assert!(c.send("G54").await.is_err());
        c.shutdown().await;
    }
    #[tokio::test]
    async fn incomplete_status_cannot_supply_ready_or_fake_work_coordinates() {
        let (_path, _listener, c, mut socket) = ready().await;
        let mut events = c.subscribe();
        data(&mut socket, "<Ready|MPos:-5,-5,-5,0|PM:1|M:5|G:54|T:2>\n").await;
        assert!(!events.recv().await.unwrap().status.unwrap().ready);
        assert!(!c.state().ready);
        assert!(!c.snapshot().status.unwrap().complete);
        assert!(c.set_probe_extended(false).await.is_err());
        assert!(c.select_wcs(55).await.is_err());
        c.shutdown().await;
    }
    #[tokio::test]
    async fn malformed_record_disconnects_and_stop_cannot_succeed_on_eof() {
        let (_path, _listener, c, mut socket) = ready().await;
        let copy = c.clone();
        let task = tokio::spawn(async move { copy.stop().await });
        frame::read_frame(&mut socket).await.unwrap();
        data(&mut socket, "<Ready|MPos:bad>\n").await;
        assert!(task.await.unwrap().is_err());
        assert!(!c.snapshot().connected);
        assert!(c.snapshot().connection_error.contains("controller"));
        c.shutdown().await;
    }
    #[tokio::test]
    async fn cancelled_write_poisoning_prevents_followup_frames() {
        let (_path, _listener, c, _socket) = ready().await;
        let copy = c.clone();
        let task =
            tokio::spawn(async move { copy.send_realtime(&vec![0; frame::MAX_FRAME_SIZE]).await });
        sleep(Duration::from_millis(20)).await;
        assert!(!task.is_finished(), "test requires socket backpressure");
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(!c.snapshot().connected);
        assert!(c.send("G54").await.is_err());
        c.shutdown().await;
    }
}
