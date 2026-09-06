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
import QtTest
import ".."
import "../Api.js" as Api

TestCase {
    name: "ProbeSettings"
    ProbeSettings { id: settings; serviceUrl: "http://unused/api/v1" }
    property var originalRequest
    property var requests

    function init() {
        requests = []
        originalRequest = Api.request
        Api.request = function(method, url, body, done) {
            requests.push({method: method, url: url, body: body, done: done})
            return {abort: function() {}}
        }
        settings.loaded = false
        settings.load()
        compare(requests[0].url, settings.serviceUrl + "/settings/schema")
        requests[0].done({ok: true, data: {centerXSearchDistance: {minimum: 0.1, maximum: 1000, default: 20}}})
        compare(requests[1].url, settings.serviceUrl + "/settings")
        requests[1].done({ok: true, data: {centerXSearchDistance: 20}})
        verify(settings.loaded)
        requests = []
    }
    function cleanup() {
        Api.request = originalRequest
    }
    function test_edits_during_save_are_serialized_without_overwriting_new_values() {
        settings.setValue("centerXSearchDistance", 120)
        settings.setValue("centerXSearchDistance", 50)
        compare(requests.length, 1)
        compare(requests[0].body.centerXSearchDistance, 120)
        requests[0].done({ok: true, data: {centerXSearchDistance: 120}})
        compare(settings.values.centerXSearchDistance, 50)
        compare(requests.length, 2)
        compare(requests[1].body.centerXSearchDistance, 50)
        requests[1].done({ok: true, data: {centerXSearchDistance: 50}})
        verify(!settings.saving)
    }
    function test_failed_save_can_retry_latest_values() {
        settings.setValue("centerXSearchDistance", 120)
        settings.setValue("centerXSearchDistance", 50)
        requests[0].done({ok: false, error: "Disconnected"})
        compare(settings.error, "Disconnected")
        compare(settings.values.centerXSearchDistance, 50)
        settings.save()
        compare(requests[1].body.centerXSearchDistance, 50)
        requests[1].done({ok: true})
        compare(settings.error, "")
    }
}
