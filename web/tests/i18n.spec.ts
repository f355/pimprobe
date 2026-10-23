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
import { translate } from "../src/i18n";
import swedish from "../src/locales/sv.json" with { type: "json" };
import chinese from "../src/locales/zh_CN.json" with { type: "json" };

test("translations retain coordinates, error codes and reordered values", () => {
  expect(
    translate("sv", "Set G%1 %2 zero to the first measured axis center?", [
      54,
      "Y/Z",
    ]),
  ).toBe("Nollställ Y/Z i G54 vid det första uppmätta axelcentrumet?");
  expect(translate("sv", "%1 offset %2 mm", ["X", -120.5])).toBe(
    "X-förskjutning -120.5 mm",
  );
  expect(translate("sv", "X offset -120.5 mm")).toBe(
    "X-förskjutning -120.5 mm",
  );
  expect(
    translate("sv", "Could not communicate with the machine: error:5"),
  ).toBe("Kunde inte kommunicera med maskinen: error:5");
  expect(
    translate(
      "sv",
      "Measurement completed, but its history could not be saved: Could not communicate with the machine: error:5",
    ),
  ).toBe(
    "Mätningen är klar, men historiken kunde inte sparas: Kunde inte kommunicera med maskinen: error:5",
  );
  expect(translate("en", "%1 offset %2 mm", ["X", -120.5])).toBe(
    "X offset -120.5 mm",
  );
  expect(translate("sv", "Unknown %1", [25])).toBe("Unknown 25");
});

test("catalog entries preserve parameter numbers", () => {
  const parameters = (text: string) =>
    [...new Set(text.match(/%[1-9]\d*/g))].sort();
  for (const catalog of [swedish, chinese]) {
    for (const [source, translated] of Object.entries(catalog)) {
      expect(translated.trim(), source).not.toBe("");
      expect(parameters(translated), source).toEqual(parameters(source));
    }
  }
});
