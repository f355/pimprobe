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

import { useEffect, useRef } from "react";
import { helpProps } from "../help/context";
import { useTranslation } from "../i18n";
import { measurementNumber, readingValue } from "./repeatability-model";
import type { RepeatabilityReport } from "./repeatability-model";

const summaries = [
  {
    label: "Mean G53",
    key: "mean",
    help: "Average of all measured machine coordinates for each axis, recalculated after every repetition.",
  },
  {
    label: "Median",
    key: "median",
    help: "Middle measured machine coordinate for each axis. With an even number of readings, this is the average of the two middle values.",
  },
  {
    label: "Std dev",
    key: "stddev",
    help: "Typical spread of the readings around their mean. A smaller value means more consistent touches.",
  },
  {
    label: "Range",
    key: "range",
    help: "Difference between the highest and lowest reading for each axis.",
  },
] as const;

export function RepeatabilityReadings({
  report,
  running,
}: {
  report: RepeatabilityReport;
  running: boolean;
}) {
  const { t } = useTranslation();
  const scroll = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (running && scroll.current)
      scroll.current.scrollTop = scroll.current.scrollHeight;
  }, [report.measurements, running]);

  return (
    <div className="pp-utility-readings">
      <div
        className="pp-utility-reading-title"
        {...helpProps(
          t("Readings"),
          t(
            "Each row is one repetition. During the check, readings use G53; afterward, each value shows its difference from that axis's mean.",
          ),
        )}
      >
        {t(running ? "G53 measurements (mm)" : "Deviation from mean (mm)")}
      </div>
      <div
        className="pp-utility-reading-grid pp-utility-reading-header"
        aria-hidden="true"
      >
        <span>{t("Run")}</span>
        {["X", "Y", "Z"].map((axis) => (
          <span key={axis}>{axis}</span>
        ))}
      </div>
      <div
        className="pp-utility-reading-scroll"
        ref={scroll}
        tabIndex={0}
        role="region"
        aria-label={t("Readings")}
      >
        <table
          className="pp-utility-reading-table"
          aria-label={t(
            running ? "G53 measurements (mm)" : "Deviation from mean (mm)",
          )}
        >
          <colgroup>
            <col />
            {["X", "Y", "Z"].map((axis) => (
              <col key={axis} />
            ))}
          </colgroup>
          <thead className="pp-utility-sr-only">
            <tr>
              <th>{t("Run")}</th>
              {["X", "Y", "Z"].map((axis) => (
                <th key={axis}>{axis}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {report.measurements.map((row, index) => (
              <tr key={index}>
                <th scope="row">{index + 1}</th>
                {row.map((value, axis) => (
                  <td key={axis}>
                    {measurementNumber(
                      readingValue(value, report.statistics[axis], running),
                    )}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div
        className="pp-utility-statistics"
        role="group"
        aria-label={t("Statistics")}
      >
        {summaries.map((summary) => (
          <div
            className="pp-utility-reading-grid"
            key={summary.key}
            {...helpProps(t(summary.label), t(summary.help))}
          >
            <span>{t(summary.label)}</span>
            {report.statistics.map((stats, axis) => (
              <span
                key={axis}
                aria-label={`${["X", "Y", "Z"][axis]} ${t(summary.label)}`}
              >
                {measurementNumber(stats?.[summary.key])}
              </span>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

export function RepeatabilityLog({ text }: { text: string }) {
  const { t } = useTranslation();
  const log = useRef<HTMLPreElement>(null);
  useEffect(() => {
    if (log.current) log.current.scrollTop = log.current.scrollHeight;
  }, [text]);
  return (
    <pre
      className="pp-utility-command-log"
      ref={log}
      tabIndex={0}
      aria-label={t("Log")}
    >
      {text}
    </pre>
  );
}
