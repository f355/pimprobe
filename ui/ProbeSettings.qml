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

Item {
    id: store
    required property string serviceUrl
    property var values: ({})
    property var schema: ({})
    property bool loaded: false
    readonly property bool saving: saveRequest.pending
    property bool dirty: false
    property string error: ""

    ServiceRequest {
        id: loadRequest
    }
    ServiceRequest {
        id: saveRequest
    }

    function load() {
        loadRequest.cancel();
        loadRequest.send("GET", serviceUrl + "/settings/schema", null, function (reply) {
            if (!reply.ok || !reply.data) {
                error = reply.error || "Could not load parameter limits";
                return;
            }
            schema = reply.data;
            loadRequest.send("GET", serviceUrl + "/settings", null, function (reply) {
                if (!reply.ok || !reply.data) {
                    error = reply.error || "Could not load probing settings";
                    return;
                }
                values = reply.data;
                loaded = true;
                error = "";
            });
        });
    }

    function setValue(key, value) {
        if (!loaded || !schema[key])
            return;
        var next = Object.assign({}, values);
        next[key] = value;
        values = next;
        dirty = true;
        save();
    }

    function save() {
        if (!loaded || saveRequest.pending)
            return;
        dirty = false;
        saveRequest.send("PATCH", serviceUrl + "/settings", values, function (reply) {
            error = reply.ok ? "" : reply.error;
            if (reply.ok && dirty)
                save();
        });
    }
}
