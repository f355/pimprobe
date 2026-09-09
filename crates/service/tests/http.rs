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

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use pimprobe_core::{Controller, MockController};
use pimprobe_service::{
    device::Device,
    http::{App, router},
    settings::Settings,
};
use serde_json::{Value, json};
use tower::ServiceExt;

fn config() -> Value {
    json!({"family":"outside","x":0,"y":0,"z":true,"wcs":54,"zero":false,
        "safeZOffset":40,"depth":5,"xSearchDistance":10,"ySearchDistance":10,
        "retract":0.5,"diameter":4,"positioningFeed":1000,"coarseFeed":30,"fineFeed":10})
}

fn app() -> (tempfile::TempDir, std::sync::Arc<App>, Router) {
    let temp = tempfile::tempdir().unwrap();
    let mock = MockController::new();
    mock.set_extended(true).unwrap();
    let app = App::new(
        Device::Mock(Box::new(mock)),
        Settings::open(temp.path().join("settings.json")).unwrap(),
        Some("test-token".into()),
    );
    (temp, app.clone(), router(app))
}

#[tokio::test]
async fn repeatability_streams_each_reading_and_axis_statistics() {
    let (_temp, app, router) = repeatability_app();
    let before = app.device.state();
    let (code, body) = request(
        &router,
        "POST",
        "/api/v1/repeatability/run",
        json!({"axes":[true,true,true],"repetitions":5,"home":false,"retract":true}),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{body}");
    let events: Vec<Value> = body
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(
        events.iter().filter(|e| e["type"] == "measurement").count(),
        15
    );
    let last = events.last().unwrap();
    let mut counts = [0_u64; 3];
    for event in events.iter().filter(|e| e["type"] == "measurement") {
        let axis = ["X", "Y", "Z"]
            .iter()
            .position(|a| event["axis"] == *a)
            .unwrap();
        counts[axis] += 1;
        for (i, count) in counts.iter().enumerate() {
            if *count == 0 {
                assert!(event["statistics"][i].is_null());
            } else {
                assert_eq!(event["statistics"][i]["count"], *count);
                assert!(event["statistics"][i]["mean"].as_f64().unwrap().abs() < 0.001);
            }
        }
    }
    assert_eq!(last["type"], "result", "{body}");
    assert_eq!(last["result"]["measurements"].as_array().unwrap().len(), 5);
    for axis in 0..3 {
        assert_eq!(last["result"]["statistics"][axis]["count"], 5);
        assert!(
            last["result"]["statistics"][axis]["stddev"]
                .as_f64()
                .unwrap()
                < 1e-9
        );
    }
    let after = app.device.state();
    assert!(after.probe_extended);
    assert_eq!(after.modes, before.modes);
    for axis in 0..3 {
        assert!(
            ((after.position[axis] - after.work_position[axis])
                - (before.position[axis] - before.work_position[axis]))
                .abs()
                < 1e-8
        );
    }
}

#[tokio::test]
async fn repeatability_rejects_empty_axes_and_fractional_repetitions() {
    let (_temp, _app, router) = app();
    for options in [
        json!({"axes":[false,false,false],"repetitions":5,"home":false,"retract":true}),
        json!({"axes":[true,true,true],"repetitions":1.5,"home":false,"retract":true}),
    ] {
        let (code, _) = request(&router, "POST", "/api/v1/repeatability/run", options).await;
        assert!(matches!(
            code,
            StatusCode::BAD_REQUEST | StatusCode::CONFLICT | StatusCode::UNPROCESSABLE_ENTITY
        ));
    }
}

#[tokio::test]
async fn repeatability_cycles_probe_between_readings_and_before_homing() {
    for home in [false, true] {
        for retract in [false, true] {
            let (_temp, app, router) = repeatability_app();
            let (code, body) = request(
                &router,
                "POST",
                "/api/v1/repeatability/run",
                json!({"axes":[true,false,false],"repetitions":2,"home":home,"retract":retract}),
            )
            .await;
            assert_eq!(code, StatusCode::OK);
            let last: Value = serde_json::from_str(body.lines().last().unwrap()).unwrap();
            assert_eq!(last["type"], "result", "{body}");
            assert!(last["result"]["statistics"][1].is_null());
            let Device::Mock(mock) = &app.device else {
                unreachable!()
            };
            let commands = mock.commands();
            assert_eq!(
                commands.iter().filter(|s| *s == "$H").count(),
                if home { 2 } else { 0 }
            );
            assert_eq!(
                commands.iter().filter(|s| *s == "M121").count(),
                if home {
                    2
                } else if retract {
                    1
                } else {
                    0
                }
            );
            let first_motion = commands
                .iter()
                .position(|s| s.starts_with("G38") || s.starts_with("G1 "))
                .unwrap();
            if home || retract {
                assert!(commands.iter().position(|s| s == "M121").unwrap() > first_motion);
            }
            let last_motion = commands
                .iter()
                .rfind(|s| s.starts_with("G38") || s.starts_with("G1 "))
                .unwrap();
            assert!(last_motion.starts_with("G38.3 X"), "{last_motion}");
        }
    }
}

fn repeatability_app() -> (tempfile::TempDir, std::sync::Arc<App>, Router) {
    let temp = tempfile::tempdir().unwrap();
    let mut state = MockController::new().state();
    state.probe_extended = true;
    // The tip starts inside the bracket and above the bed.
    state.work_position = [70.0, 20.0, 15.0, 0.0];
    let app = App::new(
        Device::Mock(Box::new(MockController::with_state(state))),
        Settings::open(temp.path().join("settings.json")).unwrap(),
        None,
    );
    (temp, app.clone(), router(app))
}

#[tokio::test]
async fn repeatability_refreshes_the_firmware_z_reference() {
    use pimprobe_controller::{SocketController, frame};
    use pimprobe_core::RepeatabilityController;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("probe.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let controller = SocketController::connect(&path, [240.0, 235.0, 125.0, 0.0])
        .await
        .unwrap();
    let (mut socket, _) = listener.accept().await.unwrap();
    assert_eq!(
        frame::read_frame(&mut socket).await.unwrap(),
        (b'Q', b"$P\n".to_vec())
    );
    frame::write_frame(&mut socket, b'D', b"$202=-12\n")
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while controller.snapshot().settings.get(&202) != Some(&-12.0) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let device = std::sync::Arc::new(Device::Machine(controller.clone()));
    let task = tokio::spawn({
        let device = device.clone();
        async move { device.refresh_probe_reference().await }
    });
    assert_eq!(
        frame::read_frame(&mut socket).await.unwrap(),
        (b'Q', b"$$\n".to_vec())
    );
    frame::write_frame(&mut socket, b'D', b"ok\n$201=-40\n")
        .await
        .unwrap();
    tokio::task::yield_now().await;
    assert!(!task.is_finished());
    frame::write_frame(&mut socket, b'D', b"$202=-42.5\n")
        .await
        .unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(device.probe_reference_z().unwrap(), -42.5);
    controller.shutdown().await;
}

async fn request(router: &Router, method: &str, path: &str, body: Value) -> (StatusCode, String) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn settings_schema_matches_accepted_values() {
    let (_temp, _app, router) = app();
    let (code, body) = request(&router, "GET", "/api/v1/settings/schema", Value::Null).await;
    assert_eq!(code, StatusCode::OK);
    let schema: Value = serde_json::from_str(&body).unwrap();
    for (key, rule) in schema.as_object().unwrap() {
        for value in [
            rule.get("minimum"),
            rule.get("maximum"),
            rule.get("default"),
        ]
        .into_iter()
        .flatten()
        {
            let (code, body) =
                request(&router, "PATCH", "/api/v1/settings", json!({key: value})).await;
            assert_eq!(code, StatusCode::OK, "{key}: {body}");
        }
    }
    assert_eq!(schema["insideDepth"]["minimum"], 0.1);
    assert_eq!(schema["centerDepth"]["minimum"], 0.1);
    assert_eq!(schema["centerXSearchDistance"]["maximum"], 1000.0);
}

#[tokio::test]
async fn settings_contract_and_validation() {
    let (_temp, _app, router) = app();
    let (code, body) = request(&router, "GET", "/api/v1/settings", Value::Null).await;
    assert_eq!(code, StatusCode::OK);
    let values: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(values["retractDistance"], 0.5);
    let (code, _) = request(
        &router,
        "PATCH",
        "/api/v1/settings",
        json!({"coarseFeed":123}),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    let (code, _) = request(
        &router,
        "PATCH",
        "/api/v1/settings",
        json!({"coarseFeed":0}),
    )
    .await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn state_actuator_and_wcs_contract() {
    let (_temp, app, router) = app();
    let (code, _) = request(
        &router,
        "POST",
        "/api/v1/probe-actuator",
        json!({"extended":false}),
    )
    .await;
    assert_eq!(code, StatusCode::NO_CONTENT);
    assert!(!app.device.state().probe_extended);
    let (code, _) = request(&router, "POST", "/api/v1/probe-actuator", json!({})).await;
    assert!(code.is_client_error());
    let (code, _) = request(&router, "POST", "/api/v1/wcs", json!({"wcs":59})).await;
    assert_eq!(code, StatusCode::NO_CONTENT);
    let (_, body) = request(&router, "GET", "/api/v1/state", Value::Null).await;
    let snapshot: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(snapshot["status"]["wcs"], 59);
    assert_eq!(snapshot["status"]["probeActuator"], 0);
    assert_eq!(snapshot["contactActive"], false);
    assert_eq!(snapshot["connected"], true);
}

#[tokio::test]
async fn review_run_stream_and_late_zero() {
    let (_temp, app, router) = app();
    let (code, body) = request(&router, "POST", "/api/v1/routine/review", config()).await;
    assert_eq!(code, StatusCode::OK, "{body}");
    let review: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(review["simulated"], true);
    assert!(review["program"].as_array().unwrap().len() > 5);
    let token = json!({"id":review["id"]});
    let (code, body) = request(&router, "POST", "/api/v1/routine/run", token.clone()).await;
    assert_eq!(code, StatusCode::OK, "{body}");
    let events: Vec<Value> = body
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(events.iter().any(|e| e["progress"]["kind"] == "script"));
    let terminal = events.last().unwrap();
    assert_eq!(terminal["type"], "result", "{body}");
    assert_eq!(terminal["result"]["zeroed"], false);
    let before = app.device.state().position;
    let (code, body) = request(&router, "POST", "/api/v1/routine/zero", token.clone()).await;
    assert_eq!(code, StatusCode::OK, "{body}");
    let result: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(result["zeroed"], true);
    assert_eq!(result["point"], terminal["result"]["point"]);
    assert_eq!(before, app.device.state().position);
    let (code, _) = request(&router, "POST", "/api/v1/routine/zero", token.clone()).await;
    assert_eq!(code, StatusCode::CONFLICT);
    let (code, _) = request(&router, "POST", "/api/v1/routine/run", token).await;
    assert_eq!(code, StatusCode::CONFLICT);
}

#[tokio::test]
async fn result_offsets_set_machine_origin_without_moving_or_changing_measurements() {
    for (z, offsets) in [(true, [0.0, 0.0, -2.0]), (false, [1.25, -3.5, 0.0])] {
        let (_temp, app, router) = app();
        let mut cfg = config();
        cfg["z"] = json!(z);
        cfg["x"] = json!(if z { 0 } else { 1 });
        cfg["y"] = json!(if z { 0 } else { 1 });
        let start = app.device.state();
        let (_, body) = request(&router, "POST", "/api/v1/routine/review", cfg).await;
        let review: Value = serde_json::from_str(&body).unwrap();
        let (_, body) = request(
            &router,
            "POST",
            "/api/v1/routine/run",
            json!({"id":review["id"]}),
        )
        .await;
        let terminal: Value = serde_json::from_str(body.lines().last().unwrap()).unwrap();
        assert_eq!(terminal["type"], "result", "{body}");
        let before = app.device.state();
        let (code, body) = request(
            &router,
            "POST",
            "/api/v1/routine/zero",
            json!({"id":review["id"], "offsets":offsets}),
        )
        .await;
        assert_eq!(code, StatusCode::OK, "{body}");
        let updated: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(updated["point"], terminal["result"]["point"]);
        let after = app.device.state();
        assert_eq!(after.position, before.position);
        for (i, offset) in offsets.iter().enumerate() {
            if let Some(point) = updated["point"][i].as_f64() {
                let origin = point + start.position[i] - start.work_position[i] + offset;
                assert!((after.position[i] - after.work_position[i] - origin).abs() < 0.002);
            } else {
                assert_eq!(after.work_position[i], before.work_position[i]);
            }
        }
    }
}

#[tokio::test]
async fn new_review_invalidates_old_token() {
    let (_temp, _app, router) = app();
    let (_, body) = request(&router, "POST", "/api/v1/routine/review", config()).await;
    let first: Value = serde_json::from_str(&body).unwrap();
    let (code, _) = request(&router, "POST", "/api/v1/routine/review", config()).await;
    assert_eq!(code, StatusCode::OK);
    let (code, _) = request(
        &router,
        "POST",
        "/api/v1/routine/run",
        json!({"id":first["id"]}),
    )
    .await;
    assert_eq!(code, StatusCode::CONFLICT);
}

#[tokio::test]
async fn zero_and_return_remain_available_in_either_order() {
    for zero_first in [false, true] {
        let (_temp, app, router) = app();
        let start = app.device.state().position;
        let mut cfg = config();
        cfg["z"] = json!(false);
        cfg["x"] = json!(1);
        cfg["y"] = json!(-1);
        let (code, body) = request(&router, "POST", "/api/v1/routine/review", cfg).await;
        assert_eq!(code, StatusCode::OK, "{body}");
        let review: Value = serde_json::from_str(&body).unwrap();
        let token = json!({"id":review["id"]});
        let (_, body) = request(&router, "POST", "/api/v1/routine/run", token.clone()).await;
        let measured: Value = serde_json::from_str(body.lines().last().unwrap()).unwrap();
        assert_eq!(measured["type"], "result", "{body}");
        let actions = if zero_first {
            ["zero", "return"]
        } else {
            ["return", "zero"]
        };
        for action in actions {
            let (code, body) = request(
                &router,
                "POST",
                &format!("/api/v1/routine/{action}"),
                token.clone(),
            )
            .await;
            assert_eq!(code, StatusCode::OK, "{body}");
            let reply: Value = serde_json::from_str(body.lines().last().unwrap()).unwrap();
            let result = if action == "return" {
                &reply["result"]
            } else {
                &reply
            };
            assert_eq!(result["point"], measured["result"]["point"]);
            assert_eq!(
                result[if action == "return" {
                    "returned"
                } else {
                    "zeroed"
                }],
                true
            );
        }
        assert!(
            app.device
                .state()
                .position
                .iter()
                .zip(start)
                .all(|(a, b)| (a - b).abs() < 0.002)
        );
        let (code, _) = request(&router, "POST", "/api/v1/routine/return", token).await;
        assert_eq!(code, StatusCode::CONFLICT);
    }
}

#[tokio::test]
async fn inside_result_can_move_over_the_measurement_after_zeroing() {
    let (_temp, app, router) = app();
    let mut cfg = config();
    cfg["family"] = json!("inside");
    cfg["z"] = json!(false);
    cfg["x"] = json!(1);
    cfg["y"] = json!(-1);
    let start = app.device.state().position;
    let (code, body) = request(&router, "POST", "/api/v1/routine/review", cfg).await;
    assert_eq!(code, StatusCode::OK, "{body}");
    let review: Value = serde_json::from_str(&body).unwrap();
    let token = json!({"id":review["id"]});
    let (_, body) = request(&router, "POST", "/api/v1/routine/run", token.clone()).await;
    let measured: Value = serde_json::from_str(body.lines().last().unwrap()).unwrap();
    assert_eq!(measured["type"], "result", "{body}");
    let at_start = app.device.state().position;
    assert!((at_start[0] - start[0]).abs() < 0.002);
    assert!((at_start[1] - start[1]).abs() < 0.002);
    assert!((at_start[2] - start[2]).abs() < 0.002);

    let (code, body) = request(
        &router,
        "POST",
        "/api/v1/routine/zero",
        json!({"id":review["id"], "offsets":[0,0,0]}),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{body}");
    let (code, body) = request(
        &router,
        "POST",
        "/api/v1/routine/measured",
        json!({"id":review["id"], "safeZOffset":25}),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{body}");
    let reply: Value = serde_json::from_str(body.lines().last().unwrap()).unwrap();
    assert_eq!(reply["result"]["positioned"], true);
    let end = app.device.state().position;
    assert!((end[2] - start[2] - 25.0).abs() < 0.002);
}

#[tokio::test]
async fn cancelled_result_move_restores_parser_modes_after_stopping() {
    let temp = tempfile::tempdir().unwrap();
    let mock = MockController::new().with_delay(std::time::Duration::from_millis(20));
    mock.set_extended(true).unwrap();
    let app = App::new(
        Device::Mock(Box::new(mock)),
        Settings::open(temp.path().join("settings.json")).unwrap(),
        Some("test-token".into()),
    );
    let router = router(app.clone());
    let original_modes = app.device.state().modes;
    let mut cfg = config();
    cfg["family"] = json!("inside");
    cfg["z"] = json!(false);
    cfg["x"] = json!(1);
    cfg["y"] = json!(-1);
    let (_, body) = request(&router, "POST", "/api/v1/routine/review", cfg).await;
    let review: Value = serde_json::from_str(&body).unwrap();
    let token = json!({"id":review["id"]});
    let (_, body) = request(&router, "POST", "/api/v1/routine/run", token.clone()).await;
    assert!(body.lines().last().unwrap().contains("\"type\":\"result\""));

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/routine/measured")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"id":review["id"], "safeZOffset":25}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    for _ in 0..100 {
        if app.device.state().modes != original_modes {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert_ne!(app.device.state().modes, original_modes);
    drop(response);
    for _ in 0..100 {
        let (_, body) = request(&router, "GET", "/api/v1/state", Value::Null).await;
        let snapshot: Value = serde_json::from_str(&body).unwrap();
        if snapshot["contactActive"] == false {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert_eq!(app.device.state().modes, original_modes);
}

#[tokio::test]
async fn mock_ready_is_explicit() {
    let (_temp, _app, router) = app();
    let (code, body) = request(&router, "GET", "/api/v1/mock/ready", Value::Null).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(body, "test-token");
}

#[tokio::test]
async fn dropping_execution_stream_cancels_before_motion_and_consumes_review() {
    let (_temp, app, router) = app();
    let (_, body) = request(&router, "POST", "/api/v1/routine/review", config()).await;
    let review: Value = serde_json::from_str(&body).unwrap();
    let token = json!({"id":review["id"]});
    let before = app.device.state().position;
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/routine/run")
                .header("content-type", "application/json")
                .body(Body::from(token.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    drop(response);
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    assert_eq!(app.device.state().position, before);
    let (code, _) = request(&router, "POST", "/api/v1/routine/run", token).await;
    assert_eq!(code, StatusCode::CONFLICT);
    let (_, body) = request(&router, "GET", "/api/v1/state", Value::Null).await;
    let state: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(state["contactActive"], false);
    assert_eq!(state["recoveryFailed"], false);
}

#[tokio::test]
async fn failed_late_zero_attempt_cannot_be_replayed() {
    let (_temp, app, router) = app();
    let (_, body) = request(&router, "POST", "/api/v1/routine/review", config()).await;
    let review: Value = serde_json::from_str(&body).unwrap();
    let token = json!({"id":review["id"]});
    let (_, body) = request(&router, "POST", "/api/v1/routine/run", token.clone()).await;
    assert!(body.lines().last().unwrap().contains("\"type\":\"result\""));
    app.device.select_wcs(55).await.unwrap();
    let (code, _) = request(&router, "POST", "/api/v1/routine/zero", token.clone()).await;
    assert_eq!(code, StatusCode::CONFLICT);
    app.device.select_wcs(54).await.unwrap();
    let (code, body) = request(&router, "POST", "/api/v1/routine/zero", token).await;
    assert_eq!(code, StatusCode::CONFLICT);
    assert!(body.contains("No unzeroed result"));
}
