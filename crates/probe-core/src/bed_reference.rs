// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::{run_contact, Axis, ContactConfig, Controller, Error, Modes, TimingPolicy};
use serde::{Deserialize, Serialize};

pub const NOMINAL_CONTACT: [f64; 3] = [-224., -72., -113.];
pub const POSITIONING_FEED: f64 = 1000.;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub raw_point: [f64; 3],
}

pub enum Progress {
    Message(&'static str),
    Contact(Axis, f64),
}

#[crate::async_trait]
pub trait Positioning: Controller {
    const BLENDS_POSITIONING: bool = true;

    async fn position(&self, source: &str) -> Result<(), Error>;
}

/// Call after homing, clearing the table and putting incremental moves in machine axes.
pub async fn measure<C: Positioning>(
    c: &C,
    expected: [f64; 3],
    settings: ContactConfig,
    mut progress: impl FnMut(Progress) + Send,
) -> Result<Reference, Error> {
    // Five millimeters from each wall contact leaves room for the nominal ball.
    let [x, y, top] = [expected[0] + 5., expected[1] - 5., expected[2] + 8.];
    progress(Progress::Message("Moving above the bed corner."));
    c.set_probe(false).await?;
    c.send("G90 G53 G0 Z0").await?;
    c.send(&format!("G90 G53 G0 X{x:.6} Y{y:.6}")).await?;
    c.set_probe(true).await?;
    c.send(&format!("G90 G53 G0 Z{top:.6}")).await?;
    progress(Progress::Message("Probing the top surface."));
    let z = contact(c, Axis::Z, -1, 16., settings).await?;
    progress(Progress::Contact(Axis::Z, z));

    let depth = z - 3.;
    let transfer_height = z + 8.;
    // When blending, keep the part of each Z move near the surface vertical.
    let (lift, lower) = if C::BLENDS_POSITIONING {
        let clearance = z + 4.;
        (
            format!("G90 G53 G1 Z{clearance:.6} F{POSITIONING_FEED}\nG64 P3 Q0\n"),
            format!("G90 G53 G1 Z{clearance:.6} F{POSITIONING_FEED}\n"),
        )
    } else {
        (String::new(), String::new())
    };
    let mut raw_point = [0., 0., z];
    for (axis, direction, xy) in [(Axis::X, 1, [x - 8., y]), (Axis::Y, -1, [x, y + 8.])] {
        progress(Progress::Message(if axis == Axis::X {
            "Probing the X wall."
        } else {
            "Probing the Y wall."
        }));
        c.position(&format!(
            "G21 G90 G94 G61\n{lift}G90 G53 G1 Z{transfer_height:.6} F{POSITIONING_FEED}\n\
             G90 G53 G1 X{:.6} Y{:.6} F{POSITIONING_FEED}\n\
             G61\n{lower}G90 G53 G1 Z{depth:.6} F{POSITIONING_FEED}",
            xy[0], xy[1]
        ))
        .await?;
        let value = contact(c, axis, direction, 8., settings).await?;
        raw_point[axis.index()] = value;
        progress(Progress::Contact(axis, value));
    }
    progress(Progress::Message("Returning to clearance."));
    c.send("G90 G53 G0 Z0").await?;
    c.set_probe(false).await?;
    Ok(Reference { raw_point })
}

async fn contact<C: Controller>(
    c: &C,
    axis: Axis,
    direction: i32,
    travel: f64,
    settings: ContactConfig,
) -> Result<f64, Error> {
    c.send(&Modes::PROBING.command()).await?;
    let result = run_contact(
        c,
        ContactConfig {
            axis,
            direction,
            coarse_travel: travel,
            ..settings
        },
        TimingPolicy::default(),
    )
    .await?;
    Ok(result.contact.position[axis.index()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockController, MockGeometry};

    #[crate::async_trait]
    impl Positioning for MockController {
        const BLENDS_POSITIONING: bool = false;

        async fn position(&self, source: &str) -> Result<(), Error> {
            for line in source.lines().filter(|line| line.starts_with("G90 G53")) {
                self.send(line).await?;
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn bed_contacts_do_not_depend_on_probe_calibration() {
        for offset in [[-55., 0.3, 45., 0.], [-100., 8., -20., 0.]] {
            let mut state = MockController::new().state();
            state.position = [-100., -50., -20., 720.];
            state.probe_offset = offset;
            let machine = MockController::with_state(state);
            machine.set_geometry(MockGeometry::Corner {
                origin: [-221.8, -75.5, 0., 0.],
                directions: [1, -1],
                travel: [8., 8.],
                top: -114.,
                floor: -200.,
                internal: false,
            });
            let config = ContactConfig {
                axis: Axis::Z,
                direction: -1,
                coarse_travel: 16.,
                retract_distance: 0.5,
                retract_feed: POSITIONING_FEED,
                coarse_feed: 300.,
                fine_feed: 50.,
            };
            let point = measure(&machine, NOMINAL_CONTACT, config, |_| {})
                .await
                .unwrap()
                .raw_point;
            for (actual, expected) in point.into_iter().zip([-225., -72.3, -114.]) {
                assert!((actual - expected).abs() < 0.002, "{point:?}");
            }
            let z_moves: Vec<_> = machine
                .commands()
                .into_iter()
                .filter(|line| line.starts_with("G90 G53 G1 Z"))
                .collect();
            assert_eq!(
                z_moves,
                [
                    "G90 G53 G1 Z-106.000000 F1000",
                    "G90 G53 G1 Z-117.000000 F1000"
                ]
                .repeat(2)
            );
            let state = machine.state();
            assert_eq!([state.position[2], state.position[3]], [0., 720.]);
            assert!(!state.probe_extended);
            machine.set_geometry(MockGeometry::Empty);
            assert_eq!(
                measure(&machine, NOMINAL_CONTACT, config, |_| {})
                    .await
                    .unwrap_err(),
                Error::CoarseNoContact
            );
        }
    }
}
