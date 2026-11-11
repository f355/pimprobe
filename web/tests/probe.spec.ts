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

import { test, expect, type Locator, type Page } from "@playwright/test";
import type { RoutineConfig, RoutineResult, Settings } from "../src/types";
import swedish from "../src/locales/sv.json" with { type: "json" };
import chinese from "../src/locales/zh_CN.json" with { type: "json" };

async function app(page: Page, language = "en_US") {
  const calls: { path: string; body: Record<string, unknown> }[] = [];
  let settings: Settings = {
    probeDiameter: 2,
    retractDistance: 0.5,
    positioningFeed: 1000,
    coarseFeed: 300,
    fineFeed: 50,
    safeZOffset: 40,
    outsideXSearchDistance: 10,
    outsideYSearchDistance: 10,
    outsideDepth: 5,
    insideXSearchDistance: 10,
    insideYSearchDistance: 10,
    insideDepth: 5,
    centerXSearchDistance: 20,
    centerYSearchDistance: 20,
    centerDepth: 5,
    anglePointSpacing: 20,
    angleSearchDistance: 10,
    rotaryRodDiameter: 10,
    rotaryXDistance: 30,
    rotaryYDistance: 10,
    rotaryZDistance: 10,
    rotaryFeed: 360,
    developmentUpdates: false,
    automaticUpdateChecks: false,
    language,
  };
  let extended = true,
    connected = true,
    warning = false,
    miss = false;
  let config: RoutineConfig;
  let result: RoutineResult = {
    point: [5, 6, null],
    machinePoint: [-150, -100, null],
    rawPoint: [-92, -92, null],
    spans: [null, null, null],
    rawSpans: [null, null, null],
    wcs: 54,
    zeroed: false,
    returned: false,
    positioned: false,
  };
  await page.route("**/api/v1/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname.replace("/api/v1/", "");
    const body = request.postDataJSON() || {};
    calls.push({ path, body });
    let response: unknown = {};
    let ndjson = false;
    if (path === "state")
      response = {
        connected,
        contactActive: false,
        recoveryFailed: false,
        coordinates: {
          probe: {
            machinePosition: [-177.628, -102.204, -90, 0],
            workPosition: [-42.5, 8, -35, 0],
          },
          spindle: {
            machinePosition: [-120.128, -95.204, -50, 0],
            workPosition: [15, 15, 5, 0],
          },
        },
        status: {
          mode: "Ready",
          wcs: 54,
          probeActuatorKnown: true,
          probeActuator: extended ? 1 : 0,
        },
      };
    else if (path === "settings/schema")
      response = Object.fromEntries(
        Object.keys(settings).map((key) => [
          key,
          { minimum: key === "rotaryXDistance" ? -200 : 0.01, maximum: 100000 },
        ]),
      );
    else if (path === "settings") {
      if (request.method() === "PATCH") settings = { ...settings, ...body };
      response = settings;
    } else if (path === "probe-actuator") extended = body.extended;
    else if (path === "routine/review") {
      config = body;
      response = {
        id: "review-1",
        config,
        program: ["G21 G94 G91", "G38.2 X-10 F300"],
        simulated: true,
      };
    } else if (
      path === "routine/run" ||
      path === "routine/return" ||
      path === "routine/measured"
    ) {
      if (config.family === "center") {
        result.spans = [20, 30, null];
        result.rawSpans = [18, 28, null];
      }
      if (config.z) {
        result.machinePoint = [null, null, -105];
        result.rawPoint = [null, null, -65];
        result.point = [null, null, -2];
      }
      if (path === "routine/return") result.returned = true;
      if (path === "routine/measured") result.positioned = true;
      const terminal = miss
        ? {
            type: "error",
            code: "probe_miss",
            message: "The probe did not touch within the search distance.",
          }
        : warning
          ? {
              type: "error",
              code: "log",
              message: "Could not save probing log",
              result,
            }
          : { type: "result", result };
      response =
        [
          {
            type: "progress",
            progress: { kind: "script", message: "G38.2 X-10 F300" },
          },
          terminal,
        ]
          .map((value) => JSON.stringify(value))
          .join("\n") + "\n";
      ndjson = true;
    } else if (path === "routine/zero") {
      result = { ...result, zeroed: true };
      response = result;
    } else if (path === "routine/calibration") {
      const options = body.options;
      const ballAxes = result.rawSpans.slice(0, 2).map((v) => v !== null);
      const estimates = options.dimensions.map((v: number | null, i: number) =>
        v === null ? null : v - result.rawSpans[i]!,
      );
      const values = estimates.filter((v: number | null) => v !== null);
      const proposedDiameter = values.length
        ? values.reduce((a: number, b: number) => a + b, 0) / values.length
        : null;
      const diameter = options.ball ? proposedDiameter : null;
      if (body.save && diameter !== null) settings.probeDiameter = diameter;
      response = {
        currentDiameter: settings.probeDiameter,
        currentOffsets: [-57.5, -7, -50],
        ballAxes,
        ballEstimates: estimates,
        offsetAxes: result.machinePoint.map(
          (v, i) =>
            v !== null &&
            !result.zeroed &&
            (i === 2 || config.family === "center"),
        ),
        proposedDiameter,
        proposedOffsets: result.machinePoint.map((v, i) =>
          v !== null && !result.zeroed ? [1.5, -2.5, -51][i] : null,
        ),
        diameter,
        offsets: options.offsets.map((v: boolean, i: number) =>
          v ? [1.5, -2.5, -51][i] : null,
        ),
        result,
      };
    } else if (path === "routine/wcs") {
      result = { ...result, wcs: body.wcs };
      response = result;
    } else if (path === "logs/history") response = [];
    else if (path === "updates/check")
      response = { currentVersion: "2026.10.0", available: null };
    await route.fulfill({
      contentType: ndjson ? "application/x-ndjson" : "application/json",
      body: ndjson ? (response as string) : JSON.stringify(response),
    });
  });
  await page.goto("/");
  await expect(page.locator(".pp-routine-button").first()).toBeEnabled();
  await page.evaluate(() => document.fonts.ready);
  return {
    calls,
    setResult: (value: RoutineResult) => (result = value),
    setExtended: (value: boolean) => (extended = value),
    setConnected: (value: boolean) => (connected = value),
    warn: () => (warning = true),
    miss: () => (miss = true),
  };
}
test("calibration separates diameter from offsets and shows the mean of both dimensions", async ({
  page,
}, info) => {
  const machine = await app(page);
  await page.getByRole("tab", { name: "Center", exact: true }).click();
  await page.locator(".pp-routine-button").first().click();
  await page.getByRole("button", { name: "Proceed", exact: true }).click();
  const calibrate = page.getByRole("button", {
    name: "Calibrate probe...",
    exact: true,
  });
  await expect(calibrate).toBeInViewport({ ratio: 1 });
  const calibrateBox = (await calibrate.boundingBox())!;
  const logBox = (await page
    .getByRole("button", { name: "Log", exact: true })
    .boundingBox())!;
  expect(calibrateBox.y + calibrateBox.height).toBeLessThanOrEqual(
    logBox.y - 8,
  );
  await capture(page, info.outputPath("calibration-result.png"));
  await page
    .getByRole("button", { name: "Calibrate probe...", exact: true })
    .click();
  await expect(
    page.getByRole("tab", { name: "Ball diameter", exact: true }),
  ).toBeEnabled();
  await expect(
    page.getByRole("button", { name: "Save ball diameter", exact: true }),
  ).toBeDisabled();
  await page.getByRole("tab", { name: "X/Y offsets", exact: true }).click();
  await expect(page.getByLabel("Calibration values")).toContainText("1.5000");
  await expect(
    page.getByRole("button", { name: "Save X/Y offsets", exact: true }),
  ).toBeEnabled();
  await expect(page.getByRole("textbox")).toHaveCount(0);
  await capture(page, info.outputPath("calibration-offsets.png"));
  await page.getByRole("tab", { name: "Ball diameter", exact: true }).click();
  await capture(page, info.outputPath("calibration.png"));
  for (const [axis, value] of [
    ["X", "20.2"],
    ["Y", "30.4"],
  ]) {
    const field = page.getByRole("textbox", {
      name: `Known ${axis} size`,
      exact: true,
    });
    await field.click();
    await field.fill(value);
    await capture(page, info.outputPath(`calibration-edit-${axis}.png`));
    await field.press("Enter");
  }
  await expect(page.getByLabel("Calibration values")).toContainText("2.3000");
  await expect(
    page.getByText("New diameter = mean of X and Y", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "X", exact: true }).click();
  await expect(page.getByLabel("Calibration values")).toContainText("2.2000");
  await page.getByRole("button", { name: "X+Y", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Save ball diameter", exact: true }),
  ).toBeEnabled();
  await expect(page.getByLabel("Calibration values")).toContainText("2.3000");
  await capture(page, info.outputPath("calibration-proposal.png"));
  // Hosts may reserve more height for navigation than the standalone header.
  await page.setViewportSize({ width: 800, height: 432 });
  const values = (await page.getByLabel("Calibration values").boundingBox())!;
  const save = page.getByRole("button", {
    name: "Save ball diameter",
    exact: true,
  });
  expect(values.y + values.height).toBeLessThanOrEqual(
    (await save.boundingBox())!.y - 8,
  );
  await capture(page, info.outputPath("calibration-short-host.png"));
  await page.setViewportSize({ width: 800, height: 480 });
  await page
    .getByRole("button", { name: "Save ball diameter", exact: true })
    .click();
  await expect(page.getByRole("status")).toContainText("Calibration saved");
  const saves = machine.calls.filter(
    (r) => r.path === "routine/calibration" && r.body.save,
  );
  expect(saves).toHaveLength(1);
  expect(saves[0].body).toEqual({
    id: "review-1",
    save: true,
    options: {
      ball: true,
      dimensions: [20.2, 30.4],
      offsets: [false, false, false],
    },
  });
  expect(machine.calls.filter((r) => r.path === "routine/zero")).toHaveLength(
    0,
  );
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await page.getByRole("tab", { name: "Settings", exact: true }).click();
  await expect(
    page.getByRole("textbox", { name: "Probe ball diameter", exact: true }),
  ).toHaveValue("2.3");
});

test("signed calibration values remain separated at the touchscreen size", async ({
  page,
}, info) => {
  await app(page);
  await page.route("**/api/v1/routine/calibration", (route) =>
    route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        currentDiameter: 2,
        currentOffsets: [-999.9999, 999.9999, -1000],
        ballAxes: [false, false],
        ballEstimates: [null, null],
        offsetAxes: [false, false, true],
        proposedDiameter: null,
        proposedOffsets: [null, null, -108.1234],
        diameter: null,
        offsets: [null, null, -108.1234],
        result: {},
      }),
    }),
  );
  await page.getByRole("button", { name: "Z surface", exact: true }).click();
  await page.getByRole("button", { name: "Proceed", exact: true }).click();
  await page
    .getByRole("button", { name: "Calibrate probe...", exact: true })
    .click();
  const row = page
    .getByRole("table", { name: "Calibration comparison", exact: true })
    .getByRole("row")
    .last();
  await expect(row).toContainText("-108.1234");
  const spans = await row
    .locator(":scope > span, :scope > strong")
    .evaluateAll((elements) =>
      elements
        .map((e) => {
          const r = e.getBoundingClientRect();
          return {
            left: r.left,
            right: r.right,
            content: e.scrollWidth,
            width: e.clientWidth,
          };
        })
        .sort((a, b) => a.left - b.left),
    );
  for (let i = 0; i < spans.length; i++) {
    expect(spans[i].content).toBeLessThanOrEqual(spans[i].width + 1);
    if (i) expect(spans[i].left - spans[i - 1].right).toBeGreaterThanOrEqual(7);
  }
  await capture(page, info.outputPath("calibration-z.png"));
});

