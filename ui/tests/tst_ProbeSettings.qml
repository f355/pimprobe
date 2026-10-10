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
import "../controls"
import QtTest
import ".."


TestCase {
    name: "ProbeSettings"
    ProbeSettings { id: settings; client: fakeClient }
    QtObject {
        id: fakeClient
        function request(operation, body, done) {
            requests.push({operation:operation, body:body, done:done});
            return {abort:function(){}};
        }
    }
    property var requests

    function init() {
        I18n.systemLanguage = "en"
        requests = []
        settings.loaded = false
        settings.load()
        compare(requests[0].operation, "settings.schema")
        requests[0].done({ok: true, data: {centerXSearchDistance: {minimum: 0.1, maximum: 1000, default: 20}, language:{default:""}}})
        compare(requests[1].operation, "settings.get")
        requests[1].done({ok: true, data: {centerXSearchDistance: 20}})
        verify(settings.loaded)
        requests = []
    }
    function cleanup() {
        I18n.systemLanguage = ""
        I18n.language = "en"
    }
    function test_language_selection_updates_labels_and_saves() {
        I18n.systemLanguage = "zh_CN"
        settings.values = {language:""}
        compare(I18n.language, "zh_CN")
        settings.setValue("language", "sv")
        compare(I18n.language, "sv")
        compare(I18n.tr("Settings"), "Inställningar")
        compare(requests[0].operation, "settings.update")
        compare(requests[0].body.language, "sv")
        requests[0].done({ok:true})
        settings.values = {language:"sv"}
        compare(I18n.language, "sv")
    }
    function test_language_defaults_data() {
        return [
            {tag:"machine-Chinese", preference:"", system:"zh_CN", expected:"zh_CN"},
            {tag:"machine-English", preference:"", system:"en", expected:"en"},
            {tag:"saved-Swedish", preference:"sv", system:"zh_CN", expected:"sv"},
            {tag:"local-Swedish", preference:"", system:"sv_SE", expected:"sv"},
            {tag:"local-Chinese", preference:"", system:"zh-Hans-CN", expected:"zh_CN"},
            {tag:"unsupported", preference:"", system:"de_DE", expected:"en"}
        ]
    }
    function test_language_defaults(data) {
        compare(I18n.resolve(data.preference, data.system), data.expected)
        compare(I18n.launchLanguage(["qmlscene", "Main.qml", "--language=" + data.system]), data.system)
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
