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
import ".."
import "Mock.js" as Mock
import "../Api.js" as Api

TestCase {
    id: test
    name: "RepeatabilityFlow"
    when: windowShown
    ApplicationWindow {
        id: view
        visible: true
        width: 800
        height: 480
        RepeatabilityFlow {
            id: flow
            serviceUrl: "http://127.0.0.1:18137/api/v1"
            uiFont: "DejaVu Sans"
            codeFont: "DejaVu Sans Mono"
        }
    }
    function descendants(item, type) {
        var matches = item instanceof type ? [item] : []
        for (var child of item.children || []) matches = matches.concat(descendants(child, type))
        return matches
    }
    function test_machine_coordinate_instructions_and_readings() {
        flow.showCheck()
        tryCompare(flow, "opened", true)
        verify(descendants(flow.contentItem, Label).some(function(label) {
            return label.visible && label.text.indexOf("machine coordinates (G53)") >= 0
        }))
        verify(descendants(flow.contentItem, Button).some(function(button) {
            return button.visible && button.text === "Fixture is ready"
        }))
        flow.phase = "running"
        verify(descendants(flow.contentItem, Label).some(function(label) {
            return label.visible && label.text === "G53 measurements (mm)"
        }))
        flow.close()
    }
    function test_preparation_options_and_keypad() {
        flow.showCheck()
        tryCompare(flow, "opened", true)
        compare(flow.phase, "prepare")
        compare(flow.axes, [true, true, true])
        compare(flow.repetitions, 5)
        compare(flow.home, false)
        compare(flow.retract, true)
        flow.confirmPreparation()
        compare(flow.phase, "options")
        var field = findChild(flow.contentItem, "repetitions")
        verify(field !== null)
        mouseClick(field)
        tryCompare(field.editor, "target", field)
        field.editor.typeKey("8")
        field.editor.accept()
        compare(flow.repetitions, 8)
        mouseClick(field)
        field.editor.typeKey("2")
        field.editor.typeKey(".")
        field.editor.typeKey("5")
        field.editor.accept()
        compare(flow.repetitions, 8)
        field.editor.cancel()
        flow.axes = [false, false, false]
        verify(!flow.canStart)
        flow.axes = [true, true, true]
        verify(flow.canStart)
        flow.close()
    }
    function test_partial_results_survive_failure() {
        flow.showCheck()
        flow.phase = "running"
        flow.handleEvent({type:"measurement", repetition:1, axis:"X", value:-232.488})
        flow.handleEvent({type:"measurement", repetition:1, axis:"Y", value:-204.308})
        compare(flow.measurements[0], [-232.488,-204.308,null])
        flow.handleEvent({type:"error", message:"coarse search made no contact", result: {
            measurements:[[-232.488,-204.308,null]], statistics:[
                {count:1,mean:-232.488,median:-232.488,stddev:null,range:0},
                {count:1,mean:-204.308,median:-204.308,stddev:null,range:0},null]}})
        compare(flow.phase, "failed")
        compare(flow.measurements[0][0], -232.488)
        compare(flow.statistics[0].count, 1)
        compare(flow.number(null), "--")
        flow.close()
    }
    function test_summary_updates_during_measurement() {
        flow.showCheck()
        flow.phase = "running"
        flow.handleEvent({type:"measurement", repetition:1, axis:"X", value:25,
            statistics:[{count:1,mean:25,median:25,stddev:null,range:0},null,null]})
        compare(flow.statistics[0].mean, 25)
        compare(flow.statistics[0].stddev, null)
        flow.handleEvent({type:"measurement", repetition:2, axis:"X", value:27,
            statistics:[{count:2,mean:26,median:26,stddev:Math.sqrt(2),range:2},null,null]})
        compare(flow.statistics[0].count, 2)
        compare(flow.statistics[0].mean, 26)
        compare(flow.readingValue(25, 0), 25)
        flow.phase = "result"
        compare(flow.readingValue(25, 0), -1)
        compare(flow.readingValue(27, 0), 1)
        compare(flow.readingValue(null, 1), null)
        flow.close()
    }
    function test_homing_implies_retraction() {
        flow.home = false
        flow.retract = false
        verify(!flow.retractEachTime)
        flow.home = true
        verify(flow.retractEachTime)
        flow.home = false
        flow.retract = true
    }
    function test_streamed_check() {
        Mock.verifyServer(test, flow.serviceUrl)
        var reply = null
        Api.request("POST", flow.serviceUrl + "/probe-actuator", {extended:false}, function(r) { reply = r })
        tryVerify(function() { return reply !== null }, 3000)
        verify(reply.ok)
        flow.showCheck()
        flow.confirmPreparation()
        flow.home = true
        flow.repetitions = 2
        flow.start()
        compare(flow.phase, "running")
        tryVerify(function() { return flow.phase !== "running" }, 15000)
        compare(flow.failure, "")
        compare(flow.phase, "result")
        compare(flow.measurements.length, 2)
        for (var i = 0; i < 3; ++i) {
            compare(flow.statistics[i].count, 2)
            verify(Math.abs(flow.statistics[i].mean - [-232.5, -204.3, -62.9][i]) < 0.001)
        }
        flow.close()
    }
}
