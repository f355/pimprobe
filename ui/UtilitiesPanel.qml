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
import QtQuick.Layouts

Item {
    id: utilitiesPanel

    property string uiFont: "sans-serif"
    property string serviceUrl: "http://127.0.0.1:8137/api/v1"
    property string message: ""
    signal closed
    signal repeatabilityRequested
    signal historyRequested

    ServiceRequest { id: exportRequest }
    ServiceRequest { id: clearRequest }

    function exportLogs() {
        message = "";
        exportRequest.send("POST", serviceUrl + "/logs/export", null, function(reply) {
            message = reply.ok && reply.data && reply.data.relativePath
                ? "Saved to USB: /" + reply.data.relativePath
                : reply.error || "Could not export logs";
        });
    }

    function clearLogs() {
        clearConfirmation.close();
        message = "";
        clearRequest.send("POST", serviceUrl + "/logs/clear", null, function(reply) {
            message = reply.ok ? "Logs cleared" : reply.error || "Could not clear logs";
        });
    }

    onVisibleChanged: if (!visible) {
        exportRequest.cancel();
        clearRequest.cancel();
    }

    Column {
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: repeatabilityButton.top
        anchors.margins: 14
        spacing: 14
        clip: true

        RowLayout {
            width: parent.width
            height: 56
            spacing: 12

            BackButton {
                Layout.preferredWidth: 70
                Layout.fillHeight: true
                onClicked: utilitiesPanel.closed()
            }

            Label {
                Layout.fillWidth: true
                text: "Utilities"
                color: Theme.text
                font.family: utilitiesPanel.uiFont
                font.pixelSize: 25
                verticalAlignment: Text.AlignVCenter
            }
        }

        LabButton {
            text: "Probe history"
            width: parent.width
            height: 64
            font.family: utilitiesPanel.uiFont
            font.pixelSize: 20
            onClicked: utilitiesPanel.historyRequested()
        }
        LabButton {
            text: "Export logs"
            width: parent.width
            height: 64
            enabled: !exportRequest.pending && !clearRequest.pending
            font.family: utilitiesPanel.uiFont
            font.pixelSize: 20
            onClicked: utilitiesPanel.exportLogs()
        }
        LabButton {
            text: "Clear logs"
            width: parent.width
            height: 64
            enabled: !exportRequest.pending && !clearRequest.pending
            font.family: utilitiesPanel.uiFont
            font.pixelSize: 20
            onClicked: clearConfirmation.open()
        }
        Label {
            width: parent.width
            text: utilitiesPanel.message
            color: utilitiesPanel.message.indexOf("Saved to USB:") === 0 || utilitiesPanel.message === "Logs cleared"
                ? Theme.accentBright : Theme.warning
            font.pixelSize: 17
            wrapMode: Text.WordWrap
        }
    }

    LabButton {
        id: repeatabilityButton
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 14
        height: 64
        text: "Probe repeatability"
        font.family: utilitiesPanel.uiFont
        font.pixelSize: 20
        onClicked: utilitiesPanel.repeatabilityRequested()
    }

    Popup {
        id: clearConfirmation
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: 540
        height: 200
        padding: 18
        modal: true
        closePolicy: Popup.NoAutoClose
        background: Rectangle { color: Theme.panel; radius: 6 }
        ColumnLayout {
            anchors.fill: parent
            spacing: 12
            Label {
                Layout.fillWidth: true
                text: "Clear probing logs?"
                color: Theme.text
                font.pixelSize: 24
                horizontalAlignment: Text.AlignHCenter
            }
            Label {
                Layout.fillWidth: true
                text: "Probe history and diagnostic traces will be removed."
                color: Theme.text
                font.pixelSize: 18
                horizontalAlignment: Text.AlignHCenter
            }
            Item { Layout.fillHeight: true }
            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 16
                LabButton { text: "Cancel"; Layout.preferredWidth: 160; onClicked: clearConfirmation.close() }
                LabButton { text: "Clear logs"; Layout.preferredWidth: 160; onClicked: utilitiesPanel.clearLogs() }
            }
        }
    }
}
