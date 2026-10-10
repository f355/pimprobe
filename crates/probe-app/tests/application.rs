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

use pimprobe_app::{
    AppError, MeasurementResult, Motion, OperationEvent, ProbeApp, ResultWcsRequest, Token,
    ZeroRequest,
    device::{ActionGuard, DeviceSnapshot, ProbeDevice},
    history::{DiagnosticEvent, Diagnostics, HistoryRecord, HistoryStore, Records},
    host::{ExportResult, HostActions, SoftwareInfo},
    settings::{ProbeSettings, Settings, SettingsStore},
};
use pimprobe_core::{
    Controller, Error, Event, MockController, RepeatabilityController, RoutineConfig, State,
    async_trait,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{Notify, broadcast};

#[derive(Default)]
struct MemoryStore {
    fail_finish: AtomicBool,
    settings: Mutex<ProbeSettings>,
    history: Mutex<Vec<HistoryRecord>>,
    diagnostics: Mutex<Vec<DiagnosticEvent>>,
}
impl SettingsStore for MemoryStore {
    fn load(&self) -> std::io::Result<ProbeSettings> {
        Ok(self.settings.lock().unwrap().clone())
    }
    fn save(&self, values: &ProbeSettings) -> std::io::Result<()> {
        *self.settings.lock().unwrap() = values.clone();
        Ok(())
    }
}
impl HistoryStore for MemoryStore {
    fn append(&self, record: &HistoryRecord) -> std::io::Result<()> {
        if self.fail_finish.load(Ordering::SeqCst)
            && matches!(
                record.event,
                pimprobe_app::history::HistoryEvent::Finish { .. }
            )
        {
            return Err(std::io::Error::other("disk full"));
        }
        self.history.lock().unwrap().push(record.clone());
        Ok(())
    }
    fn read(&self) -> std::io::Result<Vec<HistoryRecord>> {
        Ok(self.history.lock().unwrap().clone())
    }
    fn clear(&self) -> std::io::Result<()> {
        self.history.lock().unwrap().clear();
        Ok(())
    }
}

#[tokio::test]
async fn a_failed_history_save_returns_the_measurement_with_the_error() {
    let (app, _, store) = app();
    let review = app.review(config()).await.unwrap();
    store.fail_finish.store(true, Ordering::SeqCst);
    let mut operation = app
        .start_motion(Token { id: review.id }, Motion::Run)
        .await
        .unwrap();
    let mut terminal = None;
    while let Some(event) = operation.recv().await {
        terminal = Some(event);
    }
    let Some(OperationEvent::Error {
        code,
        message,
        result: Some(MeasurementResult::Routine(result)),
    }) = terminal
    else {
        panic!("missing failed-save result")
    };
    assert_eq!(code, "log");
    assert!(message.contains("disk full"));
    assert!(result.machine_point[2].is_some());
}
impl Diagnostics for MemoryStore {
    fn record(&self, event: &DiagnosticEvent) -> std::io::Result<()> {
        self.diagnostics.lock().unwrap().push(event.clone());
        Ok(())
    }
    fn clear(&self) -> std::io::Result<()> {
        self.diagnostics.lock().unwrap().clear();
        Ok(())
    }
}
struct Host;
#[async_trait]
impl HostActions for Host {
    fn software_info(&self) -> SoftwareInfo {
        SoftwareInfo {
            version: "test".into(),
            commit: "test".into(),
        }
    }
    async fn export_logs(&self) -> Result<ExportResult, AppError> {
        Err(AppError::conflict(
            "destination",
            "Choose an export destination",
        ))
    }
}

struct Device {
    mock: MockController,
    owner: Arc<tokio::sync::Mutex<()>>,
    pause: AtomicBool,
    moving: AtomicBool,
    stopped: AtomicBool,
    entered: Notify,
    stopping: Notify,
}
impl Device {
    fn new() -> Self {
        let mock = MockController::new();
        mock.set_extended(true).unwrap();
        Self {
            mock,
            owner: Arc::new(tokio::sync::Mutex::new(())),
            pause: AtomicBool::new(false),
            moving: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            entered: Notify::new(),
            stopping: Notify::new(),
        }
    }
}
#[async_trait]
impl Controller for Device {
    fn state(&self) -> State {
        let mut state = self.mock.state();
        if self.moving.load(Ordering::SeqCst) {
            state.ready = false;
        }
        state
    }
    fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.mock.subscribe()
    }
    async fn send(&self, command: &str) -> Result<(), Error> {
        if self.pause.load(Ordering::SeqCst) && command.contains("G38.") {
            self.moving.store(true, Ordering::SeqCst);
            self.entered.notify_one();
            std::future::pending::<()>().await;
        }
        self.mock.send(command).await
    }
}
#[async_trait]
impl RepeatabilityController for Device {
    async fn set_probe(&self, extended: bool) -> Result<(), Error> {
        self.mock.set_extended(extended)
    }
}
#[async_trait]
impl ProbeDevice for Device {
    fn configure_rotary(&self, config: &pimprobe_core::RotaryConfig) -> Result<(), Error> {
        self.mock
            .set_rotary_geometry(config, [-0.2, 0.3], [0.003, -0.002]);
        Ok(())
    }
    fn session_id(&self) -> u64 {
        1
    }
    fn snapshot(&self) -> DeviceSnapshot {
        DeviceSnapshot {
            connected: true,
            settings: [(110, 12000.), (111, 9000.), (112, 6000.), (113, 7200.)].into(),
            ..Default::default()
        }
    }
    fn configure(&self, config: &RoutineConfig) -> Result<(), Error> {
        self.mock.configure(config)
    }
    fn try_acquire(&self) -> Result<ActionGuard, Error> {
        self.owner
            .clone()
            .try_lock_owned()
            .map(|g| Box::new(g) as ActionGuard)
            .map_err(|_| Error::Preflight("Machine is in use".into()))
    }
    async fn select_wcs(&self, wcs: i32) -> Result<(), Error> {
        self.mock.select_wcs(wcs)
    }
    async fn stop(&self) -> Result<(), Error> {
        self.stopping.notify_one();
        if self.stopped.load(Ordering::SeqCst) {
            self.moving.store(false, Ordering::SeqCst);
            Ok(())
        } else {
            Err(Error::Timeout)
        }
    }
}
fn app() -> (Arc<ProbeApp>, Arc<Device>, Arc<MemoryStore>) {
    let device = Arc::new(Device::new());
    let store = Arc::new(MemoryStore::default());
    let app = ProbeApp::new(
        device.clone(),
        Settings::load(store.clone()).unwrap(),
        Arc::new(Host),
        Records::new(store.clone(), store.clone()),
    );
    (app, device, store)
}
fn config() -> RoutineConfig {
    RoutineConfig {
        z: true,
        x: 0,
        y: 0,
        ..RoutineConfig::default()
    }
}

