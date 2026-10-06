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
    property bool simulated: false
    property string uiFont: "Inter"
    property string codeFont: "monospace"
    property var routine: ({})
    property string phase: "review"
    property var program: []
    property string reviewID: ""
    property string failure: ""
    readonly property bool reviewing: reviewRequest.pending
    ServiceRequest {
        client: flow.client
        id: reviewRequest
    }
    ServiceRequest {
        client: flow.client
        id: zeroRequest
    }
    property var activeRequest: null
    property bool zeroed: false
    property bool returned: false
    property bool positioned: false
    property bool returning: false
    property bool positioning: false
    property real safeZOffset: 40
    signal alarmRequested()
    signal settingChanged(string key, var value)
    property string completedID: ""
    readonly property bool zeroing: zeroRequest.pending
    property string logText: ""
    property var result: []
    property var spans: [null, null, null]
    readonly property var dimensions: {
        var round = routine.feature === "boss" || routine.feature === "hole";
        return [0, 1].filter(function(i) { return spans[i] !== null; }).map(function(i) {
            var ridge = routine.feature === "y-ridge" || routine.feature === "y-valley";
            var label = round ? "Span " + ["X", "Y"][i] : i === 0 ? "Width X" : ridge ? "Width Y" : "Length Y";
            return {label: label, value: spans[i]};
        });
    }
    property var machinePoint: [null, null, null]
    property var offsets: [0, 0, 0]
    readonly property var measuredAxes: [0, 1, 2].filter(function (i) { return flow.result.length > i && flow.result[i] !== null; })
    NumericEditor { id: resultEditor }

    function measuredPosition(axis) {
        return Number(machinePoint[axis]);
    }

    function setOffset(axis, value) {
        var updated = offsets.slice();
        updated[axis] = value;
        offsets = updated;
        zeroed = false;
    }
    onPhaseChanged: {
        if (phase === "result" || phase === "failed")
            Qt.callLater(scrollLogToEnd);
    }
    readonly property string description: routine.family === "center" ? "Probing " + (routine.z ? "Z surface" : routine.feature.replace("-", " ") + " center") + "." : "Probing " + (routine.family || "outside") + " " + (routine.z ? "Z surface" : routine.x && routine.y ? "X/Y corner" : routine.x ? "X" + (routine.x > 0 ? "+" : "-") + " edge" : "Y" + (routine.y > 0 ? "+" : "-") + " edge") + "."

    x: 0
    y: 0
    width: parent ? parent.width : 800
    height: parent ? parent.height : 480
    padding: 0
    background: Rectangle {
        color: Theme.page
    }

    function showRoutine(value) {
        routine = value;
        phase = "review";
        failure = "";
        logText = "";
        result = [];
        spans = [null, null, null];
        offsets = [0, 0, 0];
        resultEditor.cancel();
        machinePoint = [null, null, null];
        program = [];
        reviewID = "";
        zeroed = false;
        completedID = "";
        returned = false;
        positioned = false;
        returning = false;
        positioning = false;
        safeZOffset = Number(value.safeZOffset);
        reviewRequest.cancel();
        zeroRequest.cancel();
        open();
        reviewRequest.send("routine.review", value, function (reply) {
            if (!reply.ok || !reply.data) {
                failure = reply.error || "Unable to review this routine";
                return;
            }
            program = reply.data.program;
            reviewID = reply.data.id;
            simulated = reply.data.simulated === true;
        });
    }

    function appendLog(message) {
        logText += message + "\n";
        Qt.callLater(scrollLogToEnd);
    }

    function scrollLogToEnd() {
        if (phase !== "review") {
            logScroll.contentItem.contentY = Math.max(0, logScroll.contentHeight - logScroll.availableHeight);
        }
    }

    function proceed() {
        if (!reviewID || reviewing)
            return;
        var id = reviewID;
        reviewID = "";
        startMotion(id, "run");
    }

    function returnToStart() {
        if (!completedID || zeroing || returned || phase !== "result") return;
        resultEditor.cancel();
        var id = completedID;
        completedID = "";
        startMotion(id, "return");
    }

    function goToMeasured() {
        if (!completedID || zeroing || positioned || phase !== "result") return;
        if (resultEditor.target) resultEditor.accept();
        if (resultEditor.target) return;
        var id = completedID;
        completedID = "";
        startMotion(id, "measured");
    }

    function acceptResult(measurement, id) {
        result = measurement.point;
        machinePoint = measurement.machinePoint;
        spans = measurement.spans || [null, null, null];
        zeroed = measurement.zeroed;
        returned = measurement.returned === true;
        positioned = measurement.positioned === true;
        completedID = id;
        phase = "result";
    }

    function startMotion(id, action) {
        returning = action === "return";
        positioning = action === "measured";
        phase = "running";
        if (action === "run") logText = "";
        failure = "";
        var terminal = false;
        var body = {id: id};
        if (positioning) body.safeZOffset = safeZOffset;
        activeRequest = client.stream("routine." + action, body, function (event) {
            if (event.type === "progress") {
                if (event.progress.kind === "script")
                    appendLog(event.progress.message);
            } else if (event.type === "result") {
                acceptResult(event.result, id);
                terminal = true;
            } else if (event.type === "error") {
                terminal = true;
                failure = event.message;
                if (event.code === "log" && event.result) {
                    acceptResult(event.result, id);
                    appendLog("; Warning: " + failure);
                } else {
                    appendLog("; Failed: " + failure);
                    phase = "failed";
                }
                if (event.code === "controller_alarm")
                    alarmRequested();
            }
        }, function (error) {
            activeRequest = null;
            if (!terminal || error) {
                phase = "failed";
                failure = error || "Routine connection ended without a confirmed result";
            }
        });
    }

    function zeroResult() {
        if (!completedID || zeroing || zeroed)
            return;
        if (resultEditor.target) resultEditor.accept();
        if (resultEditor.target) return;
        var id = completedID;
        completedID = "";
        failure = "";
        zeroRequest.send("routine.zero", {
            id: id,
            offsets: offsets.slice()
        }, function (reply) {
            if (!reply.ok || !reply.data) {
                failure = reply.error || "Unable to set work zero";
                // Busy is rejected before the backend consumes the result token.
                if (reply.data && reply.data.code === "busy")
                    completedID = id;
                return;
            }
            result = reply.data.point;
            zeroed = reply.data.zeroed;
            completedID = id;
            appendLog("; G" + routine.wcs + " work zero confirmed");
        });
    }

    Component.onDestruction: if (activeRequest) activeRequest.abort()
    onClosed: {
        resultEditor.cancel();
        reviewRequest.cancel();
        zeroRequest.cancel();
        if (activeRequest && phase === "running")
            activeRequest.abort();
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.headerHeight
            visible: flow.showHeader
            color: Theme.header
            RowLayout {
                anchors.fill: parent
                spacing: 8
                BackButton {
                    Layout.preferredWidth: Theme.headerHeight
                    Layout.fillHeight: true
                    enabled: flow.phase !== "running" && !flow.zeroing
                    onClicked: flow.close()
                }
                Label {
                    Layout.fillWidth: true
                    text: flow.description
                    color: Theme.text
                    font.family: flow.uiFont
                    font.pixelSize: 22
                    wrapMode: Text.WordWrap
                }
                Label {
                    Layout.rightMargin: 18
                    text: flow.simulated ? "Simulation" : ""
                    color: Theme.textMuted
                    font.pixelSize: 18
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 16
            spacing: 12

            Label {
                Layout.fillWidth: true
                visible: flow.failure.length > 0 || flow.reviewing
                text: flow.reviewing ? "Preparing routine..." : flow.failure
                color: Theme.warning
                font.pixelSize: 18
                wrapMode: Text.WordWrap
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: Theme.groupSpacing

                Item {
                    Layout.fillWidth: flow.phase !== "result"
                    Layout.preferredWidth: flow.phase === "result" ? Theme.columnWidth : -1
                    Layout.fillHeight: true
                    ScrollView {
                        id: logScroll
                        anchors.fill: parent
                        clip: true
                        contentWidth: Math.max(availableWidth, logArea.implicitWidth)
                        onHeightChanged: Qt.callLater(flow.scrollLogToEnd)
                        TextArea {
                            id: logArea
                            text: flow.phase === "review" ? flow.program.join("\n") : flow.logText
                            readOnly: true
                            selectByMouse: false
                            wrapMode: TextEdit.NoWrap
                            font.family: flow.codeFont
                            font.pixelSize: 18
                            padding: 8
                            color: Theme.text
                            background: Rectangle { color: Theme.control; radius: 10 }
                        }
                    }
                    NumericKeypad {
                        anchors.fill: parent
                        visible: resultEditor.target !== null
                        onKeyPressed: function(key) { resultEditor.typeKey(key); }
                        onAccepted: resultEditor.accept()
                    }
                }

                ColumnLayout {
                    visible: flow.phase === "result"
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: flow.dimensions.length ? 8 : 5
                    ColumnLayout {
                        visible: flow.dimensions.length > 0
                        Layout.fillWidth: true
                        spacing: 2
                        Repeater {
                            model: flow.dimensions
                            RowLayout {
                                required property var modelData
                                Layout.fillWidth: true
                                Label {
                                    text: modelData.label
                                    Layout.fillWidth: true
                                    font.pixelSize: 20
                                    color: Theme.text
                                }
                                Label {
                                    text: modelData.value.toFixed(3) + " mm"
                                    font.pixelSize: 22
                                    color: Theme.text
                                }
                            }
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        Label {
                            Layout.fillWidth: true
                            text: "Measured \u00b7 G53"
                            font.pixelSize: 20
                            color: Theme.text
                        }
                        Label {
                            Layout.preferredWidth: 132
                            text: "Offset (mm)"
                            font.pixelSize: 20
                            horizontalAlignment: Text.AlignRight
                            color: Theme.text
                        }
                    }
                    Repeater {
                        model: flow.measuredAxes
                        RowLayout {
                            required property int modelData
                            Layout.fillWidth: true
                            spacing: 8
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0
                                Label {
                                    Layout.fillWidth: true
                                    text: ["X", "Y", "Z"][modelData] + "  " + flow.measuredPosition(modelData).toFixed(3)
                                    font.family: flow.uiFont
                                    font.pixelSize: 26
                                    color: Theme.text
                                }
                                Label {
                                    Layout.fillWidth: true
                                    text: "G" + flow.routine.wcs + " zero at G53 " + (flow.measuredPosition(modelData) + flow.offsets[modelData]).toFixed(3)
                                    font.pixelSize: 18
                                    wrapMode: Text.WordWrap
                                    color: Theme.textMuted
                                }
                            }
                            NumberField {
                                enabled: !flow.zeroing && !flow.zeroed && flow.completedID.length > 0
                                editor: resultEditor
                                minimum: -1000
                                maximum: 1000
                                value: flow.offsets[modelData]
                                onCommitted: function(value) { flow.setOffset(modelData, value); }
                                Layout.preferredWidth: 132
                                Layout.preferredHeight: flow.dimensions.length ? 54 : 58
                                font.pixelSize: 26
                            }
                        }
                    }
                    Item { Layout.fillHeight: true }
                    RowLayout {
                        visible: flow.routine.family === "inside" && !flow.routine.z
                        Layout.fillWidth: true
                        Label {
                            Layout.fillWidth: true
                            text: "Safe Z offset"
                            color: Theme.text
                            font.pixelSize: 20
                        }
                        NumberField {
                            editor: resultEditor
                            minimum: 0.1
                            maximum: 1000
                            value: flow.safeZOffset
                            onCommitted: function(value) {
                                flow.safeZOffset = value;
                                flow.settingChanged("safeZOffset", value);
                            }
                            Layout.preferredWidth: 132
                            Layout.preferredHeight: 54
                            font.pixelSize: 24
                        }
                    }
                    Label {
                        text: flow.zeroed ? "Work zero set" : "Work zero was not changed"
                        color: flow.zeroed ? Theme.accentBright : Theme.text
                        font.pixelSize: 18
                    }
                }
            }
            RowLayout {
                Layout.fillWidth: true
                Item {
                    Layout.fillWidth: true
                }
                LabButton {
                    visible: flow.phase === "review"
                    text: "Cancel"
                    Layout.preferredWidth: 140
                    Layout.preferredHeight: 56
                    font.pixelSize: 20
                    onClicked: flow.close()
                }
                LabButton {
                    text: "Set Work Zero"
                    visible: flow.phase === "result"
                    enabled: !flow.zeroing && !flow.zeroed && flow.completedID.length > 0
                    Layout.preferredWidth: 190
                    Layout.preferredHeight: 56
                    font.pixelSize: 20
                    primary: true
                    onClicked: flow.zeroResult()
                }
                LabButton {
                    text: "Go to starting position"
                    visible: flow.phase === "result" && (flow.routine.family !== "inside" || flow.routine.z)
                    enabled: !flow.zeroing && !flow.returned && flow.completedID.length > 0
                    Layout.preferredWidth: 280
                    Layout.preferredHeight: 56
                    font.pixelSize: 20
                    onClicked: flow.returnToStart()
                }
                LabButton {
                    text: "Go to measured point"
                    visible: flow.phase === "result" && flow.routine.family === "inside" && !flow.routine.z
                    enabled: !flow.zeroing && !flow.positioned && flow.completedID.length > 0
                    Layout.preferredWidth: 240
                    Layout.preferredHeight: 56
                    font.pixelSize: 20
                    onClicked: flow.goToMeasured()
                }
                LabButton {
                    visible: flow.phase !== "running"
                    text: flow.phase === "review" ? "Proceed" : "Close"
                    enabled: !flow.zeroing && (flow.phase !== "review" || (flow.reviewID.length > 0 && !flow.reviewing))
                    Layout.preferredWidth: 140
                    Layout.preferredHeight: 56
                    font.pixelSize: 20
                    primary: flow.phase === "review"
                    onClicked: flow.phase === "review" ? flow.proceed() : flow.close()
                }
                Label {
                    visible: flow.phase === "running"
                    text: flow.returning ? "Returning to starting position..." : flow.positioning ? "Moving to measured point..." : "Probing..."
                    color: Theme.text
                    font.pixelSize: 20
                    Layout.preferredHeight: 56
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }
    }
}
