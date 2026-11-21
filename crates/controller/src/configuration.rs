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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use super::*;

async fn receive(events: &mut broadcast::Receiver<Event>) -> Result<Event, Error> {
    let event = events.recv().await.map_err(|error| match error {
        broadcast::error::RecvError::Lagged(_) => Error::EventLagged,
        broadcast::error::RecvError::Closed => Error::Disconnected,
    })?;
    if let Some(code) = event.controller_error {
        return Err(if code == -1 {
            Error::Disconnected
        } else {
            Error::Controller(format!("The machine reported error {code}."))
        });
    }
    Ok(event)
}

#[async_trait::async_trait]
impl Controller for SocketController {
    fn state(&self) -> State {
        SocketController::state(self)
    }
    fn subscribe(&self) -> pimprobe_core::EventReceiver {
        SocketController::subscribe(self).into()
    }
    async fn send(&self, command: &str) -> Result<(), Error> {
        SocketController::send(self, command).await
    }
    async fn read_settings(&self) -> Result<BTreeMap<i32, f64>, Error> {
        let mut events = self.subscribe();
        timeout(IO_TIMEOUT, async {
            self.send("$$").await?;
            let mut settings = BTreeMap::new();
            loop {
                let event = receive(&mut events).await?;
                if let Some((key, value)) = event.setting {
                    settings.insert(key, value);
                }
                if event.acknowledged && !settings.is_empty() {
                    return Ok(settings);
                }
            }
        })
        .await
        .map_err(|_| Error::Timeout)?
    }

    async fn read_modes(&self) -> Result<Modes, Error> {
        let mut events = self.subscribe();
        timeout(IO_TIMEOUT, async {
            self.send("$G").await?;
            loop {
                if let Some(modes) = receive(&mut events).await?.modes {
                    if !modes.valid() {
                        return Err(Error::Preflight(
                            "The machine did not report all active G-code modes.".into(),
                        ));
                    }
                    return Ok(modes);
                }
            }
        })
        .await
        .map_err(|_| Error::Timeout)?
    }

    async fn read_firmware_version(&self) -> Result<String, Error> {
        let mut events = self.subscribe();
        timeout(IO_TIMEOUT, async {
            self.send("$V").await?;
            loop {
                if let Some(version) = receive(&mut events).await?.firmware_version {
                    return Ok(version);
                }
            }
        })
        .await
        .map_err(|_| Error::Timeout)?
    }

    async fn read_coordinates(&self) -> Result<State, Error> {
        let mut events = self.subscribe();
        let start = self.state();
        let mut received = [
            false,
            !pimprobe_core::rotary_supported(&start.firmware_version),
            false,
            false,
            false,
        ];
        let mut calibration = [false; 3];
        timeout(IO_TIMEOUT, async {
            self.send("$$").await?;
            self.send("$#").await?;
            while !received.into_iter().all(|value| value)
                || !calibration.into_iter().all(|value| value)
            {
                let event = receive(&mut events).await?;
                if let Some((key, _)) = event.setting {
                    if let Some(i) = [35, 201, 202].iter().position(|k| *k == key) {
                        calibration[i] = true;
                    }
                }
                received[0] |= event.wcs_origin.is_some_and(|(wcs, _)| wcs == start.wcs);
                received[1] |= event.wcs_rotation.is_some_and(|(wcs, _)| wcs == start.wcs);
                received[2] |= event.coordinate_offset.is_some();
                received[3] |= event.tool_length_offset.is_some();
                received[4] |= event.probe.is_some();
            }
            Ok(self.state())
        })
        .await
        .map_err(|_| Error::Preflight("The machine did not report its work coordinates.".into()))?
    }

    async fn write_setting(&self, key: i32, value: f64) -> Result<(), Error> {
        if key < 0 || !value.is_finite() {
            return Err(Error::InvalidConfig("Invalid machine setting".into()));
        }
        let mut events = self.subscribe();
        timeout(Duration::from_secs(5), async {
            self.send(&format!("${key}={value:.6}")).await?;
            loop {
                if receive(&mut events).await?.acknowledged {
                    return Ok(());
                }
            }
        })
        .await
        .map_err(|_| Error::Timeout)?
    }

    async fn write_modes(&self, modes: Modes, plane: Option<i32>) -> Result<(), Error> {
        if !modes.valid() || plane.is_some_and(|plane| !(17..=19).contains(&plane)) {
            return Err(Error::InvalidConfig("Invalid probing modes".into()));
        }
        if self.state().motion_blocked {
            return Err(Error::MotionBlocked);
        }
        if let Some(plane) = plane {
            self.send(&format!("G{plane}")).await?;
            self.read_modes().await?;
            if self.state().plane != plane {
                return Err(Error::Controller(
                    "The machine did not confirm the requested G-code plane.".into(),
                ));
            }
        }
        self.send(&modes.command()).await?;
        if self.read_modes().await? != modes {
            return Err(Error::Controller(
                "The machine did not confirm the requested G-code modes.".into(),
            ));
        }
        Ok(())
    }

    async fn write_coordinates(
        &self,
        wcs: i32,
        axes: [Option<f64>; 4],
        rotation: Option<f64>,
    ) -> Result<(), Error> {
        if !(54..=59).contains(&wcs)
            || axes
                .iter()
                .flatten()
                .chain(rotation.iter())
                .any(|value| !value.is_finite())
        {
            return Err(Error::InvalidConfig("Invalid work coordinates".into()));
        }
        let mut command = format!("G10 L2 P{}", wcs - 53);
        for (i, value) in axes.iter().enumerate() {
            if let Some(value) = value {
                command.push_str(&format!(" {}{value:.3}", ["X", "Y", "Z", "A"][i]));
            }
        }
        if let Some(angle) = rotation {
            command.push_str(&format!(" R{angle:.6}"));
        }
        let mut events = self.subscribe();
        timeout(IO_TIMEOUT, async {
            self.send(&command).await?;
            self.send("$#").await?;
            let mut origin_received = axes.iter().all(Option::is_none);
            let mut rotation_received = rotation.is_none();
            let mut probe_received = false;
            while !origin_received || !rotation_received || !probe_received {
                let event = receive(&mut events).await?;
                // Consume the historical contact before the next motion subscribes.
                probe_received |= event.probe.is_some();
                if let Some((_, origin)) = event.wcs_origin.filter(|(selected, _)| *selected == wcs)
                {
                    if axes.iter().zip(origin).any(|(value, actual)| {
                        value.is_some_and(|value| (actual - value).abs() > 0.0015)
                    }) {
                        return Err(Error::Controller(
                            "The machine did not confirm the new work zero.".into(),
                        ));
                    }
                    origin_received = true;
                }
                if let Some(((_, actual), expected)) = event
                    .wcs_rotation
                    .filter(|(selected, _)| *selected == wcs)
                    .zip(rotation)
                {
                    let difference = (actual - expected + 180.).rem_euclid(360.) - 180.;
                    if difference.abs() > 0.0015 {
                        return Err(Error::Controller(
                            "The machine did not confirm the new work coordinate rotation.".into(),
                        ));
                    }
                    rotation_received = true;
                }
            }
            Ok(())
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
}
