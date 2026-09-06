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
        id: reviewRequest
    }
    ServiceRequest {
        id: zeroRequest
    }
    property var activeRequest: null
    property bool zeroed: false
    property bool returned: false
    property bool returning: false
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

    parent: Overlay.overlay
    x: 0
    y: 0
    width: parent ? parent.width : 800
    height: parent ? parent.height : 480
    padding: 0
    modal: true
    closePolicy: Popup.NoAutoClose
    background: Rectangle {
        color: "#2B2B2B"
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
        returning = false;
        reviewRequest.cancel();
        zeroRequest.cancel();
        open();
        reviewRequest.send("POST", serviceUrl + "/routine/review", value, function (reply) {
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
        startMotion(id, false);
    }

    function returnToStart() {
        if (!completedID || zeroing || returned || phase !== "result") return;
        resultEditor.cancel();
        var id = completedID;
        completedID = "";
        startMotion(id, true);
    }

    function startMotion(id, isReturn) {
        returning = isReturn;
        phase = "running";
        if (!isReturn) logText = "";
        failure = "";
        var terminal = false;
        activeRequest = Api.stream(serviceUrl + (isReturn ? "/routine/return" : "/routine/run"), {
            id: id
        }, function (event) {
            if (event.type === "progress") {
                if (event.progress.kind === "script")
                    appendLog(event.progress.message);
            } else if (event.type === "result") {
                result = event.result.point;
                machinePoint = event.result.machinePoint;
                spans = event.result.spans || [null, null, null];
                zeroed = event.result.zeroed;
                returned = event.result.returned === true;
                completedID = id;
                phase = "result";
                terminal = true;
            } else if (event.type === "error") {
                terminal = true;
                failure = event.message;
                appendLog("; Failed: " + failure);
                phase = "failed";
                if (event.code === "controller_alarm")
                    Qt.quit();
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
        zeroRequest.send("POST", serviceUrl + "/routine/zero", {
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
            Layout.preferredHeight: 70
            color: "#242424"
            RowLayout {
                anchors.fill: parent
                spacing: 8
                BackButton {
                    Layout.preferredWidth: 70
                    Layout.fillHeight: true
                    enabled: flow.phase !== "running" && !flow.zeroing
                    onClicked: flow.close()
                }
                Label {
                    Layout.fillWidth: true
                    text: flow.description
                    color: "#E1E1E1"
                    font.family: flow.uiFont
                    font.pixelSize: 22
                    wrapMode: Text.WordWrap
                }
                Label {
                    Layout.rightMargin: 18
                    text: flow.simulated ? "Simulation" : ""
                    color: "#B5B5B5"
                    font.pixelSize: 15
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
                color: "#FFB078"
                font.pixelSize: 18
                wrapMode: Text.WordWrap
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 18

                Item {
                    Layout.fillWidth: flow.phase !== "result"
                    Layout.preferredWidth: flow.phase === "result" ? 340 : -1
                    Layout.fillHeight: true
                    ScrollView {
                        id: logScroll
                        anchors.fill: parent
                        clip: true
                        onHeightChanged: Qt.callLater(flow.scrollLogToEnd)
                        TextArea {
                            id: logArea
                            text: flow.phase === "review" ? flow.program.join("\n") : flow.logText
                            readOnly: true
                            selectByMouse: false
                            wrapMode: TextEdit.NoWrap
                            font.family: flow.codeFont
                            font.pixelSize: 16
                            color: "#E1E1E1"
                            background: Rectangle { color: "#202020" }
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
                    spacing: flow.dimensions.length ? 8 : 14
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
                                    color: "#D8D8D8"
                                }
                                Label {
                                    text: modelData.value.toFixed(3) + " mm"
                                    font.pixelSize: 22
                                    color: "#FFFFFF"
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
                            color: "#D8D8D8"
                        }
                        Label {
                            Layout.preferredWidth: 132
                            text: "Offset (mm)"
                            font.pixelSize: 20
                            horizontalAlignment: Text.AlignRight
                            color: "#D8D8D8"
                        }
                    }
                    Repeater {
                        model: flow.measuredAxes
                        ColumnLayout {
                            required property int modelData
                            Layout.fillWidth: true
                            spacing: 6
                            RowLayout {
                                Layout.fillWidth: true
                                Label {
                                    Layout.fillWidth: true
                                    text: ["X", "Y", "Z"][modelData] + "  " + flow.measuredPosition(modelData).toFixed(3)
                                    font.family: flow.uiFont
                                    font.pixelSize: 26
                                    color: "#FFFFFF"
                                }
                                NumberField {
                                    enabled: !flow.zeroing && !flow.zeroed && flow.completedID.length > 0
                                    editor: resultEditor
                                    minimum: -1000
                                    maximum: 1000
                                    value: flow.offsets[modelData]
                                    onCommitted: function(value) { flow.setOffset(modelData, value); }
                                    Layout.preferredWidth: 132
                                    Layout.preferredHeight: flow.dimensions.length ? 54 : 62
                                    font.pixelSize: 26
                                }
                            }
                            Label {
                                Layout.fillWidth: true
                                text: "G" + flow.routine.wcs + " zero at G53 " + (flow.measuredPosition(modelData) + flow.offsets[modelData]).toFixed(3)
                                font.pixelSize: 17
                                color: "#B5B5B5"
                            }
                        }
                    }
                    Item { Layout.fillHeight: true }
                    Label {
                        text: flow.zeroed ? "Work zero set" : "Work zero was not changed"
                        color: flow.zeroed ? "#70CF7B" : "#D8D8D8"
                        font.pixelSize: 18
                    }
                }
            }
            RowLayout {
                Layout.fillWidth: true
                Item {
                    Layout.fillWidth: true
                }
                Button {
                    visible: flow.phase === "review"
                    text: "Cancel"
                    Layout.preferredWidth: 140
                    Layout.preferredHeight: 50
                    font.pixelSize: 20
                    onClicked: flow.close()
                }
                Button {
                    text: "Set Work Zero"
                    visible: flow.phase === "result"
                    enabled: !flow.zeroing && !flow.zeroed && flow.completedID.length > 0
                    Layout.preferredWidth: 190
                    Layout.preferredHeight: 50
                    font.pixelSize: 20
                    onClicked: flow.zeroResult()
                }
                Button {
                    text: "Go to starting position"
                    visible: flow.phase === "result"
                    enabled: !flow.zeroing && !flow.returned && flow.completedID.length > 0
                    Layout.preferredWidth: 280
                    Layout.preferredHeight: 50
                    font.pixelSize: 20
                    onClicked: flow.returnToStart()
                }
                Button {
                    visible: flow.phase !== "running"
                    text: flow.phase === "review" ? "Proceed" : "Close"
                    enabled: !flow.zeroing && (flow.phase !== "review" || (flow.reviewID.length > 0 && !flow.reviewing))
                    Layout.preferredWidth: 140
                    Layout.preferredHeight: 50
                    font.pixelSize: 20
                    onClicked: flow.phase === "review" ? flow.proceed() : flow.close()
                }
                Label {
                    visible: flow.phase === "running"
                    text: flow.returning ? "Returning to starting position..." : "Probing..."
                    color: "#D8D8D8"
                    font.pixelSize: 20
                    Layout.preferredHeight: 50
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }
    }
}
