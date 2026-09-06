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
    name: "ServiceRequest"
    ServiceRequest { id: operation }
    property var originalRequest
    property var callbacks
    property int aborts: 0
    property int completions: 0

    function init() {
        callbacks = []
        aborts = 0
        completions = 0
        originalRequest = Api.request
        Api.request = function(method, url, body, done) {
            callbacks.push(done)
            return {abort: function() { ++aborts }}
        }
    }
    function cleanup() {
        operation.cancel()
        Api.request = originalRequest
    }
    function test_one_pending_request() {
        verify(operation.send("POST", "/wcs", {wcs: 54}, function() { ++completions }))
        verify(!operation.send("POST", "/wcs", {wcs: 55}, function() { ++completions }))
        compare(callbacks.length, 1)
        callbacks[0]({ok: true})
        compare(completions, 1)
        verify(!operation.pending)
    }
    function test_cancelled_callback_cannot_finish_new_request() {
        operation.send("GET", "/review", null, function() { fail("Stale callback") })
        operation.cancel()
        compare(aborts, 1)
        operation.send("GET", "/review", null, function() { ++completions })
        callbacks[0]({ok: true})
        verify(operation.pending)
        compare(completions, 0)
        callbacks[1]({ok: true})
        compare(completions, 1)
        verify(!operation.pending)
    }
}
