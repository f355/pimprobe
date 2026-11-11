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

import { useEffect, useMemo, useRef, useState } from "react";
import { ServiceError, type ProbeClient } from "./client";
import type { Calibration, CalibrationOptions, RoutineResult } from "./types";
import {
  Button,
  KeypadSlot,
  Notice,
  NumberField,
  PageHeader,
  useNumeric,
} from "./controls";
import { useTranslation } from "./i18n";
import { helpProps } from "./help/context";

type Mode = "ball" | "xy" | "z";
type Direction = "X" | "Y" | "X+Y";

function choices(
  mode: Mode,
  direction: Direction,
  dimensions: (number | null)[],
  calibration: Calibration | null,
): CalibrationOptions {
  const available =
    mode === "ball" ? calibration?.ballAxes : calibration?.offsetAxes;
  const axes = [0, 1].map(
    (i) =>
      !!available?.[i] && (direction === "X+Y" || direction === ["X", "Y"][i]),
  );
  const complete =
    axes.some(Boolean) &&
    axes.every((selected, i) => !selected || dimensions[i] !== null);
  return {
    ball: mode === "ball" && complete,
    dimensions:
      mode === "ball" && complete
        ? (axes.map((selected, i) =>
            selected ? dimensions[i] : null,
          ) as CalibrationOptions["dimensions"])
        : [null, null],
    offsets:
      mode === "xy"
        ? [axes[0], axes[1], false]
        : [false, false, mode === "z" && !!calibration?.offsetAxes[2]],
  };
}

