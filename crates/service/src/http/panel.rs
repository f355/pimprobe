// PIMProbe - touch probing for the Nestworks C500.
// Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use super::{ApiError, ApiJson, App};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::{io, path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    time::timeout,
};

pub(super) struct Panel {
    brightness: PathBuf,
    relay: PathBuf,
}

impl Default for Panel {
    fn default() -> Self {
        Self {
            brightness: "/sys/class/backlight/backlight/brightness".into(),
            relay: "/run/pimprobe-panel.sock".into(),
        }
    }
}

impl Panel {
    pub(super) fn sleeping(&self) -> Option<bool> {
        if !self.relay.exists() {
            return None;
        }
        let brightness = std::fs::read_to_string(&self.brightness).ok()?;
        Some(brightness.trim().parse::<u32>().ok()? == 0)
    }

    async fn notify(&self, wake: bool) -> io::Result<()> {
        timeout(Duration::from_millis(500), async {
            let mut socket = UnixStream::connect(&self.relay).await?;
            socket.write_all(if wake { b"W" } else { b"A" }).await?;
            if socket.read_u8().await? != b'1' {
                return Err(io::Error::other("Panel activity was not accepted"));
            }
            Ok(())
        })
        .await?
    }
}

pub(super) fn local(headers: &HeaderMap) -> bool {
    headers
        .get("x-pimprobe-panel")
        .is_some_and(|value| value == "1")
}

pub(super) async fn guard(State(app): State<Arc<App>>, request: Request, next: Next) -> Response {
    // The screen can turn off between browser state updates. Reject any late
    // gesture without waking the screen or dispatching the request.
    if !request.method().is_safe() && local(request.headers()) && app.panel.sleeping() == Some(true)
    {
        return ApiError::conflict("panel_asleep", "Touch the screen to wake it.").into_response();
    }
    next.run(request).await
}

#[derive(Deserialize)]
pub(super) struct Activity {
    wake: bool,
}

pub(super) async fn activity(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    ApiJson(request): ApiJson<Activity>,
) -> Result<StatusCode, ApiError> {
    if local(&headers) && app.panel.sleeping().is_some() {
        app.panel.notify(request.wake).await.map_err(|error| {
            ApiError(StatusCode::SERVICE_UNAVAILABLE, "panel", error.to_string())
        })?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{device::Device, logs::LogStore, settings};
    use axum::{body::Body, http::header::CONTENT_TYPE};
    use pimprobe_core::MockController;
    use tokio::net::UnixListener;
    use tower::ServiceExt;

    #[tokio::test]
    async fn a_late_panel_request_is_rejected_until_the_display_wakes() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path();
        let brightness = path.join("brightness");
        std::fs::write(&brightness, "0\n").unwrap();
        let relay_path = path.join("panel.sock");
        let listener = UnixListener::bind(&relay_path).unwrap();
        let mut app = App::new(
            Device::Mock(Box::new(MockController::new())),
            settings::open(path.join("settings.json")).unwrap(),
            None,
            LogStore::open(path.join("logs")).unwrap(),
        );
        Arc::get_mut(&mut app).unwrap().panel = Panel {
            brightness: brightness.clone(),
            relay: relay_path,
        };
        let router = super::super::router(app);
        let send = |path, body| {
            router.clone().oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .header("x-pimprobe-panel", "1")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
        };
        assert_eq!(
            send("/api/v1/routine/run", "{}").await.unwrap().status(),
            StatusCode::CONFLICT,
        );
        let relay = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            assert_eq!(socket.read_u8().await.unwrap(), b'W');
            std::fs::write(brightness, "200\n").unwrap();
            socket.write_all(b"1").await.unwrap();
        });
        assert_eq!(
            send("/api/v1/panel/activity", r#"{"wake":true}"#)
                .await
                .unwrap()
                .status(),
            StatusCode::NO_CONTENT,
        );
        relay.await.unwrap();
        assert_eq!(
            send("/api/v1/routine/run", "{}").await.unwrap().status(),
            StatusCode::BAD_REQUEST,
        );
    }
}
