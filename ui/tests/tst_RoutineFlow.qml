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
import QtQuick.Controls
import QtTest
import ".."
import "../client"
import "Mock.js" as Mock
import "../client/Http.js" as Api

TestCase {
    id: test
    name: "RoutineFlow"
    when: windowShown
    HttpClient { id: http; serviceUrl:"http://127.0.0.1:18137/api/v1"; polling:false }

    ApplicationWindow {
        id: view
        visible: true
        width: 800
        height: 480
        ProbeFlow { id: flow; client: http }
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
        Mock.verifyServer(test, http.serviceUrl)
        var done = false
        var status = 0
        var request = new XMLHttpRequest()
        request.onreadystatechange = function() {
            if (request.readyState === XMLHttpRequest.DONE) { status = request.status; done = true }
        }
        request.open("POST", "http://127.0.0.1:18137/api/v1/probe-actuator")
        request.setRequestHeader("Content-Type", "application/json")
        request.send(JSON.stringify({extended: true}))
        tryVerify(function() { return done }, 3000)
        compare(status, 204)
    }

    function test_review_run_result() {
        flow.showRoutine({family:"outside",x:1,y:1,z:false,wcs:54,zero:false,safeZOffset:40,
                          depth:5,xSearchDistance:10,ySearchDistance:10,retract:0.5,diameter:4,
                          positioningFeed:1000,coarseFeed:30,fineFeed:10})
        tryVerify(function() { return !flow.reviewing }, 3000)
        compare(flow.failure, "")
        flow.proceed()
        tryCompare(flow,"phase","result",10000)
        verify(waitForPolish(view))
        verify(!flow.zeroed)

        var resultLabels = descendants(flow.contentItem, Label).filter(function(label) {
            return label.visible
        })
        var xCoordinate = resultLabels.filter(function(label) { return label.text.indexOf("X ") === 0 })[0]
        var xWorkCoordinate = resultLabels.filter(function(label) { return label.text.indexOf("G54 ") === 0 })[0]
        verify(xCoordinate && xWorkCoordinate)
        verify(flow.completedID.length > 0)
        var measuredX = flow.measuredPosition(0)
        var measuredY = flow.measuredPosition(1)
        var resultView = descendants(flow.contentItem, ProbeResultView)[0]
        var leftPane = resultView.children[0]
        var leftSize = [leftPane.width, leftPane.height]
        var offsetField = findChild(flow, "resultOffset0")
        var fieldX = offsetField.mapToItem(flow.contentItem, 0, 0).x
        offsetField.editor.begin(offsetField)
        verify(waitForPolish(view))
        compare([leftPane.width, leftPane.height], leftSize)
        compare(offsetField.mapToItem(flow.contentItem, 0, 0).x, fieldX)
        offsetField.editor.cancel()
        flow.resultWcsRequested(55)
        tryVerify(function() { return !flow.busy }, 3000)
        compare(flow.failure, "")
        compare(flow.routine.wcs, 55)
        compare(flow.measuredPosition(0), measuredX)
        compare(flow.measuredPosition(1), measuredY)
        var selectedPoint = flow.result.slice()
        flow.setOffset(0, 1.25)
        flow.setOffset(1, -2)
        flow.zeroResult()
        tryCompare(flow,"zeroing",false,5000)
        compare(flow.failure,"")
        verify(flow.zeroed)
        var zeroButton = descendants(flow.contentItem, LabButton).filter(function(button) { return button.text === "Set Work Zero" })[0]
        verify(zeroButton.enabled)
        verify(offsetField.enabled)
        compare([leftPane.width, leftPane.height], leftSize)
        var saved = resultLabels.filter(function(label) { return label.text === "Work zero set" })[0]
        verify(saved && saved.visible)
        var after = null
        Api.request("GET", http.serviceUrl + "/state", null, function(reply) { after = reply.data })
        tryVerify(function() { return after !== null }, 3000)
        tryVerify(function() {
            return JSON.stringify((http.state.status || {}).workPosition) === JSON.stringify(after.status.workPosition)
        }, 3000)
        compare(xWorkCoordinate.text, "G55 " + flow.measuredWorkPosition(0).toFixed(3))
        compare(flow.result, selectedPoint)
        var previousCoordinate = xWorkCoordinate.text
        flow.setOffset(0, -0.75)
        compare(xWorkCoordinate.text, previousCoordinate)
        flow.zeroResult()
        tryCompare(flow, "zeroing", false, 5000)
        compare(flow.failure, "")
        tryVerify(function() { return xWorkCoordinate.text !== previousCoordinate }, 3000)
        compare(xWorkCoordinate.text, "G55 " + flow.measuredWorkPosition(0).toFixed(3))
        verify(zeroButton.enabled && offsetField.enabled)
        compare([leftPane.width, leftPane.height], leftSize)
        flow.returnToStart()
        tryCompare(flow, "phase", "result", 10000)
        compare(flow.failure, "")
        verify(flow.returned)
        flow.close()
        var selected = false
        http.request("wcs.select", {wcs:54}, function(reply) { selected = reply.ok })
        tryVerify(function() { return selected }, 3000)
    }

    function api(path) {
        var result = null
        Api.request("GET", http.serviceUrl + path, null, function(reply) { result = reply })
        tryVerify(function() { return result !== null }, 3000)
        verify(result.ok, result.error)
        return result.data
    }

    function test_angle_result_rotates_the_selected_work_coordinates() {
        var before = api("/state").status.machinePosition
        flow.showRoutine({family:"angle", feature:"x-plus", x:1, y:0, z:false,
            wcs:54, zero:false, safeZOffset:40, depth:10, xSearchDistance:10, ySearchDistance:20,
            retract:0.5, diameter:2, positioningFeed:1000, coarseFeed:300, fineFeed:50})
        flow.proceed()
        tryCompare(flow, "phase", "result", 10000)
        compare(flow.failure, "")
        verify(Math.abs(flow.angle.degrees - 1.1458) < 0.0001)
        compare(api("/state").status.machinePosition, before)
        flow.resultWcsRequested(55)
        tryVerify(function() { return !flow.busy }, 3000)
        compare(flow.routine.wcs, 55)
        var button = descendants(flow.contentItem, LabButton).filter(function(control) {
            return control.text === "Set X/Y rotation"
        })[0]
        mouseClick(button)
        tryVerify(function() { return !flow.busy }, 3000)
        compare(flow.failure, "")
        verify(flow.angle.rotationApplied)
        verify(button.enabled)
        compare(api("/state").status.machinePosition, before)
        flow.close()
        var selected = false
        http.request("wcs.select", {wcs:54}, function(reply) { selected = reply.ok })
        tryVerify(function() { return selected }, 3000)
    }

    function enterReviewValue(key, value, accept) {
        var field = findChild(flow, "review-" + key)
        verify(field !== null, key)
        field.forceActiveFocus()
        field.editor.begin(field)
        String(value).split("").forEach(function(digit) { field.editor.typeKey(digit) })
        if (accept) field.editor.accept()
        return field
    }

    function test_review_options_reach_execution_without_changing_settings() {
        var settings = api("/settings")
        flow.showRoutine({family: "outside", x: 1, y: 0, z: false,
            wcs: 54, zero: false, safeZOffset: 40, depth: 5, xSearchDistance: 10, ySearchDistance: 10, retract: 0.5, diameter: 2,
            positioningFeed: 1000, coarseFeed: 300, fineFeed: 50})
        tryVerify(function() { return !flow.reviewing }, 3000)
        compare(flow.failure, "")
        enterReviewValue("coarseFeed", 125, true)
        enterReviewValue("fineFeed", 25, true)
        enterReviewValue("positioningFeed", 800, true)
        enterReviewValue("retract", 0.7, true)
        enterReviewValue("xSearchDistance", 12, false)
        var proceed = descendants(flow.contentItem, LabButton).filter(function(button) { return button.text === "Proceed" })[0]
        mouseClick(proceed)
        tryCompare(flow, "phase", "result", 10000)
        compare(flow.failure, "")
        verify(flow.logText.indexOf("G38.3 X-12 F800") !== -1, flow.logText)
        verify(/G38.2 [XYZ]-?1.2 F25/.test(flow.logText), flow.logText)
        var history = api("/logs/history")
        var entry = history.filter(function(item) { return item.id === flow.completedID })[0]
        verify(entry !== undefined)
        compare(entry.config.xSearchDistance, 12)
        compare(entry.config.coarseFeed, 125)
        compare(entry.config.retract, 0.7)
        compare(JSON.stringify(api("/settings")), JSON.stringify(settings))
        flow.close()
    }

    function test_invalid_review_options_can_be_corrected() {
        flow.showRoutine({family: "outside", x: 0, y: 0, z: true, wcs: 54, zero: false, safeZOffset: 40,
            depth: 5, xSearchDistance: 10, ySearchDistance: 10, retract: 0.5, diameter: 2,
            positioningFeed: 1000, coarseFeed: 300, fineFeed: 50})
        tryVerify(function() { return !flow.reviewing }, 3000)
        var before = api("/state").status.machinePosition
        enterReviewValue("fineFeed", 6001, true)
        flow.proceed()
        tryVerify(function() { return !flow.reviewing }, 3000)
        compare(flow.phase, "review")
        verify(flow.failure.indexOf("machine limit") !== -1, flow.failure)
        compare(api("/state").status.machinePosition, before)
        enterReviewValue("fineFeed", 50, true)
        flow.proceed()
        tryCompare(flow, "phase", "result", 10000)
        compare(flow.failure, "")
        flow.close()
    }

    function test_inside_result_starts_at_entry_and_can_move_over_measurement() {
        var before = null
        Api.request("GET", http.serviceUrl + "/state", null, function(reply) { before = reply.data })
        tryVerify(function() { return before !== null }, 3000)
        flow.showRoutine({family:"inside",x:1,y:-1,z:false,wcs:54,zero:false,safeZOffset:40,
                          depth:5,xSearchDistance:10,ySearchDistance:10,retract:0.5,diameter:4,
                          positioningFeed:1000,coarseFeed:30,fineFeed:10})
        tryVerify(function() { return !flow.reviewing }, 3000)
        compare(flow.failure, "")
        flow.proceed()
        tryCompare(flow, "phase", "result", 10000)
        var atStart = null
        Api.request("GET", http.serviceUrl + "/state", null, function(reply) { atStart = reply.data })
        tryVerify(function() { return atStart !== null }, 3000)
        for (var i = 0; i < 3; ++i)
            verify(Math.abs(atStart.status.machinePosition[i] - before.status.machinePosition[i]) < 0.002)

        var fields = descendants(flow.contentItem, NumberField).filter(function(field) {
            return field.visible && field.value === flow.safeZOffset
        })
        compare(fields.length, 1)
        fields[0].editor.begin(fields[0])
        fields[0].editor.typeKey("2")
        fields[0].editor.typeKey("5")
        flow.goToMeasured()
        tryCompare(flow, "phase", "result", 10000)
        compare(flow.failure, "")
        verify(flow.positioned)
        var positioned = null
        Api.request("GET", http.serviceUrl + "/state", null, function(reply) { positioned = reply.data })
        tryVerify(function() { return positioned !== null }, 3000)
        verify(Math.abs(positioned.status.machinePosition[2] - before.status.machinePosition[2] - 25) < 0.002)
        verify(Math.abs(positioned.status.machinePosition[0] - before.status.machinePosition[0]) > 0.1)
        verify(Math.abs(positioned.status.machinePosition[1] - before.status.machinePosition[1]) > 0.1)
        flow.close()
    }

    function test_start_error_explains_the_blocked_direction() {
        flow.showRoutine({family:"outside",x:-1,y:0,z:false,wcs:54,zero:false,safeZOffset:40,
                          depth:5,xSearchDistance:1000,ySearchDistance:10,retract:0.5,diameter:4,
                          positioningFeed:1000,coarseFeed:30,fineFeed:10})
        flow.proceed()
        tryVerify(function() { return !flow.reviewing }, 3000)
        verify(flow.failure.indexOf("X+") !== -1, flow.failure)
        verify(flow.failure.indexOf("Move toward X-") !== -1, flow.failure)
        flow.close()
    }

    function test_stream_start_error_keeps_service_message() {
        flow.startMotion("expired", "run")
        tryCompare(flow, "phase", "failed", 3000)
        verify(flow.failure.indexOf("This operation is no longer available") !== -1, flow.failure)
    }

    function test_zero_failure_retry_data() {
        return [
            {tag: "busy", reply: {ok: false, data: {code: "busy"}, error: "Busy"}, retry: true},
            {tag: "uncertain", reply: {ok: false, error: "Disconnected"}, retry: false},
            {tag: "consumed", reply: {ok: false, data: {code: "failed"}, error: "Failed"}, retry: false}
        ]
    }

    function test_center_dimension_labels_and_axes() {
        var cases = [
            {feature:"pocket", spans:[14, 16.8, null], labels:["Width X", "Length Y"]},
            {feature:"hole", spans:[14, 16.8, null], labels:["Span X", "Span Y"]},
            {feature:"y-valley", spans:[null, 16.8, null], labels:["Width Y"]}
        ]
        for (var item of cases) {
            flow.routine = {family:"center", feature:item.feature}
            var raw = item.spans.map(function(v) { return v === null ? null : v - 2 })
            flow.acceptResult({point:[0, 0, null], machinePoint:[-100, -80, null],
                spans:item.spans, rawSpans:raw, rawPoint:[-43, -70, null], zeroed:false}, "measured")
            compare(flow.dimensions.map(function(d) { return d.label }), item.labels)
            compare(flow.dimensions.map(function(d) { return d.value }), item.spans.filter(function(v) { return v !== null }))
            compare(flow.dimensions.map(function(d) { return d.raw }), raw.filter(function(v) { return v !== null }))
            compare(flow.rawPoint, [-43, -70, null])
        }
    }

    function test_measured_work_coordinates_data() {
        return [
            {tag: "unrotated", angle: 0, toolOffset: 0, expected: [14, 18, 31]},
            {tag: "rotated", angle: 90, toolOffset: 0, expected: [8, 16, 31]},
            {tag: "tool-compensated", angle: 0, toolOffset: 7, expected: [14, 18, 31]}
        ]
    }

    function test_measured_work_coordinates(data) {
        var previous = {state: http.state, routine: flow.routine, machinePoint: flow.machinePoint, offsets: flow.offsets}
        try {
            flow.routine = {wcs: 55}
            flow.machinePoint = [-96, -82, -39]
            http.state = {
                status: {wcs: 55}, wcsRotations: {55: data.angle},
                coordinates: {spindle: {
                    machinePosition: [-100, -80, -40 - data.toolOffset],
                    workPosition: [10, 20, 30 - data.toolOffset]
                }}
            }
            for (var i = 0; i < 3; ++i)
                verify(Math.abs(flow.measuredWorkPosition(i) - data.expected[i]) < 0.0001)
            flow.setOffset(0, 2)
            compare(flow.measuredWorkPosition(0), data.expected[0])
            http.state = Object.assign({}, http.state, {coordinates: {spindle: {
                machinePosition: http.state.coordinates.spindle.machinePosition,
                workPosition: [15, 20, 30 - data.toolOffset]
            }}})
            verify(Math.abs(flow.measuredWorkPosition(0) - data.expected[0] - 5) < 0.0001)
        } finally {
            http.state = previous.state
            flow.routine = previous.routine
            flow.machinePoint = previous.machinePoint
            flow.offsets = previous.offsets
        }
    }

    function test_zero_failure_retry(data) {
        var original = Api.request
        var callback
        var count = 0
        Api.request = function(method, url, body, done) {
            ++count
            callback = done
            return {abort: function() {}}
        }
        try {
            flow.zeroed = false
            flow.completedID = "measurement"
            flow.zeroResult()
            flow.zeroResult()
            compare(count, 1)
            callback(data.reply)
            compare(flow.completedID, data.retry ? "measurement" : "")
        } finally {
            Api.request = original
        }
    }
}
