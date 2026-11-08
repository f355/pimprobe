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

import { useEffect, useRef, useState } from "react";
import type { ProbeClient } from "./client";
import { ServiceError } from "./client";
import type {
  HistoryResult,
  MachineState,
  MotionEvent,
  Review,
  RotaryConfig,
  RotaryResult,
  RoutineConfig,
  RoutineResult,
  RunConfig,
} from "./types";
import {
  Button,
  ConfirmDialog,
  KeypadSlot,
  Notice,
  NumberField,
  PageHeader,
  useNumeric,
} from "./controls";
import { MotionIllustration } from "./diagrams";
import { angleName, featureName, reviewPlan } from "./model";
import { useTranslation } from "./i18n";
import { helpProps } from "./help/context";
import { descriptions, optionHelp } from "./help/text";
import { WcsPicker } from "./WcsPicker";
import { historyDetail } from "./utilities/history-detail";
import { CalibrationPage } from "./CalibrationPage";

function title(config: RunConfig) {
  if (!("family" in config))
    return config.operation === "horizontal"
      ? "Level horizontal surface"
      : config.operation === "vertical"
        ? "Align vertical surface toward Y+"
        : config.operation === "verticalNegative"
          ? "Align vertical surface toward Y−"
          : "Calibrate rotary axis";
  return featureName(config);
}

