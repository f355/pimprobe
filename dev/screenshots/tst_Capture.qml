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
import QtTest
import "../../ui"
import "../../ui/Api.js" as Api
import "../../ui/tests/Mock.js" as Mock

TestCase {
    id: capture
    name: "OperatorScreenshots"
    when: windowShown

    ProbeWindow {
        id: window
        width: 800
        height: 480
        serviceUrl: "http://127.0.0.1:18137/api/v1"
    }

    function descendants(item, type) {
        var found = []
        var children = item.children || []
        for (var i = 0; i < children.length; ++i) {
            if (children[i] instanceof type) found.push(children[i])
            found = found.concat(descendants(children[i], type))
        }
        return found
    }

    function save(name) {
        waitForRendering(window.contentItem)
        wait(100)
        var path = Qt.resolvedUrl("../../docs/images/" + name + ".png").toString().replace("file://", "")
        grabImage(window.contentItem.parent).save(path)
    }

    function test_capture() {
        Mock.verifyServer(capture, window.serviceUrl)
        tryVerify(function() { return window.settings.loaded && window.machineState.connected }, 3000)
        var reply = null
        Api.request("POST", window.serviceUrl + "/probe-actuator", {extended:true}, function(r) { reply = r })
        tryVerify(function() { return reply !== null }, 3000)
        verify(reply.ok)
        tryVerify(function() { return window.probeFullyExtended() }, 3000)
        var tabs = descendants(window.contentItem, TabBar)[0]
        var names = ["outside", "inside", "center", "settings"]
        for (var i = 0; i < names.length; ++i) {
            mouseClick(tabs.itemAt(i))
            save(names[i])
        }
        mouseClick(tabs.itemAt(0))
        var button = descendants(window.contentItem, OutsideProbeButton).filter(function(b) {
            return b.xApproach === 1 && b.yApproach === 1
        })[0]
        mouseClick(button)
        var proceed
        tryVerify(function() {
            proceed = descendants(window.Overlay.overlay, Button).filter(function(b) {
                return b.visible && b.text === "Proceed" && b.enabled
            })[0]
            return proceed !== undefined
        }, 3000)
        save("review")
        mouseClick(proceed)
        wait(350)
        save("progress")
        var zero
        tryVerify(function() {
            zero = descendants(window.Overlay.overlay, Button).filter(function(b) {
                return b.visible && b.text === "Set Work Zero" && b.enabled
            })[0]
            return zero !== undefined
        }, 10000)
        save("result")
        var back = descendants(window.Overlay.overlay, Button).filter(function(b) {
            return b.visible && b.text === "Go to starting position"
        })[0]
        var flow = window.contentData.filter(function(item) { return item instanceof ProbeFlow })[0]
        verify(flow !== undefined)
        mouseClick(back)
        tryCompare(flow, "phase", "result", 10000)
        verify(flow.returned)
        var done = descendants(window.Overlay.overlay, Button).filter(function(b) {
            return b.visible && b.text === "Close"
        })[0]
        mouseClick(done)
        tryCompare(flow, "visible", false)
        tryVerify(function() { return !window.controlsLocked() }, 3000)
        mouseClick(tabs.itemAt(2))
        var pocket = descendants(window.contentItem, CenterProbeButton).filter(function(b) {
            return b.feature === "pocket"
        })[0]
        tryVerify(function() { return pocket.visible && pocket.enabled }, 3000)
        mouseClick(pocket)
        tryVerify(function() { return proceed.visible && proceed.enabled }, 3000)
        mouseClick(proceed)
        tryVerify(function() { return zero.visible && zero.enabled }, 10000)
        save("dimensions")
    }
}
