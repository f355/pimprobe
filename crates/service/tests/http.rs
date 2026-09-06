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
