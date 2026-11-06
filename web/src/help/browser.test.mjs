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

import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { chromium } from "playwright";
import { createServer } from "vite";

test("context help and localized guide work on touch and desktop screens", async (t) => {
  const root = fileURLToPath(new URL("../../", import.meta.url));
  const repository = fileURLToPath(new URL("../../../", import.meta.url));
  const server = await createServer({
    root,
    configFile: false,
    logLevel: "error",
    server: { host: "127.0.0.1", port: 0, fs: { allow: [repository] } },
  });
  const browser = await chromium.launch({ headless: true });
  try {
    await server.listen();
    const origin = server.resolvedUrls.local[0];
    const context = await browser.newContext({
      viewport: { width: 800, height: 480 },
      hasTouch: true,
    });
    const page = await context.newPage();
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => {
      if (message.type() === "error") errors.push(message.text());
    });
    const load = (language = "en") =>
      page.goto(origin + "src/help/fixture.html?language=" + language);
    const popup = page.locator(".pp-help-popup");
    const close = () =>
      page.getByRole("button", { name: "Close explanation" }).click();
    const tapDisabled = async (selector) => {
      const box = await page.locator(selector).boundingBox();
      await page.touchscreen.tap(box.x + box.width / 2, box.y + box.height / 2);
    };
    const contained = async (selector, horizontalOnly = false) => {
      const bounds = await page.locator(selector).boundingBox();
      const viewport = page.viewportSize();
      assert.ok(
        bounds.x >= 0 &&
          bounds.x + bounds.width <= viewport.width + 1 &&
          (horizontalOnly ||
            (bounds.y >= 0 && bounds.y + bounds.height <= viewport.height + 1)),
        JSON.stringify(bounds),
      );
    };
    await load();
    await page.getByRole("button", { name: "Help", exact: true }).click();
    await page.waitForSelector(".pp-help-highlight");
    await page.getByRole("button", { name: "Proceed", exact: true }).tap();
    assert.match(await popup.innerText(), /Start this measurement/);
    assert.equal(await page.getByTestId("actions").innerText(), "0");
    await contained(".pp-help-popup");
    const closeBounds = await page
      .getByRole("button", { name: "Close explanation" })
      .boundingBox();
    assert.equal(closeBounds.width, 48);
    assert.equal(closeBounds.height, 48);
    await mkdir(new URL("./test-results/", import.meta.url), {
      recursive: true,
    });
    await page.screenshot({
      path: fileURLToPath(
        new URL("./test-results/context-800.png", import.meta.url),
      ),
    });
    await close();
    await tapDisabled("button:disabled");
    assert.match(await popup.innerText(), /Effective ball diameter/);
    await close();
    await page.getByTestId("pointer-action").click();
    assert.match(await popup.innerText(), /Read the coordinates/);
    assert.equal(await page.getByTestId("actions").innerText(), "0");
    await close();

    const scroll = await page.getByTestId("scroll").boundingBox();
    await page.mouse.move(scroll.x + 80, scroll.y + 100);
    await page.mouse.down();
    await page.mouse.move(scroll.x + 80, scroll.y + 20, { steps: 8 });
    await page.mouse.up();
    assert.ok(
      (await page
        .getByTestId("scroll")
        .evaluate((element) => element.scrollTop)) > 50,
    );
    assert.equal(await popup.count(), 0);
    assert.equal(await page.getByTestId("actions").innerText(), "0");
    const beforeTouch = await page
      .getByTestId("scroll")
      .evaluate((element) => element.scrollTop);
    const session = await context.newCDPSession(page);
    await session.send("Input.dispatchTouchEvent", {
      type: "touchStart",
      touchPoints: [{ x: scroll.x + 80, y: scroll.y + 100 }],
    });
    for (let y = 90; y >= 20; y -= 10)
      await session.send("Input.dispatchTouchEvent", {
        type: "touchMove",
        touchPoints: [{ x: scroll.x + 80, y: scroll.y + y }],
      });
    await session.send("Input.dispatchTouchEvent", {
      type: "touchEnd",
      touchPoints: [],
    });
    assert.ok(
      (await page
        .getByTestId("scroll")
        .evaluate((element) => element.scrollTop)) > beforeTouch,
    );
    assert.equal(await popup.count(), 0);
    assert.equal(await page.getByTestId("actions").innerText(), "0");

    await page.getByRole("button", { name: "Exit help", exact: true }).click();
    await page.getByRole("button", { name: "Open dialog" }).click();
    await page.getByRole("button", { name: "Toggle help" }).click();
    await page.waitForSelector("dialog .pp-help-layer");
    assert.equal(
      await page.locator(".pp-help-highlight").count(),
      await page.locator('dialog [data-help]:not([data-help=""])').count(),
    );
    await tapDisabled("dialog button:disabled");
    assert.match(await popup.innerText(), /Modal disabled/);
    await contained(".pp-help-popup");
    await page.screenshot({
      path: fileURLToPath(
        new URL("./test-results/modal-800.png", import.meta.url),
      ),
    });
    await page.getByRole("button", { name: "Close explanation" }).focus();
    await page.keyboard.press("Escape");
    assert.equal(await popup.count(), 0);
    assert.equal(await page.locator("dialog:modal").count(), 1);
    await page
      .getByRole("button", { name: "Modal action", exact: true })
      .click();
    assert.equal(await page.getByTestId("actions").innerText(), "0");
    await close();
    await page.getByRole("button", { name: "Toggle help" }).click();
    await page.getByRole("button", { name: "Cancel", exact: true }).click();

    await mkdir(new URL("./test-results/", import.meta.url), {
      recursive: true,
    });
    for (const language of ["en", "zh_CN", "sv"]) {
      await load(language);
      const expected = {
        en: "Set X zero",
        zh_CN: "\u8bbe\u7f6e X \u96f6\u70b9",
        sv: "Nollst\u00e4ll X",
      };
      assert.ok(
        (await page.getByTestId("interpolation").innerText()).startsWith(
          expected[language],
        ),
      );
      assert.match(
        await page.getByTestId("interpolation").innerText(),
        /Unknown 0\/Y/,
      );
      if (language === "sv")
        assert.equal(
          await page.getByTestId("message").textContent(),
          "Failed (error:5); historiken kunde inte sparas: Permission denied",
        );
      await page.locator(".pp-help-button").click();
      await page.locator(".pp-guide-button").click();
      await page
        .getByRole("button", {
          name: { en: "Outside", zh_CN: "外侧", sv: "Utvändigt" }[language],
          exact: true,
        })
        .click();
      await page.locator(".pp-help-section summary").first().click();
      await page.locator(".pp-help-markdown img").waitFor();
      assert.ok(
        await page
          .locator(".pp-help-markdown img")
          .evaluate((image) => image.complete && image.naturalWidth > 0),
      );
      await contained(".pp-help-markdown img", true);
      await page.screenshot({
        path: fileURLToPath(
          new URL(`./test-results/guide-${language}-800.png`, import.meta.url),
        ),
      });
      await page.locator(".pp-help-markdown img").scrollIntoViewIfNeeded();
      await page.screenshot({
        path: fileURLToPath(
          new URL(`./test-results/image-${language}-800.png`, import.meta.url),
        ),
      });
      await page.locator(".pp-back").click();
      await page
        .getByRole("button", {
          name: {
            en: "Probing guide",
            zh_CN: "测量指南",
            sv: "Guide för probning",
          }[language],
          exact: true,
        })
        .click();
      const previousTitle = await page.locator(".pp-help-guide h1").innerText();
      await page.locator(".pp-help-introduction a").first().click();
      assert.notEqual(
        await page.locator(".pp-help-guide h1").innerText(),
        previousTitle,
      );
      for (const size of [
        { width: 360, height: 640 },
        { width: 1280, height: 800 },
      ]) {
        await page.setViewportSize(size);
        await page.locator(".pp-help-section summary").first().click();
        await page.screenshot({
          path: fileURLToPath(
            new URL(
              `./test-results/guide-${language}-${size.width}.png`,
              import.meta.url,
            ),
          ),
        });
        assert.ok(
          await page
            .locator(".pp-help-guide")
            .evaluate((element) => element.scrollWidth <= element.clientWidth),
        );
        await page.locator(".pp-help-section summary").first().click();
      }
      await page.setViewportSize({ width: 800, height: 480 });
    }
    assert.deepEqual(errors, []);
    await context.close();
    t.diagnostic(
      "Captured guides in English, Simplified Chinese and Swedish at 800x480, 360x640 and 1280x800.",
    );
  } finally {
    await browser.close();
    await server.close();
  }
});
