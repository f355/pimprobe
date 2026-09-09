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
var setup = [
    {key: "probeDiameter", label: "Probe ball diameter"},
    {key: "retractDistance", label: "Retract distance"},
    {key: "positioningFeed", label: "Positioning feed", unit: "mm/min"},
    {key: "coarseFeed", label: "Coarse feed", unit: "mm/min"},
    {key: "fineFeed", label: "Fine feed", unit: "mm/min"}
];

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
    if (family === "center") {
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
