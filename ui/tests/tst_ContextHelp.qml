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
    name: "ContextHelp"
    when: windowShown
    ScreenClient { id: client }
    FontLoader { id: geist; source: "../fonts/Geist-Regular.otf" }
    ApplicationWindow {
        id: host
        width: 800
        height: 480
        visible: true
        font.family: geist.name || "DejaVu Sans"
        ProbePage { id: page; anchors.fill: parent; client: client; uiFontFamily: host.font.family }
    }
    function children(item, type) {
        var result = item instanceof type ? [item] : [];
        (item.children || []).forEach(function(child) { result = result.concat(children(child, type)); });
        return result;
    }
    function tip(title) {
        var matches = page.contextHelp.tips.filter(function(item) { return item.title === I18n.tr(title); });
        compare(matches.length, 1, title);
        return matches[0];
    }
    function toggle() {
        return children(page, HelpButton).filter(function(button) { return button.visible; })[0];
    }
    function highlight(element) {
        var matches = children(page.contextHelp, Item).filter(function(item) {
            return item.objectName === "helpHighlight" && item.modelData === element;
        });
        compare(matches.length, 1, element.title);
        return matches[0];
    }
    function tap(element) {
        var r = page.contextHelp.bounds(element);
        var popup = findChild(page, "helpPopup");
        var points = [[0.5,0.5],[0.1,0.1],[0.9,0.1],[0.1,0.9],[0.9,0.9]];
        var point = points.find(function(p) {
            return !popup.visible || !page.contextHelp.contains(Qt.rect(popup.x,popup.y,popup.width,popup.height),
                r.x+r.width*p[0],r.y+r.height*p[1]);
        });
        if (point === undefined) {
            mouseClick(findChild(page, "helpPopupClose"));
            point = [0.5,0.5];
        }
        mouseClick(page.contextHelp, r.x+r.width*point[0], r.y+r.height*point[1]);
    }
    function init() {
        failOnWarning(/^(?!.*(?:Populating font|Sans Serif)).*/);
        page.contextHelp.active = false;
        children(page, PageView).forEach(function(view) { if (view.opened) view.close(); });
        children(page, TabBar)[0].currentIndex = 0;
        client.state = client.readyState();
        page.settings.values = Object.assign({}, client.values, {language:"en"});
        verify(waitForPolish(host));
        page.contextHelp.refresh();
    }
    function cleanup() { I18n.language = "en"; }

    function test_inspect_without_activating() {
        var helpButton = toggle();
        var tabs = children(page, TabBar)[0];
        compare(tabs.count, 6);
        compare(tabs.currentItem.text, I18n.tr("Outside"));
        mouseClick(helpButton);
        tryCompare(page, "helpMode", true);
        compare(helpButton.text, "\u00d7");
        tryVerify(function() { return page.contextHelp.tips.length > 0; });
        ["Coordinates", "Work coordinates", "Extend / Retract", "Probing tabs"].forEach(function(title) {
            var element = tip(title);
            tap(element);
            compare(page.contextHelp.selected, element, title);
            compare(findChild(page, "helpPopup").visible, true);
        });
        compare(findChild(page, "droReference").probeSelected, true);
        compare(client.state.status.probeActuator, 1);
        compare(children(page, TabBar)[0].currentIndex, 0);
        compare(children(page, PageView).filter(function(view) { return view.opened; }).length, 0);

        var routine = children(page, OutsideProbeButton).filter(function(button) { return button.visible; })[0];
        mouseClick(routine);
        compare(page.contextHelp.selected.target, routine);
        compare(children(page, ProbeFlow)[0].opened, false);
        var field = findChild(page, "outsideXSearchDistance");
        mouseClick(field);
        compare(field.editor.target, null);
        mouseClick(findChild(page, "helpPopupClose"));
        compare(page.contextHelp.selected, null);
        compare(page.helpMode, true);
        mouseClick(helpButton);
        compare(page.helpMode, false);
        compare(helpButton.text, "?");
        mouseClick(field);
        compare(field.editor.target, field);
        field.editor.cancel();
    }

    function test_coordinate_reference_touch_target() {
        var dro = findChild(page, "droReference");
        mouseClick(dro, 40, dro.height / 2);
        compare(dro.probeSelected, false);
        compare(page.droCoordinates, client.state.coordinates.spindle);
        mouseClick(dro, dro.width - 40, dro.height / 2);
        compare(dro.probeSelected, true);
        compare(page.droCoordinates, client.state.coordinates.probe);
    }

    function test_probe_state_labels_follow_the_actuator() {
        var control = children(page, ProbeActuatorButton)[0];
        function activeLabel() {
            return children(control, Label).filter(function(label) { return label.opacity === 1; })[0];
        }
        compare(activeLabel().text, I18n.tr("Extended"));
        mouseClick(control);
        compare(client.state.status.probeActuator, 0);
        compare(activeLabel().text, I18n.tr("Retracted"));
        mouseClick(control);
        compare(client.state.status.probeActuator, 1);
        compare(activeLabel().text, I18n.tr("Extended"));
        [2, 3].forEach(function(actuator) {
            client.state = Object.assign({}, client.state, {status: Object.assign({}, client.state.status,
                {probeActuator:actuator})});
            compare(activeLabel(), undefined);
            verify(!control.enabled);
        });
    }

    function test_header_frames_data() {
        return ["en", "zh_CN", "sv"].reduce(function(rows, language) {
            return rows.concat([0, 1].map(function(actuator) {
                return {tag:language+"-"+actuator, language:language, actuator:actuator};
            }));
        }, []);
    }
    function test_header_frames(data) {
        page.settings.values = Object.assign({}, client.values, {language:data.language});
        client.state = Object.assign({}, client.state, {status:Object.assign({}, client.state.status,
            {probeActuator:data.actuator})});
        page.openHelp();
        verify(waitForPolish(host));
        tryVerify(function() { return page.contextHelp.tips.length > 5; });
        var dro = highlight(tip("Coordinates"));
        ["Back", "Work coordinates"].forEach(function(title) {
            var frame = highlight(tip(title));
            compare(frame.y, dro.y, title + " border top");
            compare(frame.height, dro.height, title + " border height");
        });
        var tabTip = tip("Probing tabs");
        var tabs = highlight(tabTip);
        var area = page.contextHelp.bounds(tabTip);
        verify(tabs.y + tabs.height - 2 >= area.y + area.height, "Selected underline inside border");
        var probeTip = tip("Extend / Retract");
        var probe = highlight(probeTip);
        compare(probe.y, tabs.y, "Probe switch border top");
        compare(probe.height, tabs.height, "Probe switch border height");
        children(probeTip.target, Label).forEach(function(label) {
            var labelStart = label.mapToItem(page.contextHelp, (label.width - label.paintedWidth) / 2, 0);
            verify(labelStart.x + label.paintedWidth <= probe.x + probe.width - 4,
                label.text + " ends inside border");
        });
    }

    function test_placements_data() {
        return ["en", "zh_CN", "sv"].reduce(function(rows, language) {
            return rows.concat([0,1,2,3,4,5].map(function(tab) { return {tag:language+"-"+tab,language:language,tab:tab}; }));
        }, []);
    }
    function test_placements(data) {
        page.settings.values = Object.assign({}, client.values, {language:data.language});
        children(page, TabBar)[0].currentIndex = data.tab;
        client.state = Object.assign({}, client.state, {status:Object.assign({}, client.state.status,{probeActuator:0})});
        mouseClick(toggle());
        verify(waitForPolish(host));
        tryVerify(function() { return page.contextHelp.tips.length > 5; });
        var entries = page.contextHelp.tips.filter(function(item) {
            var r = page.contextHelp.bounds(item);
            return !item.toggle && r.width > 0 && r.height > 0;
        });
        var highlights = children(page.contextHelp, Item).filter(function(item) {
            return item.objectName === "helpHighlight" && item.visible;
        });
        highlights.forEach(function(first, index) {
            verify(first.x >= 0 && first.y >= 0 && first.x + first.width <= 800 && first.y + first.height <= 480,
                first.modelData.title + " border outside screen");
            highlights.slice(index + 1).forEach(function(second) {
                verify(first.x + first.width < second.x || second.x + second.width < first.x
                    || first.y + first.height < second.y || second.y + second.height < first.y,
                    first.modelData.title + " border touches " + second.modelData.title);
            });
        });
        entries.forEach(function(item) {
            page.contextHelp.select(item);
            verify(waitForPolish(host));
            var popup = findChild(page, "helpPopup");
            verify(popup.x >= 0 && popup.y >= 0);
            verify(popup.x + popup.width <= 800 && popup.y + popup.height <= 480,
                item.title + " explanation outside screen");
            compare(page.contextHelp.overlap(Qt.rect(popup.x,popup.y,popup.width,popup.height),
                page.contextHelp.bounds(item)), 0, item.title + " explanation covers control");
            children(popup, Label).forEach(function(label) {
                verify(label.paintedWidth <= label.width + 1, item.title + " text width");
                verify(label.paintedHeight <= label.height + 1, item.title + " text height");
            });
        });
    }

    function test_guide_navigation() {
        page.openHelp();
        tryVerify(function() { return page.contextHelp.tips.length > 0; });
        mouseClick(findChild(page, "helpGuide"));
        var guide = children(page, OperatorGuide)[0];
        compare(guide.opened, true);
        compare(guide.pageIndex, -1);
        compare(page.contextHelp.visible, false);
        var entry = children(guide, LabButton).filter(function(button) { return button.visible && button.text === guide.pages[1].title; })[0];
        mouseClick(entry);
        compare(guide.pageIndex, 1);
        var document = children(guide, HelpDocument)[0];
        document.linkActivated("results.md");
        compare(guide.pages[guide.pageIndex].name, "results.md");
        guide.back();
        compare(guide.pageIndex, -1);
        guide.back();
        compare(guide.opened, false);
        compare(page.contextHelp.visible, true);
        compare(page.helpMode, true);
        mouseClick(findChild(page, "helpGuide"));
        compare(toggle().text, "\u00d7");
        mouseClick(toggle());
        compare(page.helpMode, false);
        compare(guide.opened, false);
    }

    function test_scrolling_settings() {
        page.openSettings();
        page.openHelp();
        verify(waitForPolish(host));
        tryVerify(function() { return page.contextHelp.tips.length > 0; });
        var row = tip("Probe ball diameter");
        var area = page.contextHelp.bounds(row);
        mouseDrag(page.contextHelp, area.x + area.width/2, area.y + area.height/2, 0, -180);
        var scroll = children(children(page, ProbeSetupPanel)[0], Flickable)[0];
        verify(scroll.contentY > 0);
        compare(page.contextHelp.selected, null);
        var rotary = tip("Rotary feed");
        verify(page.contextHelp.bounds(rotary).height > 0);
    }

    function test_result_controls() {
        page.openRoutineReview("inside", {x:-1,y:-1,z:false});
        var flow = children(page, ProbeFlow)[0];
        flow.acceptResult({point:[0,0,null],machinePoint:[-150,-100,null],spans:[null,null,null],zeroed:false}, "help-result");
        page.openHelp();
        verify(waitForPolish(host));
        tryVerify(function() { return page.contextHelp.tips.length > 0; });
        var wcs = tip("Choose result work coordinates");
        mouseClick(wcs.target);
        compare(page.contextHelp.selected, wcs);
        var field = findChild(page, "resultOffset0");
        tap(page.contextHelp.tips.filter(function(item) { return item.target === field; })[0]);
        compare(page.contextHelp.selected.target, field);
        compare(field.editor.target, null);
        var zero = tip("Set Work Zero");
        tap(zero);
        compare(flow.zeroed, false);
        compare(page.contextHelp.selected, zero);
        mouseClick(toggle());
        compare(page.helpMode, false);
    }

    function test_header_guide_data() {
        return [{tag:"confirm", result:false}, {tag:"result", result:true}];
    }
    function test_header_guide(data) {
        page.openRoutineReview("inside", {x:-1,y:-1,z:false});
        if (data.result)
            children(page, ProbeFlow)[0].acceptResult({point:[0,0,null],machinePoint:[-150,-100,null],
                rawPoint:[-92,-92,null],zeroed:false}, "header-guide");
        page.openHelp();
        verify(waitForPolish(host));
        tryVerify(function() { return page.contextHelp.headerGuide !== null; });
        var guide = page.contextHelp.headerGuide;
        var p = guide.mapToItem(page.contextHelp, 0, 0);
        verify(p.y >= 0 && p.y + guide.height <= Theme.headerHeight);
        mouseClick(page.contextHelp, p.x + guide.width / 2, p.y + guide.height / 2);
        compare(children(page, OperatorGuide)[0].opened, true);
    }

    function test_result_explanations_data() {
        return ["en", "zh_CN", "sv"].reduce(function(rows, language) {
            return rows.concat(["edge", "center", "axis", "horizontal", "vertical", "repeat"].map(function(kind) {
                return {tag:language+"-"+kind, language:language, kind:kind};
            }));
        }, []);
    }
    function test_result_explanations(data) {
        page.settings.values = Object.assign({}, client.values, {language:data.language});
        var titles;
        if (data.kind === "edge" || data.kind === "center") {
            page.openRoutineReview(data.kind === "edge" ? "inside" : "center",
                data.kind === "edge" ? {x:-1,y:-1,z:false} : "pocket");
            children(page, ProbeFlow)[0].acceptResult({point:[0,0,null],machinePoint:[-150,-100,null],
                spans:data.kind === "center" ? [20,30,null] : [null,null,null],rawPoint:[-92,-92,null],
                rawSpans:[18,28,null],zeroed:false}, "help-result");
            titles = data.kind === "edge" ? ["Contacts · G53 · mm", "Measured · G53 · mm"]
                : ["Size · mm", "Raw span · mm", "Measured · G53 · mm"];
        } else if (data.kind === "repeat") {
            var repeat = children(page, RepeatabilityFlow)[0];
            repeat.showCheck();
            repeat.phase = "result";
            repeat.measurements = [[-175,-180,-100]];
            repeat.statistics = [{mean:-175,median:-175,stddev:0,range:0}, null, null];
            titles = ["Readings", "Mean G53", "Median", "Std dev", "Range"];
        } else {
            var rotary = children(page, RotaryFlow)[0];
            rotary.showCalibration(Pages.rotaryConfig(client.values, data.kind));
            rotary.phase = "result";
            rotary.result = {wcs:54,stations:[{center:[-60,-95,-60]},{center:[-30,-95,-60]}],
                level:{touches:[[-60,-95,-60],[-60,-85,-60]],correction:0.01,residual:0.001}};
            titles = data.kind === "axis" ? ["Axis centers · G53 · mm", "Axis angle in X/Y", "Axis angle in X/Z"]
                : ["Touches · G53 · mm", "A correction", "Remaining tilt"];
        }
        page.openHelp();
        verify(waitForPolish(host));
        tryVerify(function() { return page.contextHelp.tips.length > 0; });
        var entries = page.contextHelp.tips.filter(function(item) { return titles.indexOf(item.title) >= 0
            || titles.some(function(title) { return I18n.tr(title) === item.title; }); });
        verify(entries.length >= titles.length);
        entries.forEach(function(item) {
            tap(item);
            compare(page.contextHelp.selected, item);
            verify(waitForPolish(host));
            var popup = findChild(page, "helpPopup");
            verify(popup.x >= 0 && popup.y >= 0 && popup.x + popup.width <= 800 && popup.y + popup.height <= 480);
            compare(page.contextHelp.overlap(Qt.rect(popup.x,popup.y,popup.width,popup.height),
                page.contextHelp.bounds(item)), 0, item.title + " popup covers result");
        });
    }
}
