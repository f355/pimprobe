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

.pragma library

function searchFields(family) {
    return [
        {key: family + "XSearchDistance", label: "X search distance"},
        {key: family + "YSearchDistance", label: "Y search distance"},
        {key: family + "Depth", label: "Depth"}
    ];
}
var outside = searchFields("outside");
var inside = searchFields("inside");
var center = searchFields("center");
var angle = [
    {key: "anglePointSpacing", label: "Point spacing"},
    {key: "angleSearchDistance", label: "Search distance"}
];
var angleFeatures = ["x-minus", "x-plus", "y-minus", "y-plus", "z-x", "z-y"];
function angleName(feature) {
    return ({"x-plus": "X+ face angle", "x-minus": "X− face angle", "y-plus": "Y+ face angle", "y-minus": "Y− face angle", "z-x": "Z slope along X", "z-y": "Z slope along Y"})[feature] || "";
}
var rotary = [
    {key: "rotaryRodDiameter", label: "Rod diameter"},
    {key: "rotaryXDistance", label: "X distance"},
    {key: "rotaryYDistance", label: "Y distance"},
    {key: "rotaryZDistance", label: "Z distance"}
];
var setup = [
    {key: "probeDiameter", label: "Probe ball diameter"},
    {key: "retractDistance", label: "Retract distance"},
    {key: "positioningFeed", label: "Positioning feed", unit: "mm/min"},
    {key: "coarseFeed", label: "Coarse feed", unit: "mm/min"},
    {key: "fineFeed", label: "Fine feed", unit: "mm/min"},
    {key: "rotaryFeed", label: "Rotary feed", unit: "°/min"}
];

function featureName(routine) {
    if (routine.family === "angle") return angleName(routine.feature);
    if (routine.z) return "Z surface";
    if (routine.family === "center") {
        var names = {
            "boss": "Boss center",
            "block": "Block center",
            "hole": "Hole center",
            "pocket": "Pocket center",
            "x-ridge": "X ridge center",
            "y-ridge": "Y ridge center",
            "x-valley": "X valley center",
            "y-valley": "Y valley center"
        };
        return names[routine.feature];
    }
    return routine.family === "inside"
        ? (routine.x && routine.y ? "Inside corner" : "Inside edge")
        : (routine.x && routine.y ? "Outside corner" : "Outside edge");
}

function routine(settings, family, selection, wcs) {
    var config = {
        family: family,
        wcs: wcs,
        zero: false,
        safeZOffset: settings.safeZOffset,
        depth: settings[family + "Depth"],
        xSearchDistance: settings[family + "XSearchDistance"],
        ySearchDistance: settings[family + "YSearchDistance"],
        retract: settings.retractDistance,
        diameter: settings.probeDiameter,
        positioningFeed: settings.positioningFeed,
        coarseFeed: settings.coarseFeed,
        fineFeed: settings.fineFeed
    };
    if (family === "angle") {
        config.feature = selection;
        config.z = selection.indexOf("z-") === 0;
        config.x = selection.indexOf("x-") === 0 ? (selection === "x-plus" ? 1 : -1) : 0;
        config.y = selection.indexOf("y-") === 0 ? (selection === "y-plus" ? 1 : -1) : 0;
        config.depth = settings.angleSearchDistance;
        config.xSearchDistance = config.x ? settings.angleSearchDistance : settings.anglePointSpacing;
        config.ySearchDistance = config.y ? settings.angleSearchDistance : settings.anglePointSpacing;
    } else if (family === "center") {
        config.feature = selection;
        config.z = selection === "z";
        config.x = config.z || selection.indexOf("y-") === 0 ? 0 : 1;
        config.y = config.z || selection.indexOf("x-") === 0 ? 0 : 1;
    } else {
        config.x = selection.x;
        config.y = selection.y;
        config.z = selection.z;
    }
    return config;
}

function rotaryConfig(settings, operation) {
    return {
        operation: operation || "axis",
        yDistance: settings.rotaryYDistance,
        zDistance: settings.rotaryZDistance,
        rodDiameter: settings.rotaryRodDiameter,
        xDistance: settings.rotaryXDistance,
        rotaryFeed: settings.rotaryFeed,
        diameter: settings.probeDiameter,
        retract: settings.retractDistance,
        positioningFeed: settings.positioningFeed,
        coarseFeed: settings.coarseFeed,
        fineFeed: settings.fineFeed
    };
}
