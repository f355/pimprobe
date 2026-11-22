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

use crate::{
    run_contact, run_guarded_move, Axis, ContactConfig, Controller, Error, GuardedMoveConfig,
    Modes, TimingPolicy,
};
use serde::{Deserialize, Serialize};

pub const NOMINAL_CONTACT: [f64; 3] = [-224., -72., -113.];
pub const POSITIONING_FEED: f64 = 1000.;
const NOMINAL_COUNTERBORE: [f64; 3] = [-216., -85., -113.];
const COUNTERBORE_CLEARANCE: f64 = 12.;
const COUNTERBORE_DEPTH: f64 = 3.;

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

pub async fn position_for_recording<C: Controller>(c: &C) -> Result<(), Error> {
    move_above(c, NOMINAL_COUNTERBORE, COUNTERBORE_CLEARANCE).await
}

/// Call after homing, clearing the table and putting incremental moves in machine axes.
pub async fn measure<C: Positioning>(
    c: &C,
    expected: [f64; 3],
    settings: ContactConfig,
    mut progress: impl FnMut(Progress) + Send,
) -> Result<Reference, Error> {
    progress(Progress::Message("Moving above the bed corner."));
    // Five millimeters from each wall contact leaves room for the nominal ball.
    let [x, y, z] = expected;
    let [x, y] = [x + 5., y - 5.];
    move_above(c, [x, y, z], 8.).await?;
    measure_corner(c, [x, y], settings, &mut progress).await
}

async fn move_above<C: Controller>(
    c: &C,
    [x, y, z]: [f64; 3],
    clearance: f64,
) -> Result<(), Error> {
    let top = z + clearance;
    c.set_probe(false).await?;
    c.send("G90 G53 G0 Z0").await?;
    c.send(&format!("G90 G53 G0 X{x:.6} Y{y:.6}")).await?;
    c.set_probe(true).await?;
    c.send(&format!("G90 G53 G0 Z{top:.6}")).await?;
    Ok(())
}

/// Use the jogged position inside the counterbore, or enter at its saved center.
pub async fn measure_counterbore<C: Controller>(
    c: &C,
    saved: Option<Reference>,
    settings: ContactConfig,
    mut progress: impl FnMut(Progress) + Send,
) -> Result<Reference, Error> {
    let expected = saved.map_or(NOMINAL_COUNTERBORE, |reference| reference.raw_point);
    let clearance = expected[2] + COUNTERBORE_CLEARANCE;
    if saved.is_some() {
        progress(Progress::Message("Moving above the bed reference hole."));
        move_above(c, expected, COUNTERBORE_CLEARANCE).await?;
        enter_counterbore(c, expected[2], settings).await?;
    }

    progress(Progress::Message("Finding the hole center."));
    let [x, y] = center_counterbore(c, settings).await?;

    progress(Progress::Message("Probing the top surface."));
    let lift = clearance.max(c.state().position[2] + 8.);
    c.send(&format!("G90 G53 G0 Z{lift:.6}")).await?;
    // The rim is 5.45 mm from center; the slot edge is 12 mm away.
    let surface_y = y + 7.5;
    c.send(&format!("G90 G53 G0 Y{surface_y:.6}")).await?;
    let z = contact(c, Axis::Z, -1, 20., settings).await?;
    progress(Progress::Contact(Axis::Z, z));

    progress(Progress::Message("Finding the hole center."));
    let lift = z + 8.;
    c.send(&format!("G90 G53 G0 Z{lift:.6}")).await?;
    c.send(&format!("G90 G53 G0 X{x:.6} Y{y:.6}")).await?;
    enter_counterbore(c, z, settings).await?;
    let [x, y] = center_counterbore(c, settings).await?;
    progress(Progress::Contact(Axis::X, x));
    progress(Progress::Contact(Axis::Y, y));
    finish(c, &mut progress).await?;
    Ok(Reference {
        raw_point: [x, y, z],
    })
}

async fn enter_counterbore<C: Controller>(
    c: &C,
    surface: f64,
    settings: ContactConfig,
) -> Result<(), Error> {
    c.send(&Modes::PROBING.command()).await?;
    // Touching the bed or screw on the way down must stop the routine.
    run_guarded_move(
        c,
        GuardedMoveConfig {
            axis: Axis::Z,
            distance: surface - COUNTERBORE_DEPTH - c.state().position[2],
            feed: settings.coarse_feed,
            retract_distance: settings.retract_distance,
        },
        TimingPolicy::default(),
    )
    .await?;
    Ok(())
}