export function ProbeFlow({
  client,
  config: initial,
  history,
  state,
  onBack,
  onBusyChange,
  onRefresh,
  onReload,
  onSetting,
  onAlarm,
}: {
  client: ProbeClient;
  config: RunConfig;
  history?: HistoryResult;
  state: MachineState;
  onBack: () => void;
  onBusyChange: (busy: boolean) => void;
  onRefresh: () => void;
  onReload: () => Promise<void>;
  onSetting: (key: string, value: number) => Promise<void>;
  onAlarm?: () => void;
}) {
  const { t } = useTranslation();
  const editor = useNumeric();
  const [config, setConfig] = useState(initial);
  const configRef = useRef(config);
  const rotary = !("family" in config);
  const group = rotary ? "rotary" : "routine";
  const [phase, setPhase] = useState<
    "review" | "running" | "result" | "failed"
  >(history ? "result" : "review");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState(history?.entry.error || "");
  const [log, setLog] = useState(
    history ? historyDetail(history.entry, t) : "",
  );
  const [result, setResult] = useState<RoutineResult | RotaryResult | null>(
    history?.result || null,
  );
  const [token, setToken] = useState(history?.canApply ? history.entry.id : "");
  const [offsets, setOffsets] = useState(
    history?.entry.workZero?.offsets || [0, 0, 0],
  );
  const [safeZ, setSafeZ] = useState(
    "safeZOffset" in initial ? initial.safeZOffset : 40,
  );
  const [showLog, setShowLog] = useState(false);
  const offsetsRef = useRef(offsets);
  const safeZRef = useRef(safeZ);
  const [chooseWcs, setChooseWcs] = useState(false);
  const [calibrating, setCalibrating] = useState(false);
  const [confirmAction, setConfirmAction] = useState("");
  const [confirmation, setConfirmation] = useState("");
  useEffect(() => {
    if (!confirmation) return;
    const timeout = setTimeout(() => setConfirmation(""), 3000);
    return () => clearTimeout(timeout);
  }, [confirmation]);
  useEffect(() => {
    if (editor.edit) setConfirmation("");
  }, [editor.edit]);
  const inFlight = useRef(false);
  const abort = useRef<AbortController | null>(null);
  const logRef = useRef<HTMLPreElement>(null);
  const busy = phase === "running" || pending;
  useEffect(() => {
    onBusyChange(busy);
  }, [busy, onBusyChange]);
  useEffect(
    () => () => {
      abort.current?.abort();
      onBusyChange(false);
    },
    [onBusyChange],
  );
  useEffect(() => {
    if (logRef.current) logRef.current.scrollTop = logRef.current.scrollHeight;
  }, [log, showLog]);
  const append = (line: string) => setLog((current) => current + line + "\n");

  async function motion(action: "run" | "return" | "measured", id: string) {
    setPhase("running");
    setError("");
    setToken("");
    setShowLog(false);
    if (action === "run") setLog("");
    const controller = new AbortController();
    abort.current = controller;
    let terminal = false;
    try {
      await client.stream<MotionEvent>(
        `${group}.${action}`,
        action === "measured" ? { id, safeZOffset: safeZRef.current } : { id },
        (event) => {
          if (event.type === "progress") {
            if (event.progress.kind === "script")
              append(event.progress.message);
          } else if (event.type === "result") {
            terminal = true;
            setResult(event.result);
            setPhase("result");
            setToken(id);
          } else {
            terminal = true;
            setError(event.message);
            append("; " + event.message);
            if (event.code === "log" && event.result) {
              setResult(event.result);
              setPhase("result");
              setToken(id);
            } else setPhase("failed");
            if (event.code === "controller_alarm") onAlarm?.();
          }
        },
        controller.signal,
      );
      if (!terminal)
        throw new Error(
          "The connection ended before the probing result arrived.",
        );
    } catch (failure) {
      if (!controller.signal.aborted) {
        setError((failure as Error).message);
        setPhase("failed");
      }
    } finally {
      abort.current = null;
      onRefresh();
    }
  }
  async function proceed() {
    if (inFlight.current || !(await editor.accept())) return;
    inFlight.current = true;
    setPending(true);
    setError("");
    try {
      const review = await client.request<Review>(
        `${group}.review`,
        configRef.current,
      );
      setConfig(review.config);
      await motion("run", review.id);
    } catch (failure) {
      setError((failure as Error).message);
    } finally {
      inFlight.current = false;
      setPending(false);
    }
  }
  async function apply(action: "zero" | "wcs" | "rotation", wcs?: number) {
    if (inFlight.current || !token || !(await editor.accept())) return;
    inFlight.current = true;
    setPending(true);
    setError("");
    setConfirmation("");
    const id = token;
    try {
      const updated = await client.request<
        (RoutineResult | RotaryResult) & { storageWarning?: string }
      >(`${history ? "history" : group}.${action}`, {
        id,
        ...(action === "zero" && !rotary
          ? { offsets: offsetsRef.current }
          : {}),
        ...(action === "wcs" ? { wcs } : {}),
      });
      setResult(updated);
      setError(updated.storageWarning || "");
      if ("family" in config) setConfig({ ...config, wcs: updated.wcs });
      if (action !== "wcs")
        setConfirmation(
          t(action === "zero" ? "Work zero set" : "X/Y rotation set"),
        );
      setChooseWcs(false);
      onRefresh();
    } catch (failure) {
      setError((failure as Error).message);
      if (
        failure instanceof ServiceError &&
        failure.code !== "busy" &&
        action === "zero"
      )
        setToken("");
    } finally {
      inFlight.current = false;
      setPending(false);
    }
  }
  function openWcs() {
    editor.cancel();
    setChooseWcs(true);
  }
  const currentWcs =
    result?.wcs || ("wcs" in config ? config.wcs : state.status?.wcs || 54);
  const routineResult = result && "machinePoint" in result ? result : null;
  const rotaryResult = result && "stations" in result ? result : null;
  const canReturn =
    routineResult &&
    !history &&
    "family" in config &&
    config.family !== "angle" &&
    (config.family !== "inside" || config.z);
  const canPosition =
    routineResult &&
    !history &&
    "family" in config &&
    config.family === "inside" &&
    !config.z;
  const canRotate = rotaryResult
    ? !rotaryResult.level && rotaryResult.rotationSupported
    : routineResult?.angle?.rotationSupported;
  const zeroAxes =
    rotary && "operation" in config
      ? config.operation === "horizontal"
        ? "A/Z"
        : config.operation === "axis"
          ? "Y/Z"
          : "A/Y"
      : "";
  if (calibrating && routineResult)
    return (
      <CalibrationPage
        client={client}
        id={token}
        result={routineResult}
        onBack={() => setCalibrating(false)}
        onBusyChange={onBusyChange}
        onInvalidated={(message) => {
          setToken("");
          setError(message);
          setCalibrating(false);
          onRefresh();
        }}
        onSaved={async (calibration) => {
          setResult(calibration.result);
          if ("family" in config && calibration.diameter !== null) {
            const next = { ...config, diameter: calibration.diameter };
            setConfig(next);
            configRef.current = next;
          }
          setError(calibration.warning || "");
          setConfirmation(t("Calibration saved"));
          await onReload();
          setCalibrating(false);
          onRefresh();
        }}
      />
    );
  if (chooseWcs)
    return (
      <WcsPicker
        current={currentWcs}
        onBack={() => setChooseWcs(false)}
        onSelect={(wcs) => apply("wcs", wcs)}
        pending={pending}
        title={t("Save result to work coordinates")}
        error={error}
      />
    );
  return (
    <>
      <PageHeader
        title={t(title(config))}
        detail={t(
          phase === "review"
            ? "Confirm"
            : phase === "running"
              ? "Running"
              : phase === "result"
                ? "Complete"
                : "Failed",
        )}
        onBack={() => {
          editor.cancel();
          onBack();
        }}
        backDisabled={busy}
      />
      <main className="pp-content">
        {error && <Notice error>{t(error)}</Notice>}
        <div className="pp-body">
          {phase === "review" ? (
            <OperationReview
              config={config}
              onChange={(next) => {
                configRef.current = next;
                setConfig(next);
              }}
              disabled={pending}
            />
          ) : phase === "result" &&
            !showLog &&
            routineResult &&
            "family" in config ? (
            <RoutineResults
              config={config}
              result={routineResult}
              state={state}
              offsets={offsets}
              onOffset={(i, value) => {
                offsetsRef.current = offsetsRef.current.map((v, j) =>
                  i === j ? value : v,
                );
                setOffsets(offsetsRef.current);
              }}
              safeZ={safeZ}
              onSafeZ={async (value) => {
                await onSetting("safeZOffset", value);
                safeZRef.current = value;
                setSafeZ(value);
              }}
              historical={!!history}
              disabled={busy || !token}
              onWcs={openWcs}
              confirmation={confirmation}
              onCalibrate={
                !history && token
                  ? () => {
                      editor.cancel();
                      setCalibrating(true);
                    }
                  : undefined
              }
            />
          ) : phase === "result" && !showLog && rotaryResult ? (
            <RotaryResults
              result={rotaryResult}
              zeroAxes={zeroAxes}
              disabled={busy || !token}
              onWcs={openWcs}
              confirmation={confirmation}
            />
          ) : (
            <pre ref={logRef} className="pp-log" aria-label={t("Command log")}>
              {log || t("Preparing routine...")}
            </pre>
          )}
        </div>
        <footer className="pp-actions pp-result-actions">
          {phase === "result" && (
            <Button
              className="pp-log-toggle"
              onClick={() => {
                editor.cancel();
                setShowLog(!showLog);
              }}
            >
              {t(showLog ? "Result" : history ? "Details" : "Log")}
            </Button>
          )}
          <span className="pp-spacer" />
          {phase === "review" ? (
            <>
              <Button disabled={busy} onClick={onBack}>
                {t("Cancel")}
              </Button>
              <Button primary disabled={busy} onClick={() => void proceed()}>
                {t(pending ? "Preparing…" : "Proceed")}
              </Button>
            </>
          ) : (
            <>
              {phase === "result" && !routineResult?.angle && (
                <Button
                  primary
                  disabled={busy || !token}
                  help={rotary ? t(descriptions.rotaryZero) : undefined}
                  onClick={() =>
                    rotary ? setConfirmAction("zero") : void apply("zero")
                  }
                >
                  {rotary ? t("Set %1 zero", [zeroAxes]) : t("Set Work Zero")}
                </Button>
              )}
              {phase === "result" && canReturn && (
                <Button
                  disabled={busy || !token || routineResult?.returned}
                  onClick={() => {
                    editor.cancel();
                    void motion("return", token);
                  }}
                >
                  {t("Return to start")}
                </Button>
              )}
              {phase === "result" && canPosition && (
                <Button
                  disabled={busy || !token || routineResult?.positioned}
                  onClick={async () => {
                    if (await editor.accept()) void motion("measured", token);
                  }}
                >
                  {t("Move to measured XY")}
                </Button>
              )}
              {phase === "result" && canRotate && (
                <Button
                  primary={!!routineResult?.angle}
                  disabled={busy || !token}
                  help={t(
                    rotary
                      ? descriptions["Set X/Y rotation"]
                      : descriptions.faceRotation,
                  )}
                  onClick={() =>
                    rotary
                      ? setConfirmAction("rotation")
                      : void apply("rotation")
                  }
                >
                  {t("Set X/Y rotation")}
                </Button>
              )}
              {(phase === "result" || phase === "failed") && (
                <Button className="pp-close" disabled={busy} onClick={onBack}>
                  {t("Close")}
                </Button>
              )}
            </>
          )}
        </footer>
      </main>
      {confirmAction && (
        <ConfirmDialog
          title={t(
            confirmAction === "zero" ? "Set work zero" : "Set X/Y rotation",
          )}
          message={
            confirmAction === "zero"
              ? t(
                  rotaryResult?.level
                    ? "Set G%1 %2 zero to the measured surface and current A angle?"
                    : "Set G%1 %2 zero to the first measured axis center?",
                  [currentWcs, zeroAxes],
                )
              : t("Set G%1 X/Y rotation to %2°?", [
                  currentWcs,
                  (rotaryResult?.xyAngle || 0).toFixed(4),
                ])
          }
          onCancel={() => setConfirmAction("")}
          onConfirm={() => {
            const action = confirmAction as "zero" | "rotation";
            setConfirmAction("");
            void apply(action);
          }}
        />
      )}
    </>
  );
}

