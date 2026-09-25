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
import QtQuick.Controls
import QtQuick.Layouts

Popup {
    id: flow
    property string serviceUrl: "http://127.0.0.1:8137/api/v1"
    property string uiFont: "sans-serif"
    property var entries: []
    property int selectedIndex: -1
    property string errorText: ""
    readonly property var selected: selectedIndex >= 0 && selectedIndex < entries.length
        ? entries[selectedIndex] : null

    parent: Overlay.overlay
    x: 0
    y: 0
    width: parent ? parent.width : 800
    height: parent ? parent.height : 480
    padding: 0
    modal: true
    closePolicy: Popup.NoAutoClose
    font.family: uiFont
    background: Rectangle { color: Theme.page }

    ServiceRequest { id: loadRequest }

    function showHistory() {
        entries = [];
        selectedIndex = -1;
        errorText = "";
        open();
        loadRequest.send("GET", serviceUrl + "/logs/history", null, function(reply) {
            if (!reply.ok || !Array.isArray(reply.data)) {
                errorText = reply.error || "Could not read probing history";
                return;
            }
            entries = reply.data;
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
        if (!entry) return "";
        var lines = [dateText(entry.timestampMs), "Status: " + entry.status];
        if (entry.error) lines.push("Error: " + entry.error);
        if (entry.category === "repeatability") {
            var report = entry.result || {};
            var options = entry.config ? entry.config.options || {} : {};
            lines.push("Repetitions: " + (options.repetitions || 0)
                + "   Home: " + (options.home ? "yes" : "no")
                + "   Retract: " + (options.retract ? "yes" : "no"));
            var measurements = report.measurements || [];
            lines.push("", "Readings (mm)");
            for (var i = 0; i < measurements.length; ++i) {
                var reading = measurements[i];
                lines.push((i + 1) + ": " + ["X", "Y", "Z"].map(function(axis, index) {
                    return reading[index] === null ? "" : axis + " " + number(reading[index]);
                }).filter(Boolean).join("   "));
            }
            var stats = report.statistics || [];
            for (var j = 0; j < stats.length; ++j) {
                if (stats[j]) lines.push(["X", "Y", "Z"][j] + " mean " + number(stats[j].mean)
                    + "   median " + number(stats[j].median)
                    + "   SD " + (stats[j].stddev === null ? "-" : number(stats[j].stddev)));
            }
        } else {
            var result = entry.result || {};
            var config = entry.config || {};
            var point = result.machinePoint || [];
            if (point.some(function(value) { return value !== null && value !== undefined; })) {
                lines.push("", "Measured G53 (mm)");
                for (var axis = 0; axis < 3; ++axis) {
                    if (point[axis] !== null && point[axis] !== undefined)
                        lines.push(["X", "Y", "Z"][axis] + "  " + number(point[axis]));
                }
            }
            var spans = result.spans || [];
            for (var span = 0; span < 2; ++span) {
                if (spans[span] !== null && spans[span] !== undefined)
                    lines.push("Span " + ["X", "Y"][span] + "  " + number(spans[span]) + " mm");
            }
            if (entry.workZero) {
                lines.push("", "Work zero set in G" + (entry.config ? entry.config.wcs : result.wcs));
                var offsets = entry.workZero.offsets || [];
                for (var offset = 0; offset < 3; ++offset) {
                    if (point[offset] !== null && point[offset] !== undefined)
                        lines.push(["X", "Y", "Z"][offset] + " offset " + number(offsets[offset] || 0) + " mm");
                }
            }
            lines.push("", "Settings", "G" + config.wcs
                + "   ball " + number(config.diameter) + " mm"
                + "   depth " + number(config.depth) + " mm");
            if (!config.z) lines.push("X search " + number(config.xSearchDistance)
                + " mm   Y search " + number(config.ySearchDistance) + " mm");
            lines.push("Feeds: " + number(config.positioningFeed) + " / "
                + number(config.coarseFeed) + " / " + number(config.fineFeed) + " mm/min");
        }
        return lines.join("\n");
    }

    onClosed: loadRequest.cancel()

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 70
            color: Theme.header
            RowLayout {
                anchors.fill: parent
                BackButton {
                    Layout.preferredWidth: 70
                    Layout.fillHeight: true
                    onClicked: flow.close()
                }
                Label {
                    Layout.fillWidth: true
                    text: "Probe history"
                    color: Theme.text
                    font.pixelSize: 24
                }
            }
        }
        Label {
            visible: flow.errorText.length > 0 || (!loadRequest.pending && flow.entries.length === 0)
            Layout.fillWidth: true
            Layout.margins: 16
            text: flow.errorText || "No probing attempts yet"
            color: flow.errorText ? Theme.warning : Theme.textMuted
            font.pixelSize: 20
        }
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 14
            spacing: 12
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
                    height: 70
                    color: flow.selectedIndex === entryRow.index ? Theme.panel : Theme.control
                    Column {
                        anchors.fill: parent
                        anchors.margins: 8
                        spacing: 3
                        Label {
                            width: parent.width
                            text: entryRow.modelData.label
                            color: Theme.text
                            font.pixelSize: 18
                            elide: Text.ElideRight
                        }
                        Label {
                            width: parent.width
                            text: flow.dateText(entryRow.modelData.timestampMs) + "  " + entryRow.modelData.status
                            color: entryRow.modelData.status === "success" ? Theme.accentBright
                                : entryRow.modelData.status === "failed" ? Theme.danger : Theme.warning
                            font.pixelSize: 14
                            elide: Text.ElideRight
                        }
                    }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: flow.selectedIndex = entryRow.index
                    }
                }
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                TextArea {
                    text: flow.detail(flow.selected)
                    readOnly: true
                    selectByMouse: false
                    wrapMode: TextEdit.WordWrap
                    color: Theme.text
                    font.pixelSize: 18
                    background: Rectangle { color: Theme.panel; radius: 4 }
                }
            }
        }
    }
}
