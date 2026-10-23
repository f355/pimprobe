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

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import type { ProbeClient } from "./client";
import type { MachineState, Schema, Settings } from "./types";

interface ProbeData {
  client: ProbeClient;
  state: MachineState;
  settings: Settings;
  schema: Schema;
  loaded: boolean;
  error: string;
  settingsError: string;
  reloadSettings: (signal?: AbortSignal) => Promise<void>;
  saveSettings: (patch: Partial<Settings>) => Promise<void>;
  refresh: () => Promise<void>;
}
const ProbeContext = createContext<ProbeData | null>(null);

/** Keep one provider for the lifetime of the connection, outside page navigation. */
export function ProbeProvider({
  client,
  connected = true,
  children,
}: {
  client: ProbeClient;
  connected?: boolean;
  children: ReactNode;
}) {
  const [state, setState] = useState<MachineState>({ connected: false });
  const [settings, setSettings] = useState<Settings>({});
  const [schema, setSchema] = useState<Schema>({});
  const [error, setError] = useState("");
  const [settingsError, setSettingsError] = useState("");
  const [loaded, setLoaded] = useState(false);
  const refresh = useCallback(async () => {
    try {
      setState(await client.request<MachineState>("state"));
      setError("");
    } catch (failure) {
      setState({ connected: false });
      setError((failure as Error).message);
    }
  }, [client]);
  useEffect(() => {
    if (!connected) {
      setState({ connected: false });
      return;
    }
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
  }, [client, connected]);
  const reloadSettings = useCallback(
    async (signal?: AbortSignal) => {
      try {
        const [values, ranges] = await Promise.all([
          client.request<Settings>("settings.get", undefined, signal),
          client.request<Schema>("settings.schema", undefined, signal),
        ]);
        if (signal?.aborted) return;
        setSettings(values);
        setSchema(ranges);
        setLoaded(true);
        setSettingsError("");
      } catch (failure) {
        if (!signal?.aborted) setSettingsError((failure as Error).message);
      }
    },
    [client],
  );
  useEffect(() => {
    if (!connected) {
      setLoaded(false);
      return;
    }
    const controller = new AbortController();
    void reloadSettings(controller.signal);
    return () => controller.abort();
  }, [connected, reloadSettings]);
  const saveSettings = useCallback(
    async (patch: Partial<Settings>) => {
      const updated = await client.request<Settings>("settings.update", patch);
      setSettings(updated);
      setSettingsError("");
    },
    [client],
  );
  return (
    <ProbeContext.Provider
      value={{
        client,
        state,
        settings,
        schema,
        loaded,
        error,
        settingsError,
        reloadSettings,
        saveSettings,
        refresh,
      }}
    >
      {children}
    </ProbeContext.Provider>
  );
}

export function useProbe() {
  const data = useContext(ProbeContext);
  if (!data) throw new Error("ProbeWorkspace needs a ProbeProvider");
  return data;
}
