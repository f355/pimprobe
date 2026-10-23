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
import { Button, ConfirmDialog, Notice, PageHeader } from "../controls";
import { useTranslation } from "../i18n";
import { errorMessage, useUtilityBusy } from "./common";
import type { UtilityPageProps } from "./UtilityPage";

export function Utilities({
  client,
  onNavigate,
  onBack,
  onBusyChange,
}: UtilityPageProps) {
  const { t } = useTranslation();
  const [pending, setPending] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);
  const [message, setMessage] = useState("");
  const [failure, setFailure] = useState(false);
  const request = useRef<AbortController | null>(null);
  useUtilityBusy(pending, onBusyChange);
  useEffect(() => () => request.current?.abort(), [client]);

  async function logs(operation: "history.export" | "history.clear") {
    if (request.current) return;
    const controller = new AbortController();
    request.current = controller;
    setPending(true);
    setMessage("");
    setFailure(false);
    try {
      if (operation === "history.export") {
        const reply = await client.request<{ relativePath: string }>(
          operation,
          undefined,
          controller.signal,
        );
        if (!reply?.relativePath) throw new Error("Could not export logs");
        if (!controller.signal.aborted)
          setMessage(t("Saved: %1", [reply.relativePath]));
      } else {
        await client.request(operation, undefined, controller.signal);
        if (!controller.signal.aborted) setMessage(t("Logs cleared"));
      }
    } catch (error) {
      if (!controller.signal.aborted) {
        setMessage(
          t(
            errorMessage(
              error,
              operation === "history.export"
                ? "Could not export logs"
                : "Could not clear logs",
            ),
          ),
        );
        setFailure(true);
      }
    } finally {
      if (!controller.signal.aborted) {
        request.current = null;
        setPending(false);
        setConfirmClear(false);
      }
    }
  }

  return (
    <section className="pp-utility-page">
      <PageHeader
        title={t("Utilities")}
        onBack={onBack}
        backDisabled={pending}
      />
      <div className="pp-utility-content pp-utility-menu">
        <Button
          className="pp-utility-history-action"
          disabled={pending}
          onClick={() => onNavigate("history")}
          help={t(
            "Browse saved measurements. Open a successful result to set work zero from it again.",
          )}
        >
          {t("Probe history")}
        </Button>
        <div className="pp-utility-log-actions">
          <Button
            disabled={pending}
            onClick={() => void logs("history.export")}
            help={t(
              "Save probe history and diagnostic logs to a downloadable archive.",
            )}
          >
            {t("Export logs")}
          </Button>
          <Button
            danger
            disabled={pending}
            onClick={() => setConfirmClear(true)}
            help={t(
              "Delete saved probe history and diagnostic logs after confirmation.",
            )}
          >
            {t("Clear logs")}
          </Button>
        </div>
        {message && <Notice error={failure}>{message}</Notice>}
        <div className="pp-utility-spacer" />
        <Button
          className="pp-utility-repeat-action"
          disabled={pending}
          onClick={() => onNavigate("repeatability")}
          help={t(
            "Repeat touches against the L bracket and bed, then compare their spread. Options include homing and probe retraction between repetitions.",
          )}
        >
          {t("Probe repeatability")}
        </Button>
      </div>
      {confirmClear && (
        <ConfirmDialog
          title={t("Clear probing logs?")}
          message={t("Probe history and diagnostic traces will be removed.")}
          confirmLabel={t("Clear logs")}
          danger
          busy={pending}
          onConfirm={() => void logs("history.clear")}
          onCancel={() => setConfirmClear(false)}
        />
      )}
    </section>
  );
}
