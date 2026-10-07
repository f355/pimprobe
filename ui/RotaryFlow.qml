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

pragma ComponentBehavior: Bound
import QtQuick
import "controls"
import QtQuick.Controls
import QtQuick.Layouts

PageView {
    id: flow
    required property var client
    property bool showHeader: true
    signal alarmRequested()
    property string uiFont
    property string codeFont
    property string phase: "review"
    property string reviewID: ""
    property string text: ""
    property string failure: ""
    property var result: ({})
    property bool simulated: false
    property var activeRequest: null
    property string requestedAction: ""
    property string operation: "axis"
    readonly property bool leveling: operation !== "axis"
    readonly property string zeroAxes: operation === "horizontal" ? "A/Z" : operation === "axis" ? "Y/Z" : "A/Y"
    readonly property string title: operation === "horizontal" ? "Level horizontal surface" : operation === "vertical" ? "Align vertical surface toward Y+" : operation === "verticalNegative" ? "Align vertical surface toward Y−" : "Calibrate rotary axis"
    readonly property bool busy: reviewRequest.pending || actionRequest.pending
    ServiceRequest { id: reviewRequest; client: flow.client }
    ServiceRequest { id: actionRequest; client: flow.client }

    x: 0
    y: 0
    width: parent ? parent.width : 800
    height: parent ? parent.height : 480
    padding: 0
    background: Rectangle { color: Theme.page }

    function showCalibration(config) {
        operation = config.operation || "axis";
        phase = "review";
        result = {};
        text = "";
        failure = "";
        reviewID = "";
        open();
        reviewRequest.send("rotary.review", config, function(reply) {
            if (!reply.ok || !reply.data) {
                failure = reply.error || "Could not prepare calibration";
                return;
            }
            reviewID = reply.data.id;
            text = reply.data.program;
            simulated = reply.data.simulated;
        });
    }
    function append(message) {
        text += message + "\n";
        Qt.callLater(function() {
            codeScroll.contentItem.contentY = Math.max(0, codeScroll.contentHeight - codeScroll.availableHeight);
        });
    }
    function proceed() {
        if (!reviewID || busy) return;
        phase = "running";
        text = "";
        var terminal = false;
        activeRequest = client.stream("rotary.run", {id: reviewID}, function(event) {
            if (event.type === "progress") {
                if (event.progress.kind === "script") append(event.progress.message);
            } else if (event.type === "result") {
                result = event.result;
                phase = "result";
                terminal = true;
            } else if (event.type === "error") {
                failure = event.message;
                if (event.code === "log" && event.result) {
                    result = event.result;
                    phase = "result";
                    append("; Warning: " + failure);
                } else {
                    append("; Failed: " + failure);
                    phase = "failed";
                }
                terminal = true;
                if (event.code === "controller_alarm") alarmRequested();
            }
        }, function(error) {
            activeRequest = null;
            if (!terminal) {
                phase = "failed";
                failure = error || "Calibration connection ended without a result";
            }
        });
    }
    function confirmAction(action) {
        requestedAction = action;
        confirmation.open();
    }
    function applyAction() {
        confirmation.close();
        failure = "";
        actionRequest.send("rotary." + requestedAction, {id: reviewID}, function(reply) {
            if (!reply.ok || !reply.data) {
                failure = reply.error || "Could not save calibration";
                return;
            }
            result = reply.data;
            append(requestedAction === "zero" ? "; " + zeroAxes + " work zero saved" : "; XY alignment saved");
        });
    }
    onClosed: {
        confirmation.close();
        reviewRequest.cancel();
        actionRequest.cancel();
        if (activeRequest) activeRequest.abort();
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 70
            visible: flow.showHeader
            color: Theme.header
            RowLayout {
                anchors.fill: parent
                BackButton {
                    Layout.preferredWidth: 70
                    Layout.fillHeight: true
                    enabled: flow.phase !== "running" && !flow.busy
                    onClicked: flow.close()
                }
                Label {
                    Layout.fillWidth: true
                    text: flow.title
                    color: Theme.text
                    font.family: flow.uiFont
                    font.pixelSize: 22
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
            Layout.margins: 14
            spacing: 10
            Label {
                Layout.fillWidth: true
                visible: flow.failure.length > 0
                text: flow.failure
                color: Theme.danger
                font.pixelSize: 17
                wrapMode: Text.WordWrap
            }
            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 16
                ScrollView {
                    id: codeScroll
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.preferredWidth: flow.phase === "result" ? 310 : 760
                    clip: true
                    contentWidth: codeText.implicitWidth
                    TextArea {
                        id: codeText
                        text: flow.text
                        readOnly: true
                        selectByMouse: false
                        font.family: flow.codeFont
                        font.pixelSize: 18
                        color: Theme.text
                        wrapMode: TextEdit.NoWrap
                        padding: 8
                        background: Rectangle { color: Theme.panel; radius: 12 }
                    }
                }
                ColumnLayout {
                    visible: flow.phase === "result"
                    Layout.preferredWidth: 420
                    Layout.fillHeight: true
                    spacing: 10
                    Label {
                        text: flow.leveling ? "Verified touches · G53" : "Axis centers · G53"
                        color: Theme.text
                        font.family: flow.uiFont
                        font.pixelSize: 21
                    }
                    Repeater {
                        model: flow.leveling ? ((flow.result.level || {}).touches || []) : (flow.result.stations || [])
                        ColumnLayout {
                            required property var modelData
                            required property int index
                            Layout.fillWidth: true
                            spacing: 4
                            Label {
                                text: (flow.leveling ? "Touch " : "Station ") + (parent.index + 1)
                                color: Theme.textMuted
                                font.pixelSize: 18
                            }
                            Label {
                                Layout.fillWidth: true
                                text: ["X", "Y", "Z"].map(function(axis, i) {
                                    var point = flow.leveling ? parent.modelData : parent.modelData.center;
                                    return axis + " " + Number(point[i]).toFixed(3);
                                }).join("   ")
                                color: Theme.text
                                font.family: flow.codeFont
                                font.pixelSize: 18
                                wrapMode: Text.WordWrap
                            }
                        }
                    }
                    Label {
                        text: flow.leveling ? "A correction  " + Number((flow.result.level || {}).correction || 0).toFixed(4) + "°"
                            : "XY alignment  " + Number(flow.result.xyAngle || 0).toFixed(4) + "°"
                        color: Theme.text
                        font.pixelSize: 20
                    }
                    Label {
                        text: flow.leveling ? "Remaining tilt  " + Number((flow.result.level || {}).residual || 0).toFixed(4) + "°"
                            : "XZ slope  " + Number(flow.result.xzAngle || 0).toFixed(4) + "°"
                        color: Theme.text
                        font.pixelSize: 20
                    }
                    Label {
                        Layout.fillWidth: true
                        text: flow.zeroAxes + " zero: " + (flow.leveling ? "measured surface" : "station 1") + " · G" + flow.result.wcs +
                              (flow.result.zeroed ? " · Saved" : "") +
                              (flow.result.rotationApplied ? "\nXY alignment saved" : "")
                        color: Theme.textMuted
                        font.pixelSize: 18
                        wrapMode: Text.WordWrap
                    }
                    Item { Layout.fillHeight: true }
                }
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: 10
                Item { Layout.fillWidth: true }
                LabButton {
                    visible: flow.phase === "review"
                    text: "Cancel"
                    Layout.preferredHeight: 56
                    font.pixelSize: 20
                    onClicked: flow.close()
                }
                LabButton {
                    visible: flow.phase === "review"
                    text: flow.busy ? "Preparing…" : "Proceed"
                    enabled: !flow.busy && flow.reviewID.length > 0
                    primary: true
                    Layout.preferredHeight: 56
                    font.pixelSize: 20
                    onClicked: flow.proceed()
                }
                LabButton {
                    visible: flow.phase === "result"
                    text: "Set " + flow.zeroAxes + " zero"
                    enabled: !flow.busy && !flow.result.zeroed
                    primary: true
                    Layout.preferredHeight: 56
                    font.pixelSize: 20
                    onClicked: flow.confirmAction("zero")
                }
                LabButton {
                    visible: flow.phase === "result" && !flow.leveling && flow.result.rotationSupported === true
                    text: "Align XY"
                    enabled: !flow.busy && !flow.result.rotationApplied
                    Layout.preferredHeight: 56
                    font.pixelSize: 20
                    onClicked: flow.confirmAction("rotation")
                }
                LabButton {
                    visible: flow.phase === "result" || flow.phase === "failed"
                    text: "Close"
                    enabled: !flow.busy
                    Layout.preferredHeight: 56
                    font.pixelSize: 20
                    onClicked: flow.close()
                }
            }
        }
    }
    Dialog {
        id: confirmation
        Component.onCompleted: if ("popupType" in confirmation) confirmation.popupType = Popup.Item
        parent: flow
        anchors.centerIn: parent
        width: 590
        height: 220
        modal: true
        closePolicy: Popup.NoAutoClose
        background: Rectangle { color: Theme.panelRaised; radius: 12; border.color: Theme.divider }
        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 16
            Label {
                Layout.fillWidth: true
                Layout.fillHeight: true
                text: flow.requestedAction === "zero"
                    ? "Set G" + flow.result.wcs + " " + flow.zeroAxes + " zero " + (flow.leveling ? "to the measured surface and current A angle?" : "to station 1?")
                    : "Set G" + flow.result.wcs + " XY rotation to " + Number(flow.result.xyAngle).toFixed(4) + "°?"
                color: Theme.text
                font.family: flow.uiFont
                font.pixelSize: 22
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 14
                LabButton { text: "Cancel"; font.pixelSize: 22; onClicked: confirmation.close() }
                LabButton { text: "Apply"; font.pixelSize: 22; primary: true; onClicked: flow.applyAction() }
            }
        }
    }
}
