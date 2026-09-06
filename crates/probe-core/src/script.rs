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

pub(crate) type Values = BTreeMap<String, f64>;
#[derive(Clone, Default)]
pub(crate) struct Expr {
    constant: f64,
    terms: BTreeMap<String, f64>,
}
impl Expr {
    fn constant(v: f64) -> Self {
        Self {
            constant: v,
            ..Self::default()
        }
    }
    fn variable(n: String) -> Self {
        Self {
            constant: 0.0,
            terms: BTreeMap::from([(n, 1.0)]),
        }
    }
    fn add(mut self, v: f64) -> Self {
        self.constant += v;
        self
    }
    fn sub(mut self, other: Self) -> Self {
        self.constant -= other.constant;
        for (n, v) in other.terms {
            *self.terms.entry(n).or_default() -= v;
        }
        self.terms.retain(|_, v| *v != 0.0);
        self
    }
    fn avg(mut self, other: Self) -> Self {
        self.constant = (self.constant + other.constant) / 2.0;
        for (n, v) in other.terms {
            *self.terms.entry(n).or_default() += v;
        }
        for v in self.terms.values_mut() {
            *v /= 2.0;
        }
        self
    }
    fn render(&self, values: Option<&Values>) -> Result<String, Error> {
        if let Some(values) = values {
            let mut v = self.constant;
            for (n, k) in &self.terms {
                v += k * values
                    .get(n)
                    .ok_or_else(|| Error::Compensation(format!("missing script value {n}")))?;
            }
            return Ok(number(v));
        }
        if self.terms.is_empty() {
            return Ok(number(self.constant));
        }
        let mut text = if quantize(self.constant) == 0.0 {
            String::new()
        } else {
            number(self.constant)
        };
        for (n, k) in &self.terms {
            if !text.is_empty() {
                text.push_str(if *k < 0.0 { " - " } else { " + " });
            } else if *k < 0.0 {
                text.push('-');
            }
            if k.abs() != 1.0 {
                text.push_str(&format!("{} * ", number(k.abs())));
            }
            text.push_str(&format!("#<{n}>"));
        }
        Ok(format!("[{text}]"))
    }
}
pub(crate) fn number(v: f64) -> String {
    let v = quantize(v);
    if v == 0.0 {
        return "0".into();
    }
    format!("{v:.3}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .into()
}
pub(crate) fn trim_command(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            if w.contains('.') {
                w.trim_end_matches('0').trim_end_matches('.')
            } else {
                w
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
pub(crate) enum LineText {
    Literal(String),
    Command {
        prefix: String,
        words: Vec<(Axis, Expr)>,
        suffix: String,
    },
    Declaration {
        name: String,
        description: String,
    },
}
pub(crate) struct Line {
    pub step: isize,
    text: LineText,
}
impl Line {
    pub fn is_command(&self) -> bool {
        match &self.text {
            LineText::Command { .. } => true,
            LineText::Literal(s) => !s.starts_with(';'),
            LineText::Declaration { .. } => false,
        }
    }
    pub fn is_declaration(&self) -> bool {
        matches!(self.text, LineText::Declaration { .. })
    }
    pub fn render(&self, values: Option<&Values>) -> Result<String, Error> {
        match &self.text {
            LineText::Literal(s) => Ok(s.clone()),
            LineText::Declaration { name, description } => Ok(format!(
                "; #<{name}> := {}",
                if let Some(v) = values {
                    number(
                        *v.get(name)
                            .ok_or_else(|| Error::Compensation(format!("missing {name}")))?,
                    )
                } else {
                    description.clone()
                }
            )),
            LineText::Command {
                prefix,
                words,
                suffix,
            } => {
                let words = words
                    .iter()
                    .map(|(a, v)| Ok(format!("{a}{}", v.render(values)?)))
                    .collect::<Result<Vec<_>, Error>>()?;
                Ok(format!("{prefix} {}{suffix}", words.join(" ")))
            }
        }
    }
}
impl RoutinePlan {
    pub fn program(&self) -> Vec<String> {
        self.script()
            .iter()
            .map(|l| l.render(None).expect("symbolic rendering is infallible"))
            .collect()
    }
    pub(crate) fn script(&self) -> Vec<Line> {
        let mut lines = vec![
            Line {
                step: -1,
                text: LineText::Literal(format!(
                    "; Probing {} {}",
                    self.config.family,
                    if self.config.feature.is_empty() {
                        "feature"
                    } else {
                        &self.config.feature
                    }
                )),
            },
            Line {
                step: -1,
                text: LineText::Literal(format!("{} ; probing modes", Modes::PROBING.command())),
            },
        ];
        let mut pos: Vec<Expr> = self.start.position[..3]
            .iter()
            .map(|v| Expr::constant(*v))
            .collect();
        let mut refs = BTreeMap::new();
        for a in [Axis::X, Axis::Y, Axis::Z] {
            refs.insert(format!("start_{}", a.name()), pos[a.index()].clone());
        }
        for (index, step) in self.steps.iter().enumerate() {
            let idx = index as isize;
            match step {
                RoutineStep::Center { axis } => {
                    let n = axis.name();
                    refs.insert(
                        format!("surface_{n}"),
                        refs[&format!("surface_{n}_low")]
                            .clone()
                            .avg(refs[&format!("surface_{n}_high")].clone()),
                    );
                    lines.push(Line {
                        step: idx,
                        text: LineText::Literal(format!(
                            "; Calculate {axis} center from opposing contacts"
                        )),
                    });
                }
                RoutineStep::Contact {
                    axis,
                    config,
                    limit,
                    measurement,
                } => {
                    lines.push(Line {
                        step: idx,
                        text: LineText::Literal(format!(
                            "; Measure toward {axis}{} and back off",
                            if config.direction < 0 { "-" } else { "+" }
                        )),
                    });
                    for s in plan_contact(*config).expect("validated contact").stages {
                        if matches!(s.kind, StageKind::CoarseProbe | StageKind::FineProbe) {
                            lines.push(Line {
                                step: idx,
                                text: LineText::Literal(format!(
                                    "; {}",
                                    if s.kind == StageKind::CoarseProbe {
                                        "Coarse"
                                    } else {
                                        "Fine"
                                    }
                                )),
                            });
                        }
                        lines.push(Line {
                            step: idx,
                            text: if s.kind == StageKind::CoarseProbe {
                                LineText::Command {
                                    prefix: "G38.3".into(),
                                    words: vec![(
                                        *axis,
                                        Expr::constant(*limit).sub(pos[axis.index()].clone()),
                                    )],
                                    suffix: format!(" F{}", number(config.coarse_feed)),
                                }
                            } else {
                                LineText::Literal(trim_command(&s.command))
                            },
                        });
                        if s.kind == StageKind::FineProbe {
                            lines.push(Line {
                                step: idx,
                                text: LineText::Declaration {
                                    name: format!("contact_{measurement}"),
                                    description: format!(
                                        "{axis} from fine-probe report (machine coordinates)"
                                    ),
                                },
                            });
                            if *axis == Axis::Z {
                                lines.push(Line {
                                    step: idx,
                                    text: LineText::Declaration {
                                        name: "probe_report_tool_value".into(),
                                        description: "trailing tool value from fine-probe report"
                                            .into(),
                                    },
                                });
                            }
                        }
                    }
                    let name = format!("{}_after_backoff", axis.name());
                    pos[axis.index()] = Expr::variable(name.clone());
                    lines.push(Line {
                        step: idx,
                        text: LineText::Declaration {
                            name,
                            description: format!("{axis} (machine coordinates)"),
                        },
                    });
                    let mut surface = Expr::variable(format!("contact_{measurement}"));
                    if *axis == Axis::Z {
                        surface.terms.insert("probe_report_tool_value".into(), 1.0);
                        surface = surface.add(-self.start.probe_offset[2]);
                    } else {
                        surface = surface.add(
                            self.start.probe_offset[axis.index()]
                                + f64::from(config.direction) * self.config.diameter / 2.0,
                        );
                    }
                    refs.insert(format!("surface_{measurement}"), surface);
                }
                RoutineStep::Move { targets } => {
                    let mut words = Vec::new();
                    for MoveTarget {
                        axis,
                        reference: target,
                        offset,
                    } in targets
                    {
                        let base = if target == &format!("current_{}", axis.name()) {
                            pos[axis.index()].clone()
                        } else {
                            refs[target].clone()
                        };
                        let destination = base.add(*offset);
                        let delta = destination.clone().sub(pos[axis.index()].clone());
                        if !delta.terms.is_empty() || quantize(delta.constant) != 0.0 {
                            words.push((*axis, delta));
                        }
                        pos[axis.index()] = destination;
                    }
                    if !words.is_empty() {
                        let lift = words.len() == 1
                            && words[0].0 == Axis::Z
                            && (words[0].1.terms.is_empty() && words[0].1.constant > 0.0
                                || self.config.z);
                        let axes = words
                            .iter()
                            .map(|(a, _)| a.to_string())
                            .collect::<Vec<_>>()
                            .join("/");
                        lines.push(Line {
                            step: idx,
                            text: LineText::Literal(if lift {
                                "; Raise Z".into()
                            } else {
                                format!("; Position {axes}, expecting no contact")
                            }),
                        });
                        lines.push(Line {
                            step: idx,
                            text: LineText::Command {
                                prefix: if lift { "G1" } else { "G38.3" }.into(),
                                words,
                                suffix: format!(" F{}", number(self.config.positioning_feed)),
                            },
                        });
                    }
                }
            }
        }
        if self.config.zero {
            let words = self
                .axes()
                .into_iter()
                .map(|a| {
                    (
                        a,
                        pos[a.index()]
                            .clone()
                            .sub(refs[&format!("surface_{}", a.name())].clone()),
                    )
                })
                .collect();
            lines.push(Line {
                step: self.steps.len() as isize,
                text: LineText::Command {
                    prefix: format!("G10 L20 P{}", self.config.wcs - 53),
                    words,
                    suffix: String::new(),
                },
            });
        }
        lines.push(Line {
            step: self.steps.len() as isize + 1,
            text: LineText::Literal(format!(
                "{} ; restore captured modes",
                self.start.modes.command()
            )),
        });
        lines
    }
}