test("ball calibration remains available after zeroing and explains the combined estimate", async ({
  page,
}, info) => {
  await app(page);
  await page.getByRole("tab", { name: "Center", exact: true }).click();
  await page.locator(".pp-routine-button").first().click();
  await page.getByRole("button", { name: "Proceed", exact: true }).click();
  await page
    .getByRole("button", { name: "Set Work Zero", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Calibrate probe...", exact: true })
    .click();
  await expect(
    page.getByRole("textbox", { name: "Known X size", exact: true }),
  ).toBeEnabled();
  await page.locator(".pp-help-button").click();
  await page
    .getByText("New diameter = mean of X and Y", { exact: true })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Mean of X and Y", exact: true }),
  ).toContainText("arithmetic mean");
  await capture(page, info.outputPath("calibration-help.png"));
});

test("calibration history restores previous values only after confirmation", async ({
  page,
}, info) => {
  await app(page);
  const before = { diameter: 1.829, offsets: [-55.872, 0.239, -49.98] };
  const after = { diameter: 1.9, offsets: [-55.8, 0.3, -50] };
  const entry = {
    id: "calibration-1",
    timestampMs: 1794384000000,
    category: "calibration",
    label: "Probe calibration",
    status: "success",
    config: { before, after },
  };
  let restores = 0;
  await page.route("**/api/v1/logs/history", (route) =>
    route.fulfill({ json: [entry] }),
  );
  await page.route("**/api/v1/logs/history/calibration", (route) => {
    expect(route.request().postDataJSON()).toEqual({ id: entry.id });
    restores++;
    return route.fulfill({ json: before });
  });
  await page.getByRole("tab", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Utilities", exact: true }).click();
  await page
    .getByRole("button", { name: "Probe history", exact: true })
    .click();
  const restore = page.getByRole("button", {
    name: "Restore previous calibration",
    exact: true,
  });
  await expect(restore).toBeInViewport({ ratio: 1 });
  await expect(page.locator(".pp-calibration-record")).toContainText("1.8290");
  await capture(page, info.outputPath("calibration-history.png"));
  await restore.click();
  const dialog = page.getByRole("dialog", {
    name: "Restore probe calibration?",
    exact: true,
  });
  await expect(dialog).toBeVisible();
  await capture(page, info.outputPath("calibration-restore.png"));
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  expect(restores).toBe(0);
  await restore.click();
  await dialog.getByRole("button", { name: "Restore", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("Calibration restored");
  expect(restores).toBe(1);
});

for (const [language, catalog] of [
  ["sv", swedish],
  ["zh_CN", chinese],
] as const) {
  test(`calibration controls fit the touchscreen in ${language}`, async ({
    page,
  }, info) => {
    const t = (key: string) => (catalog as Record<string, string>)[key] || key;
    await app(page, language);
    await page.getByRole("tab", { name: t("Center"), exact: true }).click();
    await page.locator(".pp-routine-button").first().click();
    await page.getByRole("button", { name: t("Proceed"), exact: true }).click();
    await page
      .getByRole("button", { name: t("Calibrate probe..."), exact: true })
      .click();
    await expect(
      page.getByRole("textbox", {
        name: t("Known %1 size").replace("%1", "X"),
        exact: true,
      }),
    ).toBeEnabled();
    await capture(page, info.outputPath("calibration-ball.png"));
    await page
      .getByRole("tab", { name: t("X/Y offsets"), exact: true })
      .click();
    await expect(
      page.getByRole("button", { name: t("Save X/Y offsets"), exact: true }),
    ).toBeInViewport({ ratio: 1 });
    await capture(page, info.outputPath("calibration-offsets.png"));
  });
}

async function capture(page: Page, path: string) {
  await expect(page.locator(".pp-app")).toBeInViewport({ ratio: 1 });
  expect(await page.evaluate(() => document.documentElement.scrollHeight)).toBe(
    page.viewportSize()!.height,
  );
  await page.screenshot({ path });
}

async function explain(
  page: Page,
  target: Locator,
  title: string,
  text: string | RegExp,
) {
  await target.click();
  const explanation = page.getByRole("dialog", { name: title, exact: true });
  await expect(explanation).toBeVisible();
  await expect(explanation).toContainText(text);
  const bounds = (await explanation.boundingBox())!;
  const viewport = page.viewportSize()!;
  expect(bounds.x).toBeGreaterThanOrEqual(0);
  expect(bounds.y).toBeGreaterThanOrEqual(0);
  expect(bounds.x + bounds.width).toBeLessThanOrEqual(viewport.width + 1);
  expect(bounds.y + bounds.height).toBeLessThanOrEqual(viewport.height + 1);
  await expect(page.locator(".pp-keypad")).toBeHidden();
  await explanation.getByRole("button").click();
}

test.describe("startup", () => {
  test.use({ javaScriptEnabled: false });
  test("paints the dark background before the application loads", async ({
    page,
  }, info) => {
    await page.goto("/");
    await expect(page.locator("html")).toHaveCSS(
      "background-color",
      "rgb(32, 36, 37)",
    );
    await expect(page.locator("body")).toHaveCSS("margin", "0px");
    await page.screenshot({ path: info.outputPath("startup.png") });
  });
});

test("help explains Back on home and review without exiting or navigating", async ({
  page,
}) => {
  const m = await app(page);
  await page.locator(".pp-help-button").click();
  await expect(
    page.getByRole("button", { name: "Exit help", exact: true }),
  ).toBeVisible();
  await expect(page.locator(".pp-help-button .lucide-x")).toBeVisible();
  await explain(
    page,
    page.locator(".pp-back"),
    "Back",
    /When leaving with the probe extended/,
  );
  await expect(page.locator(".pp-home-header")).toBeVisible();
  await expect(page.locator("dialog:modal")).toHaveCount(0);
  expect(m.calls.filter((call) => call.path === "probe-actuator")).toHaveLength(
    0,
  );
  await page.getByRole("button", { name: "Exit help", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Help", exact: true }),
  ).toBeVisible();
  await page.locator(".pp-routine-button").first().click();
  await page.locator(".pp-help-button").click();
  await expect(
    page.getByRole("button", { name: "Exit help", exact: true }),
  ).toBeVisible();
  await expect(page.locator(".pp-help-button .lucide-x")).toBeVisible();
  await explain(
    page,
    page.locator(".pp-back"),
    "Back",
    /Return to the previous page/,
  );
  await expect(
    page.getByRole("button", { name: "Proceed", exact: true }),
  ).toBeVisible();
  await expect(page.locator(".pp-help-button")).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await page.getByRole("button", { name: "Exit help", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Help", exact: true }),
  ).toBeVisible();
});

for (const tab of ["Outside", "Angle", "Rotary"]) {
  test(`help explains ${tab} review parameters without editing`, async ({
    page,
  }) => {
    const m = await app(page);
    await page.getByRole("tab", { name: tab, exact: true }).click();
    await page.locator(".pp-routine-button").first().click();
    await page.locator(".pp-help-button").click();
    const cases =
      tab === "Angle"
        ? ([
            ["Point spacing", /Distance along the face/],
            ["Search distance", /Maximum travel toward the face/],
          ] as const)
        : tab === "Rotary"
          ? ([
              ["Rod diameter", /Approximate rod diameter/],
              ["X distance", /Signed X travel/],
            ] as const)
          : ([
              ["X search distance", /move outward/],
              ["Depth", /Distance to lower/],
            ] as const);
    for (const [title, text] of [
      ...cases,
      ["Coarse feed", /first search/],
      ["Backoff", /back away after each touch/],
    ] as const)
      await explain(
        page,
        page.getByRole("textbox", { name: title, exact: true }),
        title,
        text,
      );
    await expect(
      page.getByRole("button", { name: "Proceed", exact: true }),
    ).toBeVisible();
    expect(
      m.calls.filter(
        (call) => call.path.endsWith("/review") || call.path.endsWith("/run"),
      ),
    ).toHaveLength(0);
  });
}

test("help explains Settings choices and keeps its floating Guide clear", async ({
  page,
}, info) => {
  const m = await app(page);
  await page.getByRole("tab", { name: "Settings", exact: true }).click();
  await page.locator(".pp-help-button").click();
  await explain(
    page,
    page
      .getByRole("group", { name: "Language", exact: true })
      .getByRole("button", { name: "Svenska", exact: true }),
    "Language",
    /choice is saved for the next startup/,
  );
  await expect(
    page.getByRole("button", { name: "English", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await explain(
    page,
    page.getByRole("button", { name: "Check for updates", exact: true }),
    "Check for updates",
    /Check GitHub for an update/,
  );
  expect(
    m.calls.filter(
      (call) =>
        call.path.startsWith("updates/") ||
        (call.path === "settings" && Object.keys(call.body).length),
    ),
  ).toHaveLength(0);
  await page.locator(".pp-parameters").evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  const guide = page.getByRole("button", { name: "Guide", exact: true });
  expect((await guide.boundingBox())!.x).toBeLessThan(400);
  await capture(page, info.outputPath("help-settings-guide.png"));
  await guide.click();
  await expect(page.locator(".pp-help-guide")).toBeVisible();
});

test("help Guide bypasses coordinate fallback when it overlaps a field", async ({
  page,
}) => {
  await app(page);
  await page.setViewportSize({ width: 800, height: 360 });
  await page.locator(".pp-help-button").click();
  const overlap = await page.evaluate(() => {
    const guide = document
      .querySelector(".pp-floating-guide")!
      .getBoundingClientRect();
    for (const field of document.querySelectorAll(".pp-number")) {
      const rect = field.getBoundingClientRect();
      const left = Math.max(guide.left, rect.left),
        top = Math.max(guide.top, rect.top);
      const right = Math.min(guide.right, rect.right),
        bottom = Math.min(guide.bottom, rect.bottom);
      if (right > left && bottom > top)
        return { x: (left + right) / 2, y: (top + bottom) / 2 };
    }
    return null;
  });
  expect(overlap).not.toBeNull();
  await page.mouse.click(overlap!.x, overlap!.y);
  await expect(page.locator(".pp-help-guide")).toBeVisible();
});

test("help explains result readings and actions without changing work coordinates", async ({
  page,
}, info) => {
  for (const kind of ["contacts", "span", "angle", "rotary", "level"]) {
    const m = await app(page);
    if (kind === "angle")
      m.setResult({
        point: [1, 2, null],
        machinePoint: [-150, -100, null],
        rawPoint: [-92, -92, null],
        spans: [null, null, null],
        rawSpans: [null, null, null],
        wcs: 54,
        zeroed: false,
        angle: {
          degrees: 1,
          spacing: 20,
          difference: 0.35,
          rotationSupported: true,
          touches: [
            [-150, -100, -30],
            [-150.35, -80, -30],
          ],
        },
      });
    const rotary = kind === "rotary" || kind === "level";
    if (rotary) {
      await page.route("**/api/v1/rotary/review", (route) =>
        route.fulfill({
          json: {
            id: "rotary-1",
            config: route.request().postDataJSON(),
            program: [],
            simulated: true,
          },
        }),
      );
      await page.route("**/api/v1/rotary/run", (route) =>
        route.fulfill({
          contentType: "application/x-ndjson",
          body:
            JSON.stringify({
              type: "result",
              result: {
                stations: [
                  { center: [-100, -90, -30] },
                  { center: [-70, -89, -29] },
                ],
                xyAngle: 0.2,
                xzAngle: 0.3,
                wcs: 54,
                zeroed: false,
                rotationApplied: false,
                rotationSupported: true,
                ...(kind === "level"
                  ? {
                      level: {
                        touches: [
                          [-100, -90, -30],
                          [-100, -70, -30],
                        ],
                        correction: 0.2,
                        residual: 0.001,
                      },
                    }
                  : {}),
              },
            }) + "\n",
        }),
      );
    }
    if (kind !== "contacts")
      await page
        .getByRole("tab", {
          name: rotary ? "Rotary" : kind === "span" ? "Center" : "Angle",
          exact: true,
        })
        .click();
    await page
      .locator(".pp-routine-button")
      .nth(kind === "level" ? 2 : 0)
      .click();
    await page.getByRole("button", { name: "Proceed", exact: true }).click();
    await expect(page.locator(".pp-log-toggle")).toBeVisible();
    await page.locator(".pp-help-button").click();
    const target = (title: string) =>
      page.locator(`[data-help-title="${title}"]`).first();
    const cases =
      kind === "contacts"
        ? ([
            [
              "Contacts · G53 · mm",
              /before probe offsets and ball compensation/,
            ],
            ["Measured · G53 · mm", /compensation applied/],
          ] as const)
        : kind === "span"
          ? ([
              ["Size · mm", /effective ball diameter applied/],
              ["Raw span · mm", /before ball compensation/],
            ] as const)
          : kind === "angle"
            ? ([
                [
                  "Measured angle",
                  /Positive X\/Y angles turn counterclockwise/,
                ],
                [
                  "Contacts · G53 · mm",
                  /probe offsets and ball diameter cancel/,
                ],
              ] as const)
            : kind === "rotary"
              ? ([
                  ["Axis centers · G53 · mm", /two X positions/],
                  ["Axis angle in X/Y", /viewed from above/],
                  ["Axis angle in X/Z", /viewed from the side/],
                ] as const)
              : ([
                  ["Touches · G53 · mm", /probe calibration applied/],
                  ["A correction", /A rotation applied/],
                  ["Remaining tilt", /after the A correction/],
                ] as const);
    for (const [title, text] of cases)
      await explain(page, target(title), title, text);
    await explain(
      page,
      page.getByRole("button", {
        name: "Choose result work coordinates",
        exact: true,
      }),
      "Choose result work coordinates",
      kind === "angle"
        ? /receives this measured angle/
        : /receives this measurement/,
    );
    if (rotary)
      await explain(
        page,
        page.getByRole("button", {
          name: kind === "level" ? "Set A/Z zero" : "Set Y/Z zero",
          exact: true,
        }),
        kind === "level" ? "Set A/Z zero" : "Set Y/Z zero",
        /Axis calibration uses the first axis center/,
      );
    if (kind === "angle")
      await explain(
        page,
        page.getByRole("button", { name: "Set X/Y rotation", exact: true }),
        "Set X/Y rotation",
        /Its zero stays in place/,
      );
    await expect(page.locator("dialog:modal")).toHaveCount(0);
    expect(
      m.calls.filter((call) => /\/(zero|wcs|rotation)$/.test(call.path)),
    ).toHaveLength(0);
    await capture(page, info.outputPath(`help-${kind}-result.png`));
  }
});

for (const language of ["en_US", "sv", "zh_CN"]) {
  test(`probe exit choices share a touch-sized row in ${language}`, async ({
    page,
  }, info) => {
    await page.addInitScript(() => {
      window.close = () => {
        const root = document.documentElement;
        root.dataset.exits = String(Number(root.dataset.exits || 0) + 1);
      };
    });
    const m = await app(page, language);
    const catalog: Record<string, string> =
      language === "sv" ? swedish : language === "zh_CN" ? chinese : {};
    const t = (source: string) => catalog[source] || source;
    const back = page.getByRole("button", { name: t("Back"), exact: true });
    await back.click();
    const dialog = page.getByRole("dialog", { name: t("Probe extended") });
    await expect(dialog).toBeVisible();
    const buttons = dialog.getByRole("button");
    await expect(buttons).toHaveCount(3);
    const bounds = await buttons.evaluateAll((elements) =>
      elements.map((element) => {
        const { x, y, width, height } = element.getBoundingClientRect();
        return {
          x,
          y,
          width,
          height,
          fits: element.scrollWidth <= element.clientWidth,
        };
      }),
    );
    const row = [...bounds].sort((a, b) => a.x - b.x);
    for (const [i, button] of row.entries()) {
      expect(button.y).toBeCloseTo(row[0].y);
      expect(Math.abs(button.width - row[0].width)).toBeLessThan(1);
      expect(button.height).toBeCloseTo(row[0].height);
      expect(button.height).toBeGreaterThanOrEqual(80);
      expect(button.y + button.height).toBeLessThanOrEqual(480);
      expect(button.fits).toBe(true);
      if (i > 0)
        expect(
          button.x - row[i - 1].x - row[i - 1].width,
        ).toBeGreaterThanOrEqual(12);
    }
    await capture(page, info.outputPath("probe-exit.png"));
    await dialog
      .getByRole("button", { name: t("Cancel"), exact: true })
      .click();
    await expect(dialog).toBeHidden();
    await expect(page.locator(".pp-routine-button").first()).toBeEnabled();
    expect(m.calls.filter((call) => call.path === "probe-actuator")).toEqual(
      [],
    );
    await back.click();
    await dialog
      .getByRole("button", { name: t("Leave extended"), exact: true })
      .click();
    await expect(page.locator("html")).toHaveAttribute("data-exits", "1");
    expect(m.calls.filter((call) => call.path === "probe-actuator")).toEqual(
      [],
    );
    await back.click();
    await dialog
      .getByRole("button", { name: t("Retract and exit"), exact: true })
      .click();
    await expect(page.locator("html")).toHaveAttribute("data-exits", "2");
    expect(m.calls.filter((call) => call.path === "probe-actuator")).toEqual([
      { path: "probe-actuator", body: { extended: false } },
    ]);
  });

  test(`help retains translated diagram instructions in ${language}`, async ({
    page,
  }) => {
    const m = await app(page, language);
    const catalog: Record<string, string> =
      language === "sv" ? swedish : language === "zh_CN" ? chinese : {};
    const t = (source: string) => catalog[source] || source;
    await page.locator(".pp-help-button").click();
    await explain(
      page,
      page.locator(".pp-routine-button").first(),
      t("Outside %1 corner").replace("%1", "X+/Y-"),
      t(
        "Start above the stock near this corner. Measure X and Y separately, crossing above the stock at the starting Z.",
      ),
    );
    await page.locator(".pp-help-button").click();
    await page.getByRole("tab").nth(3).click();
    await page.locator(".pp-help-button").click();
    await explain(
      page,
      page.locator(".pp-routine-button").first(),
      t("X− face angle"),
      t(
        "Start beside the face at measuring height. Touch twice in this direction, moving along the other positive axis between points, then return to start.",
      ),
    );
    await explain(
      page,
      page.locator(".pp-routine-button").nth(4),
      t("Z slope along X"),
      t(
        "Start above the first point. Touch Z down twice, moving in the named positive axis between points, then return to start.",
      ),
    );
    await page.locator(".pp-help-button").click();
    await page.getByRole("tab").nth(4).click();
    await page.locator(".pp-help-button").click();
    await explain(
      page,
      page.locator(".pp-routine-button").first(),
      t("Axis center"),
      t(
        "Start above a straight rod held in the chuck. Measure its axis center at two X positions and the angle of the axis.",
      ),
    );
    await explain(
      page,
      page.locator(".pp-routine-button").nth(1),
      t("Align face Y+"),
      t(
        "Start beside the lower touch point with the top edge leaning away from the shaft. Touch bottom and top, then rotate A to make the face vertical.",
      ),
    );
    expect(
      m.calls.filter(
        (call) => call.path.endsWith("/review") || call.path.endsWith("/run"),
      ),
    ).toHaveLength(0);
  });
}

test("language buttons save an explicit preference", async ({ page }) => {
  const m = await app(page);
  await page.getByRole("tab").nth(5).click();
  for (const [label, code] of [
    ["Svenska", "sv"],
    ["简体中文", "zh_CN"],
    ["English", "en_US"],
  ]) {
    await page.getByRole("button", { name: label, exact: true }).click();
    await expect(
      page.getByRole("button", { name: label, exact: true }),
    ).toHaveAttribute("aria-pressed", "true");
    expect(
      m.calls
        .filter((call) => call.path === "settings" && call.body.language)
        .at(-1)?.body,
    ).toEqual({ language: code });
  }
});

test("launcher language applies until a saved choice replaces it", async ({
  page,
}) => {
  await app(page, "");
  await page.goto("/?language=zh_CN");
  await page.getByRole("tab").nth(5).click();
  await expect(
    page.getByRole("button", { name: "简体中文", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", { name: "Svenska", exact: true }).click();
  await page.reload();
  await page.getByRole("tab").nth(5).click();
  await expect(
    page.getByRole("button", { name: "Svenska", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
});

for (const [locale, selected] of [
  ["en-GB", "English"],
  ["sv-SE", "Svenska"],
  ["zh-CN", "简体中文"],
]) {
  test.describe(`system language ${locale}`, () => {
    test.use({ locale });
    test("empty preference selects the system language button", async ({
      page,
    }) => {
      await app(page, "");
      await page.getByRole("tab").nth(5).click();
      await expect(
        page.getByRole("button", { name: selected, exact: true }),
      ).toHaveAttribute("aria-pressed", "true");
    });
  });
}
for (const language of ["en_US", "zh_CN", "sv"])
  test(`probing tabs and keypad preserve the touchscreen layout in ${language}`, async ({
    page,
  }, info) => {
    await app(page, language);
    for (const [i, tab] of [
      "Outside",
      "Inside",
      "Center",
      "Angle",
      "Rotary",
      "Settings",
    ].entries()) {
      await page.getByRole("tab").nth(i).click();
      await capture(page, info.outputPath(`${i + 1}-${tab}.png`));
      const fields = page.locator(".pp-number");
      await expect(fields.first()).toBeInViewport({ ratio: 1 });
      if (i < 5) {
        await expect(page.locator(".pp-routine-button").last()).toBeInViewport({
          ratio: 1,
        });
      }
    }
    await page.locator(".pp-number").first().click();
    await expect(page.locator(".pp-keypad")).toBeVisible();
    await capture(page, info.outputPath("7-keypad.png"));
  });
test("numeric editing replaces selection, then inserts at the chosen caret and saves", async ({
  page,
}) => {
  const m = await app(page);
  const field = page.getByRole("textbox", {
    name: "X search distance",
    exact: true,
  });
  await field.click();
  expect(
    await field.evaluate((el: HTMLInputElement) => [
      el.selectionStart,
      el.selectionEnd,
    ]),
  ).toEqual([0, 2]);
  await page.getByRole("button", { name: "2", exact: true }).click();
  await page.getByRole("button", { name: "0", exact: true }).click();
  await page
    .getByRole("button", { name: "Move cursor left", exact: true })
    .click();
  await page.getByRole("button", { name: "5", exact: true }).click();
  await expect(field).toHaveValue("250");
  await page.getByRole("button", { name: "Enter", exact: true }).click();
  await expect(page.locator(".pp-keypad")).toBeHidden();
  expect(
    m.calls.find((c) => c.path === "settings" && c.body.outsideXSearchDistance)
      ?.body,
  ).toEqual({ outsideXSearchDistance: 250 });
});
test("F1 explains the focused field without changing its value", async ({
  page,
}) => {
  const m = await app(page);
  const field = page.getByRole("textbox", {
    name: "X search distance",
    exact: true,
  });
  await field.focus();
  await field.fill("23");
  const writes = m.calls.filter((c) => c.path === "settings").length;
  await field.press("F1");
  await expect(
    page.getByRole("dialog", { name: "X search distance", exact: true }),
  ).toBeVisible();
  await expect(field).toHaveValue("23");
  expect(m.calls.filter((c) => c.path === "settings")).toHaveLength(writes);
  await page.getByRole("button", { name: "2", exact: true }).click();
  await page.getByRole("button", { name: "Enter", exact: true }).click();
  await expect(field).toHaveValue("23");
  expect(m.calls.filter((c) => c.path === "settings")).toHaveLength(writes);
  await page
    .getByRole("button", { name: "Close explanation", exact: true })
    .click();
  await expect(field).toBeFocused();
  await expect(field).toHaveValue("23");
  await page.getByRole("button", { name: "Exit help", exact: true }).click();
  await expect(field).toHaveValue("23");
  await page.getByRole("button", { name: "Enter", exact: true }).click();
  expect(
    m.calls.find(
      (c) => c.path === "settings" && c.body.outsideXSearchDistance === 23,
    )?.body,
  ).toEqual({ outsideXSearchDistance: 23 });
});

test("help explains controls without moving or editing, and the Guide returns to help", async ({
  page,
}, info) => {
  const m = await app(page);
  await page.getByRole("button", { name: "Help", exact: true }).click();
  await page
    .getByRole("textbox", { name: "X search distance", exact: true })
    .click();
  await expect(
    page.getByRole("dialog", { name: "X search distance", exact: true }),
  ).toBeVisible();
  await expect(page.locator(".pp-keypad")).toBeHidden();
  await capture(page, info.outputPath("help-field.png"));
  await page
    .getByRole("button", { name: "Close explanation", exact: true })
    .click();
  await page.locator(".pp-routine-button").first().click();
  await expect(page.getByRole("dialog")).toBeVisible();
  expect(m.calls.filter((c) => c.path === "routine/review")).toHaveLength(0);
  await page
    .getByRole("button", { name: "Close explanation", exact: true })
    .click();
  await page.getByRole("button", { name: "Guide", exact: true }).click();
  await expect(page.locator(".pp-help-guide")).toBeVisible();
  await capture(page, info.outputPath("guide-index.png"));
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await expect(page.locator(".pp-help-highlight").first()).toBeVisible();
});
test("retracted probes keep tabs available and let help explain disabled routines", async ({
  page,
}, info) => {
  const m = await app(page);
  m.setExtended(false);
  await expect(page.locator(".pp-routine-button").first()).toBeDisabled();
  await page.getByRole("tab", { name: "Inside", exact: true }).click();
  await expect(
    page.getByRole("tab", { name: "Inside", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await capture(page, info.outputPath("retracted.png"));
  await page.getByRole("button", { name: "Help", exact: true }).click();
  const bounds = await page.locator(".pp-routine-button").first().boundingBox();
  await page.mouse.click(
    bounds!.x + bounds!.width / 2,
    bounds!.y + bounds!.height / 2,
  );
  await expect(page.getByRole("dialog")).toBeVisible();
});
test("review edits apply only to the run and results can set offsets and select WCS", async ({
  page,
}, info) => {
  const m = await app(page);
  await page.getByRole("tab", { name: "Inside", exact: true }).click();
  await page.locator(".pp-routine-button").first().click();
  await expect(
    page.getByRole("button", { name: "Proceed", exact: true }),
  ).toBeVisible();
  await capture(page, info.outputPath("review.png"));
  const field = page.getByRole("textbox", {
    name: "X search distance",
    exact: true,
  });
  await field.click();
  await page.getByRole("button", { name: "3", exact: true }).click();
  await page.getByRole("button", { name: "Proceed", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Set Work Zero", exact: true }),
  ).toBeVisible();
  await capture(page, info.outputPath("corner-result.png"));
  expect(
    m.calls.find((c) => c.path === "routine/review")?.body.xSearchDistance,
  ).toBe(3);
  const offset = page.getByRole("textbox", {
    name: "X origin offset",
    exact: true,
  });
  await offset.click();
  await page.getByRole("button", { name: "2", exact: true }).click();
  await page.getByRole("button", { name: "Change sign", exact: true }).click();
  await page
    .getByRole("button", { name: "Set Work Zero", exact: true })
    .click();
  expect(m.calls.find((c) => c.path === "routine/zero")?.body).toEqual({
    id: "review-1",
    offsets: [-2, 0, 0],
  });
  await offset.click();
  await page.getByRole("button", { name: "Backspace", exact: true }).click();
  await page.getByRole("button", { name: "Enter", exact: true }).click();
  await expect(page.getByRole("alert")).toBeVisible();
  await page
    .getByRole("button", {
      name: "Choose result work coordinates",
      exact: true,
    })
    .click();
  await page.getByRole("button", { name: "G55", exact: true }).click();
  await expect(
    page.getByRole("button", {
      name: "Choose result work coordinates",
      exact: true,
    }),
  ).toHaveText("G55");
  await expect(offset).toHaveValue("-2");
  await expect(page.locator(".pp-keypad")).toBeHidden();
});
test("work-zero confirmation expires and clears when editing another offset", async ({
  page,
}) => {
  const m = await app(page);
  await page.clock.install();
  await page.locator(".pp-routine-button").first().click();
  await page.getByRole("button", { name: "Proceed", exact: true }).click();
  const zero = page.getByRole("button", { name: "Set Work Zero", exact: true });
  const heading = page.locator(".pp-result-heading [role=status]");
  await zero.click();
  await expect(heading).toHaveText("Work zero set");
  await page.clock.fastForward(3100);
  await expect(heading).toHaveText("Work zero");
  await zero.click();
  await expect(heading).toHaveText("Work zero set");
  const offset = page.getByRole("textbox", {
    name: "X origin offset",
    exact: true,
  });
  await offset.click();
  await page.getByRole("button", { name: "5", exact: true }).click();
  await expect(heading).toHaveText("Work zero");
  await page.getByRole("button", { name: "Enter", exact: true }).click();
  await expect(offset).toHaveValue("5");
  await zero.click();
  await expect(heading).toHaveText("Work zero set");
  expect(
    m.calls.filter((call) => call.path === "routine/zero").at(-1)?.body,
  ).toEqual({
    id: "review-1",
    offsets: [5, 0, 0],
  });
});
test("a storage warning keeps a successful measurement usable", async ({
  page,
}) => {
  const m = await app(page);
  m.warn();
  await page.locator(".pp-routine-button").first().click();
  await page.getByRole("button", { name: "Proceed", exact: true }).click();
  await expect(
    page.getByText("Could not save probing log", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Set Work Zero", exact: true }),
  ).toBeEnabled();
});
test("a missed touch shows the error and allows closing the result", async ({
  page,
}) => {
  const m = await app(page);
  m.miss();
  await page.locator(".pp-routine-button").first().click();
  await page.getByRole("button", { name: "Proceed", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("did not touch");
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await expect(
    page.getByRole("tab", { name: "Outside", exact: true }),
  ).toBeVisible();
});

test("Guide leaves a running operation mounted and returns to its completed result", async ({
  page,
}) => {
  await app(page);
  let finish!: () => void;
  const release = new Promise<void>((resolve) => {
    finish = resolve;
  });
  await page.route("**/api/v1/routine/run", async (route) => {
    await release;
    await route.fulfill({
      contentType: "application/x-ndjson",
      body:
        JSON.stringify({
          type: "result",
          result: {
            point: [1, 2, null],
            machinePoint: [-150, -100, null],
            rawPoint: [-92, -92, null],
            spans: [null, null, null],
            rawSpans: [null, null, null],
            wcs: 54,
            zeroed: false,
          },
        }) + "\n",
    });
  });
  await page.locator(".pp-routine-button").first().click();
  await page.getByRole("button", { name: "Proceed", exact: true }).click();
  await expect(page.getByText("Running", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Help", exact: true }).click();
  await page.getByRole("button", { name: "Guide", exact: true }).click();
  await expect(page.locator(".pp-help-guide")).toBeVisible();
  finish();
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Set Work Zero", exact: true }),
  ).toBeVisible();
});

test("history opens saved measurements and repeatability keeps its options", async ({
  page,
}, info) => {
  await app(page);
  const entry = {
    id: "saved-1",
    timestampMs: 1792585800000,
    label: "Pocket center",
    category: "routine",
    status: "success",
    config: {
      family: "center",
      feature: "pocket",
      wcs: 54,
      x: 1,
      y: 1,
      z: false,
    },
    result: {
      point: [1, 2, null],
      machinePoint: [-150, -100, null],
      rawPoint: [-92, -92, null],
      spans: [20, 30, null],
      rawSpans: [18, 28, null],
      wcs: 54,
      zeroed: false,
    },
  };
  await page.route("**/api/v1/logs/history", (route) =>
    route.fulfill({ json: [entry] }),
  );
  await page.route("**/api/v1/logs/history/open", (route) =>
    route.fulfill({ json: { entry, result: entry.result, canApply: true } }),
  );
  await page.route("**/api/v1/repeatability/run", (route) =>
    route.fulfill({
      contentType: "application/x-ndjson",
      body:
        JSON.stringify({
          type: "result",
          result: {
            measurements: [
              [-175, -180, -100],
              [-175.002, -180.001, -100],
            ],
            statistics: [
              {
                mean: -175.001,
                median: -175.001,
                stddev: 0.0014,
                range: 0.002,
              },
              {
                mean: -180.0005,
                median: -180.0005,
                stddev: 0.0007,
                range: 0.001,
              },
              { mean: -100, median: -100, stddev: 0, range: 0 },
            ],
          },
        }) + "\n",
    }),
  );
  await page.getByRole("tab", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Utilities", exact: true }).click();
  await capture(page, info.outputPath("utilities.png"));
  await page
    .getByRole("button", { name: "Probe history", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Open result", exact: true }),
  ).toBeEnabled();
  await capture(page, info.outputPath("history.png"));
  await page.getByRole("button", { name: "Open result", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Set Work Zero", exact: true }),
  ).toBeEnabled();
  await capture(page, info.outputPath("pocket-result.png"));
  await page.getByRole("button", { name: "Details", exact: true }).click();
  await expect(page.getByLabel("Command log", { exact: true })).toContainText(
    "Status: success",
  );
  await page.getByRole("button", { name: "Result", exact: true }).click();
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await page
    .getByRole("button", { name: "Probe repeatability", exact: true })
    .click();
  await capture(page, info.outputPath("repeat-prepare.png"));
  await page
    .getByRole("button", { name: "Fixture is ready", exact: true })
    .click();
  await page
    .getByRole("switch", { name: "Home each time", exact: true })
    .click();
  await expect(
    page.getByRole("switch", { name: "Retract each time", exact: true }),
  ).toHaveAttribute("aria-checked", "true");
  await capture(page, info.outputPath("repeat-options.png"));
  await page.getByRole("button", { name: "Start check", exact: true }).click();
  await expect(
    page.getByText("Deviation from mean (mm)", { exact: true }),
  ).toBeVisible();
  await capture(page, info.outputPath("repeat-result.png"));
});

test("the update channel reloads release notes and installation asks for confirmation", async ({
  page,
}, info) => {
  await app(page);
  await page.route("**/api/v1/updates/check?*", (route) =>
    route.fulfill({
      json: {
        currentVersion: "2026.10.0",
        available:
          new URL(route.request().url()).searchParams.get("development") ===
          "true"
            ? {
                token: "update-1",
                version: "dev-1234567",
                name: "Development build",
                notes: "## Changes\n- Improve probing",
                development: true,
              }
            : null,
      },
    }),
  );
  await page.route("**/api/v1/updates/install", (route) =>
    route.fulfill({ json: { operationId: "install-1" } }),
  );
  await page.route("**/api/v1/updates/status?*", (route) =>
    route.fulfill({
      json: { state: "failed", message: "Installer could not start" },
    }),
  );
  await page.getByRole("tab", { name: "Settings", exact: true }).click();
  await page
    .getByRole("button", { name: "Check for updates", exact: true })
    .click();
  await expect(
    page.getByText("You have the latest release.", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("switch", { name: "Development releases", exact: true })
    .click();
  await expect(
    page.getByText("Development build", { exact: true }),
  ).toBeVisible();
  await capture(page, info.outputPath("update.png"));
  await page
    .getByRole("button", { name: "Install update", exact: true })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Install update", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Install", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText(
    "Installer could not start",
  );
});

test("rotary review and measured centers retain their own result actions", async ({
  page,
}, info) => {
  await app(page);
  const result = {
    stations: [{ center: [-60, -95, -60] }, { center: [-30, -95.01, -60.02] }],
    xyAngle: 0.019,
    xzAngle: 0.038,
    wcs: 54,
    rotationSupported: true,
    zeroed: false,
    rotationApplied: false,
  };
  await page.route("**/api/v1/rotary/review", (route) =>
    route.fulfill({
      json: {
        id: "rotary-1",
        config: route.request().postDataJSON(),
        program: "G1 A90",
        simulated: true,
      },
    }),
  );
  await page.route("**/api/v1/rotary/run", (route) =>
    route.fulfill({
      contentType: "application/x-ndjson",
      body: JSON.stringify({ type: "result", result }) + "\n",
    }),
  );
  await page.getByRole("tab", { name: "Rotary", exact: true }).click();
  await page
    .getByRole("button", { name: "Calibrate rotary axis", exact: true })
    .click();
  await capture(page, info.outputPath("rotary-review.png"));
  await page.getByRole("button", { name: "Proceed", exact: true }).click();
  await expect(page.getByText("0.0190°", { exact: true })).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Set Y/Z zero", exact: true }),
  ).toBeEnabled();
  await capture(page, info.outputPath("rotary-result.png"));
});

test("controller alarms request the embedding application alarm action", async ({
  page,
}) => {
  await app(page);
  await page.evaluate(() => {
    window.close = () => {
      window.name = "alarm-reported";
    };
  });
  await page.route("**/api/v1/routine/run", (route) =>
    route.fulfill({
      contentType: "application/x-ndjson",
      body:
        JSON.stringify({
          type: "error",
          code: "controller_alarm",
          message: "Controller alarm: probe failed",
        }) + "\n",
    }),
  );
  await page.locator(".pp-routine-button").first().click();
  await page.getByRole("button", { name: "Proceed", exact: true }).click();
  await expect
    .poll(() => page.evaluate(() => window.name))
    .toBe("alarm-reported");
});
