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

use axum::{
    Router,
    http::{HeaderValue, header::CACHE_CONTROL},
    routing::get_service,
};
use pimprobe_controller::SocketController;
use pimprobe_core::MockController;
use pimprobe_service::{
    config::Options,
    device::Device,
    http::{App, router},
    logs::LogStore,
    settings,
};
use std::path::PathBuf;
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "pimprobe-service [--config FILE] [--listen HOST:PORT] [--settings FILE] [--web-root DIR]\n                 pimprobe-service --mock [--listen HOST:PORT] [--settings FILE] [--ready-token TOKEN] [--web-root DIR]"
        );
        return Ok(());
    }
    let options = Options::parse(args).map_err(std::io::Error::other)?;
    let log_dir = if options.mock {
        options.settings.with_file_name("logs")
    } else {
        "/userdata/cnc/logs/pimprobe".into()
    };
    let bed_path = options.settings.with_file_name("bed-counterbore.json");
    let settings = settings::open(options.settings)?;
    let logs = LogStore::open(log_dir)?;
    let device = if options.mock {
        Device::Mock(Box::new(MockController::new()))
    } else {
        Device::Machine(SocketController::start(options.config.controller_socket).await?)
    };
    let app = App::new(
        device,
        settings,
        options.mock.then_some(options.ready_token),
        logs,
        bed_path,
    );
    let listener = tokio::net::TcpListener::bind(options.config.listen_address).await?;
    eprintln!(
        "Listening on http://{}{}",
        listener.local_addr()?,
        if options.mock {
            " (mock controller)"
        } else {
            ""
        }
    );
    let shutdown_app = app.clone();
    axum::serve(listener, with_web(router(app), options.web_root))
        .with_graceful_shutdown(async move {
            #[cfg(unix)]
            {
                let mut term =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .expect("SIGTERM handler");
                tokio::select! { _ = tokio::signal::ctrl_c() => (), _ = term.recv() => () }
            }
            #[cfg(not(unix))]
            let _ = tokio::signal::ctrl_c().await;
            shutdown_app.shutdown().await;
        })
        .await?;
    Ok(())
}

fn with_web(app: Router, root: PathBuf) -> Router {
    let index = get_service(ServeFile::new(root.join("index.html"))).layer(
        SetResponseHeaderLayer::overriding(CACHE_CONTROL, HeaderValue::from_static("no-cache")),
    );
    app.route("/", index.clone())
        .route("/index.html", index)
        .fallback_service(ServeDir::new(root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn entry_page_revalidates_without_changing_asset_caching() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "<html></html>").unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        std::fs::write(dir.path().join("assets/main-hash.js"), "").unwrap();
        let app = with_web(Router::new(), dir.path().into());
        for (uri, control) in [
            ("/?kiosk&build=new", Some("no-cache")),
            ("/index.html", Some("no-cache")),
            ("/assets/main-hash.js", None),
        ] {
            let response = app
                .clone()
                .oneshot(Request::get(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{uri}");
            assert_eq!(
                response
                    .headers()
                    .get(header::CACHE_CONTROL)
                    .map(|value| value.to_str().unwrap()),
                control,
                "{uri}"
            );
        }
    }
}
