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

use pimprobe_core::{Axis, AxisStatistics, Progress, RepeatabilityReport, RoutineResult};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MeasurementResult {
    Routine(RoutineResult),
    Repeatability(RepeatabilityReport),
    Rotary(pimprobe_core::RotaryResult),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OperationEvent {
    Progress {
        #[serde(rename = "elapsedMs", skip_serializing_if = "Option::is_none")]
        elapsed_ms: Option<u64>,
        progress: Progress,
    },
    Measurement {
        repetition: usize,
        axis: Axis,
        value: f64,
        statistics: [Option<AxisStatistics>; 3],
    },
    Result {
        result: MeasurementResult,
    },
    Error {
        code: String,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<MeasurementResult>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamped_progress_round_trips_through_json() {
        let event = OperationEvent::Progress {
            elapsed_ms: Some(1250),
            progress: Progress::default(),
        };
        let encoded = serde_json::to_string(&event).unwrap();
        let decoded: OperationEvent = serde_json::from_str(&encoded).unwrap();
        assert!(matches!(
            decoded,
            OperationEvent::Progress {
                elapsed_ms: Some(1250),
                ..
            }
        ));
    }
}
