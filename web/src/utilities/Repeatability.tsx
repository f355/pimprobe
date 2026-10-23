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

import { useEffect, useRef, useState } from "react";
import {
  Button,
  KeypadSlot,
  Notice,
  NumberField,
  PageHeader,
  Switch,
  useNumeric,
} from "../controls";
import { useTranslation } from "../i18n";
import { helpProps } from "../help/context";
import { errorMessage, useUtilityBusy } from "./common";
import {
  recordMeasurement,
  runOptions,
  validOptions,
} from "./repeatability-model";
import type {
  RepeatabilityEvent,
  RepeatabilityOptions,
  RepeatabilityReport,
} from "./repeatability-model";
import {
  RepeatabilityLog,
  RepeatabilityReadings,
} from "./RepeatabilityReadings";
import type { UtilityPageProps } from "./UtilityPage";

type Phase = "prepare" | "options" | "running" | "result" | "failed";

export function Repeatability({
  client,
  onBack,
  onBusyChange,
}: UtilityPageProps) {
  const { t } = useTranslation();
  const numeric = useNumeric();
  const [phase, setPhase] = useState<Phase>("prepare");
  const [options, setOptions] = useState<RepeatabilityOptions>({
    axes: [true, true, true],
    repetitions: 5,
    jogHome: false,
    home: false,
    retract: true,
  });
  const optionsRef = useRef(options);
  const [report, setReport] = useState<RepeatabilityReport>({
    measurements: [],
    statistics: [null, null, null],
  });
  const [failure, setFailure] = useState("");
  const [log, setLog] = useState("");
  const [showLog, setShowLog] = useState(false);
  const [streamStarted, setStreamStarted] = useState(false);
  const [stopRequested, setStopRequested] = useState(false);
  const stream = useRef<AbortController | null>(null);
  const stopRequest = useRef<AbortController | null>(null);
  const terminal = useRef(false);
  const starting = useRef(false);
  const running = phase === "running";
  const showingResults = running || phase === "result" || phase === "failed";
  const effectiveOptions = runOptions(options);
  useUtilityBusy(running, onBusyChange);
  useEffect(
    () => () => {
      stream.current?.abort();
      stopRequest.current?.abort();
    },
    [client],
  );

  function changeOptions(patch: Partial<RepeatabilityOptions>) {
    optionsRef.current = { ...optionsRef.current, ...patch };
    setOptions(optionsRef.current);
  }

  function eventReceived(event: RepeatabilityEvent) {
    if (terminal.current) return;
    setStreamStarted(true);
    if (
      event.type === "progress" &&
      event.progress.kind === "script" &&
      event.progress.message
    ) {
      setLog((text) => text + event.progress.message + "\n");
    } else if (event.type === "measurement") {
      setReport((current) => recordMeasurement(current, event));
    } else if (event.type === "result" || event.type === "error") {
      terminal.current = true;
      setStopRequested(false);
      if (event.result) setReport(event.result);
      const stopped = event.type === "error" && event.code === "stopped";
      setPhase(event.type === "result" || stopped ? "result" : "failed");
      setFailure(event.type === "error" && !stopped ? event.message : "");
      if (stopped) setLog((text) => text + "; Check stopped\n");
      else if (event.type === "error" && event.message)
        setLog((text) => `${text}; Failed: ${event.message}\n`);
    }
  }

  async function start() {
    if (phase !== "options" || starting.current || stream.current) return;
    starting.current = true;
    try {
      if (!(await numeric.accept()) || !validOptions(optionsRef.current))
        return;
      const controller = new AbortController();
      stream.current = controller;
      terminal.current = false;
      setFailure("");
      setReport({ measurements: [], statistics: [null, null, null] });
      setLog("");
      setShowLog(false);
      setStreamStarted(false);
      setStopRequested(false);
      setPhase("running");
      try {
        await client.stream<RepeatabilityEvent>(
          "repeatability.run",
          runOptions(optionsRef.current),
          (event) => {
            if (!controller.signal.aborted) eventReceived(event);
          },
          controller.signal,
        );
        if (!controller.signal.aborted && !terminal.current)
          throw new Error("Connection ended before the check finished");
      } catch (error) {
        if (!controller.signal.aborted && !terminal.current) {
          terminal.current = true;
          setFailure(
            errorMessage(error, "Connection ended before the check finished"),
          );
          setPhase("failed");
          setStopRequested(false);
        }
      } finally {
        if (stream.current === controller) stream.current = null;
      }
    } finally {
      starting.current = false;
    }
  }

  async function stop() {
    if (!running || !streamStarted || stopRequested || stopRequest.current)
      return;
    const controller = new AbortController();
    stopRequest.current = controller;
    setStopRequested(true);
    setFailure("");
    try {
      await client.request("repeatability.stop", undefined, controller.signal);
    } catch (error) {
      if (!controller.signal.aborted && !terminal.current) {
        setStopRequested(false);
        setFailure(errorMessage(error, "Could not stop the check"));
      }
    } finally {
      if (stopRequest.current === controller) stopRequest.current = null;
    }
  }

  function back() {
    if (!running) {
      numeric.cancel();
      onBack();
    }
  }

  return (
    <section className="pp-utility-page">
      <PageHeader
        title={t("Probe repeatability")}
        detail={
          showingResults
            ? t("%1 / %2 runs", [
                report.measurements.length,
                options.repetitions,
              ])
            : ""
        }
        onBack={back}
        backDisabled={running}
      />
      <div className="pp-utility-content pp-utility-repeatability">
        {phase === "prepare" && (
          <div className="pp-utility-preparation">
            <svg
              className="pp-utility-fixture"
              viewBox="0 0 252 252"
              role="img"
              aria-label={t("L bracket")}
            >
              <rect
                className="pp-utility-fixture-bed"
                x="5"
                y="5"
                width="242"
                height="242"
              />
              <path
                className="pp-utility-fixture-wall"
                d="M5 5H76V176H247V247H5Z"
              />
              <path className="pp-utility-fixture-edge" d="M76 5V176H247" />
              <path
                className="pp-utility-fixture-probe"
                d="M126 112H154M140 98V126M132 112H82L90 104M82 112L90 120M140 120V170L132 162M140 170L148 162"
              />
              <text x="44" y="118">
                X
              </text>
              <text x="134" y="210">
                Y
              </text>
            </svg>
            <div
              className="pp-utility-preparation-text"
              {...helpProps(
                t("Before starting"),
                t(
                  "Leave room for the probe to extend and clear the path to the bracket.",
                ),
              )}
            >
              <h2>{t("Before starting")}</h2>
              <p>
                {t(
                  "Home the machine with the L bracket installed. The probe uses machine coordinates (G53) to find the bracket walls and bed.",
                )}
              </p>
              <p className="pp-utility-muted">
                {t(
                  "Leave room for the probe to extend and clear the path to the bracket.",
                )}
              </p>
            </div>
          </div>
        )}
        {phase === "options" && (
          <div className="pp-utility-options">
            <KeypadSlot>
              <p className="pp-utility-options-explanation">
                {t(
                  options.home
                    ? "The probe tip approaches about 15 mm from the bracket walls and 5 mm above the bed, then measures the selected surfaces.\n\nThe machine jogs near home and homes before each repetition."
                    : options.jogHome
                      ? "The probe tip approaches about 15 mm from the bracket walls and 5 mm above the bed, then measures the selected surfaces.\n\nThe machine jogs near home before each repetition."
                      : "The probe tip approaches about 15 mm from the bracket walls and 5 mm above the bed, then measures the selected surfaces.\n\nX/Y travel starts at the current height. Make sure the path is clear.",
                )}
              </p>
            </KeypadSlot>
            <div className="pp-utility-options-form">
              <div className="pp-utility-option-row">
                <span>{t("Axes")}</span>
                <div
                  className="pp-utility-axis-options"
                  role="group"
                  aria-label={t("Axes")}
                >
                  {(["X", "Y", "Z"] as const).map((axis, index) => (
                    <Button
                      key={axis}
                      className={
                        options.axes[index] ? "pp-utility-selected" : ""
                      }
                      aria-pressed={options.axes[index]}
                      helpTitle={`${t("Axes")} · ${axis}`}
                      help={t(
                        "Include this axis in the repeated measurements.",
                      )}
                      onClick={() => {
                        const axes = [
                          ...options.axes,
                        ] as RepeatabilityOptions["axes"];
                        axes[index] = !axes[index];
                        changeOptions({ axes });
                      }}
                    >
                      {axis}
                    </Button>
                  ))}
                </div>
              </div>
              <div className="pp-utility-option-row">
                <span>{t("Repetitions")}</span>
                <NumberField
                  label={t("Repetitions")}
                  value={options.repetitions}
                  min={1}
                  max={100}
                  integer
                  help={t(
                    "Number of measurements to collect. Statistics update after each repetition.",
                  )}
                  onChange={(repetitions) => changeOptions({ repetitions })}
                />
              </div>
              <Switch
                label={t("Jog home each time")}
                checked={options.jogHome}
                disabled={options.home}
                help={t(
                  "Move near the home position between repetitions, without searching for the home switches.",
                )}
                onChange={(jogHome) => changeOptions({ jogHome })}
              />
              <Switch
                label={t("Home each time")}
                checked={options.home}
                help={t(
                  "Home the machine between repetitions to include homing variation. Homing also retracts the probe.",
                )}
                onChange={(home) =>
                  changeOptions({ home, ...(home ? { jogHome: true } : {}) })
                }
              />
              <Switch
                label={t("Retract each time")}
                checked={effectiveOptions.retract}
                disabled={options.jogHome || options.home}
                help={t(
                  "Retract and extend the probe between repetitions to include actuator variation.",
                )}
                onChange={(retract) => changeOptions({ retract })}
              />
            </div>
          </div>
        )}
        {failure && <Notice error>{t(failure)}</Notice>}
        {showingResults &&
          (showLog ? (
            <RepeatabilityLog text={log} />
          ) : (
            <RepeatabilityReadings report={report} running={running} />
          ))}
        <footer className="pp-utility-repeat-actions">
          {showingResults && (
            <Button
              className="pp-utility-log-toggle"
              onClick={() => setShowLog((value) => !value)}
              help={t(
                showLog
                  ? "Show each measurement and the statistics calculated from the readings."
                  : "Show the commands and messages recorded during this measurement.",
              )}
            >
              {t(showLog ? "Readings" : "Log")}
            </Button>
          )}
          <div className="pp-utility-spacer pp-utility-run-options">
            {showingResults &&
              t("Jog home: %1    Home: %2    Retract: %3", [
                t(options.jogHome ? "yes" : "no"),
                t(options.home ? "yes" : "no"),
                t(effectiveOptions.retract ? "yes" : "no"),
              ])}
          </div>
          {!showingResults && (
            <Button className="pp-utility-cancel" onClick={back}>
              {t("Cancel")}
            </Button>
          )}
          {running ? (
            <Button
              className="pp-utility-repeat-primary"
              disabled={!streamStarted || stopRequested}
              onClick={() => void stop()}
              help={t("Stop the repeatability check.")}
            >
              {t(stopRequested ? "Stopping..." : "Stop")}
            </Button>
          ) : (
            <Button
              className="pp-utility-repeat-primary"
              primary={phase === "options"}
              disabled={phase === "options" && !validOptions(options)}
              onPointerDown={(event) => {
                if (phase === "options") event.preventDefault();
              }}
              onClick={() =>
                phase === "prepare"
                  ? setPhase("options")
                  : phase === "options"
                    ? void start()
                    : back()
              }
              help={t(
                phase === "prepare"
                  ? "Choose which axes to measure and how many repetitions to run."
                  : phase === "options"
                    ? "Run the repeatability check with the selected axes and options."
                    : "Close this page.",
              )}
            >
              {t(
                phase === "prepare"
                  ? "Fixture is ready"
                  : phase === "options"
                    ? "Start check"
                    : "Close",
              )}
            </Button>
          )}
        </footer>
      </div>
    </section>
  );
}