async fn center_counterbore<C: Controller>(
    c: &C,
    settings: ContactConfig,
) -> Result<[f64; 2], Error> {
    let side = ContactConfig {
        retract_distance: settings.retract_distance.min(0.5),
        ..settings
    };
    let mut xy = [0.; 2];
    for (axis, word, min_span) in [(Axis::X, 'X', 3.), (Axis::Y, 'Y', 6.)] {
        let lo = contact(c, axis, -1, 11., side).await?;
        let hi = contact(c, axis, 1, 11., side).await?;
        // The first pair can cross an off-center chord. The second spans the bore.
        if !(min_span..=11.).contains(&(hi - lo)) {
            return Err(Error::Position(
                "The contacts do not match the bed reference hole. Position the ball inside the hole, above the screw.".into(),
            ));
        }
        let center = (lo + hi) / 2.;
        c.send(&format!("G90 G53 G1 {word}{center:.6} F{POSITIONING_FEED}"))
            .await?;
        xy[axis.index()] = center;
    }

    Ok(xy)
}

async fn measure_corner<C: Positioning>(
    c: &C,
    [x, y]: [f64; 2],
    settings: ContactConfig,
    progress: &mut (impl FnMut(Progress) + Send),
) -> Result<Reference, Error> {
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
    finish(c, progress).await?;
    Ok(Reference { raw_point })
}

async fn finish<C: Controller>(
    c: &C,
    progress: &mut (impl FnMut(Progress) + Send),
) -> Result<(), Error> {
    progress(Progress::Message("Returning to clearance."));
    c.send("G90 G53 G0 Z0").await?;
    c.set_probe(false).await
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
    async fn recording_stages_near_the_corner_without_probing() {
        let machine = MockController::new();
        position_for_recording(&machine).await.unwrap();
        let state = machine.state();
        assert_eq!(state.position[..3], [-216., -85., -101.]);
        assert!(state.probe_extended);
        let commands = machine.commands();
        assert!(commands
            .windows(2)
            .any(|moves| { moves == ["G90 G53 G0 Z0", "G90 G53 G0 X-216.000000 Y-85.000000"] }));
        assert!(!commands.iter().any(|line| line.starts_with("G38")));
    }

    #[tokio::test]
    async fn counterbore_reference_uses_the_center_at_a_fixed_depth_below_the_bed() {
        let config = ContactConfig {
            axis: Axis::Z,
            direction: -1,
            coarse_travel: 16.,
            retract_distance: 0.5,
            retract_feed: POSITIONING_FEED,
            coarse_feed: 300.,
            fine_feed: 50.,
        };
        for automatic in [false, true] {
            let mut state = MockController::new().state();
            state.position = [-214., -87., -116., 720.];
            let machine = MockController::with_state(state);
            machine.set_probe(true).await.unwrap();
            machine.set_geometry(MockGeometry::Counterbore {
                center: [-217., -88.],
                rim_shift: [0.55, -0.35],
                radius: 4.5,
                top: -114.,
                floor: -121.,
            });
            let saved = automatic.then_some(Reference {
                raw_point: [-216., -85., -113.],
            });
            let mut surface_touch = None;
            let mut final_xy_z = None;
            let point = measure_counterbore(&machine, saved, config, |event| match event {
                Progress::Contact(Axis::Z, _) => surface_touch = Some(machine.state().position),
                Progress::Contact(Axis::X, _) => final_xy_z = Some(machine.state().position[2]),
                _ => (),
            })
            .await
            .unwrap()
            .raw_point;
            let [x, y, ..] = surface_touch.unwrap();
            assert!((x + 216.45).abs() < 0.002 && (y + 80.85).abs() < 0.002);
            assert!((final_xy_z.unwrap() + 117.).abs() < 0.002);
            for (actual, expected) in point.into_iter().zip([-217., -88., -114.]) {
                assert!((actual - expected).abs() < 0.002, "{point:?}");
            }
            assert_eq!(machine.state().position[2], 0.);
            assert!(!machine.state().probe_extended);
        }
    }

    #[tokio::test]
    async fn automatic_reference_does_not_search_sideways_if_it_misses_the_hole() {
        let machine = MockController::new();
        machine.set_geometry(MockGeometry::Plane {
            axis: Axis::Z,
            coordinate: -114.,
        });
        let result = measure_counterbore(
            &machine,
            Some(Reference {
                raw_point: [-216., -85., -113.],
            }),
            ContactConfig {
                axis: Axis::Z,
                direction: -1,
                coarse_travel: 16.,
                retract_distance: 0.5,
                retract_feed: POSITIONING_FEED,
                coarse_feed: 300.,
                fine_feed: 50.,
            },
            |_| {},
        )
        .await;
        assert!(matches!(result, Err(Error::UnexpectedContact { .. })));
        assert!(machine.state().position[2] > -114.);
        assert!(!machine
            .commands()
            .iter()
            .any(|line| line.starts_with("G38.3 X")));
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
