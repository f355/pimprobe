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

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

LabButton {
    id: control
    property bool extended: false
    property bool stateKnown: true
    property bool moving: false
    implicitWidth: 120
    implicitHeight: 56
    padding: 0
    background: null
    text: moving ? I18n.tr("Moving") : !stateKnown ? I18n.tr("Unknown") : extended ? I18n.tr("Extended") : I18n.tr("Retracted")
    Accessible.name: text
    contentItem: ColumnLayout {
        spacing: 0
        Repeater {
            model: ["Retracted", "Extended"]
            Rectangle {
                id: state
                required property int index
                required property string modelData
                readonly property bool active: control.stateKnown && !control.moving && control.extended === (index === 1)
                readonly property color stateColor: index === 0 ? Theme.danger : Theme.accentBright
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: 4
                color: active ? Qt.rgba(stateColor.r, stateColor.g, stateColor.b, 0.16) : "transparent"
                Label {
                    anchors.fill: parent
                    text: I18n.tr(state.modelData)
                    font.family: control.font.family
                    font.pixelSize: 18
                    font.weight: state.active ? Font.DemiBold : Font.Normal
                    color: state.stateColor
                    opacity: state.active ? 1 : 0.35
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }
    }
}
