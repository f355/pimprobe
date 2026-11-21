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
import type { ProbeClient } from "./client";
import {
  Button,
  KeypadSlot,
  Notice,
  NumberField,
  PageHeader,
} from "./controls";
import { useTranslation } from "./i18n";
import "./work-coordinates.css";

interface Presets {
  active: number | null;
  available: boolean;
  fourthAxis: boolean;
  origins: Record<number, number[]>;
  positions: Record<number, number[]>;
  rotations: Record<number, number>;
}

export function WorkCoordinates({
  client,
  onBack,
  onRefresh,
  connected,
}: {
  client: ProbeClient;
  onBack: () => void;
  onRefresh: () => void;
  connected: boolean;
}) {
  const { t } = useTranslation();
  const [state, setState] = useState<Presets>();
  const [selected, setSelected] = useState<number>();
  const [origin, setOrigin] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const wcs = selected ?? state?.active ?? 54;

  useEffect(() => {
    if (!connected || pending) return;
    const controller = new AbortController();
    async function read() {
      try {
        const data = await client.request<Presets>(
          "wcs.get",
          undefined,
          controller.signal,
        );
        if (!controller.signal.aborted) setState(data);
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
  }, [client, connected, pending]);

  async function change(action: () => Promise<unknown>) {
    setPending(true);
    setError("");
    try {
      await action();
      setState(await client.request<Presets>("wcs.get"));
      onRefresh();
    } finally {
      setPending(false);
    }
  }
  const reportError = (error: Error) => setError(error.message);
  function row(name: string, value: number | undefined, axis?: number) {
    const rotation = axis === undefined;
    const degrees = rotation || axis === 3;
    const help = t(
      rotation
        ? "Set the angle of the work axes relative to the machine axes."
        : origin
          ? "Edit where this work zero is in machine coordinates."
          : "Set what this coordinate reads at the current position, without moving the machine.",
    );
    const save = (value: number) =>
      change(() =>
        client.request("wcs.edit", {
          wcs,
          axis,
          value,
          atPosition: !origin && !rotation,
        }),
      );
    const bound = rotation ? 360 : axis === 3 ? 1e9 : 100000;
    const disabled =
      !connected || !state?.available || pending || value === undefined;
    return (
      <div className="pp-work-row" key={name}>
        <span className={degrees ? "pp-work-label-wide" : undefined}>
          {name}
        </span>
        {!degrees && (
          <Button
            aria-label={t("Half %1", [name])}
            help={t("Set this coordinate to half its current value.")}
            disabled={disabled}
            onClick={() => {
              if (value !== undefined) void save(value / 2).catch(reportError);
            }}
          >
            ½
          </Button>
        )}
        <Button
          aria-label={t("Zero %1", [name])}
          help={help}
          disabled={disabled}
          onClick={() => void save(0).catch(reportError)}
        >
          {t("Zero")}
        </Button>
        <NumberField
          label={name}
          decimals={3}
          value={value ?? null}
          min={-bound}
          max={bound}
          help={help}
          disabled={disabled}
          onChange={save}
        />
        <span className="pp-unit">{degrees ? "°" : "mm"}</span>
      </div>
    );
  }

  return (
    <section className="pp-utility-page pp-work-coordinates">
      <PageHeader
        title={t("Work coordinates")}
        detail={`G${wcs}`}
        onBack={onBack}
        backDisabled={pending}
      />
      <div className="pp-utility-content">
        <div className="pp-work-layout">
          <KeypadSlot>
            <div className="pp-work-controls">
              <div
                className="pp-segmented"
                role="group"
                aria-label={t("Work coordinate editing mode")}
              >
                <Button
                  aria-pressed={!origin}
                  disabled={pending}
                  help={t(
                    "Set what the selected work coordinate system reads at the current position, without moving the machine. For example, enter 0 for X to make this position X0.",
                  )}
                  onClick={() => setOrigin(false)}
                >
                  {t("Set position")}
                </Button>
                <Button
                  aria-pressed={origin}
                  disabled={pending}
                  help={t(
                    "Enter the machine coordinates (G53) of work zero. Use this when you already know where work zero is, even if the machine is somewhere else.",
                  )}
                  onClick={() => setOrigin(true)}
                >
                  {t("Origin")}
                </Button>
              </div>
              <div
                className="pp-work-presets"
                role="group"
                aria-label={t("Work coordinates")}
              >
                {[54, 55, 56, 57, 58, 59].map((n) => (
                  <Button
                    key={n}
                    className={n === wcs ? "pp-selected" : ""}
                    aria-pressed={n === wcs}
                    aria-current={state?.active === n || undefined}
                    disabled={pending}
                    help={t("View or edit this work coordinate system.")}
                    onClick={() => setSelected(n)}
                  >
                    G{n}
                  </Button>
                ))}
              </div>
              <Button
                className="pp-work-activate"
                primary
                disabled={
                  !connected ||
                  !state?.available ||
                  pending ||
                  state.active === wcs
                }
                help={t(
                  "Use this work coordinate system for the readout and the next measurement.",
                )}
                onClick={() =>
                  void change(() =>
                    client.request("wcs.select", { wcs }),
                  ).catch(reportError)
                }
              >
                {t("Activate G%1", [wcs])}
              </Button>
            </div>
          </KeypadSlot>
          <div className="pp-work-fields">
            {(state?.fourthAxis ? ["X", "Y", "Z", "A"] : ["X", "Y", "Z"]).map(
              (axis, i) =>
                row(
                  axis,
                  (origin ? state?.origins : state?.positions)?.[wcs]?.[i],
                  i,
                ),
            )}
            {state?.rotations[wcs] !== undefined &&
              row(t("XY rotation"), state.rotations[wcs])}
          </div>
        </div>
        {error && <Notice error>{t(error)}</Notice>}
      </div>
    </section>
  );
}
