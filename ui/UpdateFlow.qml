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
    property string phase: "loading"
    property bool development: false
    property string currentVersion: ""
    property var available: null
    property string errorText: ""
    property string operationId: ""
    property int statusTicks: 0

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

    ServiceRequest { id: checkRequest }
    ServiceRequest { id: installRequest }
    ServiceRequest { id: statusRequest }
    Timer {
        id: statusTimer
        interval: 1000
        repeat: true
        onTriggered: flow.pollStatus()
    }

    function show() {
        development = false;
        open();
        reload();
    }
    function reload() {
        checkRequest.cancel();
        phase = "loading";
        errorText = "";
        available = null;
        operationId = "";
        checkRequest.send("GET", serviceUrl + "/updates/check?development=" + (development ? "true" : "false"), null, function(reply) {
            if (!reply.ok || !reply.data) {
                phase = "error";
                errorText = reply.error || "Invalid update response";
                return;
            }
            currentVersion = reply.data.currentVersion || "unknown";
            available = reply.data.available || null;
            phase = available ? "available" : "current";
        });
    }
    function install() {
        if (!available || installRequest.pending) return;
        phase = "installing";
        errorText = "";
        statusTicks = 0;
        installRequest.send("POST", serviceUrl + "/updates/install", { token: available.token }, function(reply) {
            if (!reply.ok || !reply.data || !reply.data.operationId) {
                phase = "error";
                errorText = reply.error || "The updater did not return an operation ID";
                return;
            }
            operationId = reply.data.operationId;
            statusTimer.start();
        });
    }
    function pollStatus() {
        if (!operationId) return;
        if (++statusTicks > 90) {
            statusRequest.cancel();
            statusTimer.stop();
            phase = "error";
            errorText = "The installer did not report completion";
            return;
        }
        if (statusRequest.pending) return;
        statusRequest.send("GET", serviceUrl + "/updates/status?id=" + operationId, null, function(reply) {
            if (!reply.ok || !reply.data) return;
            if (reply.data.state === "failed") {
                statusTimer.stop();
                phase = "error";
                errorText = reply.data.message || "Installation failed";
            }
        });
    }
    onClosed: {
        checkRequest.cancel();
        installRequest.cancel();
        statusRequest.cancel();
        statusTimer.stop();
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
                    enabled: flow.phase !== "installing"
                    onClicked: flow.close()
                }
                Label {
                    Layout.fillWidth: true
                    text: "Software update"
                    color: Theme.text
                    font.pixelSize: 24
                }
                ProbeSwitch {
                    Layout.preferredWidth: 270
                    Layout.preferredHeight: 50
                    Layout.rightMargin: 16
                    text: "Include development releases"
                    checked: flow.development
                    enabled: flow.phase !== "installing"
                    onClicked: {
                        flow.development = !flow.development;
                        flow.reload();
                    }
                }
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 18
            spacing: 12
            Label {
                Layout.fillWidth: true
                text: "Installed: " + flow.currentVersion
                color: Theme.textMuted
                font.pixelSize: 16
            }
            Label {
                Layout.fillWidth: true
                Layout.fillHeight: flow.phase !== "available"
                visible: flow.phase !== "available"
                text: flow.phase === "loading" ? "Checking for updates..."
                    : flow.phase === "current" ? "You have the latest release."
                    : flow.phase === "installing" ? "Installing update. The interface will restart."
                    : flow.errorText
                color: flow.phase === "error" ? Theme.danger : Theme.text
                font.pixelSize: 22
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
            Label {
                Layout.fillWidth: true
                visible: flow.phase === "available"
                text: flow.available ? (flow.available.name || flow.available.version) : ""
                color: Theme.text
                font.pixelSize: 24
                font.bold: true
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                visible: flow.phase === "available"
                clip: true
                TextArea {
                    readOnly: true
                    textFormat: TextEdit.MarkdownText
                    text: flow.available ? flow.available.notes : ""
                    color: Theme.text
                    font.pixelSize: 18
                    wrapMode: TextEdit.WordWrap
                    background: Rectangle { color: Theme.panel; radius: 8 }
                }
            }
            RowLayout {
                Layout.alignment: Qt.AlignRight
                visible: flow.phase === "available" || flow.phase === "error"
                LabButton {
                    visible: flow.phase === "error"
                    text: "Try again"
                    font.pixelSize: 19
                    onClicked: flow.reload()
                }
                LabButton {
                    visible: flow.phase === "available"
                    text: "Install update"
                    primary: true
                    font.pixelSize: 19
                    onClicked: installConfirmation.open()
                }
            }
        }
    }

    Dialog {
        id: installConfirmation
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: 590
        height: 220
        modal: true
        title: "Install update"
        standardButtons: Dialog.NoButton
        closePolicy: Popup.NoAutoClose
        background: Rectangle {
            color: Theme.panelRaised
            border.color: Theme.divider
            radius: 12
        }
        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 18
            spacing: 16
            Label {
                Layout.fillWidth: true
                Layout.fillHeight: true
                text: "Make sure the machine is idle and the spindle is stopped. Install this update now?"
                color: Theme.text
                font.pixelSize: 21
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 12
                LabButton {
                    text: "Cancel"
                    font.pixelSize: 20
                    onClicked: installConfirmation.close()
                }
                LabButton {
                    text: "Install"
                    font.pixelSize: 20
                    primary: true
                    onClicked: {
                        installConfirmation.close();
                        flow.install();
                    }
                }
            }
        }
    }
}
