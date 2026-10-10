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
    property var workPosition: []
    property var machinePosition: []
    property string monoFont: "monospace"
    helpTitle: I18n.tr("Coordinates")
    helpText: I18n.tr("Tap anywhere here to switch between probe ball and tool tip coordinates. Large numbers use the selected WCS; small numbers use G53. With an empty spindle, Tool still uses the last measured tool length offset.")
    text: probeSelected ? I18n.tr('Probe') : I18n.tr('Tool')
    implicitWidth: 548
    implicitHeight: 56
    padding: 4
    topPadding: 0
    bottomPadding: 0
    font.pixelSize: 18
    background: null
    Accessible.name: I18n.tr('Coordinate reference: %1', [text])
    onClicked: probeSelected = !probeSelected

    function coordinate(position, index) {
        return position && position.length > index ? Number(position[index]).toFixed(3) : "--.---";
    }

    contentItem: RowLayout {
        spacing: 4
        Repeater {
            model: ["X", "Y", "Z"]
            Rectangle {
                id: axis
                required property string modelData
                required property int index
                Layout.preferredWidth: 144
                Layout.fillHeight: true
                color: "transparent"
                border.color: Theme.divider
                radius: 4
                ColumnLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 6
                    anchors.rightMargin: 6
                    spacing: 0
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        spacing: 4
                        Label {
                            Layout.preferredWidth: 18
                            Layout.fillHeight: true
                            text: axis.modelData
                            color: Theme.text
                            font.family: control.monoFont
                            font.pixelSize: 22
                            verticalAlignment: Text.AlignBottom
                        }
                        Label {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            text: control.coordinate(control.workPosition, axis.index)
                            color: Theme.text
                            font.family: control.monoFont
                            font.pixelSize: 22
                            horizontalAlignment: Text.AlignRight
                            verticalAlignment: Text.AlignBottom
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        text: control.coordinate(control.machinePosition, axis.index)
                        color: Theme.textMuted
                        font.family: control.monoFont
                        font.pixelSize: 16
                        horizontalAlignment: Text.AlignRight
                        verticalAlignment: Text.AlignTop
                    }
                }
            }
        }
        ColumnLayout {
            Layout.preferredWidth: 96
            Layout.fillHeight: true
            spacing: 0
            Repeater {
                model: ["Probe", "Tool"]
                Rectangle {
                    required property string modelData
                    required property int index
                    readonly property bool active: control.probeSelected === (index === 0)
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    color: active ? Theme.accentWash : "transparent"
                    radius: 4
                    Label {
                        anchors.fill: parent
                        text: I18n.tr(parent.modelData)
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
}
