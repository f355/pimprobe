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

import { test, expect } from "@playwright/test";
test.beforeEach(async ({ page }) => {
  await page.route("**/client-harness", (route) =>
    route.fulfill({
      contentType: "text/html",
      body: "<!doctype html><title>Transport test</title>",
    }),
  );
  await page.goto("/client-harness");
});
test("HTTP errors retain application messages and plain server failures", async ({
  page,
}) => {
  await page.route("**/transport/routine/review", (route) =>
    route.fulfill({
      json: {
        error: true,
        code: "probe",
        message: "Extend the probe before probing.",
        status: 409,
      },
    }),
  );
  await page.route("**/transport/settings*", (route) =>
    route.fulfill({
      status: 502,
      contentType: "text/plain",
      body: "Probe service unavailable",
    }),
  );
  const errors = await page.evaluate(async () => {
    const path = "/src/client.ts";
    const { HttpClient } = (await import(
      path
    )) as typeof import("../src/client");
    const client = new HttpClient("/transport");
    const errors: string[] = [];
    for (const operation of ["routine.review", "settings.get"])
      try {
        await client.request(operation, {});
      } catch (error) {
        errors.push((error as Error).message);
      }
    return errors;
  });
  expect(errors).toEqual([
    "Extend the probe before probing.",
    "Probe service unavailable",
  ]);
});
test("stream decoding preserves partial UTF-8 characters and a final line without newline", async ({
  page,
}) => {
  const events = await page.evaluate(async () => {
    const path = "/src/client.ts";
    const { HttpClient } = (await import(
      path
    )) as typeof import("../src/client");
    const bytes = new TextEncoder().encode(
      '{"type":"progress","message":"测量"}\n{"type":"result","value":12.5}',
    );
    window.fetch = async () =>
      new Response(
        new ReadableStream({
          start(controller) {
            for (const byte of bytes)
              controller.enqueue(new Uint8Array([byte]));
            controller.close();
          },
        }),
        { headers: { "content-type": "application/x-ndjson" } },
      );
    const events: unknown[] = [];
    await new HttpClient("/transport").stream(
      "routine.run",
      { id: "result-1" },
      (event) => events.push(event),
    );
    return events;
  });
  expect(events).toEqual([
    { type: "progress", message: "测量" },
    { type: "result", value: 12.5 },
  ]);
});
