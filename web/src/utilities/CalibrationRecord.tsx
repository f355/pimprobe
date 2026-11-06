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

import { useRef, useState } from "react";
import type { ProbeClient } from "../client";
import type { HistoryEntry, SavedProbeCalibration } from "../types";
import { Button, ConfirmDialog, Notice } from "../controls";
import { useTranslation } from "../i18n";

export function CalibrationRecord({
  entry,
  client,
  onBusyChange,
  onRestored,
}: {
  entry: HistoryEntry;
  client: ProbeClient;
  onBusyChange: (busy: boolean) => void;
  onRestored: (message: string) => Promise<void>;
}) {
  const { t } = useTranslation();
  const change = entry.config as {
    before: SavedProbeCalibration;
    after: SavedProbeCalibration;
  };
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const inFlight = useRef(false);
  async function restore() {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    onBusyChange(true);
    setError("");
    try {
      const restored = await client.request<
        SavedProbeCalibration & { storageWarning?: string }
      >("history.calibration.restore", { id: entry.id });
      setConfirm(false);
      await onRestored(restored.storageWarning || "Calibration restored");
    } catch (failure) {
      setError((failure as Error).message);
      setConfirm(false);
    } finally {
      inFlight.current = false;
      setBusy(false);
      onBusyChange(false);
    }
  }
  const previous = [change.before.diameter, ...change.before.offsets];
  const saved = [change.after.diameter, ...change.after.offsets];
  return (
    <div className="pp-calibration-record">
      <div className="pp-calibration-row pp-muted">
        <span>mm</span>
        <span>{t("Previous")}</span>
        <span>{t("Saved")}</span>
      </div>
      {["Ball", "X", "Y", "Z"].map((axis, i) => (
        <div className="pp-calibration-row" key={axis}>
          <span>{t(axis)}</span>
          <span className="pp-mono">{previous[i].toFixed(4)}</span>
          <span className="pp-mono">{saved[i].toFixed(4)}</span>
        </div>
      ))}
      {error && <Notice error>{t(error)}</Notice>}
      <Button
        primary
        disabled={busy}
        onClick={() => setConfirm(true)}
        help={t(
          "Restore the ball diameter and X/Y/Z offsets from the Previous column. The current calibration is saved in a new history entry first.",
        )}
      >
        {t("Restore previous calibration")}
      </Button>
      {confirm && (
        <ConfirmDialog
          title={t("Restore probe calibration?")}
          message={t("Restore the previous ball diameter and X/Y/Z offsets?")}
          confirmLabel={t("Restore")}
          busy={busy}
          onCancel={() => setConfirm(false)}
          onConfirm={() => void restore()}
        />
      )}
    </div>
  );
}
