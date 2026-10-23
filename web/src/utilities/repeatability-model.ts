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

export type Axes<T> = [T, T, T];
export type Reading = Axes<number | null>;
export type AxisStatistics = {
  count: number;
  mean: number;
  median: number;
  stddev: number | null;
  range: number;
};
export type RepeatabilityReport = {
  measurements: Reading[];
  statistics: Axes<AxisStatistics | null>;
};
export type RepeatabilityOptions = {
  axes: Axes<boolean>;
  repetitions: number;
  jogHome: boolean;
  home: boolean;
  retract: boolean;
};
export type RepeatabilityEvent =
  | { type: "progress"; progress: { kind: string; message?: string } }
  | {
      type: "measurement";
      repetition: number;
      axis: "X" | "Y" | "Z";
      value: number;
      statistics?: RepeatabilityReport["statistics"];
    }
  | { type: "result"; result: RepeatabilityReport }
  | {
      type: "error";
      code?: string;
      message: string;
      result?: RepeatabilityReport;
    };

export function runOptions(
  options: RepeatabilityOptions,
): RepeatabilityOptions {
  return {
    ...options,
    axes: [...options.axes],
    jogHome: options.jogHome || options.home,
    retract: options.retract || options.jogHome || options.home,
  };
}

export function validOptions(options: RepeatabilityOptions): boolean {
  return (
    options.axes.some(Boolean) &&
    Number.isInteger(options.repetitions) &&
    options.repetitions >= 1 &&
    options.repetitions <= 100
  );
}

export function recordMeasurement(
  report: RepeatabilityReport,
  event: Extract<RepeatabilityEvent, { type: "measurement" }>,
): RepeatabilityReport {
  if (
    !Number.isInteger(event.repetition) ||
    event.repetition < 1 ||
    event.repetition > 100
  )
    return report;
  const axis = ["X", "Y", "Z"].indexOf(event.axis);
  if (axis < 0 || !Number.isFinite(event.value)) return report;
  const measurements = report.measurements.map((row): Reading => [...row]);
  while (measurements.length < event.repetition)
    measurements.push([null, null, null]);
  measurements[event.repetition - 1][axis] = event.value;
  return { measurements, statistics: event.statistics ?? report.statistics };
}

export function readingValue(
  value: number | null,
  statistics: AxisStatistics | null,
  running: boolean,
): number | null {
  return value === null || running || !statistics
    ? value
    : value - statistics.mean;
}

export function measurementNumber(
  value: number | null | undefined,
  digits = 4,
): string {
  return value == null || !Number.isFinite(value)
    ? "--"
    : value.toFixed(digits);
}
