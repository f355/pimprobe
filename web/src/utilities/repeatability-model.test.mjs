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
import { test } from "node:test";
import {
  measurementNumber,
  readingValue,
  recordMeasurement,
  runOptions,
  validOptions,
} from "./repeatability-model.ts";

const options = {
  axes: [true, true, true],
  repetitions: 5,
  jogHome: false,
  home: false,
  retract: false,
};

test("homing and jogging require probe retraction while independent retraction remains optional", () => {
  assert.equal(runOptions(options).retract, false);
  assert.deepEqual(runOptions({ ...options, home: true }), {
    ...options,
    home: true,
    jogHome: true,
    retract: true,
  });
  assert.equal(runOptions({ ...options, jogHome: true }).retract, true);
  assert.equal(runOptions({ ...options, retract: true }).retract, true);
  assert.equal(options.retract, false);
});

test("repeatability requires an axis and between one and one hundred whole repetitions", () => {
  assert.equal(
    validOptions({ ...options, axes: [false, true, false], repetitions: 1 }),
    true,
  );
  assert.equal(validOptions({ ...options, repetitions: 100 }), true);
  for (const repetitions of [0, 101, 2.5, NaN])
    assert.equal(validOptions({ ...options, repetitions }), false);
  assert.equal(
    validOptions({ ...options, axes: [false, false, false] }),
    false,
  );
});

test("one-based stream readings preserve partial rows and replace live statistics", () => {
  const original = {
    measurements: [[25, null, null]],
    statistics: [null, null, null],
  };
  const stats = {
    count: 2,
    mean: 26,
    median: 26,
    stddev: Math.sqrt(2),
    range: 2,
  };
  const report = recordMeasurement(original, {
    type: "measurement",
    repetition: 2,
    axis: "X",
    value: 27,
    statistics: [stats, null, null],
  });
  const partial = recordMeasurement(report, {
    type: "measurement",
    repetition: 2,
    axis: "Y",
    value: -204.3,
  });
  assert.deepEqual(partial.measurements, [
    [25, null, null],
    [27, -204.3, null],
  ]);
  assert.deepEqual(partial.statistics, [stats, null, null]);
  assert.deepEqual(original.measurements, [[25, null, null]]);
});

test("live readings use G53 and completed or partial results show deviations", () => {
  const stats = {
    count: 2,
    mean: 26,
    median: 26,
    stddev: Math.sqrt(2),
    range: 2,
  };
  assert.equal(readingValue(25, stats, true), 25);
  assert.equal(readingValue(25, stats, false), -1);
  assert.equal(readingValue(27, stats, false), 1);
  assert.equal(readingValue(null, stats, false), null);
  assert.equal(readingValue(25, null, false), 25);
  assert.equal(measurementNumber(null), "--");
  assert.equal(measurementNumber(25), "25.0000");
});

test("invalid stream coordinates cannot corrupt the report or create unbounded rows", () => {
  const report = {
    measurements: [[25, null, null]],
    statistics: [null, null, null],
  };
  const event = { type: "measurement", repetition: 1, axis: "X", value: 27 };
  for (const repetition of [0, -1, 101, 2.5, Infinity]) {
    assert.deepEqual(
      recordMeasurement(report, { ...event, repetition }),
      report,
    );
  }
  assert.deepEqual(recordMeasurement(report, { ...event, axis: "A" }), report);
  assert.deepEqual(recordMeasurement(report, { ...event, value: NaN }), report);
  assert.deepEqual(
    recordMeasurement(report, { ...event, value: Infinity }),
    report,
  );
});
