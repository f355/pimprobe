// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

import { useEffect, useState } from "react";
import { Crosshair, Square } from "lucide-react";
import { Button, ConfirmDialog, Notice, PageHeader } from "../controls";
import { useTranslation } from "../i18n";
import { BedReferenceDiagram } from "./BedReferenceDiagram";
import { useUtilityBusy } from "./common";
import type { UtilityPageProps } from "./UtilityPage";

interface Reference {
  rawPoint: [number, number, number];
}
interface BedStatus {
  saved: Reference | null;
  busy: boolean;
  positioning: boolean;
  message: string;
  coordinates: [number | null, number | null, number | null];
  reason: string | null;
}

export function BedReference({
  client,
  onBack,
  onBusyChange,
}: UtilityPageProps) {
  const { t } = useTranslation();
  const [state, setState] = useState<BedStatus>();
  const [action, setAction] = useState<"record" | "reference">();
  const [pending, setPending] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [measuring, setMeasuring] = useState(false);
  const [error, setError] = useState("");
  const busy = pending || Boolean(state?.busy);
  const replacing = action === "record" && Boolean(state?.saved);
  useUtilityBusy(busy, onBusyChange);

  useEffect(() => {
    const controller = new AbortController();
    async function read() {
      try {
        const state = await client.request<BedStatus>(
          "bed.get",
          undefined,
          controller.signal,
        );
        if (!controller.signal.aborted) setState(state);
      } catch (error) {
        if (!controller.signal.aborted) setError((error as Error).message);
      }
    }
    void read();
    const interval = setInterval(read, 500);
    return () => {
      controller.abort();
      clearInterval(interval);
    };
  }, [client]);

  async function submit(action: "record" | "measure" | "reference" | "stop") {
    if (action === "stop") setStopping(true);
    else if (action === "measure") setMeasuring(true);
    else setPending(true);
    setAction(undefined);
    setError("");
    try {
      setState(
        await client.request<BedStatus>("bed.action", {
          action,
          confirmed_clear: true,
        }),
      );
    } catch (error) {
      setError((error as Error).message);
    } finally {
      if (action === "stop") setStopping(false);
      else if (action === "measure") setMeasuring(false);
      else setPending(false);
    }
  }

  if (state?.positioning) {
    return (
      <section className="pp-utility-page pp-bed-reference">
        <PageHeader
          title={t("Position the probe")}
          onBack={onBack}
          backDisabled
        />
        <div className="pp-utility-content pp-bed-position">
          <p>
            {t(
              "Lower the probe ball into the counterbore without touching the sides or the screw.",
            )}
          </p>
          <div className="pp-bed-picture">
            <BedReferenceDiagram />
          </div>
          {error && <Notice error>{t(error)}</Notice>}
          <div className="pp-utility-log-actions">
            <Button
              disabled={stopping || measuring}
              onClick={() => void submit("stop")}
            >
              {t("Cancel")}
            </Button>
            <Button
              primary
              disabled={stopping || measuring || !!state.reason}
              onClick={() => void submit("measure")}
            >
              <Crosshair /> {t("Measure")}
            </Button>
          </div>
        </div>
      </section>
    );
  }

  function readings(label: string, values: readonly (number | null)[]) {
    return (
      <div className="pp-bed-readings">
        <strong>{label}</strong>
        {["X", "Y", "Z"].map((axis, i) => (
          <span key={axis}>
            {axis} <output>{values[i]?.toFixed(3) ?? "--"}</output>
          </span>
        ))}
      </div>
    );
  }

  return (
    <section className="pp-utility-page pp-bed-reference">
      <PageHeader
        title={t("Bed reference")}
        onBack={onBack}
        backDisabled={busy}
      />
      <div className="pp-utility-content">
        {readings(t("Saved reference"), state?.saved?.rawPoint ?? [])}
        {readings(t("Latest measurement"), state?.coordinates ?? [])}
        {busy && state?.message && <Notice>{t(state.message)}</Notice>}
        {error ? (
          <Notice error>{t(error)}</Notice>
        ) : (
          !busy && state?.reason && <Notice>{t(state.reason)}</Notice>
        )}
        <div className="pp-utility-log-actions">
          {busy ? (
            <Button
              danger
              disabled={stopping || !state?.busy}
              onClick={() => void submit("stop")}
            >
              <Square /> {t("Stop")}
            </Button>
          ) : (
            <>
              <Button
                disabled={!state || !!state.reason}
                onClick={() => setAction("record")}
                help={t(
                  "Home the machine and jog the probe into the marked hole. Measure saves the reference automatically.",
                )}
              >
                <Crosshair /> {t("Record bed reference")}
              </Button>
              <Button
                primary
                disabled={!state?.saved || !!state.reason}
                onClick={() => setAction("reference")}
                help={t(
                  "Home, measure the hole center and bed surface, then correct machine zero to match the saved reference.",
                )}
              >
                <Crosshair /> {t("Reference from bed")}
              </Button>
            </>
          )}
        </div>
      </div>
      {action && (
        <ConfirmDialog
          title={t(
            action === "record"
              ? replacing
                ? "Replace bed reference?"
                : "Record bed reference?"
              : "Reference from bed?",
          )}
          message={
            <>
              {t("Clear the bed and remove any tool from the spindle.")}
              {replacing && (
                <Notice>
                  {t(
                    "Homing can shift machine zero and affect the tool magazine calibration and saved work offsets.",
                  )}
                </Notice>
              )}
            </>
          }
          confirmLabel={t("Start")}
          onConfirm={() => void submit(action)}
          onCancel={() => setAction(undefined)}
        />
      )}
    </section>
  );
}
