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

import type { Family, RoutineConfig, RotaryConfig, Settings } from "./types";

export interface Parameter {
  key: string;
  label: string;
  unit?: string;
  minimum?: number;
  maximum?: number;
  section?: string;
}
export const tabs = [
  "Outside",
  "Inside",
  "Center",
  "Angle",
  "Rotary",
  "Settings",
] as const;
export const centerFeatures = [
  "boss",
  "block",
  "x-ridge",
  "hole",
  "pocket",
  "x-valley",
  "z",
  "y-ridge",
  "y-valley",
];
export const angleFeatures = [
  "x-minus",
  "x-plus",
  "y-minus",
  "y-plus",
  "z-x",
  "z-y",
];
export const edges = [
  { x: 1, y: -1, z: false },
  { x: 0, y: -1, z: false },
  { x: -1, y: -1, z: false },
  { x: 1, y: 0, z: false },
  { x: 0, y: 0, z: true },
  { x: -1, y: 0, z: false },
  { x: 1, y: 1, z: false },
  { x: 0, y: 1, z: false },
  { x: -1, y: 1, z: false },
];
export function angleName(feature: string) {
  return (
    (
      {
        "x-plus": "X+ face angle",
        "x-minus": "X− face angle",
        "y-plus": "Y+ face angle",
        "y-minus": "Y− face angle",
        "z-x": "Z slope along X",
        "z-y": "Z slope along Y",
      } as Record<string, string>
    )[feature] || ""
  );
}
export function featureName(config: RoutineConfig) {
  if (config.family === "angle") return angleName(config.feature || "");
  if (config.z) return "Z surface";
  if (config.family === "center")
    return (
      {
        boss: "Boss center",
        block: "Block center",
        hole: "Hole center",
        pocket: "Pocket center",
        "x-ridge": "X ridge center",
        "y-ridge": "Y ridge center",
        "x-valley": "X valley center",
        "y-valley": "Y valley center",
      } as Record<string, string>
    )[config.feature || ""];
  return `${config.family === "inside" ? "Inside" : "Outside"} ${config.x && config.y ? "corner" : "edge"}`;
}
export function routine(
  settings: Settings,
  family: Family,
  selection: string | (typeof edges)[number],
  wcs: number,
): RoutineConfig {
  const config: RoutineConfig = {
    family,
    wcs,
    zero: false,
    safeZOffset: Number(settings.safeZOffset),
    depth: Number(settings[family + "Depth"]),
    xSearchDistance: Number(settings[family + "XSearchDistance"]),
    ySearchDistance: Number(settings[family + "YSearchDistance"]),
    retract: Number(settings.retractDistance),
    diameter: Number(settings.probeDiameter),
    positioningFeed: Number(settings.positioningFeed),
    coarseFeed: Number(settings.coarseFeed),
    fineFeed: Number(settings.fineFeed),
    x: 0,
    y: 0,
    z: false,
  };
  if (typeof selection !== "string") return { ...config, ...selection };
  config.feature = selection;
  if (family === "angle") {
    config.z = selection.startsWith("z-");
    config.x = selection.startsWith("x-")
      ? selection === "x-plus"
        ? 1
        : -1
      : 0;
    config.y = selection.startsWith("y-")
      ? selection === "y-plus"
        ? 1
        : -1
      : 0;
    config.depth = Number(settings.angleSearchDistance);
    config.xSearchDistance = Number(
      settings[config.x ? "angleSearchDistance" : "anglePointSpacing"],
    );
    config.ySearchDistance = Number(
      settings[config.y ? "angleSearchDistance" : "anglePointSpacing"],
    );
  } else {
    config.z = selection === "z";
    config.x = config.z || selection.startsWith("y-") ? 0 : 1;
    config.y = config.z || selection.startsWith("x-") ? 0 : 1;
  }
  return config;
}
export function rotaryConfig(
  settings: Settings,
  operation: string,
): RotaryConfig {
  return {
    operation,
    yDistance: Number(settings.rotaryYDistance),
    zDistance: Number(settings.rotaryZDistance),
    rodDiameter: Number(settings.rotaryRodDiameter),
    xDistance: Number(settings.rotaryXDistance),
    rotaryFeed: Number(settings.rotaryFeed),
    diameter: Number(settings.probeDiameter),
    retract: Number(settings.retractDistance),
    positioningFeed: Number(settings.positioningFeed),
    coarseFeed: Number(settings.coarseFeed),
    fineFeed: Number(settings.fineFeed),
  };
}
export function parameters(tab: string): Parameter[] {
  if (tab === "Settings")
    return [
      { key: "probeDiameter", label: "Probe ball diameter" },
      { key: "retractDistance", label: "Retract distance" },
      { key: "positioningFeed", label: "Positioning feed", unit: "mm/min" },
      { key: "coarseFeed", label: "Coarse feed", unit: "mm/min" },
      { key: "fineFeed", label: "Fine feed", unit: "mm/min" },
      { key: "rotaryFeed", label: "Rotary feed", unit: "°/min" },
    ];
  if (tab === "Angle")
    return [
      { key: "anglePointSpacing", label: "Point spacing" },
      { key: "angleSearchDistance", label: "Search distance" },
    ];
  if (tab === "Rotary")
    return [
      { key: "rotaryRodDiameter", label: "Rod diameter" },
      { key: "rotaryXDistance", label: "X distance" },
      { key: "rotaryYDistance", label: "Y distance" },
      { key: "rotaryZDistance", label: "Z distance" },
    ];
  return [
    { key: tab.toLowerCase() + "XSearchDistance", label: "X search distance" },
    { key: tab.toLowerCase() + "YSearchDistance", label: "Y search distance" },
    { key: tab.toLowerCase() + "Depth", label: "Depth" },
  ];
}
const distance = (
  key: string,
  label: string,
  minimum = 0.1,
  maximum = 100,
): Parameter => ({ key, label, unit: "mm", minimum, maximum });
export function reviewPlan(config: RoutineConfig | RotaryConfig) {
  const options: Parameter[] = [];
  let clip: string;
  let phases: string[];
  if (!("family" in config)) {
    clip = "rotary-" + config.operation;
    if (config.operation === "axis") {
      options.push(
        distance("rodDiameter", "Rod diameter", 3, 100),
        distance("xDistance", "X distance", -200, 200),
      );
      phases = [
        "Start above a smooth part of the rod.",
        "Touch the top and both rotated sides at two X positions.",
        "Return above the first position. Calculate both axis centers.",
      ];
    } else {
      options.push(
        distance("yDistance", "Y distance"),
        distance("zDistance", "Z distance"),
      );
      phases = [
        config.operation === "horizontal"
          ? "Start above the face to level."
          : "Start beside the lower touch point.",
        config.operation === "horizontal"
          ? "Touch two points. Turn A to correct the angle and check again."
          : "Touch lower, then upper. Turn A to correct the angle and check again.",
        "Back off from the face. Show the final angle and touches.",
      ];
    }
  } else if (config.family === "angle") {
    const alongX = config.feature === "z-x" || config.feature?.startsWith("y-");
    options.push(
      distance(
        alongX ? "xSearchDistance" : "ySearchDistance",
        "Point spacing",
        0.1,
        1000,
      ),
      distance(
        config.z ? "depth" : config.x ? "xSearchDistance" : "ySearchDistance",
        "Search distance",
      ),
    );
    clip = "angle-" + config.feature;
    phases = [
      config.z
        ? "Start above the first touch point."
        : "Start beside the face at measuring height.",
      alongX
        ? "Touch twice, moving X+ between points."
        : "Touch twice, moving Y+ between points.",
      "Return to start. Calculate the face angle.",
    ];
  } else if (config.z) {
    clip = config.family === "inside" ? "z-pocket" : "z-surface";
    options.push(distance("depth", "Depth"));
    phases = [
      "Start above the surface, with room to move down.",
      "Touch Z down, back off, then touch again slowly.",
      "Return to the starting Z. Show the surface height.",
    ];
  } else {
    if (config.x)
      options.push(distance("xSearchDistance", "X search distance", 0.1, 1000));
    if (config.y)
      options.push(distance("ySearchDistance", "Y search distance", 0.1, 1000));
    const center = config.family === "center";
    const internal =
      config.family === "inside" ||
      (center &&
        ["hole", "pocket", "x-valley", "y-valley"].includes(
          config.feature || "",
        ));
    if (!internal) options.push(distance("depth", "Depth"));
    clip = center
      ? "center-" + config.feature
      : `${config.family}-${config.x}-${config.y}`;
    phases = center
      ? [
          internal
            ? "Start inside the opening at the height to measure."
            : "Start above the feature, roughly centered.",
          internal
            ? "Touch opposite walls at this height. Find the midpoint."
            : "Move outside and lower by depth. Touch both sides.",
          internal
            ? "Move to the measured center at the same Z."
            : "Raise to the starting Z, then move over the center.",
        ]
      : [
          internal
            ? "Start inside the opening at the height to measure."
            : "Move outside the edge, then lower by depth.",
          config.x && config.y
            ? "Touch X, return to the starting X, then touch Y."
            : "Touch the wall, back off, then touch again slowly.",
          internal
            ? "Return to the starting X/Y at this height."
            : "Raise to the starting Z, then move over the measured point.",
        ];
  }
  options.push(
    {
      key: "coarseFeed",
      label: "Coarse feed",
      unit: "mm/min",
      minimum: 1,
      maximum: 100000,
      section: "Feeds and backoff",
    },
    {
      key: "fineFeed",
      label: "Fine feed",
      unit: "mm/min",
      minimum: 1,
      maximum: 100000,
    },
    {
      key: "positioningFeed",
      label: "Positioning feed",
      unit: "mm/min",
      minimum: 1,
      maximum: 100000,
    },
    distance("retract", "Backoff", 0.1, "family" in config ? 20 : 10),
  );
  if (!("family" in config))
    options.push({
      key: "rotaryFeed",
      label: "Rotary feed",
      unit: "°/min",
      minimum: 1,
      maximum: 100000,
    });
  return { clip, phases, options };
}
