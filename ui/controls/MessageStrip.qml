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
import QtQuick.Controls

ScrollView {
    id: message
    property string text
    property color textColor: Theme.warning
    implicitHeight: Math.min(72, body.implicitHeight)
    contentWidth: availableWidth
    clip: true
    TextArea {
        id: body
        readOnly: true
        text: message.text
        color: message.textColor
        font.pixelSize: 18
        wrapMode: TextEdit.WordWrap
        padding: 0
        background: null
    }
}
