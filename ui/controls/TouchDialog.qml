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

Dialog {
    id: dialog
    property string message
    property string errorText
    property string acceptText: I18n.tr('Apply')
    property string alternateText
    property bool busy: false
    property bool closeOnAccept: true
    property bool destructive: false
    signal alternate()
    Component.onCompleted: if ("popupType" in dialog) dialog.popupType = Popup.Item
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(alternateText.length ? 736 : 640, parent ? parent.width - 32 : 736)
    height: 280
    padding: 16
    modal: true
    closePolicy: Popup.NoAutoClose
    standardButtons: Dialog.NoButton
    header: null
    footer: null
    Overlay.modal: Rectangle { color: "#99000000" }
    background: Rectangle {
        color: Theme.panel
        border.color: Theme.divider
        radius: Theme.radius
    }
    ColumnLayout {
        anchors.fill: parent
        spacing: 12
        Label {
            text: dialog.title
            Layout.fillWidth: true
            color: Theme.text
            font.pixelSize: 24
            horizontalAlignment: Text.AlignHCenter
        }
        ScrollView {
            id: messageScroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth
            clip: true
            TextArea {
                height: Math.max(implicitHeight, messageScroll.availableHeight)
                readOnly: true
                text: dialog.message
                color: Theme.text
                font.pixelSize: 20
                wrapMode: TextEdit.WordWrap
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: TextEdit.AlignVCenter
                padding: 0
                background: null
            }
        }
        Label {
            visible: text.length > 0
            text: I18n.tr(dialog.errorText)
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            font.pixelSize: 18
            color: Theme.danger
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 12
            LabButton {
                text: I18n.tr('Cancel')
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                Layout.preferredHeight: 80
                font.pixelSize: 22
                enabled: !dialog.busy
                onClicked: dialog.reject()
            }
            LabButton {
                visible: dialog.alternateText.length > 0
                text: dialog.alternateText
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                Layout.preferredHeight: 80
                font.pixelSize: 22
                enabled: !dialog.busy
                onClicked: dialog.alternate()
            }
            LabButton {
                text: dialog.acceptText
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                Layout.preferredHeight: 80
                font.pixelSize: 22
                primary: !dialog.destructive
                textColor: dialog.destructive ? Theme.danger : Theme.primaryText
                enabled: !dialog.busy
                onClicked: dialog.closeOnAccept ? dialog.accept() : dialog.accepted()
            }
        }
    }
}
