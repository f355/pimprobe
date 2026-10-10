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

import QtQuick
import QtQuick.Controls
import QtTest
import ".."

TestCase {
    id: test
    name: "Updates"
    when: windowShown
    ApplicationWindow { id: host; visible: true; width: 800; height: 480 }
    property var requests: []
    property var preferences: ({})
    property var flow
    property var settings
    QtObject {
        id: fakeClient
        function request(operation, body, done) {
            if (operation === "settings.schema")
                done({ok: true, data: {developmentUpdates: {default: false}, automaticUpdateChecks: {default: true}}});
            else if (operation === "settings.get") done({ok: true, data: preferences});
            else if (operation === "settings.update") {
                preferences = Object.assign({}, body);
                done({ok: true, data: preferences});
            } else requests.push({body: body, done: done});
            return {abort: function() {}};
        }
    }
    Component { id: settingsComponent; ProbeSettings { client: fakeClient } }
    Component { id: flowComponent; UpdateFlow { client: fakeClient } }
    function create(automatic, development) {
        preferences = {automaticUpdateChecks: automatic, developmentUpdates: development};
        settings = settingsComponent.createObject(host);
        flow = flowComponent.createObject(host, {settings: settings});
        settings.load();
    }
    function init() { requests = []; }
    function cleanup() {
        flow.destroy();
        settings.destroy();
    }
    function switches() {
        var result = {};
        function visit(item) {
            if (item instanceof Switch) result[item.text] = item;
            Array.prototype.forEach.call(item.children || [], visit);
        }
        visit(flow.contentItem);
        return result;
    }
    function reply(available) {
        requests[requests.length - 1].done({ok: true, data: {
            currentVersion: "2026.10.0",
            available: available ? {token: "candidate", name: "Update", notes: "Changes"} : null
        }});
    }
    function test_startup_check_data() {
        return [
            {tag: "release", automatic: true, development: false},
            {tag: "dev", automatic: true, development: true},
            {tag: "manual", automatic: false, development: true}
        ];
    }
    function test_startup_check(data) {
        create(data.automatic, data.development);
        compare(requests.length, data.automatic ? 1 : 0);
        verify(!flow.opened);
        if (data.automatic) {
            compare(requests[0].body.development, data.development);
            reply(true);
            verify(flow.updateAvailable);
        }
        settings.load();
        compare(requests.length, data.automatic ? 1 : 0);
        flow.show();
        compare(requests[requests.length - 1].body.development, data.development);
        reply(false);
        verify(!flow.updateAvailable);
    }
    function test_switches_save_choices_and_reopening_keeps_channel() {
        create(true, false);
        reply(false);
        flow.show();
        reply(false);
        verify(waitForPolish(host));
        var controls = switches();
        mouseClick(controls["Development releases"]);
        compare(preferences.developmentUpdates, true);
        compare(requests[requests.length - 1].body.development, true);
        reply(true);
        mouseClick(controls["Check automatically"]);
        compare(preferences.automaticUpdateChecks, false);
        flow.close();
        flow.show();
        compare(requests[requests.length - 1].body.development, true);
    }
    function test_background_failure_can_be_retried_and_closed_check_still_completes() {
        create(true, false);
        requests[0].done({ok: false, error: "Offline"});
        verify(!flow.opened);
        verify(!flow.updateAvailable);
        flow.show();
        flow.close();
        reply(true);
        verify(flow.updateAvailable);
        compare(flow.errorText, "");
    }
}
