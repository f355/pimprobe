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
import "controls"
import QtQuick.Controls
import QtQuick.Layouts

RowLayout {
    id: row
    required property ProbeSettings settings
    required property NumericEditor editor
    required property var definition
    property int fieldHeight: 56
    property int fieldWidth: 130
    property int labelSize: 20
    property int numberSize: 24
    spacing: 12

    Label {
        Layout.fillWidth: true
        text: I18n.tr(row.definition.label)
        color: Theme.text
        font.pixelSize: row.labelSize
        wrapMode: Text.WordWrap
    }
    Item {
        Layout.preferredWidth: 194
        Layout.minimumWidth: 194
        Layout.maximumWidth: 194
        Layout.preferredHeight: row.fieldHeight
        NumberField {
            id: input
            objectName: row.definition.key
            Accessible.name: I18n.tr(row.definition.label)
            editor: row.editor
            width: row.fieldWidth
            height: row.fieldHeight
            value: row.settings.values[row.definition.key] || 0
            minimum: (row.settings.schema[row.definition.key] || {}).minimum || 0
            maximum: (row.settings.schema[row.definition.key] || {}).maximum || 0
            onCommitted: function (value) {
                row.settings.setValue(row.definition.key, value);
            }
            font.pixelSize: row.numberSize
        }
        Label {
            anchors.left: input.right
            anchors.leftMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            text: I18n.tr(row.definition.unit || 'mm')
            color: Theme.textMuted
            font.pixelSize: 16
        }
    }
}
