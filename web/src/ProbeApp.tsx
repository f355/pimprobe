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

import { useCallback, useEffect, useMemo, useState } from "react";
import { ArrowLeft, BookOpen } from "lucide-react";
import { HttpClient, type ProbeClient } from "./client";
import type {
  Family,
  HistoryResult,
  MachineState,
  RunConfig,
  Schema,
  Settings,
} from "./types";
import {
  Button,
  ConfirmDialog,
  HeaderHelpContext,
  HelpButton,
  KeypadSlot,
  Notice,
  NumberField,
  NumericProvider,
  useNumeric,
} from "./controls";
import { I18nProvider, useTranslation } from "./i18n";
import { HelpProvider } from "./help/context";
import { HelpGuide } from "./help/Guide";
import { helpProps } from "./help/context";
import { optionHelp, edgeHelp, centerHelp, descriptions } from "./help/text";
import { RoutineDiagram } from "./diagrams";
import {
  angleFeatures,
  angleName,
  centerFeatures,
  edges,
  parameters,
  rotaryConfig,
  routine,
  tabs,
} from "./model";
import { ProbeFlow } from "./ProbeFlow";
import { WcsPicker } from "./WcsPicker";
import { UtilityPage } from "./utilities/UtilityPage";
import "./style.css";

export interface ProbeAppProps {
  client?: ProbeClient;
  defaultLanguage?: string;
  onLeave?: () => void;
  onAlarm?: () => void;
}
export function ProbeApp({
  client: suppliedClient,
  defaultLanguage = "",
  onLeave,
  onAlarm,
}: ProbeAppProps) {
  const client = useMemo(
    () => suppliedClient || new HttpClient(),
    [suppliedClient],
  );
  const [state, setState] = useState<MachineState>({ connected: false });
  const [settings, setSettings] = useState<Settings>({});
  const [schema, setSchema] = useState<Schema>({});
  const [error, setError] = useState("");
  const [settingsError, setSettingsError] = useState("");
  const [loaded, setLoaded] = useState(false);
  const refresh = useCallback(async () => {
    try {
      const state = await client.request<MachineState>("state");
      setState(state);
      setError("");
    } catch (failure) {
      setState({ connected: false });
      setError((failure as Error).message);
    }
  }, [client]);
  useEffect(() => {
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      try {
        const next = await client.request<MachineState>(
          "state",
          undefined,
          controller.signal,
        );
        if (!controller.signal.aborted) {
          setState(next);
          setError("");
        }
      } catch (failure) {
        if (!controller.signal.aborted) {
          setState({ connected: false });
          setError((failure as Error).message);
        }
      }
      if (!controller.signal.aborted)
        timer = setTimeout(() => void poll(), 250);
    }
    void poll();
    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  }, [client]);
  const load = useCallback(async () => {
    try {
      const [values, ranges] = await Promise.all([
        client.request<Settings>("settings.get"),
        client.request<Schema>("settings.schema"),
      ]);
      setSettings(values);
      setSchema(ranges);
      setLoaded(true);
      setSettingsError("");
    } catch (failure) {
      setSettingsError((failure as Error).message);
    }
  }, [client]);
  useEffect(() => {
    void load();
  }, [load]);
  const saveSettings = useCallback(
    async (patch: Partial<Settings>) => {
      const updated = await client.request<Settings>("settings.update", patch);
      setSettings(updated);
      setSettingsError("");
    },
    [client],
  );
  return (
    <div className="pp-app">
      <I18nProvider language={String(settings.language || defaultLanguage)}>
        <NumericProvider>
          <ProbePage
            client={client}
            state={state}
            settings={settings}
            schema={schema}
            loaded={loaded}
            error={error}
            settingsError={settingsError}
            onReload={load}
            saveSettings={saveSettings}
            onRefresh={refresh}
            onLeave={onLeave}
            onAlarm={onAlarm || onLeave}
          />
        </NumericProvider>
      </I18nProvider>
    </div>
  );
}

