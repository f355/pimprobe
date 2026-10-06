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

use crate::{
    device::Device,
    host::LocalHost,
    logs::LogStore,
    updates::{UpdateError, UpdateManager},
};
use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, FromRequest, Query, Request, State, rejection::JsonRejection},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use pimprobe_app::{
    AppError, ErrorKind, Motion, Operation, ProbeApp, Token, ZeroRequest, history::Records,
    settings::Settings,
};
use pimprobe_core::{Controller, RepeatabilityOptions, RoutineConfig};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::{
    convert::Infallible,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio_stream::Stream;

mod rotary;

pub struct App {
    pub device: Arc<Device>,
    pub probe: Arc<ProbeApp>,
    ready_token: Option<String>,
    updates: UpdateManager,
}
impl App {
    pub fn new(
        device: Device,
        settings: Settings,
        ready_token: Option<String>,
        logs: LogStore,
    ) -> Arc<Self> {
        let device = Arc::new(device);
        let logs = Arc::new(logs);
        let probe = ProbeApp::new(
            device.clone(),
            settings,
            Arc::new(LocalHost::new(logs.clone())),
            Records::new(logs.clone(), logs),
        );
        Arc::new(Self {
            device,
            probe,
            ready_token,
            updates: UpdateManager::production(),
        })
    }
    pub async fn shutdown(&self) {
        self.probe.shutdown().await;
    }
}

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/api/v1/state", get(state))
        .route("/api/v1/settings", get(settings).patch(update_settings))
        .route("/api/v1/settings/schema", get(settings_schema))
        .route("/api/v1/probe-actuator", post(actuator))
        .route("/api/v1/wcs", post(wcs))
        .route("/api/v1/routine/review", post(review))
        .route("/api/v1/routine/run", post(run))
        .route("/api/v1/routine/zero", post(zero))
        .route("/api/v1/routine/return", post(return_start))
        .route("/api/v1/routine/measured", post(go_to_measured))
        .route("/api/v1/rotary/review", post(rotary::review))
        .route("/api/v1/rotary/run", post(rotary::run))
        .route("/api/v1/rotary/zero", post(rotary::zero))
        .route("/api/v1/rotary/rotation", post(rotary::rotation))
        .route("/api/v1/repeatability/run", post(repeatability))
        .route("/api/v1/repeatability/stop", post(stop_repeatability))
        .route("/api/v1/logs/history", get(history))
        .route("/api/v1/logs/export", post(export_logs))
        .route("/api/v1/logs/clear", post(clear_logs))
        .route("/api/v1/updates/check", get(check_update))
        .route("/api/v1/updates/install", post(install_update))
        .route("/api/v1/updates/status", get(update_status))
        .route("/api/v1/mock/ready", get(ready))
        .layer(DefaultBodyLimit::max(8192))
        .with_state(app)
}

#[derive(Debug)]
struct ApiError(StatusCode, &'static str, String);
impl ApiError {
    fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self(StatusCode::CONFLICT, code, message.into())
    }
}
impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        let status = match error.kind {
            ErrorKind::Invalid => StatusCode::BAD_REQUEST,
            ErrorKind::Conflict => StatusCode::CONFLICT,
            ErrorKind::Storage => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self(status, error.code, error.message)
    }
}
impl From<UpdateError> for ApiError {
    fn from(error: UpdateError) -> Self {
        if matches!(error, UpdateError::MachineBusy) {
            Self::conflict("machine_busy", error.to_string())
        } else {
            Self(StatusCode::BAD_GATEWAY, "update", error.to_string())
        }
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Qt 6.9's QML XMLHttpRequest discards the status and body of non-2xx
        // loopback responses. Keep application failures visible to the operator.
        Json(json!({
            "error": true,
            "code": self.1,
            "message": self.2,
            "status": self.0.as_u16()
        }))
        .into_response()
    }
}

struct ApiJson<T>(T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    S: Send + Sync,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|error| ApiError(StatusCode::BAD_REQUEST, "request", error.body_text()))
    }
}

async fn state(State(app): State<Arc<App>>) -> Json<pimprobe_app::AppState> {
    Json(app.probe.state())
}
async fn settings(State(app): State<Arc<App>>) -> Json<Map<String, Value>> {
    Json(app.probe.settings().await)
}
async fn settings_schema() -> Json<Map<String, Value>> {
    Json(pimprobe_app::settings::schema())
}
async fn update_settings(
    State(app): State<Arc<App>>,
    ApiJson(patch): ApiJson<Map<String, Value>>,
) -> Result<Json<Map<String, Value>>, ApiError> {
    Ok(Json(app.probe.update_settings(patch).await?))
}
async fn history(
    State(app): State<Arc<App>>,
) -> Result<Json<Vec<pimprobe_app::history::HistoryEntry>>, ApiError> {
    Ok(Json(app.probe.history().await?))
}
async fn export_logs(
    State(app): State<Arc<App>>,
) -> Result<Json<pimprobe_app::host::ExportResult>, ApiError> {
    Ok(Json(app.probe.export_logs().await?))
}
async fn clear_logs(State(app): State<Arc<App>>) -> Result<Json<Value>, ApiError> {
    app.probe.clear_logs().await?;
    Ok(Json(json!({"cleared":true})))
}
async fn ready(State(app): State<Arc<App>>) -> Response {
    match &app.ready_token {
        Some(token) => token.clone().into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Deserialize)]