#[tokio::test]
async fn completed_measurement_can_zero_another_wcs_without_probing_again() {
    let (app, device, _) = app();
    let review = app.review(config()).await.unwrap();
    let mut run = app
        .start_motion(
            Token {
                id: review.id.clone(),
            },
            Motion::Run,
        )
        .await
        .unwrap();
    let mut measured = None;
    while let Some(event) = run.recv().await {
        if let OperationEvent::Result {
            result: MeasurementResult::Routine(result),
        } = event
        {
            measured = Some(result);
        }
    }
    let measured = measured.unwrap();
    let position = device.state().position;
    assert!(
        app.select_result_wcs(ResultWcsRequest {
            id: review.id.clone(),
            wcs: 60,
        })
        .await
        .is_err()
    );
    let result = app
        .select_result_wcs(ResultWcsRequest {
            id: review.id.clone(),
            wcs: 55,
        })
        .await
        .unwrap();
    assert_eq!(result.wcs, 55);
    assert_eq!(result.machine_point, measured.machine_point);
    assert_eq!(device.state().wcs, 55);
    assert_eq!(device.state().position, position);
    let zero = app
        .zero(ZeroRequest {
            id: review.id.clone(),
            offsets: [0.0; 3],
        })
        .await
        .unwrap();
    assert!(zero.zeroed);
    assert_eq!(zero.wcs, 55);
    assert_eq!(device.state().position, position);
    assert!(
        device
            .mock
            .commands()
            .iter()
            .any(|s| s.starts_with("G10 L20 P2 "))
    );
    for offset in [1.25, -2.0, 0.0] {
        let updated = app
            .zero(ZeroRequest {
                id: review.id.clone(),
                offsets: [0.0, 0.0, offset],
            })
            .await
            .unwrap();
        assert_eq!(updated.machine_point, measured.machine_point);
        let state = device.state();
        assert_eq!(state.position, position);
        assert!(
            (state.position[2]
                - state.work_position[2]
                - measured.machine_point[2].unwrap()
                - offset)
                .abs()
                < 0.001
        );
    }
    let result = app
        .select_result_wcs(ResultWcsRequest {
            id: review.id.clone(),
            wcs: 56,
        })
        .await
        .unwrap();
    assert_eq!(result.machine_point, measured.machine_point);
    app.zero(ZeroRequest {
        id: review.id,
        offsets: [0.0, 0.0, -1.0],
    })
    .await
    .unwrap();
    assert_eq!(device.state().position, position);
    assert_eq!(device.state().wcs, 56);
}

