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

Rectangle {
    id: keypad

    signal keyPressed(string key)
    signal accepted

    color: Theme.page

    component KeyButton: LabButton {
        focusPolicy: Qt.NoFocus
        required property int gridColumn
        required property int gridRow
        required property string keyText
        property string keyValue: keyText
        property int columnSpan: 1
        property int rowSpan: 1
        property bool enterKey: false

        Layout.column: gridColumn
        Layout.row: gridRow
        Layout.columnSpan: columnSpan
        Layout.rowSpan: rowSpan
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.preferredWidth: 1
        Layout.preferredHeight: 1
        Layout.minimumWidth: 48
        Layout.minimumHeight: 48
        text: keyText
        font.pixelSize: keypad.height < 280 ? 28 : 40
        padding: keypad.height < 280 ? 2 : 10
        onClicked: {
            if (enterKey)
                keypad.accepted();
            else
                keypad.keyPressed(keyValue);
        }
    }

    GridLayout {
        anchors.fill: parent
        columns: 4
        rows: 4
        columnSpacing: 6
        rowSpacing: 6

        KeyButton {
            gridColumn: 0
            gridRow: 0
            keyText: "1"
        }
        KeyButton {
            gridColumn: 1
            gridRow: 0
            keyText: "2"
        }
        KeyButton {
            gridColumn: 2
            gridRow: 0
            keyText: "3"
        }
        KeyButton {
            gridColumn: 3
            gridRow: 0
            keyText: "\u2190"
            keyValue: "left"
        }

        KeyButton {
            gridColumn: 0
            gridRow: 1
            keyText: "4"
        }
        KeyButton {
            gridColumn: 1
            gridRow: 1
            keyText: "5"
        }
        KeyButton {
            gridColumn: 2
            gridRow: 1
            keyText: "6"
        }
        KeyButton {
            gridColumn: 3
            gridRow: 1
            keyText: "\u2192"
            keyValue: "right"
        }

        KeyButton {
            gridColumn: 0
            gridRow: 2
            keyText: "7"
        }
        KeyButton {
            gridColumn: 1
            gridRow: 2
            keyText: "8"
        }
        KeyButton {
            gridColumn: 2
            gridRow: 2
            keyText: "9"
        }
        KeyButton {
            gridColumn: 3
            gridRow: 2
            keyText: "\u232B"
            keyValue: "backspace"
        }

        KeyButton {
            gridColumn: 0
            gridRow: 3
            keyText: "\u00B1"
            keyValue: "sign"
        }
        KeyButton {
            gridColumn: 1
            gridRow: 3
            keyText: "0"
        }
        KeyButton {
            gridColumn: 2
            gridRow: 3
            keyText: "."
        }
        KeyButton {
            gridColumn: 3
            gridRow: 3
            keyText: "\u21B5"
            enterKey: true
            selected: true
        }
    }
}
