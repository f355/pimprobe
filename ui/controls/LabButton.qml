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

Button {
    id: control
    property bool primary: false
    property bool selected: false
    property bool notification: false
    property color textColor: selected ? Theme.accentBright : primary ? Theme.primaryText : Theme.text

    implicitHeight: Theme.buttonHeight
    implicitWidth: Math.max(56, contentItem.implicitWidth + 32)
    padding: 12
    font.pixelSize: Theme.textSize
    font.weight: Font.Normal

    contentItem: Label {
        text: control.text
        font: control.font
        color: control.enabled ? control.textColor : Theme.textMuted
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    background: Rectangle {
        color: !control.enabled ? Theme.panelRaised
            : control.down ? (control.primary ? Theme.accentPressed : Theme.pressed)
            : control.selected ? Theme.accentWash
            : control.primary ? Theme.accent : Theme.control
        border.width: control.selected || control.activeFocus ? 2 : 1
        border.color: control.selected || control.activeFocus ? Theme.accentBright : Theme.divider
        radius: Theme.radius
        opacity: control.enabled ? 1 : 0.55
    }
    NotificationDot {
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.margins: 8
        visible: control.notification
    }
}
