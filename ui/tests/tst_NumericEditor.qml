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
import QtTest
import ".."

TestCase {
    name: "NumericEditor"
    when: windowShown
    width: 400
    height: 200
    visible: true
    NumericEditor { id: editor }
    NumberField {
        id: field
        editor: editor
        minimum: 0.1
        maximum: 100
        onCommitted: function(value) { field.value = value; }
        width: 130
        height: 62
    }
    SignalSpy { id: accepted; target: editor; signalName: "accepted" }
    SignalSpy { id: committed; target: field; signalName: "committed" }
    function init() {
        field.minimum = 0.1
        field.value = 12.5
        field.text = "12.5"
        field.forceActiveFocus()
        editor.begin(field)
        accepted.clear()
        committed.clear()
    }
    function test_replace_and_accept() {
        editor.typeKey("3")
        editor.typeKey(".")
        editor.typeKey("5")
        compare(field.text, "3.5")
        compare(field.value, 12.5)
        verify(field.acceptableInput, "text=" + field.text + " locale=" + field.validator.locale)
        editor.accept()
        compare(editor.target, null)
        compare(accepted.count, 1)
        compare(field.value, 3.5)
        compare(committed.count, 1)
    }
    function test_signed_offset_and_positive_only_validation() {
        editor.typeKey("2")
        editor.typeKey("sign")
        compare(field.text, "-2")
        editor.accept()
        compare(accepted.count, 0)
        field.minimum = -1000
        editor.accept()
        compare(field.value, -2)
        compare(accepted.count, 1)
        editor.begin(field)
        editor.typeKey("sign")
        compare(field.text, "2")
        editor.accept()
        compare(field.value, 2)
    }
    function test_commit_without_focus() {
        editor.typeKey("5")
        editor.typeKey("0")
        field.focus = false
        compare(field.value, 12.5)
        editor.accept()
        compare(field.value, 50)
        compare(committed.count, 1)
        editor.accept()
        compare(committed.count, 1)
    }
    function test_focus_loss_clears_selection() {
        compare(field.selectedText, "12.5")
        field.focus = false
        compare(field.selectedText, "")
        compare(field.text, "12.5")
    }
    function test_cursor_and_backspace() {
        editor.typeKey("right")
        editor.typeKey("left")
        editor.typeKey("left")
        editor.typeKey("backspace")
        compare(field.text, "1.5")
        editor.typeKey("7")
        compare(field.text, "17.5")
        editor.typeKey("right")
        compare(field.cursorPosition, 3)
    }
    function test_visible_selection_and_arrows() {
        compare(field.selectedText, "12.5")
        editor.typeKey("left")
        compare(field.selectedText, "")
        compare(field.cursorPosition, 0)
        editor.typeKey("7")
        compare(field.text, "712.5")
        editor.begin(field)
        editor.typeKey("right")
        compare(field.cursorPosition, 5)
        compare(field.selectedText, "")
    }
    function test_tap_again_places_cursor() {
        editor.cancel()
        mouseClick(field, field.width / 2, field.height / 2)
        compare(field.selectedText, "12.5")
        var caret = field.positionToRectangle(1)
        mouseClick(field, caret.x + field.leftPadding + 1, field.height / 2)
        compare(field.selectedText, "")
        compare(field.cursorPosition, 1)
        editor.typeKey("7")
        compare(field.text, "172.5")
    }
    function test_keypad_uses_native_selection_and_cursor() {
        field.select(1, 2)
        editor.typeKey("8")
        compare(field.text, "18.5")
        field.cursorPosition = 0
        editor.typeKey("9")
        compare(field.text, "918.5")
    }
    function test_touch_selection_and_insertion() {
        editor.cancel()
        touchEvent(field).press(0, field, 65, 31).commit()
        touchEvent(field).release(0, field, 65, 31).commit()
        compare(field.selectedText, "12.5")
        var caret = field.positionToRectangle(2)
        var x = caret.x + field.leftPadding + 1
        touchEvent(field).press(0, field, x, 31).commit()
        touchEvent(field).release(0, field, x, 31).commit()
        compare(field.selectedText, "")
        compare(field.cursorPosition, 2)
        editor.typeKey("9")
        compare(field.text, "129.5")
    }
    function test_physical_keyboard_and_keypad_share_selection() {
        field.forceActiveFocus()
        editor.begin(field)
        keyClick(Qt.Key_3)
        compare(field.text, "3")
        editor.typeKey("5")
        compare(field.text, "35")
    }
    function test_decimal_and_invalid_input() {
        editor.typeKey(".")
        compare(field.text, "0.")
        editor.typeKey(".")
        compare(field.text, "0.")
        editor.accept()
        compare(editor.target, field)
        compare(accepted.count, 0)
        editor.typeKey("5")
        editor.accept()
        compare(accepted.count, 1)
    }
}
