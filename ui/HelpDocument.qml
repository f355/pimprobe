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

ScrollView {
    id: document
    required property var sections
    property string uiFont
    signal linkActivated(string link)
    clip: true
    contentWidth: availableWidth
    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

    function reset() {
        contentItem.contentY = 0;
        for (var i = 0; i < sectionList.count; ++i)
            sectionList.itemAt(i).expanded = false;
    }

    ColumnLayout {
        width: document.availableWidth
        spacing: 12
        Repeater {
            id: sectionList
            model: document.sections
            delegate: ColumnLayout {
                id: section
                required property var modelData
                property bool expanded: false
                Layout.fillWidth: true
                Layout.leftMargin: Theme.margin
                Layout.rightMargin: Theme.margin
                spacing: 12
                MenuButton {
                    Layout.fillWidth: true
                    visible: section.modelData.title.length > 0
                    text: section.modelData.title
                    textAlignment: Text.AlignLeft
                    font.family: document.uiFont
                    expanded: section.expanded
                    onClicked: section.expanded = !section.expanded
                }
                ColumnLayout {
                    visible: !section.modelData.title || section.expanded
                    Layout.fillWidth: true
                    spacing: 12
                    Repeater {
                        model: section.modelData.blocks
                        delegate: Loader {
                            id: block
                            required property var modelData
                            Layout.fillWidth: true
                            sourceComponent: modelData.image ? illustration : paragraph
                            Component {
                                id: paragraph
                                TextArea {
                                    width: block.width
                                    readOnly: true
                                    textFormat: TextEdit.MarkdownText
                                    text: block.modelData.text || ""
                            onLinkActivated: function(link) { document.linkActivated(link); }
                                    color: Theme.text
                                    font.family: document.uiFont
                                    font.pixelSize: 20
                                    wrapMode: TextEdit.Wrap
                                    padding: 0
                                    background: null
                                }
                            }
                            Component {
                                id: illustration
                                Image {
                                    width: block.width
                                    height: sourceSize.width > 0 ? width * sourceSize.height / sourceSize.width : 0
                                    source: Qt.resolvedUrl("help/" + block.modelData.image)
                                    fillMode: Image.PreserveAspectFit
                                    Accessible.name: block.modelData.caption || ""
                                }
                            }
                        }
                    }
                }
            }
        }
        Item { Layout.preferredHeight: Theme.margin }
    }
}
