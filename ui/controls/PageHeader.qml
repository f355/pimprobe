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
import QtQuick.Layouts

Rectangle {
    id: header
    property string title
    property string detail
    property string uiFont: "sans-serif"
    property bool backEnabled: true
    signal back()
    implicitHeight: Theme.headerHeight
    color: Theme.header

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 8
        anchors.rightMargin: Theme.margin
        spacing: 12
        BackButton {
            Layout.preferredWidth: 48
            Layout.fillHeight: true
            enabled: header.backEnabled
            onClicked: header.back()
        }
        Label {
            Layout.fillWidth: true
            text: header.title
            font.family: header.uiFont
            font.pixelSize: 24
            color: Theme.text
            elide: Text.ElideRight
        }
        Label {
            visible: text.length > 0
            text: header.detail
            font.family: header.uiFont
            font.pixelSize: 18
            color: Theme.textMuted
        }
    }
}
