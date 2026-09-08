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

Switch {
    id: control
    spacing: 10

    indicator: Item {
        implicitWidth: 48
        implicitHeight: 28
        x: control.leftPadding
        y: parent.height / 2 - height / 2

        Rectangle {
            anchors.fill: parent
            radius: height / 2
            color: control.checked ? Theme.accent : Theme.fieldBorder
            opacity: control.enabled ? 1 : 0.55
        }
        Rectangle {
            width: 22
            height: 22
            radius: 11
            y: 3
            x: control.checked ? parent.width - width - 3 : 3
            color: Theme.text
        }
    }
}