function ProbePage({
  client,
  state,
  settings,
  schema,
  loaded,
  error,
  settingsError,
  onReload,
  saveSettings,
  onRefresh,
  onLeave,
  onAlarm,
}: {
  client: ProbeClient;
  state: MachineState;
  settings: Settings;
  schema: Schema;
  loaded: boolean;
  error: string;
  settingsError: string;
  onReload: () => Promise<void>;
  saveSettings: (patch: Partial<Settings>) => Promise<void>;
  onRefresh: () => void;
  onLeave?: () => void;
  onAlarm?: () => void;
}) {
  const { t, language } = useTranslation();
  const editor = useNumeric();
  const [tab, setTab] = useState<(typeof tabs)[number]>("Outside");
  const [page, setPage] = useState("home");
  const [help, setHelp] = useState(false);
  const [guide, setGuide] = useState(false);
  const [config, setConfig] = useState<RunConfig | null>(null);
  const [history, setHistory] = useState<HistoryResult | undefined>();
  const [busy, setBusy] = useState(false);
  const [pending, setPending] = useState(false);
  const [actionError, setActionError] = useState("");
  const [leavePrompt, setLeavePrompt] = useState(false);
  const [probeReference, setProbeReference] = useState(true);
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const status = state.status || {};
  const extended = status.probeActuatorKnown && status.probeActuator === 1;
  const actuatorKnown =
    status.probeActuatorKnown &&
    (status.probeActuator === 0 || status.probeActuator === 1);
  const locked = busy || !!state.contactActive || !!state.recoveryFailed;
  useEffect(() => {
    if (!state.connected) editor.cancel();
  }, [state.connected]);
  useEffect(() => {
    if (!loaded || settings.automaticUpdateChecks === false) return;
    const abort = new AbortController();
    client
      .request<{ available: unknown }>(
        "updates.check",
        { development: settings.developmentUpdates === true },
        abort.signal,
      )
      .then((reply) => setUpdateAvailable(!!reply.available))
      .catch(() => {});
    return () => abort.abort();
  }, [
    client,
    loaded,
    settings.automaticUpdateChecks,
    settings.developmentUpdates,
  ]);
  function navigate(next: string) {
    editor.cancel();
    setHelp(false);
    setPage(next);
    setActionError("");
  }
  function leave() {
    if (status.probeActuatorKnown && status.probeActuator !== 0)
      setLeavePrompt(true);
    else onLeave?.();
  }
  async function actuator(extend: boolean) {
    if (pending || locked) return;
    setPending(true);
    setActionError("");
    try {
      await client.request("probe.set", { extended: extend });
      await onRefresh();
      if (!extend && leavePrompt) {
        setLeavePrompt(false);
        onLeave?.();
      }
    } catch (failure) {
      setActionError((failure as Error).message);
    } finally {
      setPending(false);
    }
  }
  function openRoutine(
    family: Family,
    selection: string | (typeof edges)[number],
  ) {
    setHistory(undefined);
    setConfig(routine(settings, family, selection, status.wcs || 54));
    navigate("routine");
  }
  function centerTitle(feature: string) {
    return t(
      feature === "z"
        ? "Z surface"
        : (
            {
              boss: "Boss center",
              block: "Block center",
              hole: "Hole center",
              pocket: "Pocket center",
              "x-ridge": "X ridge center",
              "y-ridge": "Y ridge center",
              "x-valley": "X valley center",
              "y-valley": "Y valley center",
            } as Record<string, string>
          )[feature],
    );
  }
  const toggleHelp = () => {
    editor.cancel();
    if (guide) setGuide(false);
    setHelp(!help);
  };
  const headerHelp = useMemo(
    () => ({
      active: help,
      toggle: toggleHelp,
      guide: () => {
        editor.cancel();
        setGuide(true);
      },
    }),
    [help, guide],
  );
  const body =
    page === "routine" && config ? (
      <ProbeFlow
        key={history?.entry.id || JSON.stringify(config)}
        client={client}
        config={config}
        history={history}
        state={state}
        onBack={() => navigate(history ? "history" : "home")}
        onBusyChange={setBusy}
        onRefresh={onRefresh}
        onAlarm={onAlarm}
        onSetting={(key, value) => saveSettings({ [key]: value })}
      />
    ) : page === "wcs" ? (
      <WcsPicker
        current={status.wcs || 54}
        pending={pending}
        error={actionError}
        onBack={() => navigate("home")}
        onSelect={async (wcs) => {
          setPending(true);
          setActionError("");
          try {
            await client.request("wcs.select", { wcs });
            onRefresh();
            navigate("home");
          } catch (failure) {
            setActionError((failure as Error).message);
          } finally {
            setPending(false);
          }
        }}
      />
    ) : page !== "home" ? (
      <UtilityPage
        page={page as "utilities" | "history" | "repeatability" | "updates"}
        client={client}
        settings={settings}
        onSettings={saveSettings}
        onNavigate={navigate}
        onBack={() =>
          navigate(
            page === "utilities" || page === "updates" ? "home" : "utilities",
          )
        }
        onBusyChange={setBusy}
        onOpenResult={(opened) => {
          setHistory(opened);
          setConfig(opened.entry.config as RunConfig);
          navigate("routine");
        }}
      />
    ) : (
      <>
        <header className="pp-home-header">
          <Button
            className="pp-icon pp-back"
            aria-label={t("Back")}
            helpTitle={t("Back")}
            onClick={leave}
            disabled={locked || pending || state.actuatorPending}
          >
            <ArrowLeft />
          </Button>
          <button
            className="pp-dro pp-mono"
            aria-label={t("Coordinate reference: %1", [
              t(probeReference ? "Probe" : "Tool"),
            ])}
            onClick={() => setProbeReference(!probeReference)}
            {...helpProps(
              t("Coordinates"),
              t(
                "Tap anywhere here to switch between probe ball and tool tip coordinates. Large numbers use the selected WCS; small numbers use G53. With an empty spindle, Tool still uses the last measured tool length offset.",
              ),
            )}
          >
            {["X", "Y", "Z"].map((axis, i) => {
              const position =
                state.coordinates?.[probeReference ? "probe" : "spindle"];
              return (
                <span className="pp-coordinate" key={axis}>
                  <b>{axis}</b>
                  <strong>
                    {state.connected
                      ? position?.workPosition[i]?.toFixed(3) || "--.---"
                      : "--.---"}
                  </strong>
                  <small>
                    {state.connected
                      ? position?.machinePosition[i]?.toFixed(3) || "--.---"
                      : "--.---"}
                  </small>
                </span>
              );
            })}
            <span className="pp-reference">
              {["Probe", "Tool"].map((label, i) => (
                <span
                  className={probeReference === (i === 0) ? "pp-selected" : ""}
                  key={label}
                >
                  {t(label)}
                </span>
              ))}
            </span>
          </button>
          <Button
            className="pp-icon pp-wcs-button"
            helpTitle={t("Work coordinates")}
            onClick={() => navigate("wcs")}
            disabled={locked}
            help={t(
              "Select G54–G59. Each work coordinate system stores its own zero; the selected one is used for readouts and new measurements.",
            )}
          >
            G{status.wcs || 54}
          </Button>
          <HelpButton />
        </header>
        {(error || actionError || settingsError) && (
          <div className="pp-error-row">
            <Notice error>{t(actionError || settingsError || error)}</Notice>
            {settingsError && (
              <Button
                help={t("Try the failed request again.")}
                onClick={() => void onReload()}
              >
                {t("Retry")}
              </Button>
            )}
          </div>
        )}
        <div className="pp-tabs">
          <Button
            className={`pp-actuator ${extended ? "pp-extended" : ""}`}
            aria-label={t("Extend / Retract")}
            aria-pressed={!!extended}
            disabled={
              !state.connected ||
              status.mode !== "Ready" ||
              !actuatorKnown ||
              pending ||
              locked ||
              state.actuatorPending
            }
            onClick={() => void actuator(!extended)}
            help={t(
              "Extend or retract the probe independently of measurements. Keep the ball and shaft clear while the actuator moves.",
            )}
          >
            <span
              className={`pp-actuator-state ${actuatorKnown && !extended ? "pp-current" : ""}`}
            >
              {t("Retracted")}
            </span>
            <span
              className={`pp-actuator-state ${extended ? "pp-current" : ""}`}
            >
              {t("Extended")}
            </span>
          </Button>
          <div
            role="tablist"
            aria-label={t("Probing tabs")}
            {...helpProps(
              t("Probing tabs"),
              t(
                "Choose Outside for stock edges, Inside for pocket walls, Center for opposite sides, Angle for face angles, or Rotary for the fourth axis. Settings contains ball diameter, feeds and utilities.",
              ),
            )}
          >
            {tabs.map((label) => (
              <button
                role="tab"
                key={label}
                className="pp-button pp-tab"
                aria-selected={tab === label}
                disabled={locked}
                onClick={() => {
                  editor.cancel();
                  setTab(label);
                }}
              >
                {t(label)}
                {label === "Settings" && updateAvailable && (
                  <span className="pp-notification" />
                )}
              </button>
            ))}
          </div>
        </div>
        <main
          className={`pp-tabs-body ${tab !== "Settings" && !extended ? "pp-dimmed" : ""}`}
        >
          <div className="pp-columns">
            <KeypadSlot>
              {tab === "Settings" ? (
                <div className="pp-settings-shortcuts">
                  <Button
                    onClick={() => navigate("utilities")}
                    help={t(
                      "Open probe history, log export and the repeatability check.",
                    )}
                  >
                    {t("Utilities")}
                  </Button>
                  <Button
                    helpTitle={t("Check for updates")}
                    onClick={() => navigate("updates")}
                  >
                    {t("Check for updates")}
                    {updateAvailable && <span className="pp-notification" />}
                  </Button>
                  <div
                    className="pp-language-selector"
                    role="group"
                    aria-label={t("Language")}
                    {...helpProps(
                      t("Language"),
                      t(
                        "Choose the language for controls and help. The choice is saved for the next startup.",
                      ),
                    )}
                  >
                    <span className="pp-muted">{t("Language")}</span>
                    <div>
                      {[
                        { code: "en", label: "English" },
                        { code: "zh_CN", label: "简体中文" },
                        { code: "sv", label: "Svenska" },
                      ].map((choice) => (
                        <Button
                          key={choice.code}
                          className={
                            language === choice.code ? "pp-selected" : ""
                          }
                          aria-pressed={language === choice.code}
                          disabled={!loaded || locked}
                          onClick={() =>
                            void saveSettings({ language: choice.code }).catch(
                              (failure) => setActionError(failure.message),
                            )
                          }
                        >
                          {choice.label}
                        </Button>
                      ))}
                    </div>
                  </div>
                </div>
              ) : (
                <div
                  className={`pp-diagram-grid ${tab === "Angle" ? "pp-two-columns" : tab === "Rotary" ? "pp-rotary-grid" : ""}`}
                >
                  {tab === "Outside" || tab === "Inside"
                    ? edges.map((selection, i) => {
                        const family = tab.toLowerCase() as
                          "inside" | "outside";
                        const inside = family === "inside";
                        const selected = inside
                          ? { ...selection, x: -selection.x, y: -selection.y }
                          : selection;
                        const label = selection.z
                          ? "Z surface"
                          : inside
                            ? selected.x && selected.y
                              ? "Inside %1 corner"
                              : "Inside %1 edge"
                            : selected.x && selected.y
                              ? "Outside %1 corner"
                              : "Outside %1 edge";
                        const axes = [
                          selected.x ? `X${selected.x > 0 ? "+" : "-"}` : "",
                          selected.y ? `Y${selected.y > 0 ? "+" : "-"}` : "",
                        ]
                          .filter(Boolean)
                          .join("/");
                        const helpTitle = t(label, [axes]);
                        return (
                          <Button
                            className="pp-routine-button"
                            key={i}
                            aria-label={helpTitle}
                            disabled={
                              !loaded || !state.connected || !extended || locked
                            }
                            helpTitle={helpTitle}
                            help={t(
                              edgeHelp(
                                inside,
                                selection.z,
                                !!(selected.x && selected.y),
                              ),
                            )}
                            onClick={() => openRoutine(family, selected)}
                          >
                            <RoutineDiagram family={family} {...selected} />
                          </Button>
                        );
                      })
                    : tab === "Center"
                      ? centerFeatures.map((feature) => (
                          <Button
                            key={feature}
                            className="pp-routine-button"
                            aria-label={centerTitle(feature)}
                            helpTitle={centerTitle(feature)}
                            disabled={
                              !loaded || !state.connected || !extended || locked
                            }
                            help={t(centerHelp(feature))}
                            onClick={() => openRoutine("center", feature)}
                          >
                            <RoutineDiagram family="center" feature={feature} />
                            <span className="pp-diagram-caption">
                              {t(
                                (
                                  {
                                    boss: "Boss",
                                    block: "Block",
                                    hole: "Hole",
                                    pocket: "Pocket",
                                    "x-ridge": "X ridge",
                                    "y-ridge": "Y ridge",
                                    "x-valley": "X valley",
                                    "y-valley": "Y valley",
                                    z: "Z-",
                                  } as Record<string, string>
                                )[feature],
                              )}
                            </span>
                          </Button>
                        ))
                      : tab === "Angle"
                        ? angleFeatures.map((feature) => (
                            <Button
                              key={feature}
                              className="pp-routine-button"
                              aria-label={t(angleName(feature))}
                              helpTitle={t(angleName(feature))}
                              disabled={
                                !loaded ||
                                !state.connected ||
                                !extended ||
                                locked
                              }
                              help={t(
                                feature.startsWith("z-")
                                  ? descriptions.angleSlope
                                  : descriptions.angleFace,
                              )}
                              onClick={() => openRoutine("angle", feature)}
                            >
                              <RoutineDiagram
                                family="angle"
                                feature={feature}
                              />
                            </Button>
                          ))
                        : [
                            "axis",
                            "vertical",
                            "horizontal",
                            "verticalNegative",
                          ].map((operation) => (
                            <Button
                              key={operation}
                              className="pp-routine-button"
                              aria-label={t(
                                operation === "axis"
                                  ? "Calibrate rotary axis"
                                  : operation === "horizontal"
                                    ? "Level horizontal surface"
                                    : operation === "vertical"
                                      ? "Align vertical surface toward Y+"
                                      : "Align vertical surface toward Y−",
                              )}
                              helpTitle={t(
                                operation === "axis"
                                  ? "Axis center"
                                  : operation === "horizontal"
                                    ? "Level surface"
                                    : operation === "vertical"
                                      ? "Align face Y+"
                                      : "Align face Y−",
                              )}
                              disabled={
                                !loaded ||
                                !state.connected ||
                                !extended ||
                                locked
                              }
                              help={t(
                                operation === "axis"
                                  ? descriptions.rotaryAxis
                                  : operation === "horizontal"
                                    ? descriptions.rotaryHorizontal
                                    : descriptions.rotaryVertical,
                              )}
                              onClick={() => {
                                setHistory(undefined);
                                setConfig(rotaryConfig(settings, operation));
                                navigate("routine");
                              }}
                            >
                              <RoutineDiagram
                                family="rotary"
                                feature={operation}
                              />
                              <span className="pp-diagram-caption">
                                {t(
                                  operation === "axis"
                                    ? "Axis center"
                                    : operation === "horizontal"
                                      ? "Level surface"
                                      : operation === "vertical"
                                        ? "Align face Y+"
                                        : "Align face Y−",
                                )}
                              </span>
                            </Button>
                          ))}
                </div>
              )}
            </KeypadSlot>
            <section
              className="pp-parameters"
              aria-label={t(tab + " parameters")}
            >
              {parameters(tab).map((parameter) => (
                <div className="pp-parameter" key={parameter.key}>
                  <label>{t(parameter.label)}</label>
                  <NumberField
                    label={t(parameter.label)}
                    value={Number(settings[parameter.key]) || 0}
                    min={schema[parameter.key]?.minimum}
                    max={schema[parameter.key]?.maximum}
                    disabled={
                      !loaded ||
                      !state.connected ||
                      locked ||
                      (tab !== "Settings" && !extended)
                    }
                    help={t(optionHelp(parameter.key, tab.toLowerCase()))}
                    onChange={(value) =>
                      saveSettings({ [parameter.key]: value })
                    }
                  />
                  <span className="pp-unit">{parameter.unit || "mm"}</span>
                </div>
              ))}
            </section>
          </div>
        </main>
      </>
    );
  return (
    <HeaderHelpContext.Provider value={headerHelp}>
      <HelpProvider active={help && !guide} onActiveChange={setHelp}>
        <div className="pp-view" hidden={guide}>
          {body}
        </div>
        {guide && (
          <div className="pp-view pp-guide-view">
            <HelpGuide onBack={() => setGuide(false)} />
          </div>
        )}
        {help && !guide && page === "home" && (
          <Button
            className="pp-floating-guide"
            helpTitle={t("Guide")}
            data-help-bypass=""
            style={tab === "Settings" ? { left: 16, right: "auto" } : undefined}
            onClick={() => setGuide(true)}
          >
            <BookOpen />
            {t("Guide")}
          </Button>
        )}
        {leavePrompt && (
          <ConfirmDialog
            title={t("Probe extended")}
            message={
              <>
                {t("Retract the probe before leaving?")}
                {actionError && <Notice error>{t(actionError)}</Notice>}
              </>
            }
            onCancel={() => setLeavePrompt(false)}
            onConfirm={() => void actuator(false)}
            busy={pending}
            confirmLabel={t("Retract and exit")}
            actions={
              <Button
                onClick={() => {
                  setLeavePrompt(false);
                  onLeave?.();
                }}
                disabled={pending}
              >
                {t("Leave extended")}
              </Button>
            }
          />
        )}
      </HelpProvider>
    </HeaderHelpContext.Provider>
  );
}
