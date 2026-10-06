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
import "../../ui"
import "../../ui/client/Http.js" as Api
import "../../ui/tests/Mock.js" as Mock

TestCase {
    id: capture
    name: "RotaryScreenshots"
    when: windowShown
    ProbeWindow {
        id: window
        width: 800
        height: 480
        serviceUrl: "http://127.0.0.1:18137/api/v1"
    }
    function descendants(item, type) {
        var found = [];
        var children = item.children || [];
        for (var i = 0; i < children.length; ++i) {
            var child = children[i];
            if (child instanceof type) found.push(child);
            found = found.concat(descendants(child,type));
        }
        return found;
    }
    function save(name) {
        waitForRendering(window.contentItem);
        var path = Qt.resolvedUrl("../../docs/images/" + name + ".png").toString().replace("file://","");
        grabImage(window.contentItem.parent).save(path);
    }
    function test_capture() {
        Mock.verifyServer(capture,window.serviceUrl);
        tryVerify(function() { return window.page.settings.loaded && window.page.machineState.connected; },3000);
        var reply = null;
        Api.request("POST",window.serviceUrl+"/probe-actuator",{extended:true},function(r) { reply=r; });
        tryVerify(function() { return reply !== null; },3000);
        verify(reply.ok);
        tryVerify(function() { return window.page.probeFullyExtended(); },3000);
        var tabs = descendants(window.contentItem,TabBar)[0];
        mouseClick(tabs.itemAt(3));
        save("rotary");
        var button = findChild(window.contentItem, "rotaryCalibrationButton");
        mouseClick(button);
        var flow = Array.prototype.filter.call(window.page.children, function(item) { return item instanceof RotaryFlow; })[0];
        tryVerify(function() { return !flow.busy && flow.reviewID.length > 0; },3000);
        save("rotary-review");
        flow.proceed();
        tryVerify(function() { return flow.phase !== "running"; },10000);
        compare(flow.failure,"");
        compare(flow.phase,"result");
        save("rotary-result");
        flow.close();
        ["horizontal", "vertical", "verticalNegative"].forEach(function(operation) {
            waitForRendering(window.contentItem);
            mouseClick(findChild(window.contentItem,operation + "LevelButton"));
            tryVerify(function() { return !flow.busy && flow.reviewID.length > 0; },3000);
            compare(flow.failure, "");
            flow.proceed();
            tryVerify(function() { return flow.phase !== "running"; },10000);
            compare(flow.failure, "");
            compare(flow.phase, "result");
            save("rotary-" + operation + "-result");
            flow.close();
        });
    }
}
