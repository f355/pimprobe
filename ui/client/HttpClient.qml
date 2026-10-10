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
import "Http.js" as Http

QtObject {
    id: client
    property string serviceUrl: "http://127.0.0.1:8137/api/v1"
    property var state: ({
            connected: false
        })
    property string error: "Starting service"
    property bool polling: true
    property var stateRequest: null

    function endpoint(operation, args) {
        var routes = {
            "state": ["GET", "/state"],
            "settings.get": ["GET", "/settings"],
            "settings.schema": ["GET", "/settings/schema"],
            "settings.update": ["PATCH", "/settings"],
            "probe.set": ["POST", "/probe-actuator"],
            "wcs.select": ["POST", "/wcs"],
            "routine.review": ["POST", "/routine/review"],
            "routine.run": ["POST", "/routine/run"],
            "routine.zero": ["POST", "/routine/zero"],
            "routine.wcs": ["POST", "/routine/wcs"],
            "routine.return": ["POST", "/routine/return"],
            "routine.measured": ["POST", "/routine/measured"],
            "rotary.review": ["POST", "/rotary/review"],
            "rotary.run": ["POST", "/rotary/run"],
            "rotary.zero": ["POST", "/rotary/zero"],
            "rotary.wcs": ["POST", "/rotary/wcs"],
            "rotary.rotation": ["POST", "/rotary/rotation"],
            "routine.rotation": ["POST", "/routine/rotation"],
            "repeatability.run": ["POST", "/repeatability/run"],
            "repeatability.stop": ["POST", "/repeatability/stop"],
            "history.get": ["GET", "/logs/history"],
            "history.open": ["POST", "/logs/history/open"],
            "history.wcs": ["POST", "/logs/history/wcs"],
            "history.zero": ["POST", "/logs/history/zero"],
            "history.rotation": ["POST", "/logs/history/rotation"],
            "history.export": ["POST", "/logs/export"],
            "history.clear": ["POST", "/logs/clear"],
            "updates.check": ["GET", "/updates/check?development=" + (args && args.development ? "true" : "false")],
            "updates.install": ["POST", "/updates/install"],
            "updates.status": ["GET", "/updates/status?id=" + encodeURIComponent(args ? args.id : "")]
        };
        if (!routes[operation])
            throw new Error("Unknown operation: " + operation);
        return routes[operation];
    }

    function request(operation, args, done) {
        var route = endpoint(operation, args);
        return Http.request(route[0], serviceUrl + route[1], route[0] === "GET" ? null : args, done);
    }

    function stream(operation, args, onEvent, onFinished) {
        var route = endpoint(operation, args);
        return Http.stream(serviceUrl + route[1], args, onEvent, onFinished);
    }

    function refresh() {
        if (stateRequest)
            return;
        stateRequest = request("state", null, function (reply) {
            stateRequest = null;
            if (!reply.ok || !reply.data) {
                state = {
                    connected: false
                };
                error = reply.error || "Invalid service response";
            } else {
                state = reply.data;
                error = "";
            }
        });
    }

    property Timer pollTimer: Timer {
        interval: 250
        repeat: true
        running: client.polling
        triggeredOnStart: true
        onTriggered: client.refresh()
    }
    Component.onDestruction: if (stateRequest)
        stateRequest.abort()
}
