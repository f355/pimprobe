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
import { Button, Notice, PageHeader } from "../controls";
import { helpProps } from "../help/context";
import { useTranslation } from "../i18n";
import type { HistoryEntry, HistoryResult } from "../types";
import { errorMessage, useUtilityBusy } from "./common";
import { canOpenHistory, historyDate, historyDetail } from "./history-detail";
import type { UtilityPageProps } from "./UtilityPage";

export function History({
  client,
  onBack,
  onOpenResult,
  onBusyChange,
}: UtilityPageProps) {
  const { t } = useTranslation();
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [loading, setLoading] = useState(true);
  const [opening, setOpening] = useState(false);
  const [error, setError] = useState("");
  const [reload, setReload] = useState(0);
  const openRequest = useRef<AbortController | null>(null);
  const details = useRef<HTMLPreElement>(null);
  const selected = entries.find((entry) => entry.id === selectedId) ?? null;
  useUtilityBusy(opening, onBusyChange);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setError("");
    setEntries([]);
    setSelectedId("");
    client
      .request<HistoryEntry[]>("history.get", undefined, controller.signal)
      .then((reply) => {
        if (controller.signal.aborted) return;
        if (!Array.isArray(reply))
          throw new Error("Could not read probing history");
        setEntries(reply);
        setSelectedId(reply[0]?.id ?? "");
      })
      .catch((failure: unknown) => {
        if (!controller.signal.aborted)
          setError(errorMessage(failure, "Could not read probing history"));
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [client, reload]);
  useEffect(() => () => openRequest.current?.abort(), [client]);
  useEffect(() => {
    if (details.current) details.current.scrollTop = 0;
  }, [selectedId]);

  async function openResult() {
    if (!selected || !canOpenHistory(selected) || openRequest.current) return;
    const controller = new AbortController();
    openRequest.current = controller;
    setOpening(true);
    setError("");
    try {
      const result = await client.request<HistoryResult>(
        "history.open",
        { id: selected.id },
        controller.signal,
      );
      if (!result?.entry || !result.result)
        throw new Error("Could not open this result");
      if (!controller.signal.aborted) onOpenResult(result);
    } catch (failure) {
      if (!controller.signal.aborted)
        setError(errorMessage(failure, "Could not open this result"));
    } finally {
      if (!controller.signal.aborted) {
        openRequest.current = null;
        setOpening(false);
      }
    }
  }

  return (
    <section className="pp-utility-page">
      <PageHeader
        title={t("Probe history")}
        detail={entries.length ? t("%1 attempts", [entries.length]) : ""}
        onBack={onBack}
        backDisabled={opening}
      />
      <div className="pp-utility-content">
        {error && <Notice error>{t(error)}</Notice>}
        {loading ? (
          <div className="pp-utility-empty" role="status">
            {t("Loading...")}
          </div>
        ) : entries.length === 0 ? (
          <div className="pp-utility-empty">
            {!error && <p>{t("No probing attempts yet")}</p>}
            {error && (
              <Button onClick={() => setReload((value) => value + 1)}>
                {t("Try again")}
              </Button>
            )}
          </div>
        ) : (
          <div className="pp-utility-history-layout">
            <div
              className="pp-utility-history-list"
              role="group"
              aria-label={t("Probe history")}
            >
              {entries.map((entry) => (
                <Button
                  key={entry.id}
                  className={`pp-utility-history-row ${selectedId === entry.id ? "pp-utility-selected" : ""}`}
                  aria-pressed={selectedId === entry.id}
                  disabled={opening}
                  onClick={() => setSelectedId(entry.id)}
                  helpTitle={t("Probe history")}
                  help={t(
                    "Select a saved measurement to see its details. Open result restores its measured coordinates and work-zero controls.",
                  )}
                >
                  <span className="pp-utility-history-label">
                    {t(entry.label)}
                  </span>
                  <span
                    className={`pp-utility-history-meta pp-utility-status-${entry.status === "success" ? "success" : entry.status === "failed" ? "failed" : "other"}`}
                  >
                    {historyDate(entry.timestampMs)}
                    {"  "}
                    {t(entry.status)}
                  </span>
                </Button>
              ))}
            </div>
            <div className="pp-utility-history-details">
              <pre
                ref={details}
                className="pp-utility-history-text"
                tabIndex={0}
                {...helpProps(
                  t("Details"),
                  t("Show the saved measurement's options and command log."),
                )}
              >
                {selected && historyDetail(selected, t)}
              </pre>
              <Button
                primary
                disabled={opening || !canOpenHistory(selected)}
                onClick={() => void openResult()}
                help={t(
                  "Open the saved measurement to set work zero again. The stock and machine reference must still match the measurement.",
                )}
              >
                {t("Open result")}
              </Button>
            </div>
          </div>
        )}
      </div>
    </section>
  );
}
