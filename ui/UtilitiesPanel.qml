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
import "controls"
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: utilitiesPanel

    property string uiFont: "sans-serif"
    required property var client
    property bool showHeader: true
    property string message: ""
    signal closed
    signal repeatabilityRequested
    signal historyRequested

    ServiceRequest { client: utilitiesPanel.client; id: exportRequest }
    ServiceRequest { client: utilitiesPanel.client; id: clearRequest }

    function exportLogs() {
        message = "";
        exportRequest.send("history.export", null, function(reply) {
            message = reply.ok && reply.data && reply.data.relativePath
                ? "Saved: " + reply.data.relativePath
                : reply.error || "Could not export logs";
        });
    }

    function clearLogs() {
        clearConfirmation.close();
        message = "";
        clearRequest.send("history.clear", null, function(reply) {
            message = reply.ok ? "Logs cleared" : reply.error || "Could not clear logs";
        });
    }

    onVisibleChanged: if (!visible) {
        clearConfirmation.close();
        exportRequest.cancel();
        clearRequest.cancel();
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        PageHeader {
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.headerHeight
            visible: utilitiesPanel.showHeader
            title: "Utilities"
            uiFont: utilitiesPanel.uiFont
            onBack: utilitiesPanel.closed()
        }
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 16
            spacing: 16
            LabButton {
                text: "Probe history"
                Layout.fillWidth: true
                Layout.preferredHeight: 80
                font.pixelSize: 24
                onClicked: utilitiesPanel.historyRequested()
            }
            GridLayout {
                Layout.fillWidth: true
                columns: 2
                rowSpacing: 12
                columnSpacing: 12
                LabButton {
                    text: "Export logs"
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    Layout.preferredHeight: 56
                    enabled: !exportRequest.pending && !clearRequest.pending
                    onClicked: utilitiesPanel.exportLogs()
                }
                LabButton {
                    text: "Clear logs"
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    Layout.preferredHeight: 56
                    textColor: Theme.danger
                    enabled: !exportRequest.pending && !clearRequest.pending
                    onClicked: clearConfirmation.open()
                }
            }
            Label {
                Layout.fillWidth: true
                text: utilitiesPanel.message
                color: utilitiesPanel.message.indexOf("Saved:") === 0 || utilitiesPanel.message === "Logs cleared"
                    ? Theme.accentBright : Theme.warning
                font.pixelSize: 18
                wrapMode: Text.WordWrap
            }
            Item { Layout.fillHeight: true }
            LabButton {
                text: "Probe repeatability"
                Layout.preferredWidth: 280
                Layout.preferredHeight: 56
                onClicked: utilitiesPanel.repeatabilityRequested()
            }
        }
    }

    TouchDialog {
        id: clearConfirmation
        title: "Clear probing logs?"
        message: "Probe history and diagnostic traces will be removed."
        acceptText: "Clear logs"
        destructive: true
        font.family: utilitiesPanel.uiFont
        onAccepted: utilitiesPanel.clearLogs()
    }

}
