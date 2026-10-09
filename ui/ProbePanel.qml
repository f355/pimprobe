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

Item {
    id: panel
    required property ProbeSettings settings
    required property NumericEditor editor
    required property var parameters
    property Component footer
    default property alias workContent: workArea.data

    property Item layout: RowLayout {
        parent: panel
        anchors.fill: parent
        anchors.margins: Theme.margin
        spacing: Theme.groupSpacing
        Item {
            id: workArea
            Layout.preferredWidth: Theme.columnWidth
            Layout.fillHeight: true
        }
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Flickable {
                id: parameterScroll
                anchors.fill: parent
                clip: true
                contentWidth: width
                contentHeight: Math.max(height, parameterRows.implicitHeight)
                flickableDirection: Flickable.VerticalFlick
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: ScrollBar {
                    id: parameterBar
                    visible: parameterScroll.contentHeight > parameterScroll.height + 0.5
                    policy: ScrollBar.AlwaysOn
                }
                ColumnLayout {
                    id: parameterRows
                    width: parameterScroll.width - (parameterBar.visible ? 12 : 0)
                    spacing: 12
                    Repeater {
                        model: panel.parameters
                        ParameterRow {
                            required property var modelData
                            Layout.fillWidth: true
                            settings: panel.settings
                            editor: panel.editor
                            definition: modelData
                            fieldHeight: 56
                            fieldWidth: 112
                            labelSize: 20
                            numberSize: 24
                        }
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
}
