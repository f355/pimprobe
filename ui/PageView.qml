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

Pane {
    id: view
    property bool opened: false
    signal opening
    signal closed
    z: 100
    width: parent ? parent.width : 800
    height: parent ? parent.height : 480
    visible: opened
    padding: 0
    focus: opened

    function open() {
        var siblings = parent ? parent.children : [];
        z = 100;
        for (var i = 0; i < siblings.length; ++i)
            if (siblings[i] !== view && siblings[i].opened)
                z = Math.max(z, siblings[i].z + 1);
        opened = true;
        opening();
        forceActiveFocus();
    }
    function close() {
        opened = false;
        closed();
    }

    // Consume touches in empty parts of the page.
    property MouseArea inputBlocker: MouseArea {
        parent: view
        anchors.fill: parent
        z: -1
        acceptedButtons: Qt.AllButtons
    }
}