struct UpdateQuery {
    #[serde(default)]
    development: bool,
}

async fn check_update(
    State(app): State<Arc<App>>,
    Query(query): Query<UpdateQuery>,
) -> Result<Json<crate::updates::CheckResult>, ApiError> {
    Ok(Json(app.updates.check(query.development).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InstallUpdateRequest {
    token: String,
}

async fn install_update(
    State(app): State<Arc<App>>,
    ApiJson(body): ApiJson<InstallUpdateRequest>,
) -> Result<Json<crate::updates::InstallStarted>, ApiError> {
    let _guard = app.probe.acquire()?;
    let state = app.device.state();
    if !state.connected || !state.ready || !state.spindle_stopped {
        return Err(ApiError::conflict(
            "machine_busy",
            "The machine must be idle with the spindle stopped before updating",
        ));
    }
    Ok(Json(
        app.updates
            .install(&body.token, || {
                let state = app.device.state();
                state.connected && state.ready && state.spindle_stopped
            })
            .await?,
    ))
}

#[derive(Deserialize)]
struct UpdateStatusQuery {
    id: String,
}

async fn update_status(
    State(app): State<Arc<App>>,
    Query(query): Query<UpdateStatusQuery>,
) -> Result<Json<crate::updates::InstallStatus>, ApiError> {
    Ok(Json(app.updates.status(&query.id).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActuatorRequest {
    extended: bool,
}
async fn actuator(
    State(app): State<Arc<App>>,
    ApiJson(body): ApiJson<ActuatorRequest>,
) -> Result<StatusCode, ApiError> {
    app.probe.set_extended(body.extended).await?;
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WcsRequest {
    wcs: i32,
}
async fn wcs(
    State(app): State<Arc<App>>,
    ApiJson(body): ApiJson<WcsRequest>,
) -> Result<StatusCode, ApiError> {
    app.probe.select_wcs(body.wcs).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn review(
    State(app): State<Arc<App>>,
    ApiJson(config): ApiJson<RoutineConfig>,
) -> Result<Json<pimprobe_app::Review>, ApiError> {
    Ok(Json(app.probe.review(config).await?))
}
async fn run(
    State(app): State<Arc<App>>,
    ApiJson(token): ApiJson<Token>,
) -> Result<Response, ApiError> {
    Ok(stream(app.probe.start_motion(token, Motion::Run).await?))
}
async fn return_start(
    State(app): State<Arc<App>>,
    ApiJson(token): ApiJson<Token>,
) -> Result<Response, ApiError> {
    Ok(stream(app.probe.start_motion(token, Motion::Return).await?))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MeasuredRequest {
    id: String,
    safe_z_offset: f64,
}
async fn go_to_measured(
    State(app): State<Arc<App>>,
    ApiJson(body): ApiJson<MeasuredRequest>,
) -> Result<Response, ApiError> {
    Ok(stream(
        app.probe
            .start_motion(Token { id: body.id }, Motion::Measured(body.safe_z_offset))
            .await?,
    ))
}
async fn zero(
    State(app): State<Arc<App>>,
    ApiJson(token): ApiJson<ZeroRequest>,
) -> Result<Json<pimprobe_core::RoutineResult>, ApiError> {
    Ok(Json(app.probe.zero(token).await?))
}
async fn repeatability(
    State(app): State<Arc<App>>,
    ApiJson(options): ApiJson<RepeatabilityOptions>,
) -> Result<Response, ApiError> {
    Ok(stream(app.probe.repeatability(options).await?))
}
async fn stop_repeatability(State(app): State<Arc<App>>) -> StatusCode {
    app.probe.stop_repeatability().await;
    StatusCode::NO_CONTENT
}

struct RunStream(Operation);
impl Stream for RunStream {
    type Item = Result<Bytes, Infallible>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_recv(cx).map(|event| {
            event.map(|value| {
                Ok(Bytes::from(
                    serde_json::to_string(&value).expect("operation event") + "\n",
                ))
            })
        })
    }
}
fn stream(operation: Operation) -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/x-ndjson"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        Body::from_stream(RunStream(operation)),
    )
        .into_response()
}
