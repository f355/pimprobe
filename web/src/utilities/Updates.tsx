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
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Button, ConfirmDialog, Notice, PageHeader, Switch } from "../controls";
import { helpProps } from "../help/context";
import { useTranslation } from "../i18n";
import { errorMessage, useUtilityBusy } from "./common";
import type { UtilityPageProps } from "./UtilityPage";

type AvailableUpdate = {
  token: string;
  version: string;
  name: string;
  notes: string;
  development: boolean;
};
type UpdateCheck = {
  currentVersion: string;
  available: AvailableUpdate | null;
};
type InstallStatus = { state: string; message: string };
type Phase = "loading" | "available" | "current" | "installing" | "error";

export function Updates({
  client,
  settings,
  onSettings,
  onBack,
  onBusyChange,
}: UtilityPageProps) {
  const { t } = useTranslation();
  const [phase, setPhase] = useState<Phase>("loading");
  const [currentVersion, setCurrentVersion] = useState("");
  const [available, setAvailable] = useState<AvailableUpdate | null>(null);
  const [error, setError] = useState("");
  const [preferenceError, setPreferenceError] = useState("");
  const [operationId, setOperationId] = useState("");
  const [statusMessage, setStatusMessage] = useState("");
  const [confirmInstall, setConfirmInstall] = useState(false);
  const [reload, setReload] = useState(0);
  const [saving, setSaving] = useState(false);
  const [development, setDevelopment] = useState(
    settings.developmentUpdates === true,
  );
  const [automatic, setAutomatic] = useState(
    settings.automaticUpdateChecks !== false,
  );
  const installing = useRef(false);
  const preferencePending = useRef(false);
  const installRequest = useRef<AbortController | null>(null);
  useUtilityBusy(phase === "installing", onBusyChange);
  useEffect(
    () => setDevelopment(settings.developmentUpdates === true),
    [settings.developmentUpdates],
  );
  useEffect(
    () => setAutomatic(settings.automaticUpdateChecks !== false),
    [settings.automaticUpdateChecks],
  );
  useEffect(() => () => installRequest.current?.abort(), [client]);

  useEffect(() => {
    if (installing.current) return;
    const controller = new AbortController();
    setPhase("loading");
    setError("");
    setAvailable(null);
    setConfirmInstall(false);
    client
      .request<UpdateCheck>("updates.check", { development }, controller.signal)
      .then((reply) => {
        if (controller.signal.aborted) return;
        if (!reply || typeof reply.currentVersion !== "string")
          throw new Error("Invalid update response");
        if (reply.available && !reply.available.token)
          throw new Error("Invalid update response");
        setCurrentVersion(reply.currentVersion || "unknown");
        setAvailable(reply.available ?? null);
        setPhase(reply.available ? "available" : "current");
      })
      .catch((failure: unknown) => {
        if (!controller.signal.aborted) {
          setError(errorMessage(failure, "Invalid update response"));
          setPhase("error");
        }
      });
    return () => controller.abort();
  }, [client, development, reload]);

  useEffect(() => {
    if (!operationId) return;
    const controller = new AbortController();
    let timer = window.setTimeout(poll, 1000);
    const deadline = window.setTimeout(
      () => fail("The installer did not report completion"),
      90_000,
    );

    function fail(message: string) {
      controller.abort();
      installing.current = false;
      setOperationId("");
      setError(message);
      setPhase("error");
    }
    async function poll() {
      try {
        const status = await client.request<InstallStatus>(
          "updates.status",
          { id: operationId },
          controller.signal,
        );
        if (controller.signal.aborted) return;
        if (status?.state === "failed") {
          fail(status.message || "Installation failed");
          return;
        }
        if (status?.message) setStatusMessage(status.message);
      } catch {
        // The service can disappear while the installer restarts the interface.
      }
      if (!controller.signal.aborted) timer = window.setTimeout(poll, 1000);
    }
    return () => {
      controller.abort();
      window.clearTimeout(timer);
      window.clearTimeout(deadline);
    };
  }, [client, operationId]);

  async function preference(
    key: "developmentUpdates" | "automaticUpdateChecks",
    value: boolean,
  ) {
    if (installing.current || preferencePending.current) return;
    preferencePending.current = true;
    setSaving(true);
    setPreferenceError("");
    try {
      await onSettings({ [key]: value });
      if (key === "developmentUpdates") setDevelopment(value);
      else setAutomatic(value);
    } catch (failure) {
      setPreferenceError(errorMessage(failure, "Could not save settings"));
    } finally {
      preferencePending.current = false;
      setSaving(false);
    }
  }

  async function install() {
    if (
      !available ||
      phase !== "available" ||
      installing.current ||
      preferencePending.current
    )
      return;
    installing.current = true;
    const controller = new AbortController();
    installRequest.current = controller;
    setConfirmInstall(false);
    setPhase("installing");
    setError("");
    setStatusMessage("");
    try {
      const reply = await client.request<{ operationId: string }>(
        "updates.install",
        { token: available.token },
        controller.signal,
      );
      if (controller.signal.aborted) return;
      if (!reply?.operationId)
        throw new Error("The updater did not return an operation ID");
      setOperationId(reply.operationId);
    } catch (failure) {
      if (!controller.signal.aborted) {
        installing.current = false;
        setError(errorMessage(failure, "Installation failed"));
        setPhase("error");
      }
    } finally {
      if (installRequest.current === controller) installRequest.current = null;
    }
  }

  return (
    <section className="pp-utility-page">
      <PageHeader
        title={t("Software update")}
        onBack={() => {
          if (!installing.current) onBack();
        }}
        backDisabled={phase === "installing"}
      />
      <div className="pp-utility-content pp-utility-updates">
        <div className="pp-utility-installed">
          {t("Installed: %1", [
            currentVersion === "unknown"
              ? t("unknown")
              : currentVersion || "--",
          ])}
        </div>
        <div className="pp-utility-update-switches">
          <Switch
            label={t("Check automatically")}
            checked={automatic}
            disabled={saving || phase === "installing"}
            help={t(
              "Check the selected update channel at each startup. A marker appears on Settings when an update is available.",
            )}
            onChange={(value) =>
              void preference("automaticUpdateChecks", value)
            }
          />
          <Switch
            label={t("Development releases")}
            checked={development}
            disabled={saving || phase === "installing"}
            help={t(
              "Choose development builds instead of regular releases. The selected channel is saved and checked at startup.",
            )}
            onChange={(value) => void preference("developmentUpdates", value)}
          />
        </div>
        {preferenceError && <Notice error>{t(preferenceError)}</Notice>}
        {phase === "available" && available ? (
          <>
            <h2 className="pp-utility-release-title">
              {available.name || available.version}
            </h2>
            <article
              className="pp-utility-release-notes"
              tabIndex={0}
              aria-label={t("Software update")}
              {...helpProps(
                t("Software update"),
                t(
                  "Download and install the version shown here, then restart the interface.",
                ),
              )}
            >
              <Markdown
                remarkPlugins={[remarkGfm]}
                skipHtml
                components={{
                  a: ({ children, href }) => (
                    <a href={href} target="_blank" rel="noreferrer noopener">
                      {children}
                    </a>
                  ),
                }}
              >
                {available.notes || ""}
              </Markdown>
            </article>
          </>
        ) : (
          <div
            className={`pp-utility-update-state ${phase === "error" ? "pp-utility-failure" : ""}`}
            role={phase === "error" ? "alert" : "status"}
          >
            <p>
              {t(
                phase === "loading"
                  ? "Checking for updates..."
                  : phase === "current"
                    ? development
                      ? "You have the latest development build."
                      : "You have the latest release."
                    : phase === "installing"
                      ? "Installing update. The interface will restart."
                      : error,
              )}
            </p>
            {phase === "installing" && (
              <>
                <progress
                  aria-label={t(
                    "Installing update. The interface will restart.",
                  )}
                />
                {statusMessage && (
                  <p className="pp-utility-muted">{t(statusMessage)}</p>
                )}
              </>
            )}
          </div>
        )}
        {(phase === "available" || phase === "error") && (
          <footer className="pp-utility-update-actions">
            {phase === "error" ? (
              <Button
                onClick={() => setReload((value) => value + 1)}
                disabled={saving}
                help={t(
                  "Check GitHub again for the latest version on the selected channel.",
                )}
              >
                {t("Try again")}
              </Button>
            ) : (
              <Button
                primary
                disabled={saving}
                onClick={() => setConfirmInstall(true)}
                help={t(
                  "Download and install the version shown here, then restart the interface.",
                )}
              >
                {t("Install update")}
              </Button>
            )}
          </footer>
        )}
      </div>
      {confirmInstall && (
        <ConfirmDialog
          title={t("Install update")}
          message={t(
            "Make sure the machine is idle and the spindle is stopped. Install this update now?",
          )}
          confirmLabel={t("Install")}
          busy={saving || phase === "installing"}
          onConfirm={() => void install()}
          onCancel={() => setConfirmInstall(false)}
        />
      )}
    </section>
  );
}
