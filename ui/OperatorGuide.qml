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
import QtQuick.Layouts
import "controls"
import "HelpPages.js" as HelpPages

PageView {
    id: guide
    property bool showHeader: true
    property bool helpMode: false
    property int pageIndex: -1
    readonly property var pages: HelpPages.guidesForLanguage(I18n.language)
    readonly property string title: pageIndex < 0 ? I18n.tr("Operator Guide") : pages[pageIndex].title

    function showIndex() { pageIndex = -1; open(); }
    function back() { if (pageIndex < 0) close(); else pageIndex = -1; }
    onPageIndexChanged: document.reset()
    background: Rectangle { color: Theme.page }
    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        PageHeader {
            visible: guide.showHeader
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.headerHeight
            title: guide.title
            showHelp: guide.helpMode
            onBack: guide.back()
        }
        GridLayout {
            visible: guide.pageIndex < 0
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: Theme.margin
            columns: 2
            columnSpacing: 12
            rowSpacing: 12
            Repeater {
                model: guide.pages
                LabButton {
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.preferredWidth: 1
                    text: modelData.title
                    onClicked: guide.pageIndex = index
                }
            }
        }
        HelpDocument {
            id: document
            visible: guide.pageIndex >= 0
            Layout.fillWidth: true
            Layout.fillHeight: true
            sections: guide.pageIndex >= 0 ? guide.pages[guide.pageIndex].sections : []
            uiFont: guide.font.family
            onLinkActivated: function(link) {
                var name = link.split("/").pop();
                var index = guide.pages.findIndex(function(page) { return page.name === name; });
                if (index >= 0) guide.pageIndex = index;
            }
        }
    }
}