export function CalibrationPage({
  client,
  id,
  result,
  onBack,
  onSaved,
  onInvalidated,
  onBusyChange,
}: {
  client: ProbeClient;
  id: string;
  result: RoutineResult;
  onBack: () => void;
  onSaved: (calibration: Calibration) => void | Promise<void>;
  onInvalidated: (message: string) => void;
  onBusyChange: (busy: boolean) => void;
}) {
  const { t } = useTranslation();
  const editor = useNumeric();
  const [mode, setMode] = useState<Mode>("ball");
  const [direction, setDirection] = useState<Direction>("X+Y");
  const [dimensions, setDimensions] = useState<(number | null)[]>([null, null]);
  const [calibration, setCalibration] = useState<Calibration | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const inFlight = useRef(false);
  const options = useMemo(
    () => choices(mode, direction, dimensions, calibration),
    [
      mode,
      direction,
      dimensions,
      calibration?.ballAxes[0],
      calibration?.ballAxes[1],
      calibration?.offsetAxes[0],
      calibration?.offsetAxes[1],
      calibration?.offsetAxes[2],
    ],
  );
  const optionsRef = useRef(options);
  optionsRef.current = options;
  useEffect(() => {
    const abort = new AbortController();
    setLoading(true);
    client
      .request<Calibration>(
        "routine.calibration",
        { id, options },
        abort.signal,
      )
      .then((value) => {
        if (abort.signal.aborted) return;
        setCalibration(value);
        if (!value.ballAxes.some(Boolean) && mode === "ball")
          setMode(value.offsetAxes[2] ? "z" : "xy");
        setError("");
      })
      .catch((failure: Error) => {
        if (!abort.signal.aborted) setError(failure.message);
      })
      .finally(() => {
        if (!abort.signal.aborted) setLoading(false);
      });
    return () => abort.abort();
  }, [client, id, options]);

  async function save() {
    if (inFlight.current || !(await editor.accept())) return;
    inFlight.current = true;
    setSaving(true);
    onBusyChange(true);
    setError("");
    try {
      await onSaved(
        await client.request<Calibration>("routine.calibration", {
          id,
          options: optionsRef.current,
          save: true,
        }),
      );
    } catch (failure) {
      if (
        failure instanceof ServiceError &&
        failure.code === "calibration_incomplete"
      )
        onInvalidated(failure.message);
      else setError((failure as Error).message);
    } finally {
      inFlight.current = false;
      setSaving(false);
      onBusyChange(false);
    }
  }
  function cancel() {
    editor.cancel();
    onBack();
  }
  const modes = [
    {
      id: "ball" as Mode,
      label: "Ball diameter",
      available: calibration?.ballAxes.some(Boolean),
    },
    {
      id: "xy" as Mode,
      label: "X/Y offsets",
      available: calibration?.offsetAxes.slice(0, 2).some(Boolean),
    },
    {
      id: "z" as Mode,
      label: "Z offset",
      available: calibration?.offsetAxes[2],
    },
  ].filter((item) => item.available);
  const available =
    mode === "ball" ? calibration?.ballAxes : calibration?.offsetAxes;
  const axes =
    mode === "z"
      ? [2]
      : [0, 1].filter(
          (i) =>
            available?.[i] &&
            (direction === "X+Y" || direction === ["X", "Y"][i]),
        );
  const ready = options.ball || options.offsets.some(Boolean);
  const saveLabel =
    mode === "ball"
      ? "Save ball diameter"
      : mode === "z"
        ? "Save Z offset"
        : axes.length === 2
          ? "Save X/Y offsets"
          : `Save ${["X", "Y"][axes[0]]} offset`;
  return (
    <>
      <PageHeader
        title={t("Calibrate probe")}
        onBack={cancel}
        backDisabled={saving}
      />
      <main className="pp-content">
        {modes.length > 1 && (
          <div
            className="pp-calibration-tabs"
            role="tablist"
            aria-label={t("Calibration type")}
          >
            {modes.map((item) => (
              <Button
                key={item.id}
                role="tab"
                aria-selected={mode === item.id}
                className={mode === item.id ? "pp-selected" : ""}
                disabled={saving}
                help={t(
                  item.id === "ball"
                    ? "Compare the probed width with a micrometer or gauge size. The saved diameter accounts for the extra movement before the probe sends its contact signal."
                    : "Calibrate the probe offset against a work zero established with a cutting tool or indicator before probing.",
                )}
                onClick={() => {
                  editor.cancel();
                  setDirection("X+Y");
                  setMode(item.id);
                }}
              >
                {t(item.label)}
              </Button>
            ))}
          </div>
        )}
        <div className="pp-body pp-columns pp-calibration">
          <KeypadSlot>
            <section
              className="pp-calibration-values"
              aria-label={t("Calibration values")}
            >
              {modes.length === 1 && (
                <h2>{t(modes[0].label)}</h2>
              )}
              <div
                className="pp-calibration-table"
                role="table"
                aria-label={t("Calibration comparison")}
              >
                <div className="pp-calibration-row pp-muted" role="row">
                  <span />
                  <span>{t("Current")}</span>
                  <span>{t("New · mm")}</span>
                </div>
                {mode === "ball" ? (
                  <div className="pp-calibration-row" role="row">
                    <span>{t("Ball")}</span>
                    <span className="pp-mono">
                      {calibration?.currentDiameter.toFixed(4)}
                    </span>
                    <strong className="pp-mono">
                      {calibration?.proposedDiameter?.toFixed(4) ?? "—"}
                    </strong>
                  </div>
                ) : (
                  axes.map((i) => (
                    <div className="pp-calibration-row" role="row" key={i}>
                      <span>{["X", "Y", "Z"][i]}</span>
                      <span className="pp-mono">
                        {calibration?.currentOffsets[i].toFixed(4)}
                      </span>
                      <strong className="pp-mono">
                        {calibration?.proposedOffsets[i]?.toFixed(4) ?? "—"}
                      </strong>
                    </div>
                  ))
                )}
              </div>
              {mode === "ball" && axes.length === 2 && (
                <div
                  className="pp-calibration-estimates"
                  {...helpProps(
                    t("Mean of X and Y"),
                    t(
                      "One ball diameter is saved: the arithmetic mean of the X and Y estimates.",
                    ),
                  )}
                >
                  {axes.map((i) => (
                    <div key={i}>
                      <span>{t("From %1", [["X", "Y"][i]])}</span>
                      <span className="pp-mono">
                        {calibration?.ballEstimates[i]?.toFixed(4) ?? "—"} mm
                      </span>
                    </div>
                  ))}
                  <strong>{t("New diameter = mean of X and Y")}</strong>
                </div>
              )}
            </section>
          </KeypadSlot>
          <section className="pp-calibration-inputs">
            {mode !== "z" && available?.[0] && available?.[1] && (
              <div
                className="pp-calibration-directions"
                role="group"
                aria-label={t("Measurement directions")}
              >
                {(["X", "Y", "X+Y"] as Direction[]).map((value) => (
                  <Button
                    key={value}
                    aria-pressed={direction === value}
                    className={direction === value ? "pp-selected" : ""}
                    disabled={saving}
                    help={t(
                      "Use the selected measurement directions for this calibration.",
                    )}
                    onClick={() => {
                      editor.cancel();
                      setDirection(value);
                    }}
                  >
                    {value}
                  </Button>
                ))}
              </div>
            )}
            {mode === "ball" ? (
              axes.map((i) => (
                <label className="pp-calibration-dimension" key={i}>
                  <span>
                    {t("Known %1 size", [["X", "Y"][i]])}
                    <small>mm</small>
                  </span>
                  <NumberField
                    label={t("Known %1 size", [["X", "Y"][i]])}
                    value={dimensions[i]}
                    placeholder={t("Enter size")}
                    min={0.01}
                    max={1000}
                    disabled={saving}
                    help={t(
                      "Enter the size measured with a micrometer or another suitable instrument, not the probe reading or the cutter's nominal diameter.",
                    )}
                    onChange={(value) => {
                      const next = dimensions.map((v, j) =>
                        i === j ? value : v,
                      );
                      optionsRef.current = choices(
                        mode,
                        direction,
                        next,
                        calibration,
                      );
                      setDimensions(next);
                    }}
                  />
                </label>
              ))
            ) : (
              <div
                className="pp-calibration-reference"
                {...helpProps(
                  t("Offset reference"),
                  t(
                    mode === "z"
                      ? "Set Z0 on a surface with a measured tool before probing it. Do not set this zero from the probe result."
                      : "First set the center to X0/Y0 by milling it or finding it with an indicator in the spindle. The midpoint of two opposite contacts does not depend on ball diameter. Do not press Set Work Zero before calibrating.",
                  ),
                )}
              >
                <h2>{t("Reference: G%1 zero", [result.wcs])}</h2>
                {axes.map((i) => (
                  <div className="pp-calibration-measured" key={i}>
                    <span>{t("Measured %1", [["X", "Y", "Z"][i]])}</span>
                    <span className="pp-mono">
                      {result.point[i]?.toFixed(4)} mm
                    </span>
                  </div>
                ))}
              </div>
            )}
            {error && <Notice error>{t(error)}</Notice>}
          </section>
        </div>
        <footer className="pp-actions">
          <span className="pp-spacer" />
          <Button disabled={saving} onClick={cancel}>
            {t("Cancel")}
          </Button>
          <Button
            primary
            disabled={saving || loading || !!error || !ready}
            onClick={() => void save()}
            help={t(
              "Save only the values on this page. The previous calibration is kept in Probe history. The work zero is unchanged.",
            )}
          >
            {t(saving ? "Saving…" : saveLabel)}
          </Button>
        </footer>
      </main>
    </>
  );
}
