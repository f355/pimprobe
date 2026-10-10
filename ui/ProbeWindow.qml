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
import "controls"
import QtQuick.Controls
import QtQuick.Layouts
import "client"

ApplicationWindow {
    id: window
    property string serviceUrl: "http://127.0.0.1:8137/api/v1"
    readonly property alias page: probePage
    visible: true
    color: Theme.page
    title: I18n.tr('Probing')
    palette.window: Theme.page
    palette.base: Theme.field
    palette.text: Theme.text
    palette.windowText: Theme.text
    palette.button: Theme.control
    palette.buttonText: Theme.text
    palette.highlight: Theme.accent
    palette.highlightedText: Theme.primaryText

    HttpClient {
        id: http
        serviceUrl: window.serviceUrl
    }
    FontLoader {
        id: geistRegular
        source: "fonts/Geist-Regular.otf"
    }
    FontLoader {
        source: "fonts/Geist-Medium.otf"
    }
    FontLoader {
        source: "fonts/Geist-Bold.otf"
    }
    readonly property var availableFonts: Qt.fontFamilies()
    font.family: geistRegular.name.length > 0 ? geistRegular.name : availableFonts.indexOf("DejaVu Sans") !== -1 ? "DejaVu Sans" : Qt.platform.os === "osx" ? "Helvetica Neue" : "sans-serif"

    ProbePage {
        id: probePage
        anchors.fill: parent
        client: http
        updateAvailable: updateDialog.updateAvailable
        uiFontFamily: window.font.family
        monoFontFamily: window.availableFonts.indexOf("DejaVu Sans Mono") !== -1 ? "DejaVu Sans Mono" : Qt.platform.os === "osx" ? "Menlo" : "monospace"
        onLeaveRequested: Qt.quit()
        onAlarmRequested: Qt.quit()
        settingsContribution: Component {
            LabButton {
                text: I18n.tr('Check for updates')
                implicitHeight: 56
                font.pixelSize: 19
                primary: true
                notification: updateDialog.updateAvailable
                onClicked: updateDialog.show()
            }
        }
    }
    UpdateFlow {
        id: updateDialog
        parent: probePage
        client: http
        settings: probePage.settings
        uiFont: window.font.family
    }
}
