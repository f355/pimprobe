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
import QtQuick.Layouts
import "controls"
import "ProbePages.js" as Pages

ProbePanel {
    id: panel
    property Component settingsContribution
    signal utilitiesRequested
    parameters: Pages.setup
    ColumnLayout {
        anchors.fill: parent
        spacing: 12
        LabButton {
            text: I18n.tr("Utilities")
            Layout.fillWidth: true
            Layout.preferredHeight: 56
            font.pixelSize: 20
            onClicked: panel.utilitiesRequested()
        }
        Loader {
            Layout.fillWidth: true
            sourceComponent: panel.settingsContribution
        }
        LanguageSelector {
            Layout.fillWidth: true
            language: I18n.language
            enabled: panel.settings.loaded && !panel.settings.saving
            onSelected: function(language) {
                panel.settings.setValue("language", language);
            }
        }
        Item { Layout.fillHeight: true }
    }
}
