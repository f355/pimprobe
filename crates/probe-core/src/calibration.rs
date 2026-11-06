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

use crate::*;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct CalibrationOptions {
    pub ball: bool,
    pub dimensions: [Option<f64>; 2],
    pub offsets: [bool; 3],
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Calibration {
    pub current_diameter: f64,
    /// Controller settings $33, $34 and $35, not the derived Z correction.
    pub current_offsets: [f64; 3],
    pub ball_axes: [bool; 2],
    pub ball_estimates: [Option<f64>; 2],
    pub offset_axes: [bool; 3],
    pub proposed_diameter: Option<f64>,
    pub proposed_offsets: [Option<f64>; 3],
    pub diameter: Option<f64>,
    pub offsets: [Option<f64>; 3],
    pub result: RoutineResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

fn invalid(message: &str) -> Error {
    Error::InvalidConfig(message.into())
}

/// Recalculate the completed contacts without changing the WCS or commanding motion.
pub fn calibrate_probe(
    plan: &RoutinePlan,
    result: &RoutineResult,
    state: &State,
    current_diameter: f64,
    options: &CalibrationOptions,
) -> Result<Calibration, Error> {
    if result.angle.is_some() || plan.config.family == "angle" {
        return Err(invalid("Angle measurements cannot calibrate the probe."));
    }
    if !state.probe_offset_known || !finite(state.probe_offset) {
        return Err(invalid("The saved probe offsets are unavailable."));
    }
    let z = state
        .probe_z_calibration
        .filter(|v| v.is_finite())
        .ok_or_else(|| invalid("The saved probe Z offset is unavailable."))?;
    let current_offsets = [state.probe_offset[0], state.probe_offset[1], z];
    let measured: [bool; 3] =
        std::array::from_fn(|i| result.axes.iter().any(|name| name == ["X", "Y", "Z"][i]));
    let ball_axes = std::array::from_fn(|i| {
        plan.config.family == "center" && result.raw_spans[i].is_some_and(positive)
    });
    let mut ball_estimates = [None; 2];
    for i in 0..2 {
        if let Some(width) = options.dimensions[i] {
            if !ball_axes[i] || !positive(width) {
                return Err(invalid(
                    "Enter a positive true dimension for an opposing-contact measurement.",
                ));
            }
            let span = result.raw_spans[i].unwrap();
            let diameter = if plan.config.center_internal() {
                width - span
            } else {
                span - width
            };
            if !Parameter::Diameter.range().contains(diameter) {
                return Err(invalid(
                    "The true dimension gives an invalid ball diameter. Check the dimension and units.",
                ));
            }
            ball_estimates[i] = Some(diameter);
        }
    }
    let estimates: Vec<_> = ball_estimates.iter().flatten().copied().collect();
    let proposed_diameter =
        (!estimates.is_empty()).then(|| estimates.iter().sum::<f64>() / estimates.len() as f64);
    let diameter = if options.ball {
        if proposed_diameter.is_none() {
            return Err(invalid(
                "Enter at least one true dimension to calibrate the ball.",
            ));
        }
        proposed_diameter
    } else {
        None
    };
    let ball = diameter.unwrap_or(current_diameter);
    if !Parameter::Diameter.range().contains(ball) {
        return Err(invalid("The saved ball diameter is invalid."));
    }

    // Opposing contacts give a centre independent of ball diameter.
    let (sin, cos) = result
        .measurement_rotation
        .or(plan.start.wcs_rotation)
        .unwrap_or(0.)
        .to_radians()
        .sin_cos();
    let mut offset_axes = [false; 3];
    if !result.reference_used && !result.zeroed && state.wcs == result.wcs {
        // A partial, oblique measurement gives one constraint on two machine
        // offsets. Do not guess which of the two offsets is wrong.
        offset_axes[0] = plan.config.family == "center"
            && (measured[0] && measured[1]
                || measured[0] && sin.abs() < 1e-9
                || measured[1] && cos.abs() < 1e-9);
        offset_axes[1] = plan.config.family == "center"
            && (measured[0] && measured[1]
                || measured[1] && sin.abs() < 1e-9
                || measured[0] && cos.abs() < 1e-9);
        offset_axes[2] = measured[2];
    }
    let mut proposed_offsets = [None; 3];
    let target = state.work_position_at([0.; 4]);
    let (s, c) = state.wcs_rotation.unwrap_or(0.).to_radians().sin_cos();
    let zero = [
        -c * target[0] + s * target[1],
        -s * target[0] - c * target[1],
        -target[2] - state.tool_length_offset,
    ];
    for i in 0..3 {
        if !offset_axes[i] {
            if options.offsets[i] {
                return Err(invalid(
                    "Calibrate X/Y from a centre measurement at an independently set work zero, or Z from a surface at Z0.",
                ));
            }
            continue;
        }
        let Some(raw) = result.raw_point[i].filter(|v| v.is_finite()) else {
            if options.offsets[i] {
                return Err(invalid("The original probe contacts are unavailable."));
            }
            continue;
        };
        let value = if i == 2 {
            // $35 = derived probe Z correction + ($202 - $201).
            raw - zero[i] + z - state.probe_offset[2]
        } else {
            zero[i] - raw
        };
        if !value.is_finite() || value.abs() > 1000. {
            if options.offsets[i] {
                return Err(invalid(
                    "The calculated offset is invalid. Check the reference work zero.",
                ));
            }
            continue;
        }
        proposed_offsets[i] = Some(value);
    }
    let offsets =
        std::array::from_fn(|i| options.offsets[i].then_some(proposed_offsets[i]).flatten());
    let saved: [f64; 3] = std::array::from_fn(|i| offsets[i].unwrap_or(current_offsets[i]));
    let mut updated = result.clone();
    for i in 0..3 {
        if let Some(raw) = result.raw_point[i].filter(|_| i == 2 || plan.config.family == "center")
        {
            updated.machine_point[i] = Some(if i == 2 {
                raw - (saved[2] - z + state.probe_offset[2])
            } else {
                raw + saved[i]
            });
        }
        if let Some(span) = result.raw_spans[i] {
            updated.spans[i] = Some(
                span + if plan.config.center_internal() {
                    ball
                } else {
                    -ball
                },
            );
        }
    }
    updated.update_work_coordinates(state);
    Ok(Calibration {
        current_diameter,
        current_offsets,
        ball_axes,
        ball_estimates,
        offset_axes,
        proposed_diameter,
        proposed_offsets,
        diameter,
        offsets,
        result: updated,
        warning: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(config: RoutineConfig) -> (RoutinePlan, RoutineResult, State) {
        let mut state = MockController::new().state();
        state.probe_offset = [1., -2., 20., 0.];
        state.probe_z_calibration = Some(-50.);
        state.position = [-100., -80., -40., 0.];
        state.work_position = [-10., -20., -30., 0.];
        state.tool_length_offset = 12.;
        state.wcs = 54;
        let plan = RoutinePlan {
            config,
            start: state.clone(),
            steps: vec![],
        };
        let result = RoutineResult {
            wcs: 54,
            axes: vec!["X".into(), "Y".into()],
            raw_point: [Some(-91.), Some(-58.), None],
            raw_spans: [Some(8.), Some(7.8), None],
            ..Default::default()
        };
        (plan, result, state)
    }

    #[test]
    fn internal_external_and_selected_dimensions() {
        for (feature, widths) in [("hole", [10.1, 9.9]), ("boss", [5.9, 5.7])] {
            let (plan, result, state) = fixture(RoutineConfig {
                family: "center".into(),
                feature: feature.into(),
                ..Default::default()
            });
            let options = CalibrationOptions {
                ball: true,
                dimensions: widths.map(Some),
                ..Default::default()
            };
            let cal = calibrate_probe(&plan, &result, &state, 2., &options).unwrap();
            assert!((cal.diameter.unwrap() - 2.1).abs() < 1e-10);
            assert_eq!(cal.offsets, [None; 3]);
            assert_eq!(cal.proposed_diameter, cal.diameter);
            assert!((cal.result.spans[0].unwrap() - widths[0]).abs() < 1e-10);
            let one = CalibrationOptions {
                dimensions: [Some(widths[0] + 0.2), None],
                ..options
            };
            let expected = if feature == "hole" { 2.3 } else { 1.9 };
            assert!(
                (calibrate_probe(&plan, &result, &state, 2., &one)
                    .unwrap()
                    .diameter
                    .unwrap()
                    - expected)
                    .abs()
                    < 1e-10
            );
        }
    }

    #[test]
    fn offsets_use_rotated_wcs_and_do_not_depend_on_active_tlo() {
        let (mut plan, mut result, mut state) = fixture(RoutineConfig {
            family: "center".into(),
            feature: "hole".into(),
            ..Default::default()
        });
        state.wcs_rotation = Some(30.);
        plan.start.wcs_rotation = Some(30.);
        let options = CalibrationOptions {
            offsets: [true, true, false],
            ..Default::default()
        };
        let cal = calibrate_probe(&plan, &result, &state, 2., &options).unwrap();
        assert!(cal.result.point[0].unwrap().abs() < 1e-10);
        assert!(cal.result.point[1].unwrap().abs() < 1e-10);
        result.axes = vec!["Z".into()];
        result.raw_point = [None, None, Some(-35.)];
        let options = CalibrationOptions {
            offsets: [false, false, true],
            ..Default::default()
        };
        let a = calibrate_probe(&plan, &result, &state, 2., &options).unwrap();
        state.work_position[2] += state.tool_length_offset;
        state.tool_length_offset = 0.;
        let b = calibrate_probe(&plan, &result, &state, 2., &options).unwrap();
        assert_eq!(a.offsets, b.offsets);
        assert_eq!(a.offsets[2], Some(-83.));
        assert!(a.result.point[2].unwrap().abs() < 1e-10);
    }

    #[test]
    fn z_calibration_preserves_stock_reference_conventions() {
        for reference in [0., -40., -100.] {
            let saved_length = -60. - reference;
            let origin = -120. - saved_length;
            for active in [0., saved_length] {
                let (mut plan, mut result, mut state) = fixture(RoutineConfig::default());
                state.position[2] = -110.;
                state.work_position[2] = -110. - origin - active;
                state.tool_length_offset = active;
                state.probe_z_calibration = Some(-48.);
                state.probe_offset[2] = -48. - reference;
                plan.start = state.clone();
                result.axes = vec!["Z".into()];
                result.raw_point = [None, None, Some(-110.)];
                result.raw_spans = [None; 3];
                let options = CalibrationOptions {
                    offsets: [false, false, true],
                    ..Default::default()
                };
                let cal = calibrate_probe(&plan, &result, &state, 2., &options).unwrap();
                assert_eq!(cal.offsets[2], Some(-50.));
                assert!(cal.result.point[2].unwrap().abs() < 1e-9);
            }
        }
    }

    #[test]
    fn one_axis_center_preserves_unselected_offsets() {
        let (plan, mut result, state) = fixture(RoutineConfig {
            family: "center".into(),
            feature: "x-ridge".into(),
            x: 1,
            y: 0,
            z: false,
            ..Default::default()
        });
        result.axes = vec!["X".into()];
        result.raw_point[1] = None;
        result.raw_spans = [None; 3];
        let options = CalibrationOptions {
            offsets: [true, false, false],
            ..Default::default()
        };
        let cal = calibrate_probe(&plan, &result, &state, 3., &options).unwrap();
        assert_eq!(cal.offsets, [Some(1.), None, None]);
        assert!(cal.result.point[0].unwrap().abs() < 1e-10);
        let preview =
            calibrate_probe(&plan, &result, &state, 3., &CalibrationOptions::default()).unwrap();
        assert_eq!(preview.proposed_offsets[0], cal.offsets[0]);
        assert_eq!(preview.offsets, [None; 3]);
        let mut rotated = plan.clone();
        rotated.start.wcs_rotation = Some(30.);
        assert!(calibrate_probe(&rotated, &result, &state, 3., &options).is_err());
        result.reference_used = true;
        assert!(calibrate_probe(&plan, &result, &state, 3., &options).is_err());
    }

    #[test]
    fn xy_offsets_require_opposing_contacts_even_with_a_known_ball() {
        for family in ["outside", "inside"] {
            for y in [0, 1] {
                let (plan, mut result, state) = fixture(RoutineConfig {
                    family: family.into(),
                    x: 1,
                    y,
                    z: false,
                    ..Default::default()
                });
                result.raw_spans = [None; 3];
                let options = CalibrationOptions {
                    offsets: [true, y != 0, false],
                    ..Default::default()
                };
                assert!(calibrate_probe(&plan, &result, &state, 1.83, &options).is_err());
            }
        }
    }

    #[test]
    fn invalid_diameter_is_rejected_and_zeroed_result_can_still_calibrate_ball() {
        let (plan, mut result, state) = fixture(RoutineConfig {
            family: "center".into(),
            feature: "hole".into(),
            ..Default::default()
        });
        result.reference_used = true;
        let mut options = CalibrationOptions {
            ball: true,
            dimensions: [Some(7.), None],
            ..Default::default()
        };
        assert!(calibrate_probe(&plan, &result, &state, 2., &options).is_err());
        options.dimensions[0] = Some(10.);
        let cal = calibrate_probe(&plan, &result, &state, 2., &options).unwrap();
        assert_eq!(cal.diameter, Some(2.));
        assert_eq!(cal.offset_axes, [false; 3]);
    }
}
