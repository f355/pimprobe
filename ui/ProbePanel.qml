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
import QtQuick.Layouts

Item {
    id: panel
    required property ProbeSettings settings
    required property NumericEditor editor
    required property var parameters
    property bool roomy: false
    property bool setup: false
    property Component footer
    default property alias workContent: workArea.data

    property Item layout: RowLayout {
        parent: panel
        anchors.fill: parent
        anchors.margins: 14
        spacing: 18
        Item {
            id: workArea
            Layout.preferredWidth: 340
            Layout.fillHeight: true
            Rectangle {
                anchors.fill: parent
                color: Theme.panel
                radius: 12
                z: -1
            }
        }
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Rectangle {
                anchors.fill: parent
                color: Theme.panel
                radius: 12
            }
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 14
                spacing: panel.roomy ? 14 : panel.setup ? 8 : 6
                Repeater {
                    model: panel.parameters
                    ParameterRow {
                        required property var modelData
                        Layout.fillWidth: true
                        settings: panel.settings
                        editor: panel.editor
                        definition: modelData
                        fieldHeight: panel.roomy ? 62 : panel.setup ? 48 : 50
                        fieldWidth: panel.setup ? 100 : 130
                        labelSize: panel.roomy || panel.setup ? 18 : 17
                        numberSize: panel.roomy ? 24 : panel.setup ? 23 : 22
                    }
                }
                Item {
                    Layout.fillHeight: true
                }
                Loader {
                    Layout.fillWidth: true
                    sourceComponent: panel.footer
                    visible: item !== null
                }
            }
        }
    }
}
