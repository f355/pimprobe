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
import test from "node:test";
import { guideLink, guideImage, parseGuide } from "./document.ts";
import { actionHelp, centerHelp, edgeHelp, optionHelp } from "./text.ts";
import { popupPosition, intersection } from "./geometry.ts";

test("guide sections retain the introduction and ignore headings in code fences", () => {
  const guide = parseGuide(
    "# Outside\n\nIntroduction.\n<!-- guide-only -->Hidden<!-- /guide-only -->\n\n## Edge\nFirst.\n```md\n## Example\n```\n\n## Edge\nSecond.",
  );
  assert.equal(guide.title, "Outside");
  assert.equal(guide.introduction.trim(), "Introduction.");
  assert.deepEqual(
    guide.sections.map(({ id, title }) => ({ id, title })),
    [
      { id: "edge", title: "Edge" },
      { id: "edge-1", title: "Edge" },
    ],
  );
  assert.match(guide.sections[0].markdown, /## Example/);
  assert.equal(guide.sections[1].markdown.trim(), "Second.");
});

test("localized headings have usable Unicode anchors", () => {
  assert.equal(
    parseGuide("# Guide\n## 测量结果\nText").sections[0].id,
    "\u6d4b\u91cf\u7ed3\u679c",
  );
  assert.equal(
    parseGuide("# Guide\n## Läs resultaten!\nText").sections[0].id,
    "l\u00e4s-resultaten",
  );
});

test("guide navigation resolves pages and section links without leaving the reader", () => {
  assert.deepEqual(guideLink("../sv/outside.md#edge-or-corner", "README.md"), {
    page: "outside.md",
    anchor: "edge-or-corner",
  });
  assert.deepEqual(guideLink("#ball-diameter", "settings.md"), {
    page: "settings.md",
    anchor: "ball-diameter",
  });
  assert.deepEqual(guideLink("center", "README.md"), {
    page: "center.md",
    anchor: "",
  });
  assert.equal(guideLink("https://example.com/outside.md", "README.md"), null);
  assert.equal(guideLink("javascript:alert(1)", "README.md"), null);
  assert.equal(guideLink("missing.md", "README.md"), null);
  assert.equal(guideLink("#%invalid", "README.md"), null);
});

test("guide images resolve shared illustrations and reject external or escaping paths", () => {
  assert.equal(guideImage("../images/outside.svg"), "images/outside.svg");
  assert.equal(guideImage("images/inside.svg"), "images/inside.svg");
  for (const path of [
    "https://example.com/image.svg",
    "data:image/svg+xml,anything",
    "images/../../secret",
    "images/%2e%2e/secret",
    "//example.com/image.svg",
  ])
    assert.equal(guideImage(path), undefined);
});

test("help descriptions select the actual family, parameter and operation", () => {
  assert.match(optionHelp("outsideXSearchDistance"), /move outward/);
  assert.match(optionHelp("xSearchDistance", "inside"), /pocket wall/);
  assert.match(optionHelp("angleSearchDistance"), /at each point/);
  assert.match(optionHelp("probeDiameter"), /Effective ball diameter/);
  assert.match(optionHelp("insideDepth"), /pocket bottom/);
  assert.equal(edgeHelp(false, true, false), centerHelp("z"));
  assert.match(centerHelp("x-valley"), /between the walls/);
  assert.match(centerHelp("hole"), /inside the opening/);
  assert.match(actionHelp("Proceed"), /Start this measurement/);
  assert.match(
    actionHelp("Forts\u00e4tt", (source) =>
      source === "Proceed" ? "Forts\u00e4tt" : source,
    ),
    /Start this measurement/,
  );
  assert.equal(optionHelp("unknown"), "");
});

test("explanations stay within the viewport at each corner and avoid the target when possible", () => {
  const viewport = { left: 0, top: 0, width: 800, height: 480 };
  for (const [left, top] of [
    [0, 0],
    [744, 0],
    [0, 424],
    [744, 424],
    [360, 200],
  ]) {
    const target = { left, top, width: 56, height: 56 };
    const box = popupPosition(target, 336, 280, viewport);
    assert.ok(box.left >= 8 && box.top >= 8);
    assert.ok(box.left + box.width <= 792 && box.top + box.height <= 472);
    assert.equal(
      intersection(box, target).width * intersection(box, target).height,
      0,
    );
  }
});
