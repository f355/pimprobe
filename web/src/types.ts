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

export type Point = [number | null, number | null, number | null];
export type Family = "outside" | "inside" | "center" | "angle";
export type Settings = Record<string, number | string | boolean>;
export type Schema = Record<
  string,
  { minimum?: number; maximum?: number; default?: number | string | boolean }
>;
export interface Coordinates {
  machinePosition: number[];
  workPosition: number[];
}
export interface MachineState {
  connected: boolean;
  contactActive?: boolean;
  recoveryFailed?: boolean;
  actuatorPending?: boolean;
  coordinates?: { probe?: Coordinates; spindle?: Coordinates };
  wcsRotations?: Record<string, number>;
  status?: {
    mode?: string;
    wcs?: number;
    probeActuatorKnown?: boolean;
    probeActuator?: number;
  };
}
export interface RoutineConfig {
  family: Family;
  feature?: string;
  wcs: number;
  x: number;
  y: number;
  z: boolean;
  zero: boolean;
  safeZOffset: number;
  depth: number;
  xSearchDistance: number;
  ySearchDistance: number;
  retract: number;
  diameter: number;
  positioningFeed: number;
  coarseFeed: number;
  fineFeed: number;
}
export interface RotaryConfig {
  operation: string;
  yDistance: number;
  zDistance: number;
  rodDiameter: number;
  xDistance: number;
  rotaryFeed: number;
  diameter: number;
  retract: number;
  positioningFeed: number;
  coarseFeed: number;
  fineFeed: number;
}
export type RunConfig = RoutineConfig | RotaryConfig;
export interface AngleMeasurement {
  degrees: number;
  difference: number;
  spacing: number;
  rotationSupported: boolean;
  touches: number[][];
}
export interface RoutineResult {
  point: Point;
  machinePoint: Point;
  rawPoint: Point;
  spans: Point;
  rawSpans: Point;
  wcs: number;
  zeroed: boolean;
  referenceUsed?: boolean;
  measurementRotation?: number;
  returned?: boolean;
  positioned?: boolean;
  angle?: AngleMeasurement;
}
export interface CalibrationOptions {
  ball: boolean;
  dimensions: [number | null, number | null];
  offsets: [boolean, boolean, boolean];
}
export interface Calibration {
  currentDiameter: number;
  currentOffsets: number[];
  ballAxes: boolean[];
  ballEstimates: (number | null)[];
  offsetAxes: boolean[];
  proposedDiameter: number | null;
  proposedOffsets: Point;
  diameter: number | null;
  offsets: Point;
  result: RoutineResult;
  warning?: string;
}
export interface SavedProbeCalibration {
  diameter: number;
  offsets: [number, number, number];
}
export interface RotaryResult {
  stations: { center: number[] }[];
  xyAngle: number;
  xzAngle: number;
  wcs: number;
  rotationSupported: boolean;
  zeroed: boolean;
  rotationApplied: boolean;
  level?: {
    touches: number[][];
    initialTouches?: number[][];
    correction: number;
    residual: number;
  };
}
export interface Review<C = RunConfig> {
  id: string;
  config: C;
  program: string[] | string;
  simulated: boolean;
}
export type MotionEvent<T = RoutineResult | RotaryResult> =
  | { type: "progress"; progress: { kind: string; message: string } }
  | { type: "result"; result: T }
  | { type: "error"; code: string; message: string; result?: T };
export interface HistoryEntry {
  id: string;
  timestampMs: number;
  category: string;
  label: string;
  status: string;
  config: RunConfig | Record<string, unknown>;
  result?: RoutineResult | RotaryResult | Record<string, unknown>;
  error?: string;
  workZero?: { offsets?: number[]; result?: RoutineResult };
  actions?: { action: string; data: Record<string, unknown> }[];
}
export interface HistoryResult {
  entry: HistoryEntry;
  result: RoutineResult | RotaryResult;
  canApply: boolean;
}
