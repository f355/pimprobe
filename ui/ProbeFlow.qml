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
        id: reviewRequest
        client: flow.client
    }
    ServiceRequest {
        id: zeroRequest
        client: flow.client
    }
    ServiceRequest {
        id: wcsRequest
        client: flow.client
    }
    property var activeRequest: null
    property bool zeroed: false
    property bool returned: false
    property bool positioned: false
    property bool returning: false
    property bool positioning: false
    property real safeZOffset: 40
    signal alarmRequested
    signal settingChanged(string key, var value)
    property string completedID: ""
    readonly property bool zeroing: zeroRequest.pending
    readonly property bool busy: zeroing || wcsRequest.pending
    property bool historical: false
    property string logText: ""
    property var result: []
    property var spans: [null, null, null]
    readonly property var dimensions: {
        var round = routine.feature === "boss" || routine.feature === "hole";
        return [0, 1].filter(function (i) {
            return spans[i] !== null;
        }).map(function (i) {
            var ridge = routine.feature === "y-ridge" || routine.feature === "y-valley";
            var label = round ? "Span " + ["X", "Y"][i] : i === 0 ? "Width X" : ridge ? "Width Y" : "Length Y";
            return {
                label: label,
                value: spans[i]
            };
        });
    }
    property var machinePoint: [null, null, null]
    property var offsets: [0, 0, 0]
    property bool sourceVisible: false
    signal workZeroSaved
    signal resultWcsRequested(int wcs)
    onResultWcsRequested: function (wcs) {
        selectResultWcs(wcs);
    }
    readonly property var measuredAxes: [0, 1, 2].filter(function (i) {
        return flow.result.length > i && flow.result[i] !== null;
    })
    NumericEditor {
        id: resultEditor
    }

    function measuredPosition(axis) {
        return Number(machinePoint[axis]);
    }

    function measuredWorkPosition(axis) {
        var state = client.state || {};
        var reference = (state.coordinates || {}).spindle;
        if (!reference || (state.status || {}).wcs !== routine.wcs)
            return Number(result[axis]);
        var angle = Number((state.wcsRotations || {})[routine.wcs] || 0) * Math.PI / 180;
        var dx = machinePoint[0] === null ? 0 : machinePoint[0] - reference.machinePosition[0];
        var dy = machinePoint[1] === null ? 0 : machinePoint[1] - reference.machinePosition[1];
        if (axis === 0)
            return reference.workPosition[0] + Math.cos(angle) * dx + Math.sin(angle) * dy;
        if (axis === 1)
            return reference.workPosition[1] - Math.sin(angle) * dx + Math.cos(angle) * dy;
        return reference.workPosition[2] + machinePoint[2] - reference.machinePosition[2];
    }

    function setOffset(axis, value) {
        var updated = offsets.slice();
        updated[axis] = value;
        offsets = updated;
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
    font.family: uiFont
    background: Rectangle {
        color: Theme.page
    }

    function showRoutine(value) {
        historical = false;
        routine = value;
        sourceVisible = false;
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
        wcsRequest.cancel();
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

    function showHistory(opened, details) {
        reviewRequest.cancel();
        zeroRequest.cancel();
        wcsRequest.cancel();
        resultEditor.cancel();
        historical = true;
        routine = Object.assign({}, opened.entry.config, {
            wcs: opened.result.wcs
        });
        safeZOffset = Number(routine.safeZOffset || 40);
        sourceVisible = false;
        failure = opened.entry.error || "";
        reviewID = "";
        program = [];
        logText = details;
        offsets = ((opened.entry.workZero || {}).offsets || [0, 0, 0]).slice();
        returning = false;
        positioning = false;
        acceptResult(opened.result, opened.canApply ? opened.entry.id : "");
        open();
    }

    function selectResultWcs(wcs) {
        if (!completedID || busy || phase !== "result")
            return;
        resultEditor.cancel();
        failure = "";
        var id = completedID;
        wcsRequest.send(historical ? "history.wcs" : "routine.wcs", {
            id: id,
            wcs: wcs
        }, function (reply) {
            if (!reply.ok || !reply.data) {
                failure = reply.error || "Could not select work coordinates";
                return;
            }
            routine = Object.assign({}, routine, {
                wcs: reply.data.wcs
            });
            acceptResult(reply.data, id);
            client.refresh();
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
        if (!completedID || busy || historical || returned || phase !== "result")
            return;
        resultEditor.cancel();
        var id = completedID;
        completedID = "";
        startMotion(id, "return");
    }

    function goToMeasured() {
        if (!completedID || busy || historical || positioned || phase !== "result")
            return;
        if (resultEditor.target)
            resultEditor.accept();
        if (resultEditor.target)
            return;
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
        if (action === "run")
            logText = "";
        failure = "";
        var terminal = false;
        var body = {
            id: id
        };
        if (positioning)
            body.safeZOffset = safeZOffset;
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
            if (!terminal) {
                phase = "failed";
                failure = error || "Routine connection ended without a confirmed result";
            }
        });
    }

    function zeroResult() {
        if (!completedID || busy)
            return;
        if (resultEditor.target)
            resultEditor.accept();
        if (resultEditor.target)
            return;
        var id = completedID;
        completedID = "";
        failure = "";
        zeroRequest.send(historical ? "history.zero" : "routine.zero", {
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
            workZeroSaved();
            appendLog("; G" + routine.wcs + " work zero confirmed");
            client.refresh();
        });
    }

    Component.onDestruction: if (activeRequest)
        activeRequest.abort()
    onClosed: {
        resultEditor.cancel();
        reviewRequest.cancel();
        zeroRequest.cancel();
        wcsRequest.cancel();
        if (activeRequest && phase === "running")
            activeRequest.abort();
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        PageHeader {
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.headerHeight
            visible: flow.showHeader
            title: flow.description
            detail: flow.phase === "failed" ? "Failed" : flow.phase === "result" ? "Complete" : flow.phase === "running" ? "Running" : flow.simulated ? "Simulation" : "Review"
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
                visible: flow.failure.length > 0 || flow.reviewing
                text: flow.reviewing ? "Preparing routine..." : flow.failure
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: Theme.groupSpacing

                Item {
                    visible: flow.phase !== "result" || flow.sourceVisible
                    Layout.fillWidth: true
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
                            background: Rectangle {
                                color: Theme.panel
                                radius: Theme.radius
                            }
                        }
                    }
                }

                ProbeResultView {
                    visible: flow.phase === "result" && !flow.sourceVisible
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    flow: flow
                    editor: resultEditor
                }
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: 12
                LabButton {
                    visible: flow.phase === "result"
                    text: flow.sourceVisible ? "Result" : flow.historical ? "Details" : "Log"
                    Layout.preferredWidth: 96
                    onClicked: {
                        resultEditor.cancel();
                        flow.sourceVisible = !flow.sourceVisible;
                        Qt.callLater(flow.scrollLogToEnd);
                    }
                }
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
                    enabled: !flow.busy && flow.completedID.length > 0
                    Layout.preferredWidth: 184
                    Layout.preferredHeight: 56
                    font.pixelSize: 20
                    primary: true
                    onClicked: flow.zeroResult()
                }
                LabButton {
                    text: "Return to start"
                    visible: flow.phase === "result" && !flow.historical && (flow.routine.family !== "inside" || flow.routine.z)
                    enabled: !flow.busy && !flow.returned && flow.completedID.length > 0
                    Layout.preferredWidth: 208
                    Layout.preferredHeight: 56
                    font.pixelSize: 20
                    onClicked: flow.returnToStart()
                }
                LabButton {
                    text: "Move to measured XY"
                    visible: flow.phase === "result" && !flow.historical && flow.routine.family === "inside" && !flow.routine.z
                    enabled: !flow.busy && !flow.positioned && flow.completedID.length > 0
                    Layout.preferredWidth: 244
                    Layout.preferredHeight: 56
                    font.pixelSize: 20
                    onClicked: flow.goToMeasured()
                }
                LabButton {
                    visible: flow.phase !== "running"
                    text: flow.phase === "review" ? "Proceed" : "Close"
                    enabled: !flow.busy && (flow.phase !== "review" || (flow.reviewID.length > 0 && !flow.reviewing))
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
