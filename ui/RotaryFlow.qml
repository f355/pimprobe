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
    signal alarmRequested
    signal calibrationSaved(string message)
    signal resultWcsRequested(int wcs)
    onResultWcsRequested: function (wcs) {
        selectResultWcs(wcs);
    }
    property bool historical: false
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
    property bool sourceVisible: false
    property string operation: "axis"
    property var config: ({})
    readonly property bool leveling: operation !== "axis"
    readonly property string zeroAxes: operation === "horizontal" ? "A/Z" : operation === "axis" ? "Y/Z" : "A/Y"
    readonly property string title: operation === "horizontal" ? "Level horizontal surface" : operation === "vertical" ? "Align vertical surface toward Y+" : operation === "verticalNegative" ? "Align vertical surface toward Y−" : "Calibrate rotary axis"
    readonly property bool busy: reviewRequest.pending || actionRequest.pending
    ServiceRequest {
        id: reviewRequest
        client: flow.client
    }
    ServiceRequest {
        id: actionRequest
        client: flow.client
    }

    x: 0
    y: 0
    width: parent ? parent.width : 800
    height: parent ? parent.height : 480
    padding: 0
    font.family: uiFont
    background: Rectangle {
        color: Theme.page
    }

    function showCalibration(config) {
        historical = false;
        sourceVisible = false;
        operation = config.operation || "axis";
        flow.config = Object.assign({}, config);
        phase = "review";
        result = {};
        text = "";
        failure = "";
        reviewID = "";
        reviewRequest.cancel();
        open();
    }
    function prepareRun(value) {
        config = value;
        reviewID = "";
        failure = "";
        reviewRequest.send("rotary.review", value, function (reply) {
            if (!reply.ok || !reply.data) {
                failure = reply.error || "Could not prepare calibration";
                return;
            }
            config = reply.data.config;
            reviewID = reply.data.id;
            text = reply.data.program;
            simulated = reply.data.simulated;
            runReviewed();
        });
    }
    function showHistory(opened, details) {
        reviewRequest.cancel();
        actionRequest.cancel();
        historical = true;
        operation = opened.entry.config.operation || "axis";
        result = opened.result;
        reviewID = opened.canApply ? opened.entry.id : "";
        phase = "result";
        failure = opened.entry.error || "";
        text = details;
        sourceVisible = false;
        open();
    }
    function selectResultWcs(wcs) {
        if (!reviewID || busy || phase !== "result")
            return;
        failure = "";
        actionRequest.send(historical ? "history.wcs" : "rotary.wcs", {
            id: reviewID,
            wcs: wcs
        }, function (reply) {
            if (!reply.ok || !reply.data) {
                failure = reply.error || "Could not select work coordinates";
                return;
            }
            result = reply.data;
            client.refresh();
        });
    }
    function append(message) {
        text += message + "\n";
        Qt.callLater(function () {
            codeScroll.contentItem.contentY = Math.max(0, codeScroll.contentHeight - codeScroll.availableHeight);
        });
    }
    function proceed() {
        if (phase !== "review" || busy || !operationReview.item)
            return;
        var value = operationReview.item.parameters();
        if (!value)
            return;
        prepareRun(value);
    }
    function runReviewed() {
        phase = "running";
        text = "";
        var terminal = false;
        activeRequest = client.stream("rotary.run", {
            id: reviewID
        }, function (event) {
            if (event.type === "progress") {
                if (event.progress.kind === "script")
                    append(event.progress.message);
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
                if (event.code === "controller_alarm")
                    alarmRequested();
            }
        }, function (error) {
            activeRequest = null;
            if (!terminal) {
                phase = "failed";
                failure = error || "Calibration connection ended without a result";
            }
        });
    }
    function confirmAction(action) {
        if (!reviewID || busy)
            return;
        requestedAction = action;
        confirmation.open();
    }
    function applyAction() {
        confirmation.close();
        failure = "";
        actionRequest.send((historical ? "history." : "rotary.") + requestedAction, {
            id: reviewID
        }, function (reply) {
            if (!reply.ok || !reply.data) {
                failure = reply.error || "Could not save calibration";
                return;
            }
            result = reply.data;
            calibrationSaved(requestedAction === "zero" ? "Work zero set" : "X/Y rotation set");
            append(requestedAction === "zero" ? "; " + zeroAxes + " work zero saved" : "; X/Y work-coordinate rotation saved");
            client.refresh();
        });
    }
    onClosed: {
        confirmation.close();
        reviewRequest.cancel();
        actionRequest.cancel();
        if (activeRequest)
            activeRequest.abort();
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        PageHeader {
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.headerHeight
            visible: flow.showHeader
            title: flow.title
            detail: flow.phase === "failed" ? "Failed" : flow.phase === "result" ? "Measured · G53" : flow.phase === "running" ? "Running" : "Confirm"
            uiFont: flow.uiFont
            backEnabled: flow.phase !== "running" && !flow.busy
            onBack: flow.close()
        }
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 16
            spacing: 12
            MessageStrip {
                Layout.fillWidth: true
                visible: flow.failure.length > 0
                text: flow.failure
                textColor: Theme.danger
            }
            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 16
                Loader {
                    id: operationReview
                    active: flow.opened && flow.phase === "review"
                    visible: active
                    enabled: !flow.busy
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    sourceComponent: OperationReview {
                        config: flow.config
                        rotary: true
                    }
                }
                ScrollView {
                    id: codeScroll
                    visible: flow.phase !== "review" && (flow.phase !== "result" || flow.sourceVisible)
                    Layout.fillWidth: true
                    Layout.fillHeight: true
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
                        background: Rectangle {
                            color: Theme.panel
                            radius: Theme.radius
                        }
                    }
                }
                RotaryResultView {
                    visible: flow.phase === "result" && !flow.sourceVisible
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    flow: flow
                }
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: 10
                LabButton {
                    visible: flow.phase === "result"
                    text: flow.sourceVisible ? "Result" : flow.historical ? "Details" : "Log"
                    Layout.preferredWidth: 96
                    Layout.preferredHeight: 64
                    onClicked: flow.sourceVisible = !flow.sourceVisible
                }
                Item {
                    Layout.fillWidth: true
                }
                LabButton {
                    visible: flow.phase === "review"
                    text: "Cancel"
                    Layout.preferredHeight: 64
                    Layout.preferredWidth: 144
                    font.pixelSize: 20
                    onClicked: flow.close()
                }
                LabButton {
                    visible: flow.phase === "review"
                    text: flow.busy ? "Preparing…" : "Proceed"
                    enabled: !flow.busy
                    primary: true
                    Layout.preferredHeight: 64
                    Layout.preferredWidth: 184
                    font.pixelSize: 20
                    onClicked: flow.proceed()
                }
                LabButton {
                    visible: flow.phase === "result"
                    text: "Set " + flow.zeroAxes + " zero"
                    enabled: !flow.busy && flow.reviewID.length > 0
                    primary: true
                    Layout.preferredHeight: 64
                    Layout.preferredWidth: 232
                    font.pixelSize: 20
                    onClicked: flow.confirmAction("zero")
                }
                LabButton {
                    visible: flow.phase === "result" && !flow.leveling && flow.result.rotationSupported === true
                    text: "Set X/Y rotation"
                    enabled: !flow.busy && flow.reviewID.length > 0
                    Layout.preferredHeight: 64
                    Layout.preferredWidth: 216
                    font.pixelSize: 20
                    onClicked: flow.confirmAction("rotation")
                }
                LabButton {
                    visible: flow.phase === "result" || flow.phase === "failed"
                    text: "Close"
                    enabled: !flow.busy
                    Layout.preferredHeight: 64
                    Layout.preferredWidth: 144
                    font.pixelSize: 20
                    onClicked: flow.close()
                }
            }
        }
    }
    TouchDialog {
        id: confirmation
        title: flow.requestedAction === "zero" ? "Set work zero" : "Set X/Y rotation"
        message: flow.requestedAction === "zero" ? "Set G" + flow.result.wcs + " " + flow.zeroAxes + " zero " + (flow.leveling ? "to the measured surface and current A angle?" : "to the first measured axis center?") : "Set G" + flow.result.wcs + " X/Y rotation to " + Number(flow.result.xyAngle).toFixed(4) + "°?"
        font.family: flow.uiFont
        onAccepted: flow.applyAction()
    }
}
