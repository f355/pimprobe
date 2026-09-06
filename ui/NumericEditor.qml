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

QtObject {
    id: editor
    property var target: null
    signal accepted

    property Connections inputMethodGuard: Connections {
        target: Qt.inputMethod
        function onVisibleChanged() {
            if (editor.target && Qt.inputMethod.visible)
                Qt.inputMethod.hide();
        }
    }

    function begin(field) {
        if (target && target !== field)
            cancel();
        target = field;
        field.selectAll();
        Qt.inputMethod.hide();
    }

    function cancel() {
        if (target) {
            target.text = String(target.value);
            target.focus = false;
        }
        target = null;
    }

    function typeKey(key) {
        if (!target)
            return;
        var text = target.text;
        if (key === "sign") {
            var negative = text[0] === "-";
            var position = target.cursorPosition;
            target.text = negative ? text.slice(1) : "-" + text;
            target.cursorPosition = Math.max(0, position + (negative ? -1 : 1));
            return;
        }
        var start = target.selectionStart;
        var end = target.selectionEnd;
        var selected = start !== end;
        var cursor = target.cursorPosition;
        if (key === "left" || key === "right") {
            cursor = selected ? (key === "left" ? start : end)
                              : Math.max(0, Math.min(text.length, cursor + (key === "left" ? -1 : 1)));
            target.deselect();
            target.cursorPosition = cursor;
            return;
        }
        if (!selected)
            start = end = cursor;
        var insertion = key;
        if (key === "backspace") {
            if (!selected)
                start = Math.max(0, cursor - 1);
            insertion = "";
        } else if (key === ".") {
            var remaining = text.slice(0, start) + text.slice(end);
            if (remaining.indexOf(".") !== -1)
                return;
            insertion = remaining.length === 0 ? "0." : ".";
        }
        cursor = start + insertion.length;
        target.text = text.slice(0, start) + insertion + text.slice(end);
        target.cursorPosition = cursor;
    }

    function accept() {
        if (!target || !target.commit())
            return;
        target.focus = false;
        target = null;
        accepted();
    }
}
