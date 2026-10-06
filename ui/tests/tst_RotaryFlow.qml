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

TestCase {
    id: test
    name: "RotaryFlow"
    when: windowShown
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
        (item.children || []).forEach(function(child) {
            if (child instanceof type) result.push(child);
            result = result.concat(descendants(child, type));
        });
        return result;
    }
    function initTestCase() {
        Mock.verifyServer(test, http.serviceUrl);
        var reply = null;
        Api.request("POST", http.serviceUrl + "/probe-actuator", {extended:true}, function(r) { reply = r; });
        tryVerify(function() { return reply !== null; }, 3000);
        verify(reply.ok, reply.error);
    }
    function test_review_measure_and_save() {
        flow.showCalibration({rodDiameter:6,xDistance:-20});
        tryVerify(function() { return !flow.busy; }, 3000);
        compare(flow.failure, "");
        verify(flow.reviewID.length > 0);
        verify(flow.text.indexOf("G19 G3") !== -1);
        flow.proceed();
        tryVerify(function() { return flow.phase !== "running"; }, 10000);
        compare(flow.failure, "");
        compare(flow.phase, "result");
        compare(flow.result.stations.length, 2);
        verify(Math.abs(flow.result.xyAngle - Math.atan(0.003) * 180 / Math.PI) < 0.02);
        var labels = descendants(flow.contentItem, Label).filter(function(label) { return label.visible; });
        verify(labels.some(function(label) { return label.text.indexOf("X ") === 0 && label.text.indexOf("Z ") !== -1; }));
        var buttons = descendants(flow.contentItem, Button).filter(function(button) { return button.visible; });
        buttons.forEach(function(button) {
            tryVerify(function() {
                var p = button.mapToItem(flow.contentItem, 0, 0);
                return p.y >= 0 && p.y + button.height <= flow.height;
            }, 3000, button.text + " is clipped");
        });
        flow.requestedAction = "zero";
        flow.applyAction();
        tryVerify(function() { return !flow.busy; }, 3000);
        compare(flow.failure, "");
        verify(flow.result.zeroed);
        flow.requestedAction = "rotation";
        flow.applyAction();
        tryVerify(function() { return !flow.busy; }, 3000);
        compare(flow.failure, "");
        verify(flow.result.rotationApplied);
        flow.close();
    }

    function test_level_surfaces_and_save_their_axes() {
        ["horizontal", "vertical", "verticalNegative"].forEach(function(operation) {
            flow.showCalibration({operation:operation,yDistance:10,zDistance:10});
            tryVerify(function() { return !flow.busy; }, 3000);
            compare(flow.failure, "");
            verify(flow.reviewID.length > 0);
            flow.proceed();
            tryVerify(function() { return flow.phase !== "running"; }, 10000);
            compare(flow.failure, "");
            compare(flow.phase, "result");
            compare(flow.result.level.touches.length, 2);
            verify(Math.abs(flow.result.level.residual) < 0.05);
            var zero = descendants(flow.contentItem,Button).filter(function(button) {
                return button.visible && button.text === "Set " + (operation === "horizontal" ? "A/Z" : "A/Y") + " zero";
            })[0];
            verify(zero !== undefined);
            waitForRendering(flow.contentItem);
            var point = zero.mapToItem(flow.contentItem,0,0);
            verify(point.y + zero.height <= flow.height);
            mouseClick(zero);
            var apply = descendants(view.Overlay.overlay,Button).filter(function(button) {
                return button.visible && button.text === "Apply";
            })[0];
            verify(apply !== undefined);
            mouseClick(apply);
            tryVerify(function() { return !flow.busy && flow.result.zeroed === true; },3000);
            compare(flow.failure, "");
            flow.close();
        });
    }
}
