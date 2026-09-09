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

use crate::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutineConfig {
    pub family: String,
    #[serde(default)]
    pub feature: String,
    pub x: i32,
    pub y: i32,
    pub z: bool,
    pub wcs: i32,
    pub zero: bool,
    pub safe_z_offset: f64,
    pub depth: f64,
    pub x_search_distance: f64,
    pub y_search_distance: f64,
    pub retract: f64,
    pub diameter: f64,
    pub positioning_feed: f64,
    pub coarse_feed: f64,
    pub fine_feed: f64,
}
impl Default for RoutineConfig {
    fn default() -> Self {
        Self {
            family: "outside".into(),
            feature: String::new(),
            x: 0,
            y: 0,
            z: true,
            wcs: 54,
            zero: false,
            safe_z_offset: 40.0,
            depth: 5.0,
            x_search_distance: 10.0,
            y_search_distance: 10.0,
            retract: 0.5,
            diameter: 2.0,
            positioning_feed: 1000.0,
            coarse_feed: 300.0,
            fine_feed: 50.0,
        }
    }
}
impl RoutineConfig {
    pub fn validate(&self) -> Result<(), Error> {
        if !matches!(self.family.as_str(), "inside" | "outside" | "center")
            || !(-1..=1).contains(&self.x)
            || !(-1..=1).contains(&self.y)
            || self.z == (self.x != 0 || self.y != 0)
        {
            return Err(Error::InvalidConfig("invalid feature selection".into()));
        }
        if !(54..=59).contains(&self.wcs) {
            return Err(Error::InvalidConfig("invalid WCS".into()));
        }
        if self.family == "center" {
            let expected = match self.feature.as_str() {
                "boss" | "block" | "hole" | "pocket" => (1, 1, false),
                "x-ridge" | "x-valley" => (1, 0, false),
                "y-ridge" | "y-valley" => (0, 1, false),
                "z" => (0, 0, true),
                _ => return Err(Error::InvalidConfig("unknown center feature".into())),
            };
            if (self.x, self.y, self.z) != expected {
                return Err(Error::InvalidConfig("invalid center selection".into()));
            }
        }
        for (name, value, parameter) in [
            ("depth", self.depth, Parameter::Travel),
            ("safe Z offset", self.safe_z_offset, Parameter::Travel),
            (
                "X search distance",
                self.x_search_distance,
                Parameter::SearchDistance,
            ),
            (
                "Y search distance",
                self.y_search_distance,
                Parameter::SearchDistance,
            ),
            ("retract", self.retract, Parameter::Retract),
            ("diameter", self.diameter, Parameter::Diameter),
            (
                "positioning feed",
                self.positioning_feed,
                Parameter::PositioningFeed,
            ),
            ("coarse feed", self.coarse_feed, Parameter::ProbeFeed),
            ("fine feed", self.fine_feed, Parameter::ProbeFeed),
        ] {
            if !parameter.range().contains(value) {
                return Err(Error::InvalidConfig(format!("invalid {name}")));
            }
        }
        Ok(())
    }
    pub fn center_internal(&self) -> bool {
        matches!(
            self.feature.as_str(),
            "hole" | "pocket" | "x-valley" | "y-valley"
        )
    }
    pub fn side_depth(&self) -> f64 {
        if self.family == "outside" || (self.family == "center" && !self.center_internal()) {
            self.depth
        } else {
            0.0
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RoutineStep {
    Contact {
        axis: Axis,
        config: ContactConfig,
        limit: f64,
        measurement: String,
    },
    Move {
        targets: Vec<MoveTarget>,
    },
    Center {
        axis: Axis,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveTarget {
    pub axis: Axis,
    pub reference: String,
    pub offset: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RoutinePlan {
    pub config: RoutineConfig,
    pub start: State,
    pub steps: Vec<RoutineStep>,
}
pub fn review(state: State, config: RoutineConfig) -> Result<RoutinePlan, Error> {
    preflight(&state)?;
    config.validate()?;
    if !state.modes.valid()
        || !finite(state.work_position)
        || !finite(state.probe_offset)
        || state.wcs != config.wcs
    {
        return Err(Error::Preflight(
            "invalid parser modes or coordinate system".into(),
        ));
    }
    let mut p = RoutinePlan {
        config,
        start: state,
        steps: Vec::new(),
    };
    let c = p.config.clone();
    if c.z {
        p.contact(Axis::Z, -1, c.depth, -c.depth, "z");
        p.mov(Axis::Z, "start_z", 0.0);
    } else if c.family == "center" {
        p.center();
    } else if c.family == "outside" {
        for (a, d, travel) in [
            (Axis::X, c.x, c.x_search_distance),
            (Axis::Y, c.y, c.y_search_distance),
        ] {
            if d == 0 {
                continue;
            }
            if a == Axis::Y && c.x != 0 {
                p.mov(Axis::X, "start_x", 0.0);
            }
            p.mov(a, &format!("start_{}", a.name()), -f64::from(d) * travel);
            p.mov(Axis::Z, "start_z", -c.depth);
            p.contact(a, d, travel, 0.0, a.name());
            if a == Axis::X && c.y != 0 {
                p.mov(Axis::Z, "start_z", 0.0);
            }
        }
    } else {
        if c.x != 0 {
            p.contact(
                Axis::X,
                c.x,
                c.x_search_distance,
                f64::from(c.x) * c.x_search_distance,
                "x",
            );
        }
        if c.y != 0 {
            if c.x != 0 {
                p.mov(Axis::X, "start_x", 0.0);
            }
            p.contact(
                Axis::Y,
                c.y,
                c.y_search_distance,
                f64::from(c.y) * c.y_search_distance,
                "y",
            );
        }
    }
    if !c.z {
        if c.family == "inside" {
            let targets = p
                .axes()
                .into_iter()
                .map(|axis| MoveTarget {
                    axis,
                    reference: format!("start_{}", axis.name()),
                    offset: 0.0,
                })
                .collect();
            p.steps.push(RoutineStep::Move { targets });
        } else {
            if !c.center_internal() {
                p.mov(Axis::Z, "start_z", 0.0);
            }
            let targets = p
                .axes()
                .into_iter()
                .map(|axis| MoveTarget {
                    axis,
                    reference: format!("surface_{}", axis.name()),
                    offset: -p.start.probe_offset[axis.index()],
                })
                .collect();
            p.steps.push(RoutineStep::Move { targets });
        }
    }
    p.validate_envelope()?;
    Ok(p)
}
pub fn plan_routine(state: State, config: RoutineConfig) -> Result<RoutinePlan, Error> {
    review(state, config)
}
impl RoutinePlan {
    pub fn axes(&self) -> Vec<Axis> {
        if self.config.z {
            vec![Axis::Z]
        } else {
            [(Axis::X, self.config.x), (Axis::Y, self.config.y)]
                .into_iter()
                .filter_map(|(a, d)| (d != 0).then_some(a))
                .collect()
        }
    }
    fn mov(&mut self, axis: Axis, target: &str, offset: f64) {
        self.steps.push(RoutineStep::Move {
            targets: vec![MoveTarget {
                axis,
                reference: target.into(),
                offset,
            }],
        });
    }
    fn contact(
        &mut self,
        axis: Axis,
        direction: i32,
        coarse_travel: f64,
        end_offset: f64,
        measurement: &str,
    ) {
        let c = &self.config;
        self.steps.push(RoutineStep::Contact {
            axis,
            limit: self.start.position[axis.index()] + end_offset,
            measurement: measurement.into(),
            config: ContactConfig {
                axis,
                direction,
                coarse_travel,
                retract_distance: c.retract,
                retract_feed: c.positioning_feed,
                coarse_feed: c.coarse_feed,
                fine_feed: c.fine_feed,
            },
        });
    }
    fn center(&mut self) {
        let c = self.config.clone();
        let internal = c.center_internal();
        for (axis, enabled) in [(Axis::X, c.x), (Axis::Y, c.y)] {
            if enabled == 0 {
                continue;
            }
            let distance = if axis == Axis::X {
                c.x_search_distance
            } else {
                c.y_search_distance
            };
            for side in [-1, 1] {
                let (mut direction, mut travel, mut end_offset) =
                    (side, 2.0 * distance, f64::from(side) * distance);
                if !internal {
                    self.mov(
                        axis,
                        &format!("start_{}", axis.name()),
                        f64::from(side) * distance,
                    );
                    self.mov(Axis::Z, "start_z", -c.depth);
                    direction = -side;
                    travel = distance;
                    end_offset = 0.0;
                }
                self.contact(
                    axis,
                    direction,
                    travel,
                    end_offset,
                    &format!("{}_{}", axis.name(), if side < 0 { "low" } else { "high" }),
                );
                if !internal && (side < 0 || (axis == Axis::X && c.y != 0)) {
                    self.mov(Axis::Z, "start_z", 0.0);
                }
            }
            self.steps.push(RoutineStep::Center { axis });
            if axis == Axis::X && c.y != 0 {
                self.mov(
                    axis,
                    &format!("surface_{}", axis.name()),
                    -self.start.probe_offset[axis.index()],
                );
            }
        }
    }
    pub(crate) fn check_state(&self, s: &State, position: Position) -> Result<(), Error> {
        preflight(s)?;
        if s.wcs != self.start.wcs
            || s.tool != self.start.tool
            || !within(s.position, position, 0.05)
            || !within(s.probe_offset, self.start.probe_offset, 0.0005)
        {
            return Err(Error::Preflight(
                "machine state changed since review".into(),
            ));
        }
        if !finite(s.work_position)
            || (0..3).any(|i| {
                ((s.position[i] - s.work_position[i])
                    - (self.start.position[i] - self.start.work_position[i]))
                    .abs()
                    > 0.05
            })
        {
            return Err(Error::Preflight("work offset changed since review".into()));
        }
        Ok(())
    }
    pub fn validate_envelope(&self) -> Result<(), Error> {
        let (mut lo, mut hi) = (self.start.position, self.start.position);
        let mut refs = BTreeMap::<String, [f64; 2]>::new();
        for a in [Axis::X, Axis::Y, Axis::Z] {
            refs.insert(
                format!("start_{}", a.name()),
                [lo[a.index()], hi[a.index()]],
            );
        }
        for step in &self.steps {
            match step {
                RoutineStep::Center { axis } => {
                    let n = axis.name();
                    let l = refs[&format!("surface_{n}_low")];
                    let h = refs[&format!("surface_{n}_high")];
                    refs.insert(
                        format!("surface_{n}"),
                        [(l[0] + h[0]) / 2.0, (l[1] + h[1]) / 2.0],
                    );
                }
                RoutineStep::Contact {
                    axis,
                    config,
                    limit,
                    measurement,
                } => {
                    let i = axis.index();
                    let d = f64::from(config.direction);
                    let (cl, ch) = if d > 0.0 {
                        (lo[i] - self.config.retract, limit + 0.5)
                    } else {
                        (limit - 0.5, hi[i] + self.config.retract)
                    };
                    check_path(
                        &self.start,
                        *axis,
                        lo[i].min(cl).min(cl - d * self.config.retract),
                        hi[i].max(ch).max(ch - d * self.config.retract),
                    )?;
                    let off = self.start.probe_offset[i] + d * self.config.diameter / 2.0;
                    refs.insert(format!("surface_{measurement}"), [cl + off, ch + off]);
                    lo[i] = cl - d * self.config.retract;
                    hi[i] = ch - d * self.config.retract;
                }
                RoutineStep::Move { targets } => {
                    for MoveTarget {
                        axis,
                        reference: target,
                        offset,
                    } in targets
                    {
                        let i = axis.index();
                        let r = if target == &format!("current_{}", axis.name()) {
                            [lo[i], hi[i]]
                        } else {
                            *refs.get(target).ok_or_else(|| {
                                Error::InvalidConfig(format!("missing target {target}"))
                            })?
                        };
                        let l = r[0] + offset;
                        let h = r[1] + offset;
                        check_path(&self.start, *axis, lo[i].min(l), hi[i].max(h))?;
                        lo[i] = l;
                        hi[i] = h;
                    }
                }
            }
        }
        Ok(())
    }
}
