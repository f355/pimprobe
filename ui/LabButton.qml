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
    property color textColor: selected ? Theme.accentBright : Theme.text

    implicitHeight: 48
    padding: 8
    font.pixelSize: 20
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
        border.width: control.selected ? 2 : 0
        border.color: Theme.accentBright
        radius: 10
        opacity: control.enabled ? 1 : 0.55
    }
}
