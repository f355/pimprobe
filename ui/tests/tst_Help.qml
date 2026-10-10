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
import QtTest
import ".."
import "../controls"
import "../HelpPages.js" as HelpPages

TestCase {
    name: "Help"
    when: windowShown
    ApplicationWindow {
        id: host
        width: 800
        height: 480
        visible: true
        font.family: "DejaVu Sans"
        HelpDocument {
            id: document
            anchors.fill: parent
            sections: []
            uiFont: "DejaVu Sans"
        }
    }
    function children(item, type) {
        var result = item instanceof type ? [item] : [];
        (item.children || []).forEach(function(child) { result = result.concat(children(child, type)); });
        return result;
    }
    function test_expanded_pages_data() {
        return ["en", "zh_CN", "sv"].reduce(function(rows, language) {
            return rows.concat([0,1,2,3,4].map(function(page) {
                return {tag:language + "-" + page, language:language, page:page};
            }));
        }, []);
    }
    function test_expanded_pages(data) {
        failOnWarning(/^(?!.*(?:Populating font|Sans Serif)).*/);
        document.sections = HelpPages.forLanguage(data.language)[data.page];
        verify(waitForPolish(host));
        var headings = children(document, MenuButton).filter(function(button) { return button.visible; });
        verify(headings.length > 0);
        headings.forEach(function(button) { button.clicked(); });
        verify(waitForPolish(host));
        tryVerify(function() {
            return children(document, TextArea).filter(function(area) { return area.visible; })
                .every(function(area) { return area.width > 0; });
        });
        compare(document.contentWidth, document.availableWidth);
        children(document, TextArea).filter(function(area) { return area.visible; }).forEach(function(area) {
            verify(area.text.length > 0);
            verify(area.contentWidth <= area.width + 1, data.tag + " help text needs horizontal scrolling: "
                + area.contentWidth + " > " + area.width + " for " + area.text.slice(0, 80));
            verify(area.contentHeight > 0);
        });
        children(document, Image).filter(function(image) { return image.visible; }).forEach(function(image) {
            tryCompare(image, "status", Image.Ready);
            verify(image.width <= document.availableWidth);
            verify(image.height > 0);
        });
        document.contentItem.contentY = 100;
        document.reset();
        compare(document.contentItem.contentY, 0);
        headings.forEach(function(button) { compare(button.expanded, false); });
    }
}
