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

.pragma library

function distance(key, label, minimum, maximum) {
    return {key: key, label: label, unit: "mm", minimum: minimum === undefined ? 0.1 : minimum, maximum: maximum === undefined ? 100 : maximum};
}

function describe(config, rotary) {
    var options = [];
    var phases;
    var clip;
    if (rotary) {
        clip = "rotary-" + (config.operation || "axis");
        if (!config.operation || config.operation === "axis") {
            options.push(distance("rodDiameter", "Rod diameter", 3, 100));
            options.push(distance("xDistance", "X distance", -200, 200));
            phases = [
                "Start above a smooth part of the rod.",
                "Touch the top and both rotated sides at two X positions.",
                "Return above the first position. Calculate both axis centers."
            ];
        } else {
            options.push(distance("yDistance", "Y distance", 0.1, 100));
            options.push(distance("zDistance", "Z distance", 0.1, 100));
            phases = [
                config.operation === "horizontal" ? "Start above the face to level." : "Start beside the lower touch point.",
                config.operation === "horizontal" ? "Touch two points. Turn A to correct the angle and check again." : "Touch lower, then upper. Turn A to correct the angle and check again.",
                "Back off from the face. Show the final angle and touches."
            ];
        }
    } else if (config.z) {
        clip = config.family === "inside" ? "z-pocket" : "z-surface";
        options.push(distance("depth", "Depth"));
        phases = [
            "Start above the surface, with room to move down.",
            "Touch Z down, back off, then touch again slowly.",
            "Return to the starting Z. Show the surface height."
        ];
    } else {
        if (config.x) options.push(distance("xSearchDistance", "X search distance", 0.1, 1000));
        if (config.y) options.push(distance("ySearchDistance", "Y search distance", 0.1, 1000));
        var center = config.family === "center";
        var internal = config.family === "inside" || (center && ["hole", "pocket", "x-valley", "y-valley"].indexOf(config.feature) >= 0);
        if (!internal) options.push(distance("depth", "Depth"));
        clip = center ? "center-" + config.feature : config.family + "-" + config.x + "-" + config.y;
        if (center) {
            phases = [
                internal ? "Start inside the opening at the height to measure." : "Start above the feature, roughly centered.",
                internal ? "Touch opposite walls at this height. Find the midpoint." : "Move outside and lower by depth. Touch both sides.",
                internal ? "Move to the measured center at the same Z." : "Raise to the starting Z, then move over the center."
            ];
        } else {
            phases = [
                internal ? "Start inside the opening at the height to measure." : "Move outside the edge, then lower by depth.",
                config.x && config.y ? "Touch X, return to the starting X, then touch Y." : "Touch the wall, back off, then touch again slowly.",
                internal ? "Return to the starting X/Y at this height." : "Raise to the starting Z, then move over the measured point."
            ];
        }
    }
    var feeds = [
        {key: "coarseFeed", label: "Coarse feed", unit: "mm/min", minimum: 1, maximum: 100000},
        {key: "fineFeed", label: "Fine feed", unit: "mm/min", minimum: 1, maximum: 100000},
        {key: "positioningFeed", label: "Positioning feed", unit: "mm/min", minimum: 1, maximum: 100000},
        distance("retract", "Backoff", 0.1, rotary ? 10 : 20)
    ];
    if (rotary) feeds.push({key: "rotaryFeed", label: "Rotary feed", unit: "\u00b0/min", minimum: 1, maximum: 100000});
    feeds[0].section = "Feeds and backoff";
    return {clip: clip, phases: phases, options: options.concat(feeds)};
}
