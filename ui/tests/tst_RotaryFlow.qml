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
import "../controls"
import "../client"
import QtQuick.Controls
import QtTest
import ".."
import "Mock.js" as Mock
import "../client/Http.js" as Api
import "../ProbePages.js" as Pages

TestCase {
    id: test
    name: "RotaryFlow"
    when: windowShown
    property var settings: ({})
    HttpClient { id: http; serviceUrl: "http://127.0.0.1:18137/api/v1"; polling: false }
    ApplicationWindow {
        id: view
        visible: true
        width: 800
        height: 480
        RotaryFlow { id: flow; client: http; uiFont: "DejaVu Sans"; codeFont: "DejaVu Sans Mono" }
    }
    function descendants(item, type) {
        var result = [];
        var children = item.children || [];
        for (var i = 0; i < children.length; ++i) {
            var child = children[i];
            if (child instanceof type) result.push(child);
            result = result.concat(descendants(child, type));
        }
        return result;
    }
    function initTestCase() {
        Mock.verifyServer(test, http.serviceUrl);
        var reply = null;
        Api.request("POST", http.serviceUrl + "/probe-actuator", {extended:true}, function(r) { reply = r; });
        tryVerify(function() { return reply !== null; }, 3000);
        verify(reply.ok, reply.error);
        reply = null;
        Api.request("GET", http.serviceUrl + "/settings", null, function(r) { reply = r; });
        tryVerify(function() { return reply !== null; }, 3000);
        verify(reply.ok, reply.error);
        settings = reply.data;
    }
    function test_review_measure_and_save() {
        flow.showCalibration(Object.assign(Pages.rotaryConfig(settings), {rodDiameter:6,xDistance:-20}));
        tryVerify(function() { return !flow.busy; }, 3000);
        compare(flow.failure, "");
        var fineFeed = findChild(flow, "review-fineFeed");
        compare(Number(fineFeed.text), flow.config.fineFeed);
        var distance = findChild(flow, "review-xDistance");
        distance.forceActiveFocus();
        distance.editor.begin(distance);
        distance.editor.typeKey("2");
        distance.editor.typeKey("5");
        distance.editor.typeKey("sign");
        flow.proceed();
        tryCompare(flow, "phase", "result", 10000);
        compare(flow.failure, "");
        compare(flow.phase, "result");
        compare(flow.result.stations.length, 2);
        verify(Math.abs(flow.result.stations[1].center[0] - flow.result.stations[0].center[0] + 25) < 0.001);
        var labels = descendants(flow.contentItem, Label).filter(function(label) { return label.visible; });
        flow.result.stations.forEach(function(station) {
            station.center.forEach(function(value) {
                verify(labels.some(function(label) { return label.text === Number(value).toFixed(3); }));
            });
        });
        var buttons = descendants(flow.contentItem, Button).filter(function(button) { return button.visible; });
        var stations = JSON.stringify(flow.result.stations);
        flow.resultWcsRequested(56);
        tryVerify(function() { return !flow.busy; }, 3000);
        compare(flow.failure, "");
        compare(flow.result.wcs, 56);
        compare(JSON.stringify(flow.result.stations), stations);
        flow.requestedAction = "zero";
        flow.applyAction();
        tryVerify(function() { return !flow.busy; }, 3000);
        compare(flow.failure, "");
        verify(flow.result.zeroed);
        var zero = buttons.filter(function(button) { return button.text === "Set Y/Z zero"; })[0];
        verify(zero.enabled);
        flow.applyAction();
        tryVerify(function() { return !flow.busy; }, 3000);
        compare(flow.failure, "");
        compare(JSON.stringify(flow.result.stations), stations);
        flow.requestedAction = "rotation";
        flow.applyAction();
        tryVerify(function() { return !flow.busy; }, 3000);
        compare(flow.failure, "");
        verify(flow.result.rotationApplied);
        verify(buttons.filter(function(button) { return button.text === "Set X/Y rotation"; })[0].enabled);
        flow.close();
        var restored = false;
        http.request("wcs.select", {wcs:54}, function(reply) { restored = reply.ok; });
        tryVerify(function() { return restored; }, 3000);
    }

    function test_level_review_options_and_zero_confirmation_data() {
        return [{tag:"horizontal"}, {tag:"vertical"}, {tag:"verticalNegative"}];
    }

    function test_level_review_options_and_zero_confirmation(data) {
        flow.showCalibration(Pages.rotaryConfig(settings, data.tag));
        tryVerify(function() { return !flow.busy; }, 3000);
        compare(flow.failure, "");
        var yDistance = findChild(flow, "review-yDistance");
        yDistance.editor.begin(yDistance);
        yDistance.editor.typeKey("1");
        yDistance.editor.typeKey("2");
        yDistance.editor.accept();
        var zDistance = findChild(flow, "review-zDistance");
        zDistance.editor.begin(zDistance);
        zDistance.editor.typeKey("8");
        flow.proceed();
        tryCompare(flow, "phase", "result", 10000);
        compare(flow.failure, "");
        compare(flow.config.yDistance, 12);
        compare(flow.config.zDistance, 8);
        compare(flow.result.level.touches.length, 2);
        var horizontal = data.tag === "horizontal";
        var axis = horizontal ? 1 : 2;
        var spacing = flow.result.level.touches[1][axis] - flow.result.level.touches[0][axis];
        verify(Math.abs(spacing - (horizontal ? 12 : -8)) < 0.001);
        var zero = descendants(flow.contentItem,Button).filter(function(button) {
            return button.visible && button.text === (horizontal ? "Set A/Z zero" : "Set A/Y zero");
        })[0];
        verify(zero !== undefined);
        verify(waitForPolish(view));
        mouseClick(zero);
        var apply = descendants(view.Overlay.overlay,Button).filter(function(button) {
            return button.visible && button.text === "Apply";
        })[0];
        verify(apply !== undefined);
        mouseClick(apply);
        tryVerify(function() { return !flow.busy && flow.result.zeroed === true; },3000);
        compare(flow.failure, "");
        verify(zero.enabled);
        flow.close();
    }
}
