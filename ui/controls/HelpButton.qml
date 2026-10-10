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

LabButton {
    id: button
    readonly property var controller: {
        var item = parent;
        while (item) {
            if (item.contextHelp !== undefined)
                return item.contextHelp;
            item = item.parent;
        }
        return null;
    }
    visible: controller !== null
    implicitWidth: 48
    implicitHeight: 48
    text: controller && controller.active ? "\u00d7" : "?"
    helpText: ""
    font.pixelSize: 25
    font.bold: true
    padding: 4
    Accessible.name: controller && controller.active ? I18n.tr("Exit help") : I18n.tr("Help")
    background: Rectangle {
        color: button.down ? Theme.pressed : button.controller && button.controller.active ? Theme.accentWash : Theme.panel
        border.color: button.controller && button.controller.active ? Theme.accentBright : Theme.divider
        border.width: 1
        radius: width / 2
    }
    onClicked: controller.active = !controller.active
    HelpTip { target: button; toggle: true }
}
