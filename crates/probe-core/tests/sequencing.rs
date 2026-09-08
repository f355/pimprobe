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

use pimprobe_core::*;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::broadcast;

struct Scripted {
    state: Mutex<State>,
    events: broadcast::Sender<Event>,
    replies: Mutex<VecDeque<Vec<Event>>>,
    commands: Mutex<Vec<String>>,
    delay: Duration,
    modes_ack_only: bool,
}
impl Scripted {
    fn new(replies: Vec<Vec<Event>>) -> Self {
        let mut state = MockController::new().state();
        state.position[2] = -40.0;
        state.probe_extended = true;
        state.modes = Modes::PROBING;
        Self {
            state: Mutex::new(state),
            events: broadcast::channel(64).0,
            replies: Mutex::new(replies.into()),
            commands: Mutex::new(Vec::new()),
            delay: Duration::ZERO,
            modes_ack_only: false,
        }
    }
}
#[async_trait]
impl Controller for Scripted {
    fn state(&self) -> State {
        self.state.lock().unwrap().clone()
    }
    fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }
    async fn send(&self, command: &str) -> Result<(), Error> {
        self.commands.lock().unwrap().push(command.into());
        if command == "$G" {
            self.events
                .send(Event {
                    modes: (!self.modes_ack_only).then(|| self.state().modes),
                    acknowledged: self.modes_ack_only,
                    ..Event::default()
                })
                .unwrap();
            return Ok(());
        }
        if matches!(command, "G21 G94 G91" | "G21 G94 G90") {
            self.state.lock().unwrap().modes.distance =
                if command.ends_with("91") { 91 } else { 90 };
            return Ok(());
        }
        let replies = self.replies.lock().unwrap().pop_front().unwrap_or_default();
        for event in replies {
            if !self.delay.is_zero() {
                tokio::time::sleep(self.delay).await;
            }
            if let Some(status) = &event.status {
                let mut state = self.state.lock().unwrap();
                state.position = status.position;
                state.work_position = status.work_position;
                state.wcs = status.wcs;
                state.ready = status.ready;
                state.motion_blocked = status.motion_blocked;
            }
            self.events.send(event).unwrap();
        }
        Ok(())
    }
}
fn pos(x: f64) -> Position {
    [x, -110.0, -40.0, 0.0]
}
fn status(x: f64, ready: bool) -> Event {
    Event {
        status: Some(MotionStatus {
            position: pos(x),
            work_position: [x + 155.0, 20.0, -8.0, 0.0],
            ready,
            wcs: 54,
            ..MotionStatus::default()
        }),
        ..Event::default()
    }
}
fn contact(x: f64, success: bool) -> Event {
    Event {
        probe: Some(Contact {
            position: pos(x),
            success,
            tool_length: Some(-57.75),
        }),
        ..Event::default()
    }
}
fn config() -> ContactConfig {
    ContactConfig {
        axis: Axis::X,
        direction: 1,
        coarse_travel: 5.0,
        retract_distance: 0.5,
        coarse_feed: 1000.0,
        fine_feed: 1000.0,
        retract_feed: 1000.0,
    }
}
fn timing() -> TimingPolicy {
    TimingPolicy {
        response_margin: Duration::from_millis(20),
    }
}
fn good() -> Vec<Vec<Event>> {
    vec![
        vec![
            status(-120.0, false),
            contact(f64::NAN, true),
            status(-118.0, true),
        ],
        vec![status(-118.0, false), status(-118.5, true)],
        vec![
            status(-118.5, false),
            contact(-118.0, true),
            status(-117.96, true),
        ],
        vec![status(-117.96, false), status(-118.46, true)],
    ]
}

#[tokio::test]
async fn coarse_uses_actual_stop_fine_uses_trigger_and_retract_uses_stop() {
    let controller = Scripted::new(good());
    let r = run_contact(&controller, config(), timing()).await.unwrap();
    assert_eq!(r.contact.position, pos(-118.0));
    assert_eq!(r.final_position, pos(-118.46));
    assert_eq!(controller.commands.lock().unwrap().len(), 4);
}

#[tokio::test]
async fn stale_probe_and_ready_before_motion_are_ignored() {
    let mut events = good();
    events[0].splice(0..0, [contact(-119.0, true), status(-119.0, true)]);
    let c = Scripted::new(events);
    assert!(run_contact(&c, config(), timing()).await.is_ok());
}

