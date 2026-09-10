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
import QtQuick.Controls
import QtQuick.Layouts
import QtTest
import ".."
import "../Api.js" as Api
import "Mock.js" as Mock

TestCase {
    id: test
    name: "ProbeWindow"
    when: windowShown
    ProbeWindow {
        id: probeWindow
        width: 800
        height: 480
        serviceUrl: "http://127.0.0.1:18137/api/v1"
    }
    Component {
        id: updateFlowFixture
        UpdateFlow {
            width: 800
            height: 480
        }
    }
    function callApi(method, path, body) {
        var reply = null
        Api.request(method, probeWindow.serviceUrl + path, body, function(value) { reply = value })
        tryVerify(function() { return reply !== null }, 3000)
        verify(reply.ok, reply.error)
        return reply.data
    }
    function descendants(item, type) {
        var matches = []
        var children = item.children || []
        for (var i = 0; i < children.length; ++i) {
            if (children[i] instanceof type) matches.push(children[i])
            matches = matches.concat(descendants(children[i], type))
        }
        return matches
    }
    function initTestCase() {
        Mock.verifyServer(test, probeWindow.serviceUrl)
        tryVerify(function() { return probeWindow.settings.loaded && probeWindow.machineState.connected }, 3000)
    }
    function test_tabs_and_numeric_settings() {
        compare(probeWindow.title, "Probing")
        callApi("POST", "/probe-actuator", {extended: false})
        tryVerify(function() { return !probeWindow.probeFullyExtended() }, 3000)
        var tabs = descendants(probeWindow.contentItem, TabBar)[0]
        verify(tabs.enabled)
        mouseClick(tabs.itemAt(1))
        compare(tabs.currentIndex, 1)
        mouseClick(tabs.itemAt(2))
        compare(tabs.currentIndex, 2)
        mouseClick(tabs.itemAt(0))
        callApi("POST", "/probe-actuator", {extended: true})
        tryVerify(function() { return probeWindow.probeFullyExtended() }, 3000)
        probeWindow.requestActivate()
        var field = descendants(probeWindow.contentItem, NumberField).filter(function(f) { return f.visible && f.enabled })[0]
        verify(field !== undefined)
        field.forceActiveFocus()
        tryCompare(field.editor, "target", field)
        field.editor.typeKey("3")
        field.editor.typeKey(".")
        field.editor.typeKey("5")
        field.editor.accept()
        tryVerify(function() { return !probeWindow.settings.saving }, 3000)
        compare(callApi("GET", "/settings", null).outsideXSearchDistance, 3.5)
        waitForRendering(probeWindow.contentItem)
        var screenshot = grabImage(probeWindow.contentItem.parent)
        screenshot.save("/tmp/pimprobe-window.png")
        mouseClick(tabs.itemAt(2))
        compare(descendants(probeWindow.contentItem, CenterProbeButton).length, 9)
        waitForRendering(probeWindow.contentItem)
        grabImage(probeWindow.contentItem.parent).save("/tmp/pimprobe-center.png")
		mouseClick(tabs.itemAt(3))
		waitForRendering(probeWindow.contentItem)
		grabImage(probeWindow.contentItem.parent).save("/tmp/pimprobe-setup.png")
		mouseClick(tabs.itemAt(2))
		var center = descendants(probeWindow.contentItem, ProbePanel)[2]
		var centerField = descendants(center, NumberField)[0]
		mouseClick(centerField)
		var keypad = descendants(probeWindow.contentItem, NumericKeypad)[0]
		function tapKey(text) {
			var key = descendants(keypad, Button).filter(function(b) { return b.text === text })[0]
			verify(key !== undefined)
			mouseClick(key)
		}
		tapKey("8")
		tapKey("0")
		tapKey("\u21B5")
		compare(centerField.editor.target, null)
		verify(!keypad.visible)
		tryVerify(function() { return !probeWindow.settings.saving }, 3000)
		compare(callApi("GET", "/settings", null).centerXSearchDistance, 80)
		var ridge = descendants(center, CenterProbeButton).filter(function(b) { return b.feature === "x-ridge" })[0]
		mouseClick(ridge)
		tryVerify(function() {
			return descendants(probeWindow.Overlay.overlay, TextArea).some(function(a) {
				return a.visible && a.text.indexOf("contact_x_low") !== -1
			})
		}, 3000)
        var cancel = descendants(probeWindow.Overlay.overlay, Button).filter(function(b) {
            return b.visible && b.text === "Cancel"
        })[0]
        mouseClick(cancel)
        mouseClick(centerField)
        tapKey("5")
        tapKey("0")
        tapKey("\u21B5")
        compare(probeWindow.settings.values.centerXSearchDistance, 50)
        mouseClick(ridge)
        tryVerify(function() {
            return descendants(probeWindow.Overlay.overlay, TextArea).some(function(a) {
                return a.visible && a.text.indexOf("G38.3 X-50 ") !== -1
            })
        }, 3000)
        mouseClick(cancel)
    }

    function test_all_tabs_commit_values_on_enter() {
        probeWindow.requestActivate()
        callApi("POST", "/probe-actuator", {extended: true})
        tryVerify(function() { return probeWindow.probeFullyExtended() }, 3000)
        var tabs = descendants(probeWindow.contentItem, TabBar)[0]
        var keys = [
            ["outsideXSearchDistance", "outsideYSearchDistance", "outsideDepth"],
            ["insideXSearchDistance", "insideYSearchDistance", "insideDepth"],
            ["centerXSearchDistance", "centerYSearchDistance", "centerDepth"],
            ["probeDiameter", "retractDistance", "positioningFeed", "coarseFeed", "fineFeed"]
        ]
        for (var tab = 0; tab < keys.length; ++tab) {
            mouseClick(tabs.itemAt(tab))
            var fields = descendants(probeWindow.contentItem, NumberField).filter(function(f) { return f.visible })
            compare(fields.length, keys[tab].length)
            for (var index = 0; index < fields.length; ++index) {
                var field = fields[index]
                var oldValue = field.value
                mouseClick(field)
                field.forceActiveFocus()
                tryCompare(field.editor, "target", field)
                var keypad = descendants(probeWindow.contentItem, NumericKeypad).filter(function(k) { return k.visible })[0]
                waitForRendering(keypad)
                var buttons = descendants(keypad, Button)
                var digit = buttons.filter(function(b) { return b.text === "6" })[0]
                mouseClick(digit)
                verify(field.activeFocus)
                compare(field.text, "6", keys[tab][index])
                compare(probeWindow.settings.values[keys[tab][index]], oldValue)
                var enter = buttons.filter(function(b) { return b.text === "\u21B5" })[0]
                mouseClick(enter)
                compare(field.value, 6)
                compare(probeWindow.settings.values[keys[tab][index]], 6)
                tryVerify(function() { return !probeWindow.settings.saving }, 3000)
                compare(callApi("GET", "/settings", null)[keys[tab][index]], 6)
                field.editor.begin(field)
                String(oldValue).split("").forEach(function(key) { field.editor.typeKey(key) })
                field.editor.accept()
                tryVerify(function() { return !probeWindow.settings.saving }, 3000)
            }
        }
    }

    function test_switching_tabs_discards_uncommitted_edit() {
        probeWindow.requestActivate()
        callApi("POST", "/probe-actuator", {extended: true})
        tryVerify(function() { return probeWindow.probeFullyExtended() }, 3000)
        var tabs = descendants(probeWindow.contentItem, TabBar)[0]
        mouseClick(tabs.itemAt(0))
        var field = descendants(probeWindow.contentItem, NumberField).filter(function(f) { return f.visible })[0]
        var saved = field.value
        field.forceActiveFocus()
        field.editor.typeKey("7")
        mouseClick(tabs.itemAt(1))
        compare(field.editor.target, null)
        compare(Number(field.text), saved)
        compare(probeWindow.settings.values.outsideXSearchDistance, saved)
    }

    function test_disconnect_discards_uncommitted_edit() {
        probeWindow.requestActivate()
        callApi("POST", "/probe-actuator", {extended: true})
        tryVerify(function() { return probeWindow.probeFullyExtended() }, 3000)
        var tabs = descendants(probeWindow.contentItem, TabBar)[0]
        mouseClick(tabs.itemAt(0))
        var field = descendants(probeWindow.contentItem, NumberField).filter(function(f) { return f.visible })[0]
        var previous = field.value
        field.forceActiveFocus()
        field.editor.typeKey("7")
        probeWindow.requestFailed("Disconnected")
        compare(field.editor.target, null)
        compare(Number(field.text), previous)
        probeWindow.refreshState()
        tryVerify(function() { return probeWindow.machineState.connected }, 3000)
    }

    function test_disconnected_status_discards_draft_even_if_probe_is_extended() {
        probeWindow.requestActivate()
        callApi("POST", "/probe-actuator", {extended: true})
        tryVerify(function() { return probeWindow.probeAvailable() }, 3000)
        var tabs = descendants(probeWindow.contentItem, TabBar)[0]
        mouseClick(tabs.itemAt(0))
        var field = descendants(probeWindow.contentItem, NumberField).filter(function(f) { return f.visible })[0]
        var previous = field.value
        field.forceActiveFocus()
        field.editor.typeKey("7")
        var original = Api.request
        Api.request = function(method, url, body, done) {
            return original(method, url, body, function(reply) {
                if (url.endsWith("/state") && reply.ok)
                    reply.data.connected = false
                done(reply)
            })
        }
        try {
            probeWindow.refreshState()
            tryVerify(function() { return probeWindow.machineState.connected === false }, 3000)
            verify(probeWindow.probeFullyExtended())
            compare(field.editor.target, null)
            compare(Number(field.text), previous)
        } finally {
            Api.request = original
            probeWindow.refreshState()
            tryVerify(function() { return probeWindow.probeAvailable() }, 3000)
        }
    }

    function test_stop_confirmation_does_not_latch_ui_lock() {
        var original = Api.request
        var waiting = true
        Api.request = function(method, url, body, done) {
            return original(method, url, body, function(reply) {
                if (url.endsWith("/state") && reply.ok)
                    reply.data.recoveryFailed = waiting
                done(reply)
            })
        }
        try {
            probeWindow.refreshState()
            tryVerify(function() { return probeWindow.machineState.recoveryFailed === true }, 3000)
            verify(probeWindow.controlsLocked())
            waiting = false
            probeWindow.refreshState()
            tryVerify(function() { return probeWindow.machineState.recoveryFailed === false }, 3000)
            verify(!probeWindow.controlsLocked())
        } finally {
            Api.request = original
            probeWindow.refreshState()
        }
    }

    function test_wcs_requests_are_guarded_and_scoped_to_popup() {
        var original = Api.request
        var replies = []
        var opener = descendants(probeWindow.header, Button).filter(function(b) { return /^G5[4-9]$/.test(b.text) })[0]
        var overlay = probeWindow.Overlay.overlay
        function back() {
            return descendants(overlay, Button).filter(function(b) { return b.visible && b.text === "\u2190" })[0]
        }
        Api.request = function(method, url, body, done) {
            if (url.endsWith("/wcs")) {
                replies.push(done)
                return {abort: function() {}}
            }
            return original(method, url, body, done)
        }
        try {
            mouseClick(opener)
            probeWindow.chooseWcs(55)
            probeWindow.chooseWcs(56)
            compare(replies.length, 1)
            mouseClick(back())
            tryVerify(function() { return back() === undefined })
            mouseClick(opener)
            probeWindow.chooseWcs(57)
            compare(replies.length, 2)
            replies[0]({ok: true})
            verify(back() !== undefined, "Old response cannot close reopened selector")
            replies[1]({ok: true})
            tryVerify(function() { return back() === undefined })
        } finally {
            if (back()) mouseClick(back())
            Api.request = original
        }
    }

    function test_exit_dialog_actions_and_centering() {
        callApi("POST", "/probe-actuator", {extended: true})
        tryVerify(function() { return probeWindow.probeFullyExtended() }, 3000)
        probeWindow.requestExit()
        var overlay = probeWindow.Overlay.overlay
        var prompt
        tryVerify(function() {
            prompt = descendants(overlay, Label).filter(function(label) {
                return label.visible && label.text === "Retract the probe before leaving?"
            })[0]
            return prompt !== undefined
        })
        try {
            var panel = prompt.parent
            while (panel && !(panel.width === 640 && panel.height === 230)) panel = panel.parent
            verify(panel !== null, "Exit dialog panel exists")
            var position = panel.mapToItem(overlay, 0, 0)
            verify(Math.abs(position.x + panel.width / 2 - overlay.width / 2) <= 1,
                   "Exit dialog is horizontally centered in the window")
            verify(Math.abs(position.y + panel.height / 2 - overlay.height / 2) <= 1,
                   "Exit dialog is vertically centered in the window")
            var title = descendants(panel, Label).filter(function(label) {
                return label.visible && label.text === "Probe extended"
            })[0]
            verify(title !== undefined)
            var buttons = descendants(panel, Button)
            compare(buttons.length, 3)
            buttons.forEach(function(button) {
                verify(!/nestprobe|pimprobe/i.test(button.text))
            })
        } finally {
            var cancel = descendants(overlay, Button).filter(function(button) {
                return button.visible && button.text === "Cancel"
            })[0]
            if (cancel) mouseClick(cancel)
            callApi("POST", "/probe-actuator", {extended: false})
        }
    }

    function test_contextual_help() {
        var tabs = descendants(probeWindow.contentItem, TabBar)[0]
        var help = descendants(probeWindow.header, Button).filter(function(b) { return b.text === "?" })[0]
        var expectedText = ["How far to move the probe ball out", "How far from starting X or Y", "What the buttons measure", "Editing values"]
        for (var i = 0; i < 4; ++i) {
            mouseClick(tabs.itemAt(i))
            mouseClick(help)
            var body
            tryVerify(function() {
                body = descendants(probeWindow.Overlay.overlay, TextArea).filter(function(a) {
                    return a.visible && a.text.indexOf(expectedText[i]) !== -1
                })[0]
                return body !== undefined
            })
            verify(body.height > 410)
            waitForRendering(probeWindow.contentItem)
            grabImage(probeWindow.contentItem.parent).save("/tmp/pimprobe-help-" + i + ".png")
            var back = descendants(probeWindow.Overlay.overlay, Button).filter(function(b) {
                return b.visible && b.text === "\u2190"
            })[0]
            mouseClick(back)
        }
    }

    function test_update_page_checks_release_channel_and_installs_selected_build() {
        var tabs = descendants(probeWindow.contentItem, TabBar)[0]
        mouseClick(tabs.itemAt(3))
        var keypad = descendants(probeWindow.contentItem, NumericKeypad)[0]
        verify(!keypad.visible)
        var original = Api.request
        var checks = []
        var installedToken = ""
        Api.request = function(method, url, body, done) {
            if (url.indexOf("/updates/check") !== -1) {
                checks.push(url)
                var development = url.indexOf("development=true") !== -1
                done({ok: true, data: {
                    currentVersion: "2026.09.0",
                    available: {token: development ? "dev-token" : "stable-token",
                        version: development ? "dev" : "2026.09.1",
                        name: development ? "Development Build" : "2026.09.1",
                        notes: development ? "[bugfix] Development fix" : "[feature] Stable feature"}
                }})
                return {abort: function() {}}
            }
            if (url.endsWith("/updates/install")) {
                installedToken = body.token
                done({ok: true, data: {operationId: "01234567-89ab-cdef-0123-456789abcdef"}})
                return {abort: function() {}}
            }
            if (url.indexOf("/updates/status") !== -1) {
                done({ok: true, data: {state: "failed", message: "Test installer stopped"}})
                return {abort: function() {}}
            }
            return original(method, url, body, done)
        }
        try {
            var checkButton = descendants(probeWindow.contentItem, Button).filter(function(button) {
                return button.visible && button.text === "Check for updates"
            })[0]
            verify(checkButton !== undefined)
            mouseClick(checkButton)
            tryCompare(checks, "length", 1)
            var overlay = probeWindow.Overlay.overlay
            tryVerify(function() {
                return descendants(overlay, TextArea).some(function(area) {
                    return area.visible && area.text.indexOf("Stable feature") !== -1
                })
            })
            var channel = descendants(overlay, Switch).filter(function(control) {
                return control.visible && control.text === "Include development releases"
            })[0]
            verify(channel !== undefined)
            mouseClick(channel)
            tryCompare(checks, "length", 2)
            verify(checks[1].indexOf("development=true") !== -1)
            tryVerify(function() {
                return descendants(overlay, TextArea).some(function(area) {
                    return area.visible && area.text.indexOf("Development fix") !== -1
                })
            })
            waitForRendering(probeWindow.contentItem)
            grabImage(probeWindow.contentItem.parent).save("/tmp/pimprobe-update.png")
            var install = descendants(overlay, Button).filter(function(button) {
                return button.visible && button.text === "Install update"
            })[0]
            mouseClick(install)
            compare(installedToken, "")
            var confirm = descendants(overlay, Button).filter(function(button) {
                return button.visible && button.text === "Install"
            })[0]
            verify(confirm !== undefined)
            mouseClick(confirm)
            compare(installedToken, "dev-token")
            tryVerify(function() {
                return descendants(overlay, Label).some(function(label) {
                    return label.visible && label.text.indexOf("Test installer stopped") !== -1
                })
            }, 3000)
        } finally {
            Api.request = original
            var back = descendants(probeWindow.Overlay.overlay, Button).filter(function(button) {
                return button.visible && button.text === "\u2190"
            })[0]
            if (back && back.enabled) mouseClick(back)
        }
    }

    function test_update_status_times_out_while_request_is_pending() {
        var original = Api.request
        var statusAborted = false
        Api.request = function(method, url, body, done) {
            if (url.indexOf("/updates/status") !== -1) {
                return {abort: function() { statusAborted = true }}
            }
            return original(method, url, body, done)
        }
        var updateFlow = updateFlowFixture.createObject(probeWindow)
        try {
            updateFlow.operationId = "01234567-89ab-cdef-0123-456789abcdef"
            updateFlow.phase = "installing"
            updateFlow.pollStatus()
            verify(updateFlow.statusTicks > 0)
            updateFlow.statusTicks = 90
            updateFlow.pollStatus()
            compare(updateFlow.phase, "error")
            verify(statusAborted)
        } finally {
            Api.request = original
            updateFlow.destroy()
        }
    }

}
