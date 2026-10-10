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

pragma ComponentBehavior: Bound
import QtQuick
import "controls"
import QtQuick.Controls
import QtQuick.Layouts

PageView {
    id: flow
    property bool showHeader: true
    required property var client
    property string uiFont: "sans-serif"
    property var entries: []
    property int selectedIndex: -1
    signal resultRequested(var entry)
    property string errorText: ""
    readonly property var selected: selectedIndex >= 0 && selectedIndex < entries.length ? entries[selectedIndex] : null

    x: 0
    y: 0
    width: parent ? parent.width : 800
    height: parent ? parent.height : 480
    padding: 0
    font.family: uiFont
    background: Rectangle {
        color: Theme.page
    }

    ServiceRequest {
        id: loadRequest
        client: flow.client
    }
    ServiceRequest {
        id: openRequest
        client: flow.client
    }

    function openResult() {
        if (!selected || openRequest.pending)
            return;
        errorText = "";
        openRequest.send("history.open", {
            id: selected.id
        }, function (reply) {
            if (!reply.ok || !reply.data) {
                errorText = reply.error || "Could not open this result";
                return;
            }
            resultRequested(reply.data);
        });
    }

    function showHistory() {
        entries = [];
        selectedIndex = -1;
        errorText = "";
        open();
        loadRequest.send("history.get", null, function (reply) {
            var data = reply.data;
            if (!reply.ok || data === null || typeof data !== "object" || typeof data.length !== "number") {
                errorText = reply.error || "Could not read probing history";
                return;
            }
            // Native clients supply Qt list wrappers rather than JavaScript arrays.
            entries = Array.prototype.slice.call(data);
            selectedIndex = entries.length ? 0 : -1;
        });
    }

    function dateText(timestamp) {
        return Qt.formatDateTime(new Date(Number(timestamp)), "yyyy-MM-dd hh:mm:ss");
    }

    function number(value) {
        return Number(value).toFixed(3);
    }

    function detail(entry) {
        if (!entry)
            return "";
        var lines = [dateText(entry.timestampMs), I18n.tr('Status: %1', [I18n.tr(entry.status)])];
        if (entry.error)
            lines.push(I18n.tr('Error: %1', [I18n.tr(entry.error)]));
        if (entry.category === "repeatability") {
            var report = entry.result || {};
            var options = entry.config ? entry.config.options || {} : {};
            lines.push(I18n.tr('Repetitions: %1   Home: %2   Retract: %3', [options.repetitions || 0,
                options.home ? I18n.tr('yes') : I18n.tr('no'), options.retract ? I18n.tr('yes') : I18n.tr('no')]));
            var measurements = report.measurements || [];
            lines.push("", I18n.tr('Readings (mm)'));
            for (var i = 0; i < measurements.length; ++i) {
                var reading = measurements[i];
                lines.push((i + 1) + ": " + ["X", "Y", "Z"].map(function (axis, index) {
                    return reading[index] === null ? "" : axis + " " + number(reading[index]);
                }).filter(Boolean).join("   "));
            }
            var stats = report.statistics || [];
            for (var j = 0; j < stats.length; ++j) {
                if (stats[j])
                    lines.push(I18n.tr('%1 mean %2   median %3   SD %4', [["X", "Y", "Z"][j],
                        number(stats[j].mean), number(stats[j].median), stats[j].stddev === null ? "-" : number(stats[j].stddev)]));
            }
        } else if (entry.category === "rotary") {
            var calibration = entry.result || {};
            (entry.actions || []).forEach(function (action) {
                if (action.data && action.data.result)
                    calibration = action.data.result;
            });
            var setup = entry.config || {};
            if (calibration.level || setup.operation === "horizontal" || setup.operation === "vertical" || setup.operation === "verticalNegative") {
                var level = calibration.level || {};
                var axes = (level.operation || setup.operation) === "horizontal" ? "A/Z" : "A/Y";
                ["initialTouches", "touches"].forEach(function (key) {
                    lines.push("", key === "touches" ? I18n.tr('Verified touches · G53') : I18n.tr('Initial touches · G53'));
                    (level[key] || []).forEach(function (point, index) {
                        lines.push(I18n.tr('Touch %1: %2', [index + 1, ["X", "Y", "Z"].map(function (axis, i) {
                            return axis + " " + number(point[i]);
                        }).join("   ")]));
                    });
                });
                if (calibration.level) {
                    lines.push(I18n.tr('A correction %1°', [Number(level.correction).toFixed(4)]),
                        I18n.tr('Remaining tilt %1°', [Number(level.residual).toFixed(4)]),
                        I18n.tr('%1 zero: %2', [axes, calibration.zeroed ? I18n.tr('saved') : I18n.tr('unchanged')]));
                }
                lines.push("", I18n.tr('G%1   Y distance %2 mm', [setup.wcs, number(setup.yDistance)]),
                    I18n.tr('Z distance %1 mm', [number(setup.zDistance)]));
            } else {
                lines.push("", I18n.tr('Axis centers · G53 (mm)'));
                (calibration.stations || []).forEach(function (station, index) {
                    lines.push(I18n.tr('Axis center %1: %2', [index + 1, ["X", "Y", "Z"].map(function (axis, i) {
                        return axis + " " + number(station.center[i]);
                    }).join("   ")]));
                });
                if ((calibration.stations || []).length === 2) {
                    lines.push(I18n.tr('Axis angle in X/Y %1°', [Number(calibration.xyAngle).toFixed(4)]),
                        I18n.tr('Axis angle in X/Z %1°', [Number(calibration.xzAngle).toFixed(4)]),
                        I18n.tr('Y/Z zero: %1', [calibration.zeroed ? I18n.tr('saved') : I18n.tr('unchanged')]),
                        I18n.tr('X/Y rotation: %1', [calibration.rotationApplied ? I18n.tr('saved') : I18n.tr('unchanged')]));
                }
                lines.push("", I18n.tr('G%1   rod %2 mm', [setup.wcs, number(setup.rodDiameter)]),
                    I18n.tr('X distance %1 mm', [number(setup.xDistance)]));
            }
            lines.push(I18n.tr('Rotary feed %1°/min', [number(setup.rotaryFeed)]));
        } else {
            var result = entry.result || {};
            var config = entry.config || {};
            if (result.angle) {
                lines.push("", I18n.tr("Measured angle") + " " + result.angle.degrees.toFixed(4) + "°",
                    I18n.tr("Change %1 mm over %2 mm", [number(result.angle.difference), number(result.angle.spacing)]));
            }
            var point = result.machinePoint || [];
            if (point.some(function (value) {
                return value !== null && value !== undefined;
            })) {
                lines.push("", I18n.tr('Measured G53 (mm)'));
                for (var axis = 0; axis < 3; ++axis) {
                    if (point[axis] !== null && point[axis] !== undefined)
                        lines.push(["X", "Y", "Z"][axis] + "  " + number(point[axis]));
                }
            }
            var spans = result.spans || [];
            for (var span = 0; span < 2; ++span) {
                if (spans[span] !== null && spans[span] !== undefined)
                    lines.push(I18n.tr('Span %1  %2 mm', [["X", "Y"][span], number(spans[span])]));
            }
            if (entry.workZero) {
                lines.push("", I18n.tr('Work zero set in G%1', [(entry.workZero.result || result).wcs]));
                var offsets = entry.workZero.offsets || [];
                for (var offset = 0; offset < 3; ++offset) {
                    if (point[offset] !== null && point[offset] !== undefined)
                        lines.push(I18n.tr('%1 offset %2 mm', [["X", "Y", "Z"][offset], number(offsets[offset] || 0)]));
                }
            }
            lines.push("", I18n.tr('Settings'), I18n.tr('G%1   ball %2 mm   depth %3 mm', [config.wcs, number(config.diameter), number(config.depth)]));
            if (!config.z)
                lines.push(I18n.tr('X search %1 mm   Y search %2 mm', [number(config.xSearchDistance), number(config.ySearchDistance)]));
            lines.push(I18n.tr('Feeds: %1 / %2 / %3 mm/min', [number(config.positioningFeed), number(config.coarseFeed), number(config.fineFeed)]));
        }
        return lines.join("\n");
    }

    onClosed: {
        loadRequest.cancel();
        openRequest.cancel();
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        PageHeader {
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.headerHeight
            visible: flow.showHeader
            title: I18n.tr('Probe history')
            detail: flow.entries.length ? I18n.tr('%1 attempts', [flow.entries.length]) : ""
            uiFont: flow.uiFont
            backEnabled: !openRequest.pending
            onBack: flow.close()
        }
        Label {
            visible: flow.errorText.length > 0 || (!loadRequest.pending && flow.entries.length === 0)
            Layout.fillWidth: true
            Layout.fillHeight: flow.entries.length === 0
            Layout.margins: 16
            text: flow.errorText ? I18n.tr(flow.errorText) : I18n.tr('No probing attempts yet')
            color: flow.errorText ? Theme.warning : Theme.textMuted
            font.pixelSize: 20
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            wrapMode: Text.WordWrap
        }
        RowLayout {
            visible: flow.entries.length > 0
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 16
            spacing: 16
            ListView {
                id: historyList
                Layout.preferredWidth: 310
                Layout.fillHeight: true
                clip: true
                model: flow.entries
                spacing: 2
                ScrollBar.vertical: ScrollBar {}
                delegate: Rectangle {
                    id: entryRow
                    required property var modelData
                    required property int index
                    width: historyList.width
                    height: 80
                    color: flow.selectedIndex === entryRow.index ? Theme.accentWash : Theme.panel
                    radius: Theme.radius
                    border.color: flow.selectedIndex === entryRow.index ? Theme.accentBright : Theme.divider
                    HelpTip {
                        title: I18n.tr('Probe history')
                        text: I18n.tr("Select a saved measurement to see its details. Open result restores its measured coordinates and work-zero controls.")
                    }
                    Column {
                        anchors.fill: parent
                        anchors.margins: 12
                        spacing: 3
                        Label {
                            width: parent.width
                            text: I18n.tr(entryRow.modelData.label)
                            color: Theme.text
                            font.pixelSize: 20
                            elide: Text.ElideRight
                        }
                        Label {
                            width: parent.width
                            text: flow.dateText(entryRow.modelData.timestampMs) + "  " + I18n.tr(entryRow.modelData.status)
                            color: entryRow.modelData.status === "success" ? Theme.accentBright : entryRow.modelData.status === "failed" ? Theme.danger : Theme.warning
                            font.pixelSize: 18
                            elide: Text.ElideRight
                        }
                    }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: flow.selectedIndex = entryRow.index
                    }
                }
            }
            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 12
                ScrollView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    contentWidth: availableWidth
                    clip: true
                    TextArea {
                        text: flow.detail(flow.selected)
                        readOnly: true
                        selectByMouse: false
                        wrapMode: TextEdit.WordWrap
                        color: Theme.text
                        font.pixelSize: 18
                        padding: 12
                        background: Rectangle {
                            color: Theme.panel
                            radius: Theme.radius
                        }
                    }
                }
                LabButton {
                    objectName: "historyOpenResult"
                    text: I18n.tr('Open result')
                    helpText: I18n.tr("Open the saved measurement to set work zero again. The stock and machine reference must still match the measurement.")
                    Layout.fillWidth: true
                    enabled: !openRequest.pending && flow.selected !== null && flow.selected.result && (flow.selected.result.machinePoint !== undefined || flow.selected.category === "rotary")
                    primary: true
                    onClicked: flow.openResult()
                }
            }
        }
    }
}
