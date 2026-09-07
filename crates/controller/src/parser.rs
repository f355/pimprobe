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

use pimprobe_core::{Contact, Modes, Position};
use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineStatus {
    pub mode: String,
    #[serde(rename = "machinePosition")]
    pub m_pos: Position,
    #[serde(rename = "workPosition")]
    pub w_pos: Position,
    pub complete: bool,
    pub wcs: i32,
    pub tool: i32,
    pub spindle_mode: i32,
    pub probe_actuator: i32,
    pub probe_actuator_known: bool,
    pub probe_trigger_known: bool,
    pub probe_triggered: bool,
    pub door_open: bool,
    pub motion_blocked: bool,
}

#[derive(Clone, Debug)]
pub enum Record {
    Ack,
    Error(i32),
    Status(MachineStatus),
    Probe(Contact),
    Modes(Modes),
    Setting(i32, f64),
}

fn number<T: std::str::FromStr>(s: &str) -> Result<T, String> {
    s.trim().parse().map_err(|_| format!("invalid number: {s}"))
}
fn finite(s: &str) -> Result<f64, String> {
    let n: f64 = number(s)?;
    if n.is_finite() {
        Ok(n)
    } else {
        Err("non-finite number".into())
    }
}
fn position(s: &str) -> Result<Position, String> {
    let v = s.split(',').map(finite).collect::<Result<Vec<_>, _>>()?;
    v.try_into().map_err(|_| "expected four coordinates".into())
}

