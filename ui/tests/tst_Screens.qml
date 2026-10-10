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
import "../ProbePages.js" as Pages

TestCase {
    id: test
    name: "Screens"
    when: windowShown
    ScreenClient { id: client }
    FontLoader { id: geist; source: "../fonts/Geist-Regular.otf" }
    ApplicationWindow {
        id: host
        visible: true
        width: 800
        height: 480
        font.family: geist.name || "DejaVu Sans"
        color: Theme.page
        palette.windowText: Theme.text
        palette.text: Theme.text
        palette.buttonText: Theme.text
        ProbePage {
            id: page
            anchors.fill: parent
            client: client
            updateAvailable: updates.updateAvailable
            uiFontFamily: host.font.family
            monoFontFamily: "DejaVu Sans Mono"
            settingsContribution: Component {
                LabButton { text: I18n.tr("Check for updates"); primary: true; notification: updates.updateAvailable; onClicked: updates.show() }
            }
        }
        UpdateFlow { id: updates; parent: page; client: client; settings: page.settings; uiFont: host.font.family }
    }
    readonly property var point: [-120.128, -95.204, -90.375]
    readonly property var readingRows: [
        [-120.128, -95.204, -90.375], [-120.130, -95.206, -90.374],
        [-120.127, -95.203, -90.376], [-120.129, -95.205, -90.375],
        [-120.128, -95.204, -90.375]
    ]

    function children(item, type) {
        var found = item instanceof type ? [item] : [];
        var list = item.children || [];
        for (var i = 0; i < list.length; ++i) found = found.concat(children(list[i], type));
        return found;
    }
    function component(type) {
        return children(page, type)[0];
    }
    function button(item, label) {
        var found = children(item, Button).filter(function(b) { return b.visible && b.text === I18n.tr(label); })[0];
        verify(found !== undefined, label + " button");
        return found;
    }
    function stats() {
        return [0, 1, 2].map(function(axis) {
            var values = test.readingRows.map(function(row) { return row[axis]; }).sort(function(a, b) { return a-b; });
            var mean = values.reduce(function(a, b) { return a+b; }, 0) / values.length;
            return {mean: mean, median: values[2], stddev: 0.00114,
                range: values[values.length-1] - values[0]};
        });
    }
    function init() {
        failOnWarning(/(?!(.*Populating font|.*Sans Serif)).*/);
        client.error = "";
        page.contextHelp.active = false;
        page.actionError = "";
        client.state = client.readyState();
        children(page, PageView).forEach(function(view) { if (view.opened) view.close(); });
        updates.close();
        updates.available = null;
        findChild(page, "exitDialog").close();
        children(host.Overlay.overlay, WorkCoordinatePicker).forEach(function(view) { view.close(); });
        children(page, NumberField).forEach(function(field) { field.editor.cancel(); field.deselect(); });
        children(page, TabBar)[0].currentIndex = 0;
        findChild(page, "droReference").probeSelected = true;
        children(page, Flickable).forEach(function(view) { view.contentY = view.originY; view.contentX = view.originX; });
        verify(waitForPolish(host));
    }
    function cleanupTestCase() {
        I18n.language = "en";
    }
    function showResult(family, selection, axes, spans) {
        page.openRoutineReview(family, selection);
        var flow = component(ProbeFlow);
        var rawSpans = (spans || [null, null, null]).map(function(v) {
            var internal = selection === "hole" || selection === "pocket" || String(selection).indexOf("valley") >= 0;
            return v === null ? null : v + (internal ? -2 : 2);
        });
        flow.acceptResult({point: [0, 0, 0].map(function(v, i) { return axes[i] ? v : null; }),
            machinePoint: point.map(function(v, i) { return axes[i] ? v : null; }),
            rawPoint: [-124, -114, -40].map(function(v, i) { return axes[i] ? v : null; }),
            rawSpans: rawSpans,
            spans: spans || [null, null, null], zeroed: false}, "screen-result");
        flow.logText = client.program.join("\n");
        return flow;
    }
    function showRotary(operation) {
        var flow = component(RotaryFlow);
        flow.showCalibration(Pages.rotaryConfig(client.values, operation));
        return flow;
    }
    function showRepeatability(phase) {
        var flow = component(RepeatabilityFlow);
        flow.showCheck();
        flow.phase = phase;
        if (flow.showingResults) {
            flow.measurements = readingRows;
            flow.statistics = stats();
            flow.logText = client.program.join("\n");
        }
        return flow;
    }
    function test_layout_data() {
        var screens = [
            {tag:"01-outside", kind:"tab", tab:0},
            {tag:"02-inside", kind:"tab", tab:1},
            {tag:"03-center", kind:"tab", tab:2},
            {tag:"04-rotary", kind:"tab", tab:3},
            {tag:"05-settings", kind:"tab", tab:4},
            {tag:"06-distance-keypad", kind:"edit", tab:0},
            {tag:"07-settings-keypad", kind:"edit", tab:4},
            {tag:"08-settings-scroll", kind:"settings-scroll"},
            {tag:"09-retracted", kind:"retracted"},
            {tag:"10-disconnected", kind:"disconnected"},
            {tag:"11-large-coordinates", kind:"large"},
            {tag:"12-work-coordinates", kind:"wcs"},
            {tag:"13-outside-help", kind:"help", tab:0},
            {tag:"14-inside-help", kind:"help", tab:1},
            {tag:"15-center-help", kind:"help", tab:2},
            {tag:"16-rotary-help", kind:"help", tab:3},
            {tag:"17-settings-help", kind:"help", tab:4},
            {tag:"18-exit", kind:"exit"},
            {tag:"19-exit-error", kind:"exit-error"},
            {tag:"20-utilities", kind:"utilities"},
            {tag:"21-clear-history", kind:"clear"},
            {tag:"22-history", kind:"history"},
            {tag:"23-history-empty", kind:"history-empty"},
            {tag:"24-review", kind:"review"},
            {tag:"25-running", kind:"running"},
            {tag:"26-probing-error", kind:"failed"},
            {tag:"27-outside-result", kind:"outside"},
            {tag:"28-inside-result", kind:"inside"},
            {tag:"29-z-result", kind:"z"},
            {tag:"30-hole-result", kind:"hole"},
            {tag:"31-pocket-result", kind:"pocket"},
            {tag:"32-ridge-result", kind:"ridge"},
            {tag:"33-offset-keypad", kind:"offset"},
            {tag:"34-result-work-coordinates", kind:"result-wcs"},
            {tag:"35-zero-saved", kind:"zeroed"},
            {tag:"36-result-log", kind:"log"},
            {tag:"37-rotary-review", kind:"rotary-review"},
            {tag:"38-rotary-result", kind:"rotary"},
            {tag:"39-level-result", kind:"level"},
            {tag:"40-face-result", kind:"face"},
            {tag:"41-rotary-confirmation", kind:"rotary-confirm"},
            {tag:"42-repeatability-preparation", kind:"repeat-prepare"},
            {tag:"43-repeatability-options", kind:"repeat-options"},
            {tag:"44-repetitions-keypad", kind:"repeat-edit"},
            {tag:"45-repeatability-running", kind:"repeat-running"},
            {tag:"46-repeatability-result", kind:"repeat-result"},
            {tag:"47-repeatability-error", kind:"repeat-failed"},
            {tag:"48-repeatability-log", kind:"repeat-log"},
            {tag:"49-update", kind:"update"},
            {tag:"50-update-development", kind:"update-dev"},
            {tag:"51-update-confirmation", kind:"update-confirm"},
            {tag:"52-update-current", kind:"update-current"},
            {tag:"53-update-error", kind:"update-error"},
            {tag:"54-update-installing", kind:"update-installing"},
            {tag:"55-tool-reference", kind:"tool-reference"},
            {tag:"56-inside-review", kind:"operation-review", family:"inside", selection:{x:-1,y:-1,z:false}},
            {tag:"57-hole-review", kind:"operation-review", family:"center", selection:"hole"},
            {tag:"58-block-review", kind:"operation-review", family:"center", selection:"block"},
            {tag:"59-z-review", kind:"operation-review", family:"outside", selection:{x:0,y:0,z:true}},
            {tag:"60-review-keypad", kind:"operation-review", family:"outside", selection:{x:-1,y:-1,z:false}, edit:true},
            {tag:"61-review-feeds", kind:"operation-review", family:"center", selection:"hole", scroll:true},
            {tag:"62-level-review", kind:"rotary-review", operation:"horizontal"},
            {tag:"63-face-review", kind:"rotary-review", operation:"vertical"},
            {tag:"64-boss-review", kind:"operation-review", family:"center", selection:"boss"},
            {tag:"65-valley-review", kind:"operation-review", family:"center", selection:"x-valley"},
            {tag:"66-pocket-z-review", kind:"operation-review", family:"inside", selection:{x:0,y:0,z:true}},
            {tag:"67-update-marker", kind:"update-marker", tab:0},
            {tag:"68-settings-update-marker", kind:"update-marker", tab:4},
            {tag:"69-outside-help-details", kind:"help-details", tab:0},
            {tag:"70-rotary-help-details", kind:"help-details", tab:3},
            {tag:"71-settings-help-details", kind:"help-details", tab:4},
            {tag:"72-help-path", kind:"help-image", tab:0},
            {tag:"73-help-coordinates", kind:"help-tip", title:"Coordinates"},
            {tag:"74-help-wcs", kind:"help-tip", title:"Work coordinates"},
            {tag:"76-help-actuator", kind:"help-tip", title:"Extend / Retract"},
            {tag:"77-help-tabs", kind:"help-tip", title:"Probing tabs"},
            {tag:"78-help-distance", kind:"help-tip", title:"X search distance", tab:1},
            {tag:"79-help-settings", kind:"help-tip", title:"Probe ball diameter", tab:4},
            {tag:"80-guide-index", kind:"guide"},
            {tag:"81-help-retracted", kind:"help", tab:0, actuator:0},
            {tag:"82-help-result-contacts", kind:"inside", helpTitle:"Contacts · G53 · mm"},
            {tag:"83-help-result-span", kind:"pocket", helpTitle:"Raw span · mm"},
            {tag:"84-help-rotary-centers", kind:"rotary", helpTitle:"Axis centers · G53 · mm"},
            {tag:"85-help-rotary-tilt", kind:"level", helpTitle:"Remaining tilt"},
            {tag:"86-help-repeatability-spread", kind:"repeat-result", helpTitle:"Std dev"},
            {tag:"87-help-confirmation", kind:"review", help:true}
        ];
        return ["en", "zh_CN", "sv"].reduce(function(rows, language) {
            return rows.concat(screens.map(function(screen) {
                return Object.assign({}, screen, {tag: language + "-" + screen.tag, language: language});
            }));
        }, []);
    }
    function test_layout(data) {
        client.values = Object.assign({}, client.values, {language:data.language});
        page.settings.values = client.values;
        var tabs = children(page, TabBar)[0];
        var kind = data.kind;
        if (data.tab !== undefined) tabs.currentIndex = data.tab;
        if (data.actuator !== undefined)
            client.state = Object.assign({}, client.state, {status:Object.assign({}, client.state.status,
                {probeActuator:data.actuator})});
        verify(waitForPolish(host));
        host.update();
        verify(waitForRendering(host.contentItem));
        if (kind === "update-marker") {
            updates.available = {name: "Update", notes: "Changes"};
        } else if (kind === "tab") {
            // The tab index above selects the routine controls.
        } else if (kind === "edit") {
            var field = children(page, NumberField).filter(function(f) { return f.visible; })[0];
            field.editor.begin(field);
        } else if (kind === "settings-scroll") {
            tabs.currentIndex = 4;
            var scroll = children(children(page, ProbeSetupPanel)[0], Flickable)[0];
            scroll.contentY = scroll.contentHeight - scroll.height;
        } else if (kind === "retracted") {
            client.state = Object.assign({}, client.state, {status: Object.assign({}, client.state.status, {probeActuator:0})});
        } else if (kind === "disconnected") {
            client.error = "Controller disconnected. Waiting for the connection.";
            client.state = {connected:false};
        } else if (kind === "large") {
            client.state = Object.assign({}, client.state, {coordinates: {
                probe: {workPosition:[-999.999,-999.999,-999.999],machinePosition:[-999.999,-999.999,-999.999]},
                spindle: client.state.coordinates.spindle
            }});
        } else if (kind === "tool-reference") {
            var reference = findChild(page, "droReference");
            mouseClick(reference);
            compare(reference.probeSelected, false);
            compare(page.droCoordinates.workPosition, client.state.coordinates.spindle.workPosition);
        } else if (kind === "wcs") page.openWcs();
        else if (kind === "help" || kind === "help-details" || kind === "help-image") {
            page.openHelp();
            if (kind === "help-details" || kind === "help-image") {
                page.openGuide();
                component(OperatorGuide).pageIndex = data.tab + 1;
                var document = component(HelpDocument);
                verify(waitForPolish(host));
                var sections = children(document, MenuButton).filter(function(b) { return b.visible; });
                verify(sections.length > 0);
                var opened = kind === "help-image" ? sections : [sections[0]];
                opened.forEach(function(section) {
                    section.clicked();
                    compare(section.expanded, true);
                });
                verify(waitForPolish(host));
                if (kind === "help-image") {
                    var illustration = children(document, Image).filter(function(item) { return item.visible; })[0];
                    tryCompare(illustration, "status", Image.Ready);
                    document.contentItem.contentY = illustration.mapToItem(document.contentItem, 0, 0).y;
                }
                compare(document.contentWidth, document.availableWidth);
            }
        }
        else if (kind === "help-tip") {
            page.openHelp();
            verify(waitForPolish(host));
            tryVerify(function() { return page.contextHelp.tips.length > 0; });
            var tip = page.contextHelp.tips.filter(function(item) { return item.title === I18n.tr(data.title); })[0];
            verify(tip !== undefined);
            page.contextHelp.select(tip);
        } else if (kind === "guide") { page.openHelp(); page.openGuide(); }
        else if (kind.indexOf("exit") === 0) {
            page.requestExit();
            if (kind === "exit-error") page.actionError = "Probe retraction failed. Check the actuator.";
        } else if (kind === "utilities" || kind === "clear") {
            page.openUtilities();
            if (kind === "clear") mouseClick(button(component(UtilitiesPanel), "Clear logs"));
        } else if (kind.indexOf("history") === 0) {
            client.history = kind === "history-empty" ? [] : [
                {label:"Inside X/Y corner",timestampMs:1791522360000,status:"success",category:"routine",
                    config:Pages.routine(client.values,"inside",{x:-1,y:-1,z:false},54),
                    result:{machinePoint:[point[0],point[1],null],spans:[null,null,null]}},
                {label:"Pocket center",timestampMs:1791522240000,status:"success",category:"routine",
                    config:Pages.routine(client.values,"center","pocket",54),
                    result:{machinePoint:[point[0],point[1],null],spans:[20.015,30.021,null]}},
                {label:"Outside Z surface",timestampMs:1791522060000,status:"failed",category:"routine",
                    config:Pages.routine(client.values,"outside",{x:0,y:0,z:true},54),error:"No contact"}
            ];
            component(HistoryFlow).showHistory();
        } else if (kind === "operation-review") {
            page.openRoutineReview(data.family,data.selection);
            var review = component(OperationReview);
            if (data.edit) {
                var input = findChild(review,"review-xSearchDistance");
                mouseClick(input);
                input.editor.typeKey("2");
                input.editor.typeKey("5");
            }
            if (data.scroll) {
                var optionScroll = findChild(review,"reviewOptions");
                optionScroll.contentY = optionScroll.contentHeight - optionScroll.height;
            }
        } else if (["review","running","failed"].indexOf(kind) >= 0) {
            page.openRoutineReview("outside",{x:-1,y:-1,z:false});
            var flow = component(ProbeFlow);
            if (kind !== "review") {
                flow.phase = kind;
                flow.logText = client.program.join("\n");
                if (kind === "failed") flow.failure = "No contact on Y+. Move the probe closer to the surface and check the search distance.";
            }
        } else if (kind.indexOf("rotary") === 0 || ["level","face"].indexOf(kind) >= 0) {
            var operation = data.operation || (kind === "level" ? "horizontal" : kind === "face" ? "vertical" : "axis");
            var rotary = showRotary(operation);
            if (kind !== "rotary-review") {
                rotary.phase = "result";
                rotary.result = {wcs:54,zeroed:false,rotationApplied:false,rotationSupported:true,
                    xyAngle:0.0412,xzAngle:-0.0194,
                    stations:[{center:[-60.125,-95.204,-60.375]},{center:[-30.125,-95.182,-60.385]}],
                    level:{touches:[[-60.125,-95.204,-60.375],
                        operation === "horizontal" ? [-60.125,-85.204,-60.377] : [-60.125,-95.202,-50.375]],
                        correction:0.0138,residual:0.0014}};
                if (kind === "rotary-confirm") rotary.confirmAction("zero");
            }
        } else if (kind.indexOf("repeat-") === 0) {
            var phase = kind.slice(7);
            var repeat = showRepeatability(["edit","log"].indexOf(phase) >= 0 ? (phase === "edit" ? "options" : "result") : phase);
            if (phase === "edit") findChild(repeat,"repetitions").editor.begin(findChild(repeat,"repetitions"));
            if (phase === "log") repeat.sourceVisible = true;
            if (phase === "failed") repeat.failure = "Probe did not touch X-. Partial measurements are shown below.";
        } else if (kind.indexOf("update") === 0) {
            updates.open();
            updates.currentVersion = "2026.10.0";
            updates.available = {name:"2026.10.1",notes:"## Changes\n\n- Clearer measurements and work-zero offsets.\n- Larger controls for touchscreens.\n- Improved rotary probing."};
            updates.phase = kind === "update-current" ? "current" : kind === "update-error" ? "error"
                : kind === "update-installing" ? "installing" : "available";
            updates.errorText = "Could not reach GitHub. Check the network connection and try again.";
            page.settings.setValue("developmentUpdates", kind === "update-dev");
            if (kind === "update-confirm") mouseClick(button(updates.contentItem,"Install update"));
        } else {
            var feature = kind === "ridge" ? "x-ridge" : kind;
            var family = ["hole","pocket","ridge"].indexOf(kind) >= 0 ? "center" : kind === "inside" ? "inside" : "outside";
            var selection = family === "center" ? feature : {x:kind==="z"?0:-1,y:kind==="z"?0:-1,z:kind==="z"};
            var axes = kind === "z" ? [false,false,true] : kind === "ridge" ? [true,false,false] : [true,true,false];
            var spans = kind === "hole" ? [19.984,19.991,null] : kind === "pocket" ? [20.015,30.021,null]
                : kind === "ridge" ? [12.004,null,null] : null;
            var result = showResult(family, selection, axes, spans);
            if (kind === "offset") {
                var offset = findChild(result,"resultOffset0");
                offset.editor.begin(offset);
                offset.editor.typeKey("2");
                offset.editor.typeKey("sign");
            }
            if (kind === "result-wcs") mouseClick(findChild(result,"resultWcs"));
            if (kind === "zeroed") {
                result.zeroed = true;
                result.workZeroSaved();
            }
            if (kind === "log") result.sourceVisible = true;
        }
        if (data.helpTitle || data.help) {
            page.openHelp();
            verify(waitForPolish(host));
            tryVerify(function() { return page.contextHelp.headerGuide !== null; });
            if (data.helpTitle) {
                var resultTip = page.contextHelp.tips.filter(function(item) { return item.title === I18n.tr(data.helpTitle); })[0];
                verify(resultTip !== undefined);
                page.contextHelp.select(resultTip);
            }
        }
        verify(waitForPolish(host));
        page.contextHelp.refresh();
        host.update();
        verify(waitForRendering(host.contentItem));
        // Wrapping translated labels can schedule a second text-layout pass.
        verify(waitForPolish(host));
        host.update();
        verify(waitForRendering(host.contentItem));
        var path = Qt.resolvedUrl("../../build/ui-captures/" + data.tag + ".png").toString().replace("file://","");
        var capture = grabImage(host.contentItem.parent);
        compare(capture.width, 800);
        compare(capture.height, 480);
        capture.save(path);
        checkControls(host.contentItem);
        checkControls(host.Overlay.overlay);
        if (kind === "update-confirm") mouseClick(button(host.Overlay.overlay, "Cancel"));
    }
    function checkControls(item) {
        children(item, Button).concat(children(item, TabButton), children(item, TextField)).filter(function(control) {
            if (!control.visible) return false;
            var parent = control.parent;
            while (parent && parent !== item) {
                if (parent instanceof Flickable && !(control instanceof TabButton)) return false;
                parent = parent.parent;
            }
            return true;
        }).forEach(function(control) {
            var p = control.mapToItem(host.contentItem,0,0);
            verify(control.width >= 48 && control.height >= 48, control.text + " touch target");
            verify(p.x >= -0.5 && p.y >= -0.5 && p.x + control.width <= 800.5 && p.y + control.height <= 480.5,
                control.text + " outside screen at " + p.x + "," + p.y + " size " + control.width + "," + control.height);
            if (control instanceof Button && control.contentItem instanceof Label) {
                verify(control.contentItem.implicitWidth <= control.contentItem.width + 0.5,
                    control.text + " label is clipped");
                verify(control.contentItem.paintedHeight <= control.contentItem.height + 0.5,
                    control.text + " label height");
            }
        });
        children(item, Label).filter(function(label) {
            return label.visible && label.text.length > 0 && label.elide === Text.ElideNone;
        }).forEach(function(label) {
            verify(label.paintedWidth <= label.width + 0.5, label.text + " text width");
            verify(label.paintedHeight <= label.height + 0.5, label.text + " text height");
        });
    }
}
