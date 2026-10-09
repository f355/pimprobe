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

LabButton {
    id: control
    property bool probeSelected: true
    text: probeSelected ? "Probe" : "Tool"
    implicitWidth: 96
    implicitHeight: 56
    padding: 4
    font.pixelSize: 18
    background: null
    Accessible.name: "Coordinate reference: " + text
    onClicked: probeSelected = !probeSelected

    contentItem: ColumnLayout {
        spacing: 0
        Repeater {
            model: ["Probe", "Tool"]
            Rectangle {
                required property string modelData
                required property int index
                readonly property bool active: control.probeSelected === (index === 0)
                implicitWidth: label.implicitWidth + 8
                implicitHeight: label.implicitHeight
                Layout.fillWidth: true
                Layout.fillHeight: true
                color: active ? Theme.accentWash : "transparent"
                radius: 4
                Label {
                    id: label
                    anchors.fill: parent
                    text: parent.modelData
                    color: parent.active ? Theme.accentBright : Theme.textMuted
                    font.family: control.font.family
                    font.pixelSize: control.font.pixelSize
                    font.weight: parent.active ? Font.DemiBold : Font.Normal
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }
    }
}