function OperationReview({
  config,
  onChange,
  disabled,
}: {
  config: RunConfig;
  onChange: (config: RunConfig) => void;
  disabled: boolean;
}) {
  const { t } = useTranslation();
  const plan = reviewPlan(config);
  const [phase, setPhase] = useState(0);
  return (
    <div className="pp-columns">
      <KeypadSlot>
        <div className="pp-review-left">
          <div className="pp-review-illustration">
            <MotionIllustration clip={plan.clip} onPhase={setPhase} />
          </div>
          <div className="pp-review-phases">
            {plan.phases.map((text, i) => (
              <div
                key={text}
                className={`pp-review-phase ${phase === i ? "pp-current" : ""}`}
                {...helpProps(t("Motion sequence"), t(text))}
              >
                <b>{i + 1}</b>
                <span>{t(text)}</span>
              </div>
            ))}
          </div>
        </div>
      </KeypadSlot>
      <section className="pp-parameters" aria-label={t("Options for this run")}>
        <div>{t("Options for this run")}</div>
        {plan.options.map((option) => (
          <div key={option.key}>
            {option.section && (
              <div className="pp-section-label">{t(option.section)}</div>
            )}
            <div className="pp-parameter">
              <label>{t(option.label)}</label>
              <NumberField
                label={t(option.label)}
                value={Number(config[option.key as keyof RunConfig]) || 0}
                min={option.minimum}
                max={option.maximum}
                disabled={disabled}
                help={t(
                  optionHelp(
                    "family" in config && config.family === "angle"
                      ? option.label === "Point spacing"
                        ? "anglePointSpacing"
                        : option.label === "Search distance"
                          ? "angleSearchDistance"
                          : option.key
                      : option.key,
                    "family" in config ? config.family : undefined,
                  ),
                )}
                onChange={(value) =>
                  onChange({ ...config, [option.key]: value })
                }
              />
              <span className="pp-unit">{option.unit}</span>
            </div>
          </div>
        ))}
      </section>
    </div>
  );
}
function RoutineResults({
  config,
  result,
  state,
  offsets,
  onOffset,
  safeZ,
  onSafeZ,
  historical,
  disabled,
  onWcs,
  confirmation,
  onCalibrate,
}: {
  config: RoutineConfig;
  result: RoutineResult;
  state: MachineState;
  offsets: number[];
  onOffset: (i: number, value: number) => void;
  safeZ: number;
  onSafeZ: (value: number) => Promise<void>;
  historical: boolean;
  disabled: boolean;
  onWcs: () => void;
  confirmation: string;
  onCalibrate?: () => void;
}) {
  const { t } = useTranslation();
  const axes = [0, 1, 2].filter((i) => result.machinePoint[i] != null);
  const spans = [0, 1].filter((i) => result.spans?.[i] != null);
  function work(i: number) {
    const reference = state.coordinates?.spindle;
    if (!reference || state.status?.wcs !== result.wcs) return result.point[i];
    const radians = ((state.wcsRotations?.[result.wcs] || 0) * Math.PI) / 180;
    const dx =
      result.machinePoint[0] == null
        ? 0
        : result.machinePoint[0] - reference.machinePosition[0];
    const dy =
      result.machinePoint[1] == null
        ? 0
        : result.machinePoint[1] - reference.machinePosition[1];
    return i === 0
      ? reference.workPosition[0] +
          Math.cos(radians) * dx +
          Math.sin(radians) * dy
      : i === 1
        ? reference.workPosition[1] -
          Math.sin(radians) * dx +
          Math.cos(radians) * dy
        : reference.workPosition[2] +
          result.machinePoint[2]! -
          reference.machinePosition[2];
  }
  if (result.angle)
    return (
      <div className="pp-columns pp-result-columns">
        <div
          className="pp-result-left"
          {...helpProps(t("Measured angle"), t(descriptions.measuredAngle))}
        >
          <h2>{t(angleName(config.feature || ""))}</h2>
          <div className="pp-angle-value pp-mono">
            {result.angle.degrees.toFixed(4)}°
          </div>
          <p className="pp-muted">
            {t("Change %1 mm over %2 mm", [
              result.angle.difference.toFixed(3),
              result.angle.spacing.toFixed(3),
            ])}
          </p>
        </div>
        <div className="pp-result-divider" />
        <section
          className="pp-scroll"
          {...helpProps(
            t("Contacts · G53 · mm"),
            t(descriptions.angleContacts),
          )}
        >
          <div className="pp-result-heading">
            <h2>{confirmation || t("Work coordinates")}</h2>
            <Button
              disabled={disabled}
              onClick={onWcs}
              aria-label={t("Choose result work coordinates")}
              helpTitle={t("Choose result work coordinates")}
              help={t(
                "Choose which work coordinate system receives this measured angle.",
              )}
            >
              G{result.wcs}
            </Button>
          </div>
          <p className="pp-muted">{t("Contacts · G53 · mm")}</p>
          {result.angle.touches.map((point, i) => (
            <div key={i}>
              <p className="pp-muted">{t("Touch %1", [i + 1])}</p>
              <p className="pp-mono">
                {point
                  .slice(0, 3)
                  .map((v, i) => `${["X", "Y", "Z"][i]} ${v.toFixed(3)}`)
                  .join("  ")}
              </p>
            </div>
          ))}
        </section>
      </div>
    );
  return (
    <div className="pp-columns pp-result-columns">
      <KeypadSlot>
        <div
          className="pp-result-left"
          {...(!spans.length
            ? helpProps(t("Contacts · G53 · mm"), t(descriptions.rawContacts))
            : {})}
        >
          <h2>{t(featureName(config))}</h2>
          {spans.length ? (
            <>
              <div className="pp-dimension pp-muted">
                <span {...helpProps(t("Size · mm"), t(descriptions.size))}>
                  {t("Size · mm")}
                </span>
                <span />
                <span
                  {...helpProps(t("Raw span · mm"), t(descriptions.rawSpan))}
                >
                  {t("Raw span · mm")}
                </span>
              </div>
              {spans.map((i) => (
                <div key={i} className="pp-dimension">
                  <span>
                    {t(
                      config.feature === "hole" || config.feature === "boss"
                        ? ["Span X", "Span Y"][i]
                        : i === 0
                          ? "Width X"
                          : config.feature?.startsWith("y-")
                            ? "Width Y"
                            : "Length Y",
                    )}
                  </span>
                  <span
                    className="pp-mono"
                    {...helpProps(t("Size · mm"), t(descriptions.size))}
                  >
                    {result.spans[i]!.toFixed(3)}
                  </span>
                  <span
                    className="pp-mono"
                    {...helpProps(t("Raw span · mm"), t(descriptions.rawSpan))}
                  >
                    {result.rawSpans?.[i]?.toFixed(3) || "—"}
                  </span>
                </div>
              ))}
            </>
          ) : (
            <>
              <p className="pp-muted">{t("Contacts · G53 · mm")}</p>
              {axes.map((i) => (
                <div key={i} className="pp-large-reading pp-mono">
                  {["X", "Y", "Z"][i]} {result.rawPoint?.[i]?.toFixed(3) || "—"}
                </div>
              ))}
            </>
          )}
          {onCalibrate && (config.family === "center" || axes.includes(2)) && (
            <Button
              className="pp-calibrate-button"
              disabled={disabled}
              onClick={onCalibrate}
              help={t(
                "Calibrate ball diameter from a known feature size, or probe offsets against an independently set work zero. No movement is needed.",
              )}
            >
              {t("Calibrate probe...")}
            </Button>
          )}
        </div>
      </KeypadSlot>
      <div className="pp-result-divider" />
      <section className="pp-result-right pp-scroll">
        <div className="pp-result-heading">
          <h2 role="status">{confirmation || t("Work zero")}</h2>
          <Button
            disabled={disabled}
            onClick={onWcs}
            aria-label={t("Choose result work coordinates")}
            helpTitle={t("Choose result work coordinates")}
          >
            G{result.wcs}
          </Button>
        </div>
        <div className="pp-result-labels">
          <span>{t("Measured · G53 · mm")}</span>
          <span>{t("Offset · mm")}</span>
        </div>
        {axes.map((i) => (
          <div key={i} className="pp-result-axis">
            <div
              className="pp-mono"
              {...helpProps(
                t("Measured · G53 · mm"),
                t(
                  "Surface or center coordinates with probe offsets and ball compensation applied. The smaller numbers show the same point in the selected work coordinate system.",
                ),
              )}
            >
              <strong>
                {["X", "Y", "Z"][i]} {result.machinePoint[i]!.toFixed(3)}
              </strong>
              <small>
                G{result.wcs} {work(i)?.toFixed(3) || "—"}
              </small>
            </div>
            <NumberField
              label={t("%1 origin offset", [["X", "Y", "Z"][i]])}
              value={offsets[i]}
              min={-1000}
              max={1000}
              disabled={disabled}
              help={t(
                "Add this distance to the measured machine coordinate when setting work zero. Each measured axis has its own offset.",
              )}
              onChange={(value) => onOffset(i, value)}
            />
          </div>
        ))}
        {!historical && config.family === "inside" && !config.z && (
          <div className="pp-result-axis">
            <label>{t("Safe Z lift · mm")}</label>
            <NumberField
              label={t("Safe Z lift · mm")}
              value={safeZ}
              min={0.1}
              max={1000}
              onChange={onSafeZ}
              help={t(
                "Distance to raise from the current Z before moving the ball to the measured X/Y point.",
              )}
            />
          </div>
        )}
      </section>
    </div>
  );
}
function RotaryResults({
  result,
  zeroAxes,
  disabled,
  onWcs,
  confirmation,
}: {
  result: RotaryResult;
  zeroAxes: string;
  disabled: boolean;
  onWcs: () => void;
  confirmation: string;
}) {
  const { t } = useTranslation();
  return (
    <div className="pp-columns pp-result-columns">
      <section
        className="pp-result-left"
        {...helpProps(
          t(result.level ? "Touches · G53 · mm" : "Axis centers · G53 · mm"),
          t(
            result.level
              ? descriptions.rotaryTouches
              : descriptions.rotaryCenters,
          ),
        )}
      >
        <h2>{t(result.level ? "Surface alignment" : "Rotary axis")}</h2>
        <p className="pp-muted">
          {t(result.level ? "Touches · G53 · mm" : "Axis centers · G53 · mm")}
        </p>
        <table className="pp-result-table pp-mono">
          <thead>
            <tr>
              {["X", "Y", "Z"].map((x) => (
                <th key={x}>{x}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {(result.level
              ? result.level.touches
              : result.stations.map((s) => s.center)
            ).map((point, i) => (
              <tr key={i}>
                {point.slice(0, 3).map((v, j) => (
                  <td key={j}>{v.toFixed(3)}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </section>
      <div className="pp-result-divider" />
      <section className="pp-scroll">
        <div className="pp-result-heading">
          <h2>{confirmation || t("Work zero · %1", [zeroAxes])}</h2>
          <Button
            disabled={disabled}
            onClick={onWcs}
            aria-label={t("Choose result work coordinates")}
            helpTitle={t("Choose result work coordinates")}
            help={t(descriptions["Choose result work coordinates"])}
          >
            G{result.wcs}
          </Button>
        </div>
        {(result.level
          ? [
              {
                label: "A correction",
                value: result.level.correction,
                help: descriptions.aCorrection,
              },
              {
                label: "Remaining tilt",
                value: result.level.residual,
                help: descriptions.remainingTilt,
              },
            ]
          : [
              {
                label: "Axis angle in X/Y",
                value: result.xyAngle,
                help: descriptions.xyAngle,
              },
              {
                label: "Axis angle in X/Z",
                value: result.xzAngle,
                help: descriptions.xzAngle,
              },
            ]
        ).map(({ label, value, help }) => (
          <div
            className="pp-result-axis"
            key={label}
            {...helpProps(t(label), t(help))}
          >
            <span>{t(label)}</span>
            <span className="pp-mono">{Number(value).toFixed(4)}°</span>
          </div>
        ))}
        <p className="pp-muted">
          {t(
            result.level
              ? "Zero uses the measured surface and A angle."
              : "Zero uses the first axis center.",
          )}
        </p>
      </section>
    </div>
  );
}
