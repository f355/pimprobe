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

var descriptions = {
    "outsideSearch": "Distance to move outward from the starting position before lowering, then search back toward the stock.",
    "insideSearch": "Maximum distance from the starting position toward the pocket wall.",
    "centerSearch": "Distance from the starting position to search on each side. For a boss, block or ridge, it also puts the ball outside the stock before lowering.",
    "outsideDepth": "Distance to lower from the starting Z for side touches, or the maximum downward search for a Z measurement.",
    "insideDepth": "Maximum downward search for the pocket bottom. Wall touches stay at the starting Z.",
    "centerDepth": "Distance to lower for boss, block and ridge side touches, or the maximum downward Z search.",
    "diameter": "Effective ball diameter, including probe mechanism takeup and shaft flex at contact. The physical ball is 2 mm; use a known block or pin to calibrate this value as described in the Guide.",
    "retract": "Distance to back away after each touch. The fine search returns to the coarse contact and continues up to 0.5 mm beyond it.",
    "positioningFeed": "Speed for positioning moves and backing away from a touch, in mm/min.",
    "coarseFeed": "Speed of the first search for each surface, in mm/min.",
    "fineFeed": "Speed of the second, measured touch, in mm/min.",
    "rotaryFeed": "Speed for A-axis rotations, in degrees per minute.",
    "rodDiameter": "Approximate rod diameter, used to place the initial side searches. The measured touches determine the axis center.",
    "xDistance": "Signed X travel between the two places measured along the rod. Both must be clear of the chuck jaws.",
    "yDistance": "For a horizontal face, spacing between downward touches. For a vertical face, maximum sideways search.",
    "zDistance": "For a vertical face, upward spacing between touches. For a horizontal face, maximum downward search.",
    "offset": "Add this distance to the measured machine coordinate when setting work zero. Each measured axis has its own offset.",
    "safeZ": "Distance to raise from the current Z before moving the ball to the measured X/Y point.",
    "size": "Distance between the measured surfaces, with the effective ball diameter applied. X and Y are separate measurements.",
    "rawSpan": "Distance between the two fine contacts, before ball compensation. The absolute difference between this span and a known size gives the effective ball diameter.",
    "rawContacts": "Machine coordinates recorded at the fine touches, before probe offsets and ball compensation.",
    "measuredPoint": "Surface or center coordinates with probe offsets and ball compensation applied. The smaller numbers show the same point in the selected work coordinate system.",
    "rotaryCenters": "Measured centers of rotation at the two X positions, in machine coordinates. Their difference gives the rotary axis angles; Set Work Zero uses the first center.",
    "rotaryTouches": "Machine coordinates of the two fine touches, with probe calibration applied. Their difference gives the surface tilt.",
    "aCorrection": "A rotation applied to level this face. Set Work Zero saves its A zero and the measured surface coordinate.",
    "remainingTilt": "Surface tilt measured after the A correction. A value near zero means the face is level.",
    "xyAngle": "Angle of the rotary axis viewed from above. Set X/Y rotation applies this angle to the selected work coordinate system.",
    "xzAngle": "Angle of the rotary axis viewed from the side, showing how much its height changes along X.",
    "repeatReadings": "Each row is one repetition. During the check, readings use G53; afterward, each value shows its difference from that axis's mean.",
    "repeatMean": "Average of all measured machine coordinates for each axis, recalculated after every repetition.",
    "repeatMedian": "Middle measured machine coordinate for each axis. With an even number of readings, this is the average of the two middle values.",
    "repeatStddev": "Typical spread of the readings around their mean. A smaller value means more consistent touches.",
    "repeatRange": "Difference between the highest and lowest reading for each axis.",
    "Proceed": "Start this measurement from the probe ball's current position, using the options shown here.",
    "Cancel": "Return without starting the operation.",
    "Close": "Close this page.",
    "Set Work Zero": "Set the selected work coordinate system's zero from the measured point and offsets. Only measured axes are changed.",
    "Return to start": "Move back to the position captured when probing started: X/Y first, then Z.",
    "Move to measured XY": "Raise by the Safe Z lift, then move the probe ball to the measured X/Y point.",
    "Log": "Show the commands and messages recorded during this measurement.",
    "Result": "Show the measured coordinates and work-zero controls.",
    "Details": "Show the saved measurement's options and command log.",
    "Utilities": "Open probe history, log export and the repeatability check.",
    "Probe history": "Browse saved measurements. Open a successful result to set work zero from it again.",
    "Export logs": "Save probe history and diagnostic logs to a downloadable archive.",
    "Clear logs": "Delete saved probe history and diagnostic logs after confirmation.",
    "Probe repeatability": "Repeat touches against the L bracket and bed, then compare their spread. Options include homing and probe retraction between repetitions.",
    "Check for updates": "Check GitHub for an update on the selected release or development channel.",
    "Install update": "Download and install the version shown here, then restart the interface.",
    "Check again": "Check GitHub again for the latest version on the selected channel.",
    "Choose result work coordinates": "Choose which work coordinate system receives this measurement. You can use the same result to set more than one.",
    "Set X/Y rotation": "Rotate the selected work coordinate system to match the measured rotary axis angle in X/Y."
};

function action(label, i18n) {
    var keys = Object.keys(descriptions);
    for (var i = 0; i < keys.length; ++i)
        if (/^[A-Z]/.test(keys[i]) && i18n.tr(keys[i]) === label)
            return descriptions[keys[i]];
    return "";
}

function option(key, family) {
    if (key.indexOf("SearchDistance") >= 0)
        return descriptions[(family || key.replace(/[XY]SearchDistance$/, "")) + "Search"] || descriptions.centerSearch;
    if (key === "depth" || /Depth$/.test(key))
        return descriptions[(family || key.replace(/Depth$/, "")) + "Depth"] || descriptions.centerDepth;
    var aliases = {
        probeDiameter: "diameter", retractDistance: "retract",
        rotaryRodDiameter: "rodDiameter", rotaryXDistance: "xDistance",
        rotaryYDistance: "yDistance", rotaryZDistance: "zDistance",
        safeZOffset: "safeZ"
    };
    return descriptions[aliases[key] || key] || "";
}

function edge(inside, z, corner) {
    if (z)
        return "Measure the surface below the ball, up to Depth, then return to the starting Z.";
    if (inside)
        return corner ? "Start inside the pocket at the height to measure. Touch X, return to starting X, then touch Y and return to the starting point."
            : "Start inside the pocket at the height to measure. Search toward this wall, then return to the starting point.";
    return corner ? "Start above the stock near this corner. Measure X and Y separately, crossing above the stock at the starting Z."
        : "Start above the stock near this edge. Move outward, lower by Depth, then touch back toward the stock and rise to the starting Z.";
}

function center(feature) {
    if (feature === "z") return edge(false, true, false);
    if (feature === "boss" || feature === "block")
        return "Start roughly centered above the stock. Touch opposite X and Y sides to find its center and size, then finish above the center at the starting Z.";
    if (feature === "hole" || feature === "pocket")
        return "Start inside the opening at the height to measure. Touch opposite X and Y walls to find its center and size, then finish at the center at the same Z.";
    return feature.indexOf("valley") >= 0
        ? "Start between the walls at the height to measure. Touch both walls across the named axis to find their center and separation."
        : "Start above the ridge. Touch both sides across the named axis, then finish above their center at the starting Z.";
}
