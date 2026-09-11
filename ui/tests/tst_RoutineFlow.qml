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
    name: "RoutineFlow"
    when: windowShown

    ApplicationWindow {
        id: view
        visible: true
        width: 800
        height: 480
        ProbeFlow { id: flow; serviceUrl: "http://127.0.0.1:18137/api/v1" }
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
        Mock.verifyServer(test, flow.serviceUrl)
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
        var snapshot = null
        Api.request("GET", flow.serviceUrl + "/state", null, function(reply) { snapshot = reply.data })
        tryVerify(function() { return snapshot !== null }, 3000)
        var expectedX = snapshot.status.workPosition[0] + snapshot.settings["33"] - 2
        var expectedY = snapshot.status.workPosition[1] + snapshot.settings["34"] - 2
        flow.showRoutine({family:"outside",x:1,y:1,z:false,wcs:54,zero:false,safeZOffset:40,
                          depth:5,xSearchDistance:10,ySearchDistance:10,retract:0.5,diameter:4,
                          positioningFeed:1000,coarseFeed:30,fineFeed:10})
        tryVerify(function() { return !flow.reviewing }, 3000)
        compare(flow.failure, "")
        verify(flow.reviewID.length > 0)
        compare(flow.width,800)
        compare(flow.height,480)
        verify(flow.program.join("\n").indexOf("G10 L20") === -1)
        var captured = false
        flow.contentChildren[0].grabToImage(function(result) { result.saveToFile("/tmp/pimprobe-review.png"); captured = true })
        tryVerify(function() { return captured }, 3000)
        flow.proceed()
        compare(flow.phase,"running")
        tryVerify(function() { return flow.logText.length > 0 }, 5000)
        verify(flow.phase === "running", "progress should arrive before completion")
        captured = false
        flow.contentChildren[0].grabToImage(function(result) { result.saveToFile("/tmp/pimprobe-progress.png"); captured = true })
        tryVerify(function() { return captured }, 3000)
        tryCompare(flow,"phase","result",10000)
        verify(Math.abs(flow.result[0] - expectedX) < 0.001)
        verify(Math.abs(flow.result[1] - expectedY) < 0.001,
               "Expected Y " + expectedY + ", got " + flow.result[1])
        compare(flow.result[2],null)
        verify(!flow.zeroed)

		var resultLabels = descendants(flow.contentItem, Label).filter(function(label) {
			return label.visible
		})
		var xCoordinate = resultLabels.filter(function(label) { return label.text.indexOf("X  ") === 0 })[0]
		var xZero = resultLabels.filter(function(label) { return label.text.indexOf("G54 zero at G53") === 0 })[0]
		var yCoordinate = resultLabels.filter(function(label) { return label.text.indexOf("Y  ") === 0 })[0]
		verify(xCoordinate && xZero && yCoordinate)
		var xCoordinateBottom = xCoordinate.mapToItem(flow.contentItem, 0, xCoordinate.height).y
		var xZeroTop = xZero.mapToItem(flow.contentItem, 0, 0).y
		var xZeroBottom = xZero.mapToItem(flow.contentItem, 0, xZero.height).y
		var yCoordinateTop = yCoordinate.mapToItem(flow.contentItem, 0, 0).y
		verify(xZeroTop - xCoordinateBottom <= 2,
		       "Zero line is too far below X coordinate: " + (xZeroTop - xCoordinateBottom))
		verify(yCoordinateTop - xZeroBottom <= 12,
		       "Y coordinate is too far below X zero line: " + (yCoordinateTop - xZeroBottom))
		verify(flow.completedID.length > 0)
		var measuredX = flow.measuredPosition(0)
		var measuredY = flow.measuredPosition(1)
		flow.setOffset(0, 1.25)
		flow.setOffset(1, -2)
		flow.zeroResult()
		tryCompare(flow,"zeroing",false,5000)
		compare(flow.failure,"")
		verify(flow.zeroed)
		verify(flow.completedID.length > 0)
		var after = null
		Api.request("GET", flow.serviceUrl + "/state", null, function(reply) { after = reply.data })
		tryVerify(function() { return after !== null }, 3000)
		verify(Math.abs(after.status.machinePosition[0] - after.status.workPosition[0] - measuredX - 1.25) < 0.002)
		verify(Math.abs(after.status.machinePosition[1] - after.status.workPosition[1] - measuredY + 2) < 0.002)
		verify(Math.abs(flow.result[0] - expectedX) < 0.001)
		verify(flow.logText.indexOf("; Coarse") !== -1)
		verify(flow.logText.indexOf("; Fine") !== -1)
		var backoff = /#<x_after_backoff> := (-?[0-9.]+)/.exec(flow.logText)
		verify(backoff !== null)
		verify(Math.abs(Number(backoff[1]) - (snapshot.status.machinePosition[0] - 4.5)) < 0.002)
		verify(flow.logText.indexOf("X[") === -1)
        verify(flow.simulated)
        waitForRendering(flow.contentChildren[0])
        captured = false
        flow.contentChildren[0].grabToImage(function(result) { result.saveToFile("/tmp/pimprobe-result.png"); captured = true })
        tryVerify(function() { return captured }, 3000)
        flow.returnToStart()
        tryCompare(flow, "phase", "result", 10000)
        compare(flow.failure, "")
        verify(flow.returned)
        flow.close()
    }

    function test_inside_result_starts_at_entry_and_can_move_over_measurement() {
        var before = null
        Api.request("GET", flow.serviceUrl + "/state", null, function(reply) { before = reply.data })
        tryVerify(function() { return before !== null }, 3000)
        flow.showRoutine({family:"inside",x:1,y:-1,z:false,wcs:54,zero:false,safeZOffset:40,
                          depth:5,xSearchDistance:10,ySearchDistance:10,retract:0.5,diameter:4,
                          positioningFeed:1000,coarseFeed:30,fineFeed:10})
        tryVerify(function() { return !flow.reviewing }, 3000)
        compare(flow.failure, "")
        flow.proceed()
        tryCompare(flow, "phase", "result", 10000)
        var atStart = null
        Api.request("GET", flow.serviceUrl + "/state", null, function(reply) { atStart = reply.data })
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
        Api.request("GET", flow.serviceUrl + "/state", null, function(reply) { positioned = reply.data })
        tryVerify(function() { return positioned !== null }, 3000)
        verify(Math.abs(positioned.status.machinePosition[2] - before.status.machinePosition[2] - 25) < 0.002)
        verify(Math.abs(positioned.status.machinePosition[0] - before.status.machinePosition[0]) > 0.1)
        verify(Math.abs(positioned.status.machinePosition[1] - before.status.machinePosition[1]) > 0.1)
        flow.close()
    }

    function test_review_error_explains_the_blocked_direction() {
        flow.showRoutine({family:"outside",x:-1,y:0,z:false,wcs:54,zero:false,safeZOffset:40,
                          depth:5,xSearchDistance:1000,ySearchDistance:10,retract:0.5,diameter:4,
                          positioningFeed:1000,coarseFeed:30,fineFeed:10})
        tryVerify(function() { return !flow.reviewing }, 3000)
        verify(flow.failure.indexOf("X+") !== -1, flow.failure)
        verify(flow.failure.indexOf("Move toward X-") !== -1, flow.failure)
        flow.close()
    }

    function test_stream_start_error_keeps_service_message() {
        flow.startMotion("expired", "run")
        tryCompare(flow, "phase", "failed", 3000)
        verify(flow.failure.indexOf("Review expired") !== -1, flow.failure)
    }

    function test_zero_failure_retry_data() {
        return [
            {tag: "busy", reply: {ok: false, data: {code: "busy"}, error: "Busy"}, retry: true},
            {tag: "uncertain", reply: {ok: false, error: "Disconnected"}, retry: false},
            {tag: "consumed", reply: {ok: false, data: {code: "failed"}, error: "Failed"}, retry: false}
        ]
    }

    function test_center_dimensions_data() {
        return [
            {tag:"pocket", feature:"pocket", labels:["Width X", "Length Y"], values:[14, 16.8]},
            {tag:"hole", feature:"hole", labels:["Span X", "Span Y"], values:[2 * Math.sqrt(25 - 2.25) + 4, 14]},
            {tag:"boss", feature:"boss", labels:["Span X", "Span Y"], values:[2 * Math.sqrt(81 - 2.25) - 4, 14]},
            {tag:"x-ridge", feature:"x-ridge", labels:["Width X"], values:[14]},
            {tag:"y-valley", feature:"y-valley", labels:["Width Y"], values:[16.8]}
        ]
    }
    function test_center_dimensions(data) {
        flow.showRoutine({family:"center", feature:data.feature,
                          x:data.feature.indexOf("y-") === 0 ? 0 : 1,
                          y:data.feature.indexOf("x-") === 0 ? 0 : 1, z:false,
                          wcs:54, zero:false, safeZOffset:40, depth:5,
                          xSearchDistance:20, ySearchDistance:24, retract:0.5, diameter:4,
                          positioningFeed:1000, coarseFeed:30, fineFeed:10})
        tryVerify(function() { return !flow.reviewing }, 3000)
        compare(flow.failure, "")
        flow.proceed()
        tryCompare(flow, "phase", "result", 10000)
        compare(flow.dimensions.length, data.labels.length)
        for (var i = 0; i < data.labels.length; ++i) {
            compare(flow.dimensions[i].label, data.labels[i])
            verify(Math.abs(flow.dimensions[i].value - data.values[i]) < 0.002)
        }
        waitForRendering(flow.contentChildren[0])
        grabImage(view.contentItem.parent).save("/tmp/pimprobe-" + data.feature + "-result.png")
        flow.returnToStart()
        tryCompare(flow, "phase", "result", 10000)
        compare(flow.failure, "")
        verify(flow.returned)
        flow.setOffset(0, 1.25)
        flow.zeroResult()
        tryCompare(flow, "zeroing", false, 5000)
        compare(flow.failure, "")
        verify(flow.zeroed)
        for (var j = 0; j < data.labels.length; ++j)
            verify(Math.abs(flow.dimensions[j].value - data.values[j]) < 0.002)
        flow.close()
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
