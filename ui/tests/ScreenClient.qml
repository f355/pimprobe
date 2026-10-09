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

import QtQuick

QtObject {
    id: client
    property var state: readyState()
    property string error: ""
    property var history: []
    property var values: ({
        probeDiameter: 2, retractDistance: 0.5, positioningFeed: 1000,
        coarseFeed: 300, fineFeed: 50, safeZOffset: 40,
        outsideXSearchDistance: 10, outsideYSearchDistance: 10, outsideDepth: 5,
        insideXSearchDistance: 10, insideYSearchDistance: 10, insideDepth: 5,
        centerXSearchDistance: 20, centerYSearchDistance: 20, centerDepth: 5,
        rotaryRodDiameter: 10, rotaryXDistance: 30, rotaryYDistance: 10,
        rotaryZDistance: 10, rotaryFeed: 360
    })
    property var program: [
        "; Move over the edge and lower down",
        "G21 G94 G91",
        "G38.3 X10 F1000 ; expect no contact",
        "G38.3 Z-5 F1000 ; expect no contact",
        "; Measure toward X- and back off",
        "; Coarse",
        "G38.3 X-10 F300",
        "G1 X0.5 F1000",
        "; Fine",
        "G38.2 X-1 F50",
        "G1 X0.5 F1000",
        "; Return to starting height",
        "G1 Z5 F1000",
        "G21 G94 G90 ; restore captured modes"
    ]

    function readyState() {
        return {connected: true, contactActive: false, recoveryFailed: false,
            coordinates: {
                probe: {machinePosition: [-177.628, -102.204, -90, 0], workPosition: [-42.5, 8, -35, 0]},
                spindle: {machinePosition: [-120.128, -95.204, -50, 0], workPosition: [15, 15, 5, 0]}
            },
            actuatorPending: false, status: {mode: "Ready", wcs: 54,
                probeActuatorKnown: true, probeActuator: 1,
                machinePosition: [-120.128, -95.204, -50, 0],
                workPosition: [15, 15, 5, 0]}};
    }
    function refresh() {}
    function request(operation, args, done) {
        var data = {};
        if (operation === "settings.schema") {
            Object.keys(values).forEach(function(key) {
                data[key] = {minimum: 0.01, maximum: 10000};
            });
        } else if (operation === "settings.get") data = values;
        else if (operation === "settings.update") {
            values = Object.assign({}, values, args);
            data = values;
        } else if (operation === "routine.review")
            data = {id: "screen-review", program: program, simulated: true};
        else if (operation === "rotary.review")
            data = {id: "screen-rotary", program: program.join("\n"), simulated: true};
        else if (operation === "history.get") data = history;
        else if (operation === "wcs.select")
            state = Object.assign({}, state, {status: Object.assign({}, state.status, {wcs: args.wcs})});
        else if (operation === "probe.set")
            state = Object.assign({}, state, {status: Object.assign({}, state.status, {probeActuator: args.extended ? 1 : 0})});
        else {
            done({ok: false, error: "Screen fixture has no response for " + operation});
            return {abort: function() {}};
        }
        done({ok: true, data: data});
        return {abort: function() {}};
    }
    function stream() { return {abort: function() {}}; }
}
