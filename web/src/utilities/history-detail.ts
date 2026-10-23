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

import type { HistoryEntry } from "../types";
import type { RepeatabilityReport } from "./repeatability-model";
import { measurementNumber } from "./repeatability-model";

type Translate = (message: string, values?: (string | number)[]) => string;
type Config = {
  options?: {
    repetitions?: number;
    home?: boolean;
    jogHome?: boolean;
    retract?: boolean;
  };
  operation?: string;
  wcs?: number;
  diameter?: number;
  depth?: number;
  z?: boolean;
  yDistance?: number;
  zDistance?: number;
  rodDiameter?: number;
  xDistance?: number;
  rotaryFeed?: number;
  xSearchDistance?: number;
  ySearchDistance?: number;
  positioningFeed?: number;
  coarseFeed?: number;
  fineFeed?: number;
};
type Result = Partial<RepeatabilityReport> & {
  machinePoint?: (number | null)[];
  spans?: (number | null)[];
  wcs?: number;
  angle?: { degrees: number; difference: number; spacing: number };
  stations?: { center: number[] }[];
  xyAngle?: number;
  xzAngle?: number;
  zeroed?: boolean;
  rotationApplied?: boolean;
  level?: {
    operation?: string;
    initialTouches?: number[][];
    touches?: number[][];
    correction?: number;
    residual?: number;
  };
};

export function historyDate(timestamp: number): string {
  const date = new Date(Number(timestamp));
  if (!Number.isFinite(date.getTime())) return "--";
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

export function canOpenHistory(entry: HistoryEntry | null): boolean {
  return (
    !!entry?.result &&
    (entry.category === "rotary" || "machinePoint" in entry.result)
  );
}

export function historyDetail(entry: HistoryEntry, t: Translate): string {
  const n = (value: number | null | undefined) => measurementNumber(value, 3);
  const pointText = (point: (number | null)[]) =>
    ["X", "Y", "Z"]
      .flatMap((axis, index) =>
        point[index] == null ? [] : [`${axis} ${n(point[index])}`],
      )
      .join("   ");
  const config = entry.config as Config;
  let result = (entry.result ?? {}) as Result;
  const lines = [
    historyDate(entry.timestampMs),
    t("Status: %1", [t(entry.status)]),
  ];
  if (entry.error) lines.push(t("Error: %1", [t(entry.error)]));

  if (entry.category === "repeatability") {
    const options = config.options ?? {};
    lines.push(
      t("Repetitions: %1   Home: %2   Retract: %3", [
        options.repetitions ?? 0,
        t(options.home ? "yes" : "no"),
        t(options.retract ? "yes" : "no"),
      ]),
      "",
      t("Readings (mm)"),
    );
    (result.measurements ?? []).forEach((reading, index) =>
      lines.push(`${index + 1}: ${pointText(reading)}`),
    );
    (result.statistics ?? []).forEach((stats, index) => {
      if (stats)
        lines.push(
          t("%1 mean %2   median %3   SD %4", [
            ["X", "Y", "Z"][index],
            n(stats.mean),
            n(stats.median),
            stats.stddev === null ? "-" : n(stats.stddev),
          ]),
        );
    });
  } else if (entry.category === "rotary") {
    for (const action of entry.actions ?? []) {
      if (action.data?.result) result = action.data.result as Result;
    }
    if (
      result.level ||
      ["horizontal", "vertical", "verticalNegative"].includes(
        config.operation ?? "",
      )
    ) {
      const level = result.level ?? {};
      const axes =
        (level.operation ?? config.operation) === "horizontal" ? "A/Z" : "A/Y";
      for (const key of ["initialTouches", "touches"] as const) {
        lines.push(
          "",
          t(
            key === "touches"
              ? "Verified touches · G53"
              : "Initial touches · G53",
          ),
        );
        (level[key] ?? []).forEach((point, index) =>
          lines.push(t("Touch %1: %2", [index + 1, pointText(point)])),
        );
      }
      if (result.level)
        lines.push(
          t("A correction %1°", [measurementNumber(level.correction)]),
          t("Remaining tilt %1°", [measurementNumber(level.residual)]),
          t("%1 zero: %2", [axes, t(result.zeroed ? "saved" : "unchanged")]),
        );
      lines.push(
        "",
        t("G%1   Y distance %2 mm", [config.wcs ?? "--", n(config.yDistance)]),
        t("Z distance %1 mm", [n(config.zDistance)]),
      );
    } else {
      lines.push("", t("Axis centers · G53 (mm)"));
      (result.stations ?? []).forEach((station, index) =>
        lines.push(
          t("Axis center %1: %2", [index + 1, pointText(station.center)]),
        ),
      );
      if (result.stations?.length === 2)
        lines.push(
          t("Axis angle in X/Y %1°", [measurementNumber(result.xyAngle)]),
          t("Axis angle in X/Z %1°", [measurementNumber(result.xzAngle)]),
          t("Y/Z zero: %1", [t(result.zeroed ? "saved" : "unchanged")]),
          t("X/Y rotation: %1", [
            t(result.rotationApplied ? "saved" : "unchanged"),
          ]),
        );
      lines.push(
        "",
        t("G%1   rod %2 mm", [config.wcs ?? "--", n(config.rodDiameter)]),
        t("X distance %1 mm", [n(config.xDistance)]),
      );
    }
    lines.push(t("Rotary feed %1°/min", [n(config.rotaryFeed)]));
  } else {
    if (result.angle)
      lines.push(
        "",
        `${t("Measured angle")} ${measurementNumber(result.angle.degrees)}°`,
        t("Change %1 mm over %2 mm", [
          n(result.angle.difference),
          n(result.angle.spacing),
        ]),
      );
    const point = result.machinePoint ?? [];
    if (point.some((value) => value != null)) {
      lines.push("", t("Measured G53 (mm)"));
      point.slice(0, 3).forEach((value, axis) => {
        if (value != null) lines.push(`${["X", "Y", "Z"][axis]}  ${n(value)}`);
      });
    }
    (result.spans ?? []).slice(0, 2).forEach((span, axis) => {
      if (span != null)
        lines.push(t("Span %1  %2 mm", [["X", "Y"][axis], n(span)]));
    });
    if (entry.workZero) {
      lines.push(
        "",
        t("Work zero set in G%1", [
          entry.workZero.result?.wcs ?? result.wcs ?? "--",
        ]),
      );
      point.slice(0, 3).forEach((value, axis) => {
        if (value != null)
          lines.push(
            t("%1 offset %2 mm", [
              ["X", "Y", "Z"][axis],
              n(entry.workZero?.offsets?.[axis] ?? 0),
            ]),
          );
      });
    }
    lines.push(
      "",
      t("Settings"),
      t("G%1   ball %2 mm   depth %3 mm", [
        config.wcs ?? "--",
        n(config.diameter),
        n(config.depth),
      ]),
    );
    if (!config.z)
      lines.push(
        t("X search %1 mm   Y search %2 mm", [
          n(config.xSearchDistance),
          n(config.ySearchDistance),
        ]),
      );
    lines.push(
      t("Feeds: %1 / %2 / %3 mm/min", [
        n(config.positioningFeed),
        n(config.coarseFeed),
        n(config.fineFeed),
      ]),
    );
  }
  return lines.join("\n");
}
