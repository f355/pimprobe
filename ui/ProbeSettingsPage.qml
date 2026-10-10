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
import "controls"

PageView {
    id: page
    required property ProbeSettings settings
    property bool showHeader: true
    property bool editable: true
    property Component settingsContribution
    signal utilitiesRequested
    function cancelEdit() { editor.cancel(); }
    onClosed: cancelEdit()
    NumericEditor { id: editor }
    background: Rectangle { color: Theme.page }
    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        PageHeader {
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.headerHeight
            visible: page.showHeader
            title: I18n.tr("Probe settings")
            uiFont: page.font.family
            onBack: page.close()
        }
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            ProbeSetupPanel {
                anchors.fill: parent
                enabled: page.editable
                opacity: page.editable ? 1 : 0.35
                settings: page.settings
                editor: editor
                settingsContribution: page.settingsContribution
                onUtilitiesRequested: {
                    page.cancelEdit();
                    page.utilitiesRequested();
                }
            }
            NumericKeypad {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.margins: Theme.margin
                width: Theme.columnWidth
                visible: editor.target !== null
                onKeyPressed: function(key) { editor.typeKey(key); }
                onAccepted: editor.accept()
            }
        }
    }
}
