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
    signal closed
    signal repeatabilityRequested

    Column {
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.margins: 14
        spacing: 14

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
            text: "Probe repeatability"
            width: parent.width
            height: 64
            font.family: utilitiesPanel.uiFont
            font.pixelSize: 20
            onClicked: utilitiesPanel.repeatabilityRequested()
        }
    }
}
