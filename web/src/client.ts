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

export interface ProbeClient {
  request<T>(
    operation: string,
    args?: object,
    signal?: AbortSignal,
  ): Promise<T>;
  stream<T>(
    operation: string,
    args: object,
    onEvent: (event: T) => void,
    signal?: AbortSignal,
  ): Promise<void>;
}
const routes: Record<string, [string, string]> = {
  state: ["GET", "/state"],
  "settings.get": ["GET", "/settings"],
  "settings.schema": ["GET", "/settings/schema"],
  "settings.update": ["PATCH", "/settings"],
  "probe.set": ["POST", "/probe-actuator"],
  "wcs.select": ["POST", "/wcs"],
  "history.get": ["GET", "/logs/history"],
  "history.open": ["POST", "/logs/history/open"],
  "history.wcs": ["POST", "/logs/history/wcs"],
  "history.zero": ["POST", "/logs/history/zero"],
  "history.rotation": ["POST", "/logs/history/rotation"],
  "history.calibration.restore": ["POST", "/logs/history/calibration"],
  "history.export": ["POST", "/logs/export"],
  "history.clear": ["POST", "/logs/clear"],
  "updates.check": ["GET", "/updates/check"],
  "updates.install": ["POST", "/updates/install"],
  "updates.status": ["GET", "/updates/status"],
};
for (const group of ["routine", "rotary"]) {
  for (const action of [
    "review",
    "run",
    "zero",
    "wcs",
    "rotation",
    ...(group === "routine" ? ["return", "measured", "calibration"] : []),
  ]) {
    routes[`${group}.${action}`] = ["POST", `/${group}/${action}`];
  }
}

export class ServiceError extends Error {
  constructor(
    message: string,
    readonly code = "request",
  ) {
    super(message);
  }
}
export class HttpClient implements ProbeClient {
  constructor(readonly base = "/api/v1") {}
  private async fetch(operation: string, args?: object, signal?: AbortSignal) {
    const route = routes[operation];
    if (!route) throw new Error(`Unknown operation: ${operation}`);
    const [method, path] = route;
    const query =
      method === "GET" && args
        ? "?" +
          new URLSearchParams(
            Object.entries(args).map(([key, value]) => [key, String(value)]),
          )
        : "";
    const response = await fetch(this.base + path + query, {
      method,
      signal,
      headers:
        method === "GET" ? undefined : { "Content-Type": "application/json" },
      body: method === "GET" ? undefined : JSON.stringify(args || {}),
      cache: "no-store",
    });
    if (
      !response.ok ||
      response.headers.get("content-type")?.includes("application/json")
    ) {
      const text = await response.text();
      let value;
      if (response.headers.get("content-type")?.includes("application/json")) {
        try {
          value = text ? JSON.parse(text) : null;
        } catch {
          throw new ServiceError("Invalid service response");
        }
      } else if (!response.ok) {
        throw new ServiceError(text.trim() || response.statusText);
      }
      if (!response.ok || value?.error)
        throw new ServiceError(
          value?.message || response.statusText,
          value?.code,
        );
      return { response, value };
    }
    return { response, value: undefined };
  }
  async request<T>(
    operation: string,
    args?: object,
    signal?: AbortSignal,
  ): Promise<T> {
    const { value } = await this.fetch(operation, args, signal);
    return value as T;
  }
  async stream<T>(
    operation: string,
    args: object,
    onEvent: (event: T) => void,
    signal?: AbortSignal,
  ) {
    const { response } = await this.fetch(operation, args, signal);
    if (!response.body) throw new Error("No operation response");
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let pending = "";
    try {
      while (true) {
        const { value, done } = await reader.read();
        pending += decoder.decode(value, { stream: !done });
        let end: number;
        while ((end = pending.indexOf("\n")) >= 0) {
          const line = pending.slice(0, end).trim();
          pending = pending.slice(end + 1);
          if (line) onEvent(JSON.parse(line));
        }
        if (done) break;
      }
      if (pending.trim()) onEvent(JSON.parse(pending));
    } finally {
      await reader.cancel();
      reader.releaseLock();
    }
  }
}
