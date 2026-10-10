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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: selector
    required property string language
    signal selected(string language)
    spacing: 8
    HelpTip {
        title: I18n.tr("Language")
        text: I18n.tr("Choose the language for controls and help. The choice is saved for the next startup.")
    }
    Label {
        text: I18n.tr("Language")
        leftPadding: 8
        color: Theme.textMuted
        font.pixelSize: 18
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: 4
        Repeater {
            model: [{code:"en", name:"English"}, {code:"zh_CN", name:"简体中文"}, {code:"sv", name:"Svenska"}]
            LabButton {
                required property var modelData
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                text: modelData.name
                font.pixelSize: 18
                padding: 8
                selected: selector.language === modelData.code
                onClicked: selector.selected(modelData.code)
            }
        }
    }
}
