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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelResult {
    pub operation: RotaryOperation,
    pub initial_touches: [Position; 2],
    pub touches: [Position; 2],
    pub correction: f64,
    pub residual: f64,
    pub center: Position,
}

fn axes(operation: RotaryOperation) -> (Axis, Axis, i32) {
    match operation {
        RotaryOperation::Horizontal => (Axis::Y, Axis::Z, -1),
        RotaryOperation::Vertical => (Axis::Z, Axis::Y, 1),
        RotaryOperation::VerticalNegative => (Axis::Z, Axis::Y, -1),
        RotaryOperation::Axis => unreachable!(),
    }
}

fn distances(config: &RotaryConfig) -> (f64, f64) {
    if config.operation == RotaryOperation::Horizontal {
        (config.y_distance, config.z_distance)
    } else {
        (config.z_distance, config.y_distance)
    }
}

pub(super) fn check_review(state: &State, config: &RotaryConfig) -> Result<(), Error> {
    let (spacing, measured, direction) = axes(config.operation);
    let i = spacing.index();
    let j = measured.index();
    let (spacing_distance, search_distance) = distances(config);
    let second = state.position[i] + spacing_distance;
    check_path(
        state,
        spacing,
        state.position[i].min(second),
        state.position[i].max(second),
    )?;
    let end = state.position[j] + f64::from(direction) * search_distance;
    check_path(
        state,
        measured,
        state.position[j].min(end),
        state.position[j].max(end),
    )?;
    Ok(())
}

fn correction(operation: RotaryOperation, points: &[Position; 2]) -> f64 {
    let (spacing, measured, _) = axes(operation);
    let slope = (points[1][measured.index()] - points[0][measured.index()])
        / (points[1][spacing.index()] - points[0][spacing.index()]);
    slope.atan().to_degrees()
        * if operation == RotaryOperation::Horizontal {
            -1.
        } else {
            1.
        }
}

async fn position<C: Controller + ?Sized>(
    c: &C,
    p: Position,
    config: &RotaryConfig,
) -> Result<(), Error> {
    if config.operation == RotaryOperation::Horizontal {
        return move_tip(c, p, config).await;
    }
    let state = c.state();
    let target = machine_tip(&state, p);
    let delta = std::array::from_fn(|i| quantize(target[i] - state.position[i]));
    if delta[..3].iter().all(|value| value.abs() <= 0.001) {
        return Ok(());
    }
    // A vertical face can intersect upward travel as well as downward travel.
    run_position_stage(
        c,
        Stage::movement(StageKind::GuardedMove, delta, config.positioning_feed),
        config.retract,
        TimingPolicy::default(),
    )
    .await?;
    Ok(())
}

async fn clear<C: Controller + ?Sized>(
    c: &C,
    start: Position,
    config: &RotaryConfig,
) -> Result<(), Error> {
    let (spacing, measured, _) = axes(config.operation);
    let mut p = tip(&c.state());
    p[measured.index()] = start[measured.index()];
    position(c, p, config).await?;
    p[spacing.index()] = start[spacing.index()];
    position(c, p, config).await
}

async fn pair<C: Controller + ?Sized>(
    c: &C,
    start: Position,
    config: &RotaryConfig,
    observe: &impl Fn(Progress),
) -> Result<[Position; 2], Error> {
    let (spacing, measured, direction) = axes(config.operation);
    let mut points = [[0.; 4]; 2];
    let (spacing_distance, search_distance) = distances(config);
    for (index, distance) in [0., spacing_distance].into_iter().enumerate() {
        let mut p = tip(&c.state());
        p[measured.index()] = start[measured.index()];
        position(c, p, config).await?;
        p[spacing.index()] = start[spacing.index()] + distance;
        position(c, p, config).await?;
        comment(observe, format!("Touch {}", index + 1));
        let surface = touch(c, measured, direction, search_distance, config, observe).await?;
        points[index] = tip(&c.state());
        points[index][measured.index()] = surface;
        if config.operation != RotaryOperation::Horizontal {
            points[index][2] += config.diameter / 2.;
        }
    }
    clear(c, start, config).await?;
    Ok(points)
}