#[tokio::test(start_paused = true)]
async fn stale_ready_and_ack_alone_do_not_complete_probe() {
    let c = Scripted::new(vec![vec![
        Event {
            acknowledged: true,
            ..Event::default()
        },
        contact(-118.0, true),
        status(-118.0, true),
    ]]);
    assert_eq!(
        run_contact(&c, config(), timing()).await.unwrap_err(),
        Error::Timeout
    );
    assert_eq!(c.commands.lock().unwrap().len(), 1);
}

#[tokio::test(start_paused = true)]
async fn delayed_release_waits_for_consistent_endpoint() {
    let mut replies = good();
    replies[1].insert(1, status(-118.0, true));
    let mut c = Scripted::new(replies);
    c.delay = Duration::from_millis(2);
    assert!(run_contact(&c, config(), timing()).await.is_ok());
}

#[tokio::test(start_paused = true)]
async fn inconsistent_release_times_out_as_position_error_and_never_fine_probes() {
    let mut replies = good();
    replies[1] = vec![status(-118.0, false), status(-118.0, true)];
    let c = Scripted::new(replies);
    assert!(matches!(
        run_contact(&c, config(), timing()).await,
        Err(Error::Position(_))
    ));
    assert_eq!(c.commands.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn bad_fine_coordinates_and_stop_are_rejected() {
    for bad in [
        contact(f64::NAN, true),
        contact(-119.0, true),
        contact(-117.0, true),
        Event {
            probe: Some(Contact {
                position: [-118.0, -109.0, -40.0, 0.0],
                success: true,
                tool_length: None,
            }),
            ..Event::default()
        },
    ] {
        let mut replies = good();
        replies[2][1] = bad;
        let c = Scripted::new(replies);
        assert!(matches!(
            run_contact(&c, config(), timing()).await,
            Err(Error::Position(_))
        ));
        assert_eq!(c.commands.lock().unwrap().len(), 3);
    }
    for stop in [-118.10, -117.80] {
        let mut replies = good();
        replies[2][2] = status(stop, true);
        let c = Scripted::new(replies);
        assert!(matches!(
            run_contact(&c, config(), timing()).await,
            Err(Error::Position(_))
        ));
    }
}

#[tokio::test]
async fn changed_wcs_nonfinite_status_and_controller_errors_abort() {
    let mut changed = status(-118.0, true);
    changed.status.as_mut().unwrap().wcs = 55;
    for bad in [
        changed,
        status(f64::INFINITY, true),
        Event {
            controller_error: Some(9),
            ..Event::default()
        },
    ] {
        let c = Scripted::new(vec![vec![
            status(-120.0, false),
            contact(-118.0, true),
            bad,
        ]]);
        assert!(run_contact(&c, config(), timing()).await.is_err());
        assert_eq!(c.commands.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn guarded_move_requires_failed_probe_report_and_full_endpoint() {
    for events in [
        vec![status(-120.0, false), status(-115.0, true)],
        vec![
            status(-120.0, false),
            contact(-117.0, false),
            status(-117.0, true),
        ],
    ] {
        let c = Scripted::new(vec![events]);
        assert!(matches!(
            run_guarded_move(
                &c,
                GuardedMoveConfig {
                    axis: Axis::X,
                    distance: 5.0,
                    feed: 1000.0,
                    retract_distance: 0.5
                },
                timing()
            )
            .await,
            Err(Error::Position(_))
        ));
    }
}

#[tokio::test]
async fn fine_miss_is_not_a_success_and_never_retracts_blindly() {
    let mut replies = good();
    replies[2] = vec![
        status(-118.5, false),
        contact(-117.5, false),
        status(-117.5, true),
    ];
    let c = Scripted::new(replies);
    assert_eq!(
        run_contact(&c, config(), timing()).await.unwrap_err(),
        Error::NoContact
    );
    assert_eq!(c.commands.lock().unwrap().len(), 3);
}

#[tokio::test(start_paused = true)]
async fn event_loss_and_silent_controller_fail_closed() {
    let c = Scripted::new(vec![vec![Event::default(); 100]]);
    assert_eq!(
        run_contact(&c, config(), timing()).await.unwrap_err(),
        Error::EventLagged
    );
    let c = Scripted::new(vec![vec![]]);
    assert_eq!(
        run_contact(&c, config(), timing()).await.unwrap_err(),
        Error::Timeout
    );
}

#[tokio::test]
async fn alarm_never_restores_modes_or_writes_zero_and_logs_attempt() {
    let mut alarm = status(-120.0, false);
    alarm.status.as_mut().unwrap().motion_blocked = true;
    let c = Scripted::new(vec![vec![alarm]]);
    c.state.lock().unwrap().modes.distance = 90;
    let plan = review(
        c.state(),
        RoutineConfig {
            zero: true,
            ..RoutineConfig::default()
        },
    )
    .unwrap();
    let script = Mutex::new(Vec::new());
    let error = run(&c, &plan, timing(), CancellationToken::new(), |p| {
        if p.kind == "script" {
            script.lock().unwrap().push(p.message);
        }
    })
    .await
    .unwrap_err();
    assert_eq!(error, Error::MotionBlocked);
    let commands = c.commands.lock().unwrap();
    assert!(!commands
        .iter()
        .any(|v| v.starts_with("G10") || v == "G21 G94 G90"));
    assert!(script
        .lock()
        .unwrap()
        .iter()
        .any(|v| v.starts_with("G38.3 Z-5")));
}

#[tokio::test(start_paused = true)]
async fn cancellation_stops_submission_and_never_restores_unsettled_motion() {
    let c = Arc::new(Scripted::new(vec![vec![status(-120.0, false)]]));
    c.state.lock().unwrap().modes.distance = 90;
    let plan = review(c.state(), RoutineConfig::default()).unwrap();
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    let worker = c.clone();
    let task =
        tokio::spawn(async move { run(worker.as_ref(), &plan, timing(), cancel, |_| {}).await });
    tokio::task::yield_now().await;
    trigger.cancel();
    assert_eq!(task.await.unwrap().unwrap_err(), Error::Cancelled);
    let commands = c.commands.lock().unwrap();
    assert!(!commands.iter().any(|v| v == "G21 G94 G90"));
    assert_eq!(commands.iter().filter(|v| v.starts_with("G38")).count(), 1);
}

#[tokio::test(start_paused = true)]
async fn already_cancelled_run_sends_nothing() {
    let c = Scripted::new(vec![]);
    let plan = review(c.state(), RoutineConfig::default()).unwrap();
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        run(&c, &plan, timing(), cancel, |_| {}).await.unwrap_err(),
        Error::Cancelled
    );
    assert!(c.commands.lock().unwrap().is_empty());
}

#[tokio::test(start_paused = true)]
async fn paced_mock_cancellation_has_no_pending_movement() {
    let c = MockController::new().with_delay(Duration::from_millis(100));
    c.set_extended(true).unwrap();
    let cfg = RoutineConfig::default();
    c.configure(&cfg).unwrap();
    let p = review(c.state(), cfg).unwrap();
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(250)).await;
        trigger.cancel();
    });
    assert_eq!(
        run(&c, &p, timing(), cancel, |_| {}).await.unwrap_err(),
        Error::Cancelled
    );
    assert_eq!(c.state().position, p.start.position);
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(c.state().position, p.start.position);
}

#[test]
fn deadlines_allow_for_the_firmware_feed_cap() {
    let mut stage = plan_contact(config()).unwrap().stages.remove(0);
    stage.distance = 250.0;
    stage.feed = 3000.0;
    assert_eq!(
        TimingPolicy::default().deadline(&stage).unwrap(),
        Duration::from_secs(20)
    );
    stage.feed = 300.0;
    assert_eq!(
        TimingPolicy::default().deadline(&stage).unwrap(),
        Duration::from_secs(55)
    );
}

#[test]
fn feed_and_deadline_validation_has_no_rapid_fallback() {
    let mut c = config();
    c.retract_feed = 0.0;
    assert!(plan_contact(c).is_err());
    let mut stage = plan_contact(config()).unwrap().stages.remove(0);
    stage.feed = 0.0;
    assert!(timing().deadline(&stage).is_err());
    stage.feed = f64::MIN_POSITIVE;
    stage.distance = f64::MAX;
    assert!(timing().deadline(&stage).is_err());
}

#[tokio::test(start_paused = true)]
async fn modes_require_fresh_parser_report_not_acknowledgment() {
    let mut c = Scripted::new(vec![]);
    c.modes_ack_only = true;
    assert_eq!(query_modes(&c).await.unwrap_err(), Error::Timeout);
    let p = review(c.state(), RoutineConfig::default()).unwrap();
    assert_eq!(
        run(&c, &p, timing(), CancellationToken::new(), |_| {})
            .await
            .unwrap_err(),
        Error::Timeout
    );
    assert!(!c
        .commands
        .lock()
        .unwrap()
        .iter()
        .any(|v| v.starts_with("G38")));
}

fn measured_x(c: &Scripted) -> (RoutinePlan, RoutineResult) {
    let plan = review(
        c.state(),
        RoutineConfig {
            family: "inside".into(),
            x: 1,
            z: false,
            safe_z_offset: 20.0,
            ..RoutineConfig::default()
        },
    )
    .unwrap();
    let result = RoutineResult {
        point: [Some(35.0), None, None],
        wcs: 54,
        axes: vec!["X".into()],
        settled: true,
        ..RoutineResult::default()
    };
    (plan, result)
}

#[tokio::test(start_paused = true)]
async fn zero_requires_confirmed_coordinates_not_ack_or_unchanged_status() {
    for replies in [
        vec![Event {
            acknowledged: true,
            ..Event::default()
        }],
        vec![status(-120.0, true)],
    ] {
        let c = Scripted::new(vec![replies]);
        let (plan, result) = measured_x(&c);
        assert_eq!(
            zero_result(
                &c,
                &plan,
                &result,
                [0.0; 3],
                timing(),
                CancellationToken::new()
            )
            .await
            .unwrap_err(),
            Error::Timeout
        );
        assert!(!result.zeroed);
    }
}

#[tokio::test]
async fn zero_rejects_machine_motion_or_changed_wcs() {
    let mut changed = status(-120.0, true);
    changed.status.as_mut().unwrap().wcs = 55;
    for reply in [status(-119.0, true), changed] {
        let c = Scripted::new(vec![vec![reply]]);
        let (p, r) = measured_x(&c);
        assert!(matches!(
            zero_result(&c, &p, &r, [0.0; 3], timing(), CancellationToken::new()).await,
            Err(Error::Position(_))
        ));
    }
}

#[tokio::test]
async fn late_zero_rejects_changed_offsets_before_sending() {
    let c = Scripted::new(vec![]);
    let (p, r) = measured_x(&c);
    c.state.lock().unwrap().work_position[1] += 1.0;
    assert!(matches!(
        zero_result(&c, &p, &r, [0.0; 3], timing(), CancellationToken::new()).await,
        Err(Error::Preflight(_))
    ));
    assert!(c.commands.lock().unwrap().is_empty());
}

#[tokio::test]
async fn script_captures_stream_between_fine_and_backoff_submission() {
    let c = Scripted::new(good());
    let cfg = RoutineConfig {
        family: "inside".into(),
        x: 1,
        z: false,
        depth: 1.0,
        safe_z_offset: 20.0,
        ..RoutineConfig::default()
    };
    let p = review(c.state(), cfg).unwrap();
    let log = Mutex::new(Vec::new());
    // The final Z positioning deliberately receives no response; only the contact
    // section's ordering is under test, and cancellation stops before that wait.
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    let _ = run(&c, &p, timing(), cancel, |event| {
        if event.kind == "script" {
            log.lock().unwrap().push(event.message);
        }
        if event.command.starts_with("G1 X-0.500")
            && c.commands
                .lock()
                .unwrap()
                .iter()
                .filter(|v| v.starts_with("G1 X-0.500"))
                .count()
                == 2
        {
            trigger.cancel();
        }
    })
    .await;
    let log = log.lock().unwrap();
    let capture = log
        .iter()
        .position(|v| v.starts_with("; #<contact_x> :="))
        .unwrap();
    let fine = log.iter().position(|v| v.starts_with("G38.2")).unwrap();
    let final_release = log.iter().rposition(|v| v.starts_with("G1 X-0.5")).unwrap();
    assert!(fine < capture && capture < final_release);
}
