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

use serde::Deserialize;
use std::{net::SocketAddr, path::PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Config {
    pub controller_socket: PathBuf,
    pub listen_address: SocketAddr,
    pub travel_limits: [f64; 3],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            controller_socket: "/run/pimprobe-controller.sock".into(),
            listen_address: "127.0.0.1:8137".parse().unwrap(),
            travel_limits: [210., 235., 125.],
        }
    }
}

#[derive(Debug)]
pub struct Options {
    pub config: Config,
    pub settings: PathBuf,
    pub mock: bool,
    pub ready_token: String,
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let mut config_path = None;
        let mut listen = None;
        let mut settings = None;
        let mut mock = false;
        let mut ready_token = "manual".to_owned();
        while let Some(arg) = args.next() {
            if arg == "--mock" {
                mock = true;
                continue;
            }
            let value = args
                .next()
                .ok_or_else(|| format!("missing value for {arg}"))?;
            match arg.as_str() {
                "--config" => config_path = Some(value),
                "--listen" => listen = Some(value),
                "--settings" => settings = Some(PathBuf::from(value)),
                "--ready-token" => ready_token = value,
                _ => return Err(format!("unknown option {arg}")),
            }
        }
        let mut config = match config_path {
            Some(path) => {
                serde_json::from_slice::<Config>(&std::fs::read(path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?
            }
            None => Config::default(),
        };
        if let Some(address) = listen {
            config.listen_address = address
                .parse()
                .map_err(|e| format!("listen address: {e}"))?;
        }
        if config
            .travel_limits
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.)
        {
            return Err("travel limits must be finite and positive".into());
        }
        // Remote access needs an explicit authentication/access-control design.
        if !config.listen_address.ip().is_loopback() {
            return Err("HTTP must listen on a loopback address".into());
        }
        let settings = settings.unwrap_or_else(|| {
            let base = std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| "/userdata".into()))
                        .join(".config")
                });
            base.join("pimprobe").join(if mock {
                "preview-settings.json"
            } else {
                "settings.json"
            })
        });
        Ok(Self {
            config,
            settings,
            mock,
            ready_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_options_and_public_listener() {
        for args in [
            vec!["--listen"],
            vec!["--listen", "0.0.0.0:8137"],
            vec!["--unknown", "1"],
        ] {
            assert!(Options::parse(args.into_iter().map(str::to_owned)).is_err());
        }
        let options = Options::parse(
            [
                "--mock",
                "--listen",
                "127.0.0.1:18137",
                "--ready-token",
                "qml-test",
            ]
            .map(str::to_owned),
        )
        .unwrap();
        assert!(options.mock);
        assert_eq!(options.config.listen_address.port(), 18137);
        assert_eq!(options.ready_token, "qml-test");
    }
}