pub fn parse_line(line: &str) -> Result<Option<Record>, String> {
    let line = line.trim();
    let line = if let Some(report) = line.strip_prefix("[SerialReport:") {
        let (sequence, record) = report.split_once("] ").ok_or("invalid report prefix")?;
        number::<u64>(sequence)?;
        record
    } else {
        line
    };
    if line == "ok" {
        return Ok(Some(Record::Ack));
    }
    if let Some(s) = line.strip_prefix("error:") {
        return Ok(Some(Record::Error(number(s)?)));
    }
    if let Some(s) = line.strip_prefix("[GC:") {
        let s = s.strip_suffix(']').ok_or("missing closing bracket")?;
        let mut modes = Modes::default();
        for word in s.split_whitespace() {
            match word {
                "G20" | "G21" => modes.units = number(&word[1..])?,
                "G90" | "G91" => modes.distance = number(&word[1..])?,
                "G93" | "G94" => modes.feed = number(&word[1..])?,
                _ => (),
            }
        }
        return Ok(Some(Record::Modes(modes)));
    }
    if let Some(s) = line
        .strip_prefix("[PRB:")
        .or_else(|| line.strip_prefix("[PROBE:"))
    {
        let s = s.strip_suffix(']').ok_or("missing closing bracket")?;
        let (pos, result) = s.split_once(':').ok_or("missing probe result")?;
        let mut parts = result.split(',');
        let success = number::<i32>(parts.next().unwrap_or(""))? != 0;
        let tool_length = parts.next().map(finite).transpose()?;
        if parts.next().is_some() {
            return Err("too many probe result values".into());
        }
        return Ok(Some(Record::Probe(Contact {
            position: position(pos)?,
            success,
            tool_length,
        })));
    }
    if let Some(s) = line.strip_prefix('<') {
        let s = s.strip_suffix('>').ok_or("missing closing bracket")?;
        let mut parts = s.split('|');
        let mode = parts.next().unwrap_or("");
        if mode.is_empty() {
            return Err("missing state".into());
        }
        let mut status = MachineStatus {
            mode: mode.into(),
            ..Default::default()
        };
        let mut have_pos = false;
        let mut fields = 0u8;
        let mut compact_actuator = None;
        for field in parts {
            let Some((key, value)) = field.split_once(':') else {
                continue;
            };
            match key {
                "MS" => {
                    // The thirtieth compact field is the probe actuator state.
                    compact_actuator = value.as_bytes().get(29).and_then(|byte| {
                        (b'0'..=b'3').contains(byte).then(|| i32::from(byte - b'0'))
                    });
                }
                "MPos" => {
                    status.m_pos = position(value)?;
                    have_pos = true;
                }
                "WPos" => {
                    status.w_pos = position(value)?;
                    fields |= 1;
                }
                "T" => {
                    status.tool = number(value)?;
                    fields |= 2;
                }
                "M" => {
                    status.spindle_mode = number(value)?;
                    fields |= 4;
                }
                "G" => {
                    status.wcs = number(value)?;
                    fields |= 8;
                }
                "PM" => {
                    status.probe_actuator = number(value)?;
                    if !(0..=3).contains(&status.probe_actuator) {
                        return Err("invalid actuator state".into());
                    }
                    status.probe_actuator_known = true;
                }
                "Pn" => {
                    status.probe_trigger_known = true;
                    status.probe_triggered = value.contains('P');
                }
                "Abnormal" => {
                    let list = value
                        .trim()
                        .strip_prefix('[')
                        .and_then(|v| v.strip_suffix(']'))
                        .ok_or("invalid abnormal list")?;
                    if !list.trim().is_empty() {
                        for code in list.split(',') {
                            if number::<i32>(code)? == 106 {
                                status.door_open = true;
                            } else {
                                status.motion_blocked = true;
                            }
                        }
                    }
                }
                _ => (),
            }
        }
        if !have_pos {
            return Err("missing MPos".into());
        }
        if !status.probe_actuator_known {
            if let Some(actuator) = compact_actuator {
                status.probe_actuator = actuator;
                status.probe_actuator_known = true;
            }
        }
        status.complete = fields == 15 && (54..=59).contains(&status.wcs) && status.tool >= 0;
        if status.mode.starts_with("Alarm:") && !status.door_open {
            status.motion_blocked = true;
        }
        return Ok(Some(Record::Status(status)));
    }
    if let Some((key, value)) = line.strip_prefix('$').and_then(|s| s.split_once('=')) {
        if let Ok(key) = number::<i32>(key) {
            return Ok(Some(Record::Setting(key, finite(value)?)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numbered_reports_with_compact_actuator_state() {
        for actuator in 0..=3 {
            let mut compact = b"000000010010110011010000001010001001000001".to_vec();
            compact[29] = b'0' + actuator;
            let line = format!(
                "[SerialReport:180] <Ready|MPos:0,0,0,0|WPos:232.410,204.066,121.781,0|T:0|M:5|G:54|MS:{}|Abnormal:[]>",
                String::from_utf8(compact).unwrap()
            );
            let Some(Record::Status(status)) = parse_line(&line).unwrap() else {
                panic!("expected status");
            };
            assert!(status.complete && status.probe_actuator_known);
            assert_eq!(status.probe_actuator, i32::from(actuator));
        }
    }

    #[test]
    fn original_cnc_lab_protocol_fixtures() {
        let Some(Record::Status(s)) = parse_line("<Ready|MPos:-156.755,-100.852,0.000,0.000|WPos:3.480,-7.588,38.500,0.000|T:2|M:5|G:55|PM:1|Pn:P|MS:1100100100100100110100000010100000010000|Abnormal:[106]>").unwrap() else { panic!() };
        assert!(s.complete && s.door_open && !s.motion_blocked && s.probe_triggered);
        assert_eq!(s.m_pos, [-156.755, -100.852, 0.0, 0.0]);
        assert_eq!(s.w_pos, [3.480, -7.588, 38.500, 0.0]);
        assert_eq!((s.tool, s.spindle_mode, s.wcs), (2, 5, 55));
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["machinePosition"][0], -156.755);
        assert_eq!(json["workPosition"][2], 38.5);
        assert!(json.get("mPos").is_none() && json.get("wPos").is_none());
        let Some(Record::Status(s)) =
            parse_line("<Alarm:2|MPos:0,0,0,0|WPos:0,0,0,0|M:5|G:54|PM:0|Abnormal:[2,106]>")
                .unwrap()
        else {
            panic!()
        };
        assert!(s.motion_blocked && s.door_open && !s.complete);
        for (line, tool) in [
            ("[PRB:-104.686,-23.692,-102.073,0.000:1]", None),
            (
                "[PROBE:-104.686,-23.692,-102.073,0.000:1,-38.500]",
                Some(-38.5),
            ),
        ] {
            let Some(Record::Probe(p)) = parse_line(line).unwrap() else {
                panic!()
            };
            assert_eq!(p.position, [-104.686, -23.692, -102.073, 0.0]);
            assert!(p.success);
            assert_eq!(p.tool_length, tool);
        }
        for (line, expected) in [
            ("[GC:G0 G54 G17 G21 G90 G94 M5 M9 T0 F0 S0]", (21, 90, 94)),
            (
                "[GC:G38.2 G55 G17 G20 G91 G93 M5 M9 T0 F10 S0]",
                (20, 91, 93),
            ),
        ] {
            let Some(Record::Modes(m)) = parse_line(line).unwrap() else {
                panic!()
            };
            assert_eq!((m.units, m.distance, m.feed), expected);
        }
        assert!(parse_line("CNC_Lab started").unwrap().is_none());
    }
    #[test]
    fn records_and_safety() {
        let Some(Record::Status(s)) =
            parse_line("<Ready|MPos:1,2,3,4|PM:1|Pn:P|Abnormal:[106]|M:5|G:54>").unwrap()
        else {
            panic!()
        };
        assert!(s.door_open && !s.motion_blocked && s.probe_triggered);
        assert!(matches!(
            parse_line("error:9").unwrap(),
            Some(Record::Error(9))
        ));
        assert!(matches!(
            parse_line("[PROBE:1,2,3,4:1,5]").unwrap(),
            Some(Record::Probe(_))
        ));
        assert!(matches!(
            parse_line("$33=1.5").unwrap(),
            Some(Record::Setting(33, 1.5))
        ));
        let Some(Record::Modes(m)) = parse_line("[GC:G21 G91 G94]").unwrap() else {
            panic!()
        };
        assert_eq!((m.units, m.distance, m.feed), (21, 91, 94));
        assert!(parse_line("banner").unwrap().is_none());
    }
    #[test]
    fn rejects_malformed() {
        for s in [
            "<Ready>",
            "<Ready|MPos:1,2,3,4|PM:4>",
            "[PRB:1,2,3:1]",
            "$33=NaN",
            "error:no",
            "[GC:G21",
            "<Ready|MPos:NaN,2,3,4>",
        ] {
            assert!(parse_line(s).is_err(), "{s}");
        }
    }
}
