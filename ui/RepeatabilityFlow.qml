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
import "Api.js" as Api

Popup {
    id: flow
    property string serviceUrl: "http://127.0.0.1:8137/api/v1"
    property string uiFont: "sans-serif"
    property string codeFont: "monospace"
    property string phase: "prepare"
    property var axes: [true, true, true]
    property int repetitions: 5
    property bool home: false
    property bool retract: true
    readonly property bool retractEachTime: home || retract
    property string failure: ""
    property string logText: ""
    property var measurements: []
    property var statistics: [null, null, null]
    property var activeRequest: null
    property bool terminalReceived: false
    readonly property bool canStart: axes.some(function(a) { return a; }) && repetitions >= 1 && repetitions <= 100
    readonly property bool showingResults: phase === "running" || phase === "result" || phase === "failed"
    NumericEditor { id: editor }

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

    function showCheck() {
        editor.cancel();
        phase = "prepare";
        failure = "";
        logText = "";
        measurements = [];
        statistics = [null, null, null];
        terminalReceived = false;
        open();
    }
    function confirmPreparation() { phase = "options"; }
    function number(value) { return value === null || value === undefined ? "--" : Number(value).toFixed(4); }
    function readingValue(value, axis) {
        if (value === null || value === undefined) return null;
        return phase === "running" || !statistics[axis] ? value : value - statistics[axis].mean;
    }
    function appendLog(message) {
        logText += message + "\n";
        Qt.callLater(function() { logScroll.contentItem.contentY = Math.max(0, logScroll.contentHeight - logScroll.availableHeight); });
    }
    function handleEvent(event) {
        if (event.type === "progress" && event.progress.kind === "script") {
            appendLog(event.progress.message);
        } else if (event.type === "measurement") {
            var rows = measurements.map(function(row) { return row.slice(); });
            while (rows.length < event.repetition) rows.push([null, null, null]);
            rows[event.repetition - 1][["X", "Y", "Z"].indexOf(event.axis)] = event.value;
            measurements = rows;
            if (event.statistics) statistics = event.statistics;
            Qt.callLater(function() { readings.positionViewAtEnd(); });
        } else if (event.type === "result" || event.type === "error") {
            terminalReceived = true;
            if (event.result) {
                measurements = event.result.measurements;
                statistics = event.result.statistics;
            }
            phase = event.type === "result" ? "result" : "failed";
            failure = event.message || "";
            if (failure) appendLog("; Failed: " + failure);
        }
    }
    function start() {
        if (phase !== "options" || !canStart) return;
        if (editor.target) editor.accept();
        if (editor.target) return;
        phase = "running";
        terminalReceived = false;
        activeRequest = Api.stream(serviceUrl + "/repeatability/run", {
            axes: axes.slice(), repetitions: repetitions, home: home, retract: retractEachTime
        }, handleEvent, function(error) {
            activeRequest = null;
            if (error || !terminalReceived) {
                failure = error || "Connection ended before the check finished";
                phase = "failed";
            }
        });
    }
    onClosed: {
        editor.cancel();
        if (activeRequest && phase === "running") activeRequest.abort();
    }

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
                    enabled: flow.phase !== "running"
                    onClicked: flow.close()
                }
                Label {
                    Layout.fillWidth: true
                    text: "Probe repeatability"
                    font.pixelSize: 24
                    color: Theme.text
                }
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 16
            spacing: 12
            Label {
                visible: flow.phase === "prepare"
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.margins: 16
                text: "First, set G54 X and Y zero on the inside corner of the L bracket, and Z zero on the bed.\n\nSelect G54. Leave room for the probe to extend and clear the path to X15 Y15 inside the bracket."
                color: Theme.text
                font.pixelSize: 24
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
            RowLayout {
                visible: flow.phase === "options"
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 18
                Item {
                    Layout.preferredWidth: 340
                    Layout.fillHeight: true
                    Label {
                        anchors.fill: parent
                        anchors.margins: 14
                        text: "The probe travels to X15 Y15, then Z5, and measures toward the walls and bed.\n\n" +
                            (flow.home ? "The machine homes before each repetition." : "X/Y travel starts at the current height. Make sure the path is clear.")
                        color: Theme.text
                        font.pixelSize: 20
                        wrapMode: Text.WordWrap
                        verticalAlignment: Text.AlignVCenter
                    }
                    NumericKeypad {
                        anchors.fill: parent
                        visible: editor.target !== null
                        onKeyPressed: function(key) { editor.typeKey(key); }
                        onAccepted: editor.accept()
                    }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: 10
                    RowLayout {
                        Layout.fillWidth: true
                        Label { text: "Axes"; color: Theme.text; font.pixelSize: 19; Layout.fillWidth: true }
                        Repeater {
                            model: ["X", "Y", "Z"]
                            LabButton {
                                required property string modelData
                                required property int index
                                Layout.preferredWidth: 56
                                Layout.preferredHeight: 52
                                text: modelData
                                font.pixelSize: 23
                                checkable: true
                                checked: flow.axes[index]
                                selected: checked
                                onClicked: {
                                    var selected = flow.axes.slice();
                                    selected[index] = checked;
                                    flow.axes = selected;
                                }
                            }
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        Label { text: "Repetitions"; color: Theme.text; font.pixelSize: 19; Layout.fillWidth: true }
                        NumberField {
                            objectName: "repetitions"
                            editor: editor
                            minimum: 1
                            maximum: 100
                            value: flow.repetitions
                            validator: IntValidator { bottom: 1; top: 100 }
                            onCommitted: function(value) { flow.repetitions = value; }
                            Layout.preferredWidth: 110
                            Layout.preferredHeight: 54
                            font.pixelSize: 26
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        Label { text: "Home each time"; color: Theme.text; font.pixelSize: 19; Layout.fillWidth: true }
                        ProbeSwitch {
                            Layout.preferredWidth: 110
                            Layout.preferredHeight: 54
                            checked: flow.home
                            onClicked: flow.home = checked
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        Label { text: "Retract each time"; color: Theme.text; font.pixelSize: 19; Layout.fillWidth: true }
                        ProbeSwitch {
                            Layout.preferredWidth: 110
                            Layout.preferredHeight: 54
                            checked: flow.retractEachTime
                            enabled: !flow.home
                            onClicked: flow.retract = checked
                        }
                    }
                    Item { Layout.fillHeight: true }
                }
            }
            Label {
                visible: flow.failure.length > 0
                Layout.fillWidth: true
                text: flow.failure
                color: Theme.warning
                font.pixelSize: 18
                wrapMode: Text.WordWrap
            }
            RowLayout {
                visible: flow.showingResults
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 16
                ScrollView {
                    id: logScroll
                    Layout.preferredWidth: 270
                    Layout.fillHeight: true
                    clip: true
                    TextArea {
                        text: flow.logText
                        readOnly: true
                        selectByMouse: false
                        wrapMode: TextEdit.NoWrap
                        font.family: flow.codeFont
                        font.pixelSize: 15
                        color: Theme.text
                        background: Rectangle { color: Theme.control; radius: 10 }
                    }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: 4
                    Label {
                        text: flow.phase === "running" ? "G54 measurements (mm)" : "Deviation from mean (mm)"
                        color: Theme.text
                        font.pixelSize: 18
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        Label { text: "Run"; color: Theme.textMuted; font.pixelSize: 17; Layout.preferredWidth: 78 }
                        Repeater {
                            model: ["X", "Y", "Z"]
                            Label {
                                required property string modelData
                                text: modelData
                                Layout.fillWidth: true
                                Layout.preferredWidth: 100
                                color: Theme.text
                                font.pixelSize: 19
                                horizontalAlignment: Text.AlignRight
                            }
                        }
                    }
                    ListView {
                        id: readings
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        clip: true
                        model: flow.measurements
                        ScrollBar.vertical: ScrollBar {}
                        delegate: RowLayout {
                            id: reading
                            required property var modelData
                            required property int index
                            width: readings.width
                            height: 32
                            Label { text: reading.index + 1; color: Theme.textMuted; font.pixelSize: 18; Layout.preferredWidth: 78 }
                            Repeater {
                                model: reading.modelData
                                Label {
                                    required property var modelData
                                    required property int index
                                    text: flow.number(flow.readingValue(modelData, index))
                                    Layout.fillWidth: true
                                    Layout.preferredWidth: 100
                                    color: Theme.text
                                    font.family: flow.codeFont
                                    font.pixelSize: 18
                                    horizontalAlignment: Text.AlignRight
                                }
                            }
                        }
                    }
                    Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.divider }
                    Repeater {
                        model: [{label:"Mean G54",key:"mean"}, {label:"Median",key:"median"}, {label:"Std dev",key:"stddev"}, {label:"Range",key:"range"}]
                        RowLayout {
                            id: summary
                            required property var modelData
                            Layout.fillWidth: true
                            Label { text: summary.modelData.label; color: Theme.textMuted; font.pixelSize: 17; Layout.preferredWidth: 78 }
                            Repeater {
                                model: flow.statistics
                                Label {
                                    required property var modelData
                                    text: flow.number(modelData ? modelData[summary.modelData.key] : null)
                                    Layout.fillWidth: true
                                    Layout.preferredWidth: 100
                                    color: Theme.text
                                    font.family: flow.codeFont
                                    font.pixelSize: 18
                                    horizontalAlignment: Text.AlignRight
                                }
                            }
                        }
                    }
                }
            }
            RowLayout {
                Layout.fillWidth: true
                Label {
                    Layout.fillWidth: true
                    text: flow.showingResults ? (flow.home ? "Home: yes" : "Home: no") + "    " + (flow.retractEachTime ? "Retract: yes" : "Retract: no") : ""
                    color: Theme.textMuted
                    font.pixelSize: 16
                }
                LabButton {
                    visible: !flow.showingResults
                    text: "Cancel"
                    Layout.preferredWidth: 140
                    Layout.preferredHeight: 50
                    font.pixelSize: 20
                    onClicked: flow.close()
                }
                LabButton {
                    visible: flow.phase !== "running"
                    text: flow.phase === "prepare" ? "G54 is ready" : flow.phase === "options" ? "Start check" : "Close"
                    enabled: flow.phase !== "options" || flow.canStart
                    Layout.preferredWidth: 180
                    Layout.preferredHeight: 50
                    font.pixelSize: 20
                    primary: flow.phase === "options"
                    onClicked: flow.phase === "prepare" ? flow.confirmPreparation() : flow.phase === "options" ? flow.start() : flow.close()
                }
                Label {
                    visible: flow.phase === "running"
                    text: "Checking..."
                    color: Theme.text
                    font.pixelSize: 20
                    Layout.preferredHeight: 50
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }
    }
}
