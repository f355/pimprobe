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

TextField {
    id: field
    required property NumericEditor editor
    required property real minimum
    required property real maximum
    property real value: minimum
    signal committed(real value)
    text: String(value)
    color: Theme.text
    selectedTextColor: Theme.text
    selectionColor: Theme.accent
    padding: 10
    onValueChanged: text = String(value)

    function commit() {
        if (!acceptableInput)
            return false;
        committed(Number(text));
        return true;
    }

    onAccepted: editor.accept()
    validator: DoubleValidator {
        locale: "C"
        bottom: field.minimum
        top: field.maximum
    }
    horizontalAlignment: TextInput.AlignRight
    inputMethodHints: Qt.ImhFormattedNumbersOnly
    background: Rectangle {
        color: Theme.field
        border.color: field.activeFocus ? Theme.accentBright : Theme.fieldBorder
        border.width: field.activeFocus ? 2 : 1
        radius: 8
    }
    onActiveFocusChanged: {
        if (activeFocus)
            editor.begin(field)
        else
            deselect()
    }

    // First tap selects the value; subsequent taps position the caret.
    MouseArea {
        anchors.fill: parent
        function placeCursor(mouse) {
            var editing = field.activeFocus && field.editor.target === field;
            field.forceActiveFocus();
            if (editing) {
                field.deselect();
                field.cursorPosition = field.positionAt(mouse.x, mouse.y);
            } else {
                field.editor.begin(field);
            }
            Qt.inputMethod.hide();
        }
        onPressed: function(mouse) { placeCursor(mouse); }
        onDoubleClicked: function(mouse) { placeCursor(mouse); }
    }

}