pub(super) async fn run<C: Controller + ?Sized>(
    c: &C,
    plan: &RotaryPlan,
    observe: &impl Fn(Progress),
) -> Result<LevelResult, Error> {
    let config = &plan.config;
    let start = tip(&plan.start);
    comment(observe, "Measure the initial surface angle");
    let initial = pair(c, start, config, observe).await?;
    let mut points = initial;
    let mut angle = correction(config.operation, &points);
    let mut total = 0.;
    for pass in 0..3 {
        if angle.abs() > 20. {
            return Err(Error::Preflight(
                "position the surface within 20 degrees of its final orientation".into(),
            ));
        }
        comment(observe, format!("A correction {angle:.4} degrees"));
        if angle.abs() >= 0.001 {
            execute(
                c,
                &Stage::movement(
                    StageKind::PositionMove,
                    [0., 0., 0., quantize(angle)],
                    config.rotary_feed,
                ),
                TimingPolicy::default(),
                false,
            )
            .await?;
            total += quantize(angle);
        }
        comment(observe, format!("Verify surface, pass {}", pass + 1));
        points = pair(c, start, config, observe).await?;
        angle = correction(config.operation, &points);
        if angle.abs() <= 0.05 {
            let mut center: Position = std::array::from_fn(|i| (points[0][i] + points[1][i]) / 2.);
            center[3] = c.state().position[3];
            return Ok(LevelResult {
                operation: config.operation,
                initial_touches: initial,
                touches: points,
                correction: total,
                residual: angle,
                center,
            });
        }
    }
    Err(Error::Compensation(format!(
        "surface remains tilted by {angle:.4} degrees after rechecking"
    )))
}

pub(super) async fn zero<C: Controller + ?Sized>(
    c: &C,
    wcs: i32,
    result: &LevelResult,
) -> Result<(), Error> {
    let state = query_rotary_frame(c).await?;
    let a = result.center[3] - state.coordinate_offset[3];
    let linear = if result.operation == RotaryOperation::Horizontal {
        (2, result.center[2] - state.coordinate_offset[2])
    } else {
        (
            1,
            y_origin_for_point(
                &state,
                [result.center[0], result.center[1], result.center[2]],
                state.wcs_rotation.unwrap_or(0.),
            )?,
        )
    };
    let modes = query_modes(c).await?;
    set_modes(c, Modes { units: 21, ..modes }).await?;
    let write = write_coordinate_data(c, wcs, &[(3, a), linear], None).await;
    if !c.state().motion_blocked {
        set_modes(c, modes).await?;
    }
    write
}

pub(super) fn program(plan: &RotaryPlan) -> String {
    let c = &plan.config;
    let (spacing, measured, direction) = axes(c.operation);
    let (spacing_distance, search_distance) = distances(c);
    let mut lines = vec![
        format!("; {}", c.operation.label()),
        "; current_y/current_z := G53 carriage position after each completed move".into(),
        "G21 G94 G91".into(),
    ];
    if rotary_supported(&plan.start.firmware_version) {
        lines.push(format!(
            "G10 L2 P{} R0 ; measure in machine axes",
            plan.start.wcs - 53
        ));
    }
    for phase in ["initial", "verified"] {
        lines.push(format!(
            "; {phase} pair, {} mm between points",
            spacing_distance.abs()
        ));
        for index in 0..2 {
            if index == 1 {
                lines.push(format!(
                    "G38.3 {spacing}{} F{} ; position for touch {}",
                    spacing_distance,
                    c.positioning_feed,
                    index + 1
                ));
            }
            lines.push(format!(
                "G38.3 {measured}{} F{} ; coarse",
                f64::from(direction) * search_distance,
                c.coarse_feed
            ));
            lines.push(format!(
                "G1 {measured}{} F{}",
                -f64::from(direction) * c.retract,
                c.positioning_feed
            ));
            lines.push(format!(
                "G38.2 {measured}{} F{} ; fine",
                f64::from(direction) * (c.retract + 0.5),
                c.fine_feed
            ));
            lines.push(format!(
                "; #<{phase}_{}> := {measured} from fine contact (G53)",
                index + 1
            ));
            lines.push(format!(
                "G1 {measured}{} F{}",
                -f64::from(direction) * c.retract,
                c.positioning_feed
            ));
            lines.push(format!(
                "{} {measured}[{} - #<current_{}>] F{} ; clear surface",
                if measured == Axis::Z { "G1" } else { "G38.3" },
                plan.start.position[measured.index()],
                measured.to_string().to_lowercase(),
                c.positioning_feed
            ));
        }
        lines.push(format!(
            "G38.3 {spacing}{} F{} ; return to starting point",
            -spacing_distance, c.positioning_feed
        ));
        if phase == "initial" {
            lines.push(format!(
                "; #<a_correction> := {}ATAN((initial_2 - initial_1) / {})",
                if c.operation == RotaryOperation::Horizontal {
                    "-"
                } else {
                    ""
                },
                spacing_distance
            ));
            lines.push(format!("G1 A#<a_correction> F{}", c.rotary_feed));
        }
    }
    lines.push("; Corrections are rechecked, with up to three pairs if needed".into());
    if rotary_supported(&plan.start.firmware_version) {
        lines.push(format!(
            "G10 L2 P{} R{} ; restore work rotation",
            plan.start.wcs - 53,
            plan.start.wcs_rotation.unwrap_or(0.)
        ));
    }
    lines.push(format!("G{} ; restore arc plane", plan.start.plane));
    lines.push(format!(
        "{} ; restore captured modes",
        plan.start.modes.command()
    ));
    lines.join("\n")
}
