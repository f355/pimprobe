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
import "controls"

Popup {
    id: picker
    property int currentWcs: 54
    property string title: "Work coordinates"
    signal selected(int wcs)
    Component.onCompleted: if ("popupType" in picker) picker.popupType = Popup.Item
    parent: Overlay.overlay
    width: parent ? parent.width : 800
    height: parent ? parent.height : 480
    padding: 0
    modal: true
    closePolicy: Popup.NoAutoClose
    background: Rectangle { color: Theme.page }
    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        PageHeader {
            Layout.fillWidth: true
            title: picker.title
            onBack: picker.close()
        }
        GridLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 16
            columns: 3
            rowSpacing: 12
            columnSpacing: 12
            Repeater {
                model: [54, 55, 56, 57, 58, 59]
                LabButton {
                    required property int modelData
                    text: "G" + modelData
                    selected: picker.currentWcs === modelData
                    font.pixelSize: 32
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.preferredWidth: 1
                    Layout.preferredHeight: 1
                    onClicked: {
                        picker.selected(modelData);
                        picker.close();
                    }
                }
            }
        }
    }
}
