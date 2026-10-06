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
import "../controls"

TestCase {
    id: test
    name: "EmbeddedPage"
    when: windowShown

    QtObject {
        id: client
        property var state: ({
                connected: true,
                contactActive: false,
                recoveryFailed: false,
                actuatorPending: false,
                status: {
                    mode: "Ready",
                    wcs: 54,
                    probeActuatorKnown: true,
                    probeActuator: 1,
                    machinePosition: [-100, -100, -40, 0],
                    workPosition: [10, 10, 5, 0]
                }
            })
        property string error: ""
        property var values: ({
                probeDiameter: 2,
                retractDistance: 0.5,
                positioningFeed: 1000,
                coarseFeed: 300,
                fineFeed: 50,
                safeZOffset: 40,
                outsideXSearchDistance: 10,
                outsideYSearchDistance: 10,
                outsideDepth: 5,
                insideXSearchDistance: 10,
                insideYSearchDistance: 10,
                insideDepth: 5,
                centerXSearchDistance: 20,
                centerYSearchDistance: 20,
                centerDepth: 5,
                rotaryRodDiameter: 10,
                rotaryXDistance: 30,
                rotaryYDistance: 10,
                rotaryZDistance: 10,
                rotaryFeed: 360
            })
        property var events
        property var finish
        property int aborts: 0
        property bool rejectRetraction: false
        function refresh() {
        }
        function request(operation, args, done) {
            var data = {};
            if (operation === "settings.schema") {
                Object.keys(values).forEach(function (key) {
                    data[key] = {
                        minimum: 0.01,
                        maximum: 10000
                    };
                });
            } else if (operation === "settings.get")
                data = values;
            else if (operation === "settings.update") {
                values = Object.assign({}, values, args);
                data = values;
            } else if (operation === "routine.review") {
                data = {
                    id: "review-1",
                    program: ["G21 G94 G91", "G38.3 X-10 F300"],
                    simulated: true
                };
            } else if (operation === "rotary.review") {
                data = {id: "rotary-1", program: "G21 G94 G91", simulated: true};
            } else if (operation === "probe.set") {
                if (!args.extended && rejectRetraction) {
                    done({ok: false, error: "Retraction failed"});
                    return {abort: function() {}};
                }
                var updated = Object.assign({}, state);
                updated.status = Object.assign({}, state.status, {
                    probeActuator: args.extended ? 1 : 0
                });
                state = updated;
            } else {
                done({
                    ok: false,
                    error: "Unsupported test operation: " + operation
                });
                return {
                    abort: function () {}
                };
            }
            done({
                ok: true,
                data: data
            });
            return {
                abort: function () {}
            };
        }
        function stream(operation, args, onEvent, onFinished) {
            events = onEvent;
            finish = onFinished;
            return {
                abort: function () {
                    ++client.aborts;
                }
            };
        }
    }

    ApplicationWindow {
        id: host
        visible: true
        width: 800
        height: 480
        Rectangle {
            id: header
            height: 64
            width: parent.width
            color: "#242424"
            Label {
                anchors.centerIn: parent
                text: page.pageTitle
                color: "#D8D8D8"
                font.pixelSize: 24
            }
        }
        ProbePage {
            id: page
            anchors.top: header.bottom
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            client: client
            showHeader: false
            uiFontFamily: "DejaVu Sans"
            monoFontFamily: "DejaVu Sans Mono"
        }
    }
    SignalSpy {
        id: left
        target: page
        signalName: "leaveRequested"
    }
    SignalSpy {
        id: alarm
        target: page
        signalName: "alarmRequested"
    }
    Component {
        id: styledPage
        ProbePage {
            style: Style { headerHeight: 72; text: "#123456" }
        }
    }
    Component {
        id: sharedButton
        LabButton {}
    }
    Component {
        id: backButton
        BackButton {}
    }
    Component {
        id: exitCancelledSpy
        SignalSpy { signalName: "exitCancelled" }
    }
    Component {
        id: leaveSpy
        SignalSpy { signalName: "leaveRequested" }
    }
    Component {
        id: embeddedContainer
        Item {
            id: container
            required property var client
            property int rootTop: 64
            property int rootHeight: 416
            property bool compactLayout: false
            width: 800
            height: 480
            property alias component: embeddedPage
            property alias subpageOpen: embeddedPage.subpageOpen
            Loader {
                objectName: "actuatorStrip"
                width: parent.width
                height: 64
                visible: !container.subpageOpen
                sourceComponent: embeddedPage.headerControls
            }
            ProbePage {
                id: embeddedPage
                y: subpageOpen ? 0 : container.rootTop
                width: parent.width
                height: subpageOpen ? parent.height : container.rootHeight
                client: container.client
                showHeader: subpageOpen
                compactLayout: container.compactLayout
            }
        }
    }

    function navigationContainer() {
        var container = createTemporaryObject(embeddedContainer, page.parent, {client: client});
        verify(container !== null);
        compare(container.component.settings.loaded, true);
        return container;
    }

    function test_public_settings_discards_draft_and_closes_subpages() {
        var container = navigationContainer();
        var embedded = container.component;
        var field = descendants(embedded, NumberField).filter(function(item) { return item.visible; })[0];
        verify(field !== undefined);
        var original = field.text;
        field.editor.begin(field);
        field.editor.typeKey("7");
        embedded.openHelp();
        embedded.openWcs();
        embedded.openSettings();
        compare(embedded.subpageOpen, false);
        compare(field.editor.target, null);
        compare(field.text, original);
        embedded.openHelp();
        compare(embedded.pageTitle, "Settings help");
        embedded.requestBack();
    }

    function test_public_utilities_uses_full_page_and_restores_strip() {
        var container = navigationContainer();
        var embedded = container.component;
        var strip = findChild(container, "actuatorStrip");
        compare(embedded.y, 64);
        compare(embedded.height, 416);
        compare(embedded.showHeader, false);
        compare(strip.visible, true);
        embedded.openUtilities();
        compare(container.subpageOpen, true);
        compare(embedded.pageTitle, "Utilities");
        compare(embedded.y, 0);
        compare(embedded.height, 480);
        compare(embedded.showHeader, true);
        compare(strip.visible, false);
        compare(embedded.topPage.height, 480);
        compare(descendants(embedded.topPage, UtilitiesPanel)[0].showHeader, true);
        var back = descendants(embedded.topPage, BackButton)[0];
        verify(back.contentItem.paintedHeight <= back.contentItem.height + 0.5);
        embedded.requestBack();
        compare(container.subpageOpen, false);
        compare(embedded.y, 64);
        compare(embedded.height, 416);
        compare(strip.visible, true);
    }

    function test_back_glyph_fits_header_data() {
        return [{tag: "utility", width: 70, height: 56},
            {tag: "page", width: 64, height: 64}];
    }
    function test_back_glyph_fits_header(data) {
        var back = createTemporaryObject(backButton, host.contentItem,
            {width: data.width, height: data.height});
        verify(back !== null);
        verify(waitForPolish(host));
        verify(back.width >= 48 && back.height >= 48);
        verify(back.contentItem.paintedWidth <= back.contentItem.width + 0.5);
        verify(back.contentItem.paintedHeight <= back.contentItem.height + 0.5);
    }

    function test_public_navigation_keeps_running_operation() {
        var container = navigationContainer();
        var embedded = container.component;
        embedded.openRoutineReview("inside", {x: 1, y: 1, z: false});
        var flow = embedded.topPage;
        flow.proceed();
        compare(embedded.busy, true);
        var aborts = client.aborts;
        embedded.openSettings();
        embedded.openUtilities();
        compare(embedded.topPage, flow);
        compare(flow.phase, "running");
        compare(client.aborts, aborts);
        client.events({type: "result", result: {
            point: [1, 2, null], machinePoint: [-99, -98, null], spans: [null, null, null],
            zeroed: false, returned: false, positioned: false
        }});
        client.finish("");
        compare(flow.phase, "result");
        embedded.openSettings();
        compare(embedded.subpageOpen, false);
        embedded.openHelp();
        compare(embedded.pageTitle, "Settings help");
    }

    function test_public_navigation_respects_recovery_lock() {
        var container = navigationContainer();
        var embedded = container.component;
        embedded.machineState = Object.assign({}, client.state, {recoveryFailed: true});
        compare(embedded.busy, true);
        embedded.openSettings();
        embedded.openUtilities();
        compare(embedded.subpageOpen, false);
        embedded.openHelp();
        compare(embedded.pageTitle, "Outside help");
        embedded.requestBack();
        compare(embedded.subpageOpen, true);
    }

    function test_exit_cancel_reports_abandoned_navigation_data() {
        return [{tag: "prompt", retractFailure: false}, {tag: "failed-retraction", retractFailure: true}];
    }
    function test_exit_cancel_reports_abandoned_navigation(data) {
        var container = navigationContainer();
        var embedded = container.component;
        var cancelled = createTemporaryObject(exitCancelledSpy, test, {target: embedded});
        var leaves = createTemporaryObject(leaveSpy, test, {target: embedded});
        embedded.requestExit();
        var buttons = descendants(host.Overlay.overlay, Button);
        var cancel = buttons.filter(function(button) { return button.visible && button.text === "Cancel"; })[0];
        verify(cancel !== undefined);
        compare(cancelled.count, 0);
        if (data.retractFailure) {
            client.rejectRetraction = true;
            var retract = buttons.filter(function(button) { return button.visible && button.text === "Retract and exit"; })[0];
            mouseClick(retract);
            client.rejectRetraction = false;
            compare(embedded.exitAfterRetract, false);
            compare(embedded.actionError, "Retraction failed");
            compare(cancelled.count, 0);
        }
        mouseClick(cancel);
        compare(cancelled.count, 1);
        compare(leaves.count, 0);
        compare(embedded.probeFullyExtended(), true);
        compare(cancel.visible, false);
        embedded.requestExit();
        compare(cancelled.count, 1);
        var leave = buttons.filter(function(button) { return button.visible && button.text === "Leave extended"; })[0];
        mouseClick(leave);
        compare(leaves.count, 1);
        compare(cancelled.count, 1);
    }

    function test_compact_root_targets_data() {
        return [
            {tag: "outside", index: 0}, {tag: "inside", index: 1},
            {tag: "center", index: 2}, {tag: "rotary", index: 3}
        ];
    }
    function test_compact_root_targets(data) {
        var container = createTemporaryObject(embeddedContainer, page.parent,
            {client: client, rootTop: 120, rootHeight: 304, compactLayout: true});
        var embedded = container.component;
        descendants(embedded, TabBar)[0].currentIndex = data.index;
        waitForRendering(embedded);
        descendants(embedded, TabButton).filter(function(item) { return item.visible; }).forEach(function(tab) {
            compare(tab.height, 56);
        });
        descendants(embedded, Button).filter(function(item) { return item.visible; }).forEach(function(button) {
            var position = button.mapToItem(embedded, 0, 0);
            verify(button.width >= 48 && button.height >= 48, button + " touch target");
            verify(position.x >= 0 && position.y >= 0
                && position.x + button.width <= embedded.width + 0.5
                && position.y + button.height <= embedded.height + 0.5, button + " outside compact body");
        });
        var fields = descendants(embedded, NumberField).filter(function(item) { return item.visible; });
        compare(fields.length, data.index === 3 ? 4 : 3);
        fields.forEach(function(field) {
            verify(field.height >= 48);
            verify(field.contentWidth <= field.width - field.leftPadding - field.rightPadding + 0.5);
            var scroll = field.parent;
            while (scroll && !(scroll instanceof Flickable)) scroll = scroll.parent;
            verify(scroll !== null);
            var position = field.mapToItem(scroll.contentItem, 0, 0);
            scroll.contentY = Math.min(position.y, scroll.contentHeight - scroll.height);
            var shown = field.mapToItem(scroll, 0, 0);
            verify(shown.y >= -0.5 && shown.y + field.height <= scroll.height + 0.5);
        });
        var field = fields[0];
        field.editor.begin(field);
        waitForRendering(embedded);
        var keypad = descendants(embedded, NumericKeypad).filter(function(item) { return item.visible; })[0];
        descendants(keypad, Label).forEach(function(label) {
            verify(label.paintedWidth <= label.width + 0.5 && label.paintedHeight <= label.height + 0.5);
        });
        field.editor.cancel();
    }

    function test_compact_settings_is_full_page_with_keypad() {
        var container = createTemporaryObject(embeddedContainer, page.parent,
            {client: client, rootTop: 120, rootHeight: 304, compactLayout: true});
        var embedded = container.component;
        embedded.openSettings();
        compare(container.subpageOpen, true);
        compare(embedded.height, 480);
        compare(embedded.pageTitle, "Probe settings");
        compare(embedded.topPage.showHeader, true);
        var field = descendants(embedded.topPage, NumberField)[0];
        field.editor.begin(field);
        var keypad = descendants(embedded.topPage, NumericKeypad)[0];
        compare(keypad.visible, true);
        descendants(keypad, Button).forEach(function(button) {
            verify(button.width >= 48 && button.height >= 48);
        });
        embedded.requestBack();
        compare(field.editor.target, null);
        compare(container.subpageOpen, false);
        compare(embedded.height, 304);
    }

    function test_review_code_remains_a_scrollable_command() {
        var flow = showRoutine();
        var command = "G1 X-999.999 Y-999.999 Z-999.999 A-999.999 F1000 ; "
            + "complete command with a long review comment ".repeat(8);
        flow.program = [command];
        waitForRendering(flow);
        var code = descendants(flow, TextArea).filter(function(item) { return item.visible; })[0];
        compare(code.text, command);
        compare(code.wrapMode, TextEdit.NoWrap);
        compare(code.font.pixelSize, 18);
        var scroll = descendants(flow, ScrollView)[0];
        verify(scroll.contentWidth > scroll.availableWidth);
        scroll.contentItem.contentX = scroll.contentWidth - scroll.availableWidth;
        verify(scroll.contentItem.contentX > 0);
    }

    function descendants(item, type) {
        var found = item instanceof type ? [item] : [];
        var children = item.children || [];
        for (var i = 0; i < children.length; ++i)
            found = found.concat(descendants(children[i], type));
        return found;
    }
    function routine() {
        return descendants(page, ProbeFlow)[0];
    }
    function showRoutine(family, selection) {
        page.openRoutineReview(family || "inside", selection || {
            x: 1,
            y: 1,
            z: false
        });
        compare(routine().opened, true);
        compare(routine().reviewID, "review-1");
        return routine();
    }
    function save(name) {
        waitForRendering(host.contentItem);
        var path = Qt.resolvedUrl("../../build/ui-captures/" + name + ".png").toString().replace("file://", "");
        grabImage(host.contentItem).save(path);
    }
    function checkBounds(item) {
        descendants(item, Button).concat(descendants(item, TextField)).forEach(function (control) {
            if (!control.visible)
                return;
            var p = control.mapToItem(page, 0, 0);
            verify(p.x >= -1 && p.y >= -1 && p.x + control.width <= page.width + 1 && p.y + control.height <= page.height + 1, (control.text || control.toString()) + " outside page");
        });
    }
    function cleanup() {
        page.visible = true;
        descendants(page, PageView).forEach(function(view) {
            if (view.opened) view.close();
        });
        left.clear();
        alarm.clear();
    }

    function test_storage_warning_keeps_result_actions_data() {
        return [
            {tag: "inside", family: "inside", selection: {x: 1, y: 1, z: false}, spans: [null, null, null]},
            {tag: "outside", family: "outside", selection: {x: 1, y: 1, z: false}, spans: [null, null, null]},
            {tag: "pocket", family: "center", selection: "pocket", spans: [10, 12, null]}
        ];
    }

    function test_storage_warning_keeps_result_actions(data) {
        var flow = showRoutine(data.family, data.selection);
        flow.proceed();
        client.events({
            type: "error", code: "log", message: "Measurement completed, but its history could not be saved: No space left on device",
            result: {
                point: [1, 2, null], machinePoint: [-99, -98, null],
                spans: data.spans, zeroed: false, returned: false, positioned: false
            }
        });
        client.finish("");
        compare(flow.phase, "result");
        compare(flow.machinePoint[0], -99);
        compare(flow.spans[1], data.spans[1]);
        compare(flow.completedID, "review-1");
        verify(flow.failure.indexOf("history") !== -1);
        var zero = descendants(flow, Button).filter(function(button) { return button.text === "Set Work Zero"; })[0];
        verify(zero.visible && zero.enabled);
        save("embedded-storage-warning-" + data.tag);
        checkBounds(flow);
        if (data.family === "inside") flow.goToMeasured();
        else flow.returnToStart();
        compare(flow.phase, "running");
        client.events({type: "error", code: "controller_alarm", message: "Motion blocked"});
        client.finish("");
    }

    function test_back_follows_visible_page() {
        page.openWcs();
        compare(page.pageTitle, "Work coordinates");
        page.openHelp();
        compare(page.pageTitle, "Outside help");
        page.requestBack();
        compare(page.pageTitle, "Work coordinates");
        page.openHelp();
        page.openWcs();
        compare(page.pageTitle, "Work coordinates");
        page.requestBack();
        compare(page.pageTitle, "Outside help");
        page.requestBack();
        compare(page.subpageOpen, false);
    }

    function test_rotary_uses_host_navigation_and_keeps_completed_measurements() {
        var flow = descendants(page, RotaryFlow)[0];
        flow.showCalibration({operation: "horizontal", yDistance: 10, zDistance: 10});
        compare(page.pageTitle, "Level horizontal surface");
        compare(flow.showHeader, false);
        flow.proceed();
        client.events({type: "error", code: "controller_alarm", message: "Motion blocked"});
        client.finish("");
        compare(alarm.count, 1);
        flow.close();
        flow.showCalibration({operation: "horizontal", yDistance: 10, zDistance: 10});
        flow.proceed();
        client.events({type: "error", code: "log", message: "History could not be saved", result: {
            level: {touches: [[-100, -100, -45, 8], [-100, -90, -45, 8]], correction: 8, residual: 0},
            wcs: 54, zeroed: false
        }});
        client.finish("");
        compare(flow.phase, "result");
        compare(flow.result.level.touches.length, 2);
        verify(descendants(flow, Button).some(function(button) {
            return button.text === "Set A/Z zero" && button.visible && button.enabled;
        }));
        checkBounds(flow);
        page.requestBack();
        compare(page.subpageOpen, false);
    }

    function test_destroyed_style_restores_defaults() {
        failOnWarning(/.*/);
        var styled = styledPage.createObject(host.contentItem, {client: client});
        compare(Theme.headerHeight, 72);
        compare(Theme.text.toString(), "#123456");
        styled.destroy();
        wait(0);
        tryCompare(Theme, "headerHeight", Theme.defaults.headerHeight);
        compare(Theme.text, Theme.defaults.text);
        var button = createTemporaryObject(sharedButton, host.contentItem);
        compare(button.implicitHeight, Theme.defaults.buttonHeight);
        compare(button.textColor, Theme.defaults.text);
    }

    function test_navigation_keeps_running_operation_and_result() {
        var flow = showRoutine();
        save("embedded-review");
        flow.proceed();
        compare(flow.phase, "running");
        page.visible = false;
        client.events({
            type: "progress",
            progress: {
                kind: "script",
                message: "G38.3 X-10 F300"
            }
        });
        compare(flow.logText, "G38.3 X-10 F300\n");
        client.events({
            type: "result",
            result: {
                point: [1, 2, null],
                machinePoint: [-99, -98, null],
                spans: [null, null, null],
                zeroed: false,
                returned: false,
                positioned: false
            }
        });
        client.finish("");
        page.visible = true;
        compare(flow.phase, "result");
        compare(flow.machinePoint[0], -99);
        compare(client.aborts, 0);
        wait(30);
        checkBounds(flow);
        save("embedded-result");
        var offset = descendants(flow, NumberField)[0];
        mouseClick(offset);
        verify(offset.editor.target === offset);
        checkBounds(flow);
        save("embedded-result-keypad");
        page.requestBack();
        compare(flow.opened, false);
        compare(left.count, 0);
    }

    function test_alarm_requests_host_navigation() {
        var flow = showRoutine();
        flow.proceed();
        client.events({
            type: "error",
            code: "controller_alarm",
            message: "Probe alarm"
        });
        client.finish("");
        compare(alarm.count, 1);
        compare(flow.phase, "failed");
    }

    function test_retract_before_leaving_uses_host_signal() {
        page.requestExit();
        var buttons = descendants(host.Overlay.overlay, Button);
        var retract = buttons.filter(function (button) {
            return button.text === "Retract and exit";
        })[0];
        verify(retract !== undefined);
        mouseClick(retract);
        compare(left.count, 1);
        compare(client.state.status.probeActuator, 0);
    }
}
