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
use pimprobe_core::MockController;
use pimprobe_service::{
    config::Options,
    device::Device,
    http::{App, router},
    settings::Settings,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "pimprobe-service [--config FILE] [--listen HOST:PORT] [--settings FILE]\n                 pimprobe-service --mock [--listen HOST:PORT] [--settings FILE] [--ready-token TOKEN]"
        );
        return Ok(());
    }
    let options = Options::parse(args).map_err(std::io::Error::other)?;
    let settings = Settings::open(options.settings)?;
    let device = if options.mock {
        Device::Mock(Box::new(
            MockController::new().with_delay(std::time::Duration::from_millis(75)),
        ))
    } else {
        let [x, y, z] = options.config.travel_limits;
        Device::Machine(
            SocketController::start(options.config.controller_socket, [x, y, z, 0.]).await?,
        )
    };
    let app = App::new(
        device,
        settings,
        options.mock.then_some(options.ready_token),
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
    axum::serve(listener, router(app))
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