#[tokio::test]
async fn rotary_uses_host_ownership_and_keeps_results_when_history_fails() {
    for fail_history in [false, true] {
        let (app, device, store) = app();
        let review = app
            .review_rotary(pimprobe_core::RotaryConfig::default())
            .await
            .unwrap();
        let owner = device.owner.lock().await;
        assert_eq!(
            app.run_rotary(Token {
                id: review.id.clone()
            })
            .await
            .err()
            .unwrap()
            .code,
            "busy"
        );
        drop(owner);
        store.fail_finish.store(fail_history, Ordering::SeqCst);
        let mut run = app
            .run_rotary(Token {
                id: review.id.clone(),
            })
            .await
            .unwrap();
        let mut terminal = None;
        while let Some(event) = run.recv().await {
            terminal = Some(event);
        }
        let result = match terminal.unwrap() {
            OperationEvent::Result {
                result: MeasurementResult::Rotary(result),
            } if !fail_history => result,
            OperationEvent::Error {
                code,
                result: Some(MeasurementResult::Rotary(result)),
                ..
            } if fail_history => {
                assert_eq!(code, "log");
                result
            }
            other => panic!("unexpected rotary result: {other:?}"),
        };
        assert_eq!(result.stations.len(), 2);
        assert!(!app.state().contact_active);
        let result = app
            .apply_rotary(Token { id: review.id }, false)
            .await
            .unwrap();
        assert!(result.zeroed);
    }
}

#[tokio::test]
async fn rotary_position_change_rejects_run_without_controller_recovery() {
    let (app, device, _) = app();
    let review = app
        .review_rotary(pimprobe_core::RotaryConfig::default())
        .await
        .unwrap();
    device.mock.send("G21 G94 G91").await.unwrap();
    device.mock.send("G1 Y0.6 F1000").await.unwrap();
    let commands_before = device.mock.commands();
    let mut run = app.run_rotary(Token { id: review.id }).await.unwrap();
    let mut terminal = None;
    while let Some(event) = run.recv().await {
        terminal = Some(event);
    }
    let Some(OperationEvent::Error { code, message, .. }) = terminal else {
        panic!("expected a rejected rotary review");
    };
    assert_eq!(code, "failed");
    assert!(message.contains("The machine position or probe calibration changed"));
    assert_eq!(device.mock.commands(), commands_before);
    assert!(!app.state().recovery_failed);
}

#[tokio::test]
async fn host_ownership_conflict_can_be_retried() {
    let (app, device, _) = app();
    let owner = device.owner.lock().await;
    let error = app.review(config()).await.err().unwrap();
    assert_eq!(error.code, "busy");
    drop(owner);
    assert!(app.review(config()).await.is_ok());
}

#[tokio::test]
async fn dropping_a_run_keeps_machine_owned_until_stopping_is_confirmed() {
    let (app, device, _) = app();
    let review = app.review(config()).await.unwrap();
    device.pause.store(true, Ordering::SeqCst);
    let operation = app
        .start_motion(Token { id: review.id }, Motion::Run)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), device.entered.notified())
        .await
        .unwrap();
    drop(operation);
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        device.stopping.notified(),
    )
    .await
    .unwrap();
    assert!(device.owner.try_lock().is_err());
    assert!(app.select_wcs(55).await.is_err());
    device.stopped.store(true, Ordering::SeqCst);
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while app.state().contact_active {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(device.owner.try_lock().is_ok());
    app.select_wcs(55).await.unwrap();
    assert_eq!(device.state().wcs, 55);
}

#[tokio::test]
async fn measurement_and_later_work_zero_are_available_through_host_storage() {
    let (app, _, store) = app();
    let review = app.review(config()).await.unwrap();
    let mut operation = app
        .start_motion(
            Token {
                id: review.id.clone(),
            },
            Motion::Run,
        )
        .await
        .unwrap();
    let mut measured = None;
    while let Some(event) = operation.recv().await {
        match event {
            OperationEvent::Result {
                result: MeasurementResult::Routine(result),
            } => measured = Some(result),
            OperationEvent::Error { message, .. } => panic!("{message}"),
            _ => {}
        }
    }
    let measured = measured.unwrap();
    let result = app
        .zero(ZeroRequest {
            id: review.id,
            offsets: [0., 0., -2.],
        })
        .await
        .unwrap();
    assert!(result.zeroed);
    assert_eq!(result.machine_point, measured.machine_point);
    let history = app.history().await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].status, "success");
    assert_eq!(history[0].actions.len(), 1);
    assert_eq!(history[0].work_zero.as_ref().unwrap()["offsets"][2], -2.);
    assert!(!store.diagnostics.lock().unwrap().is_empty());
    assert_eq!(app.export_logs().await.unwrap_err().code, "destination");
    app.clear_logs().await.unwrap();
    assert!(app.history().await.unwrap().is_empty());
}
