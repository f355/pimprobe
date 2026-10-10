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

Item {
    id: help
    required property Item scope
    property bool active: false
    property bool suspended: false
    property var tips: []
    property var selected: null
    property int revision: 0
    property GuideButton headerGuide: null
    signal guideRequested()
    visible: active && !suspended
    z: 10000
    anchors.fill: parent

    onActiveChanged: {
        selected = null;
        if (active) {
            function endEdit(item) {
                if (item instanceof NumberField) item.editor.cancel();
                (item.children || []).forEach(endEdit);
            }
            endEdit(scope);
            scope.forceActiveFocus();
            Qt.callLater(refresh);
        }
    }
    onScopeChanged: if (active) { selected = null; Qt.callLater(refresh); }
    onSuspendedChanged: if (!suspended && active) Qt.callLater(refresh)

    function refresh() {
        var found = [];
        var guideInHeader = null;
        function visit(item) {
            if (item === help)
                return;
            if (item instanceof HelpTip) {
                if (item.target && item.target.visible && (item.text || item.toggle))
                    found.push(item);
                return;
            }
            if (!item.visible)
                return;
            if (item instanceof GuideButton) {
                guideInHeader = item;
                return;
            }
            (item.children || []).forEach(visit);
        }
        visit(scope);
        headerGuide = guideInHeader;
        if (found.length !== tips.length || found.some(function(tip, i) { return tip !== tips[i]; }))
            tips = found;
        if (selected && found.indexOf(selected) < 0)
            selected = null;
        revision++;
    }

    function bounds(tip) {
        // mapToItem does not notify bindings when an ancestor scrolls.
        var update = revision;
        var target = tip.target;
        var p = target.mapToItem(help, 0, 0);
        var left = Math.max(0, p.x), top = Math.max(0, p.y);
        var right = Math.min(width, p.x + target.width), bottom = Math.min(height, p.y + target.height);
        for (var item = target.parent; item; item = item.parent) {
            if (item.clip) {
                var clip = item.mapToItem(help, 0, 0);
                left = Math.max(left, clip.x);
                top = Math.max(top, clip.y);
                right = Math.min(right, clip.x + item.width);
                bottom = Math.min(bottom, clip.y + item.height);
            }
        }
        return Qt.rect(left, top, Math.max(0, right - left), Math.max(0, bottom - top));
    }
    Timer {
        interval: 250
        repeat: true
        running: help.visible
        onTriggered: help.refresh()
    }
    function contains(rect, x, y) {
        return x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height;
    }
    function overlap(a, b) {
        return Math.max(0, Math.min(a.x + a.width, b.x + b.width) - Math.max(a.x, b.x))
            * Math.max(0, Math.min(a.y + a.height, b.y + b.height) - Math.max(a.y, b.y));
    }
    function at(x, y) {
        for (var i = tips.length - 1; i >= 0; --i)
            if (contains(bounds(tips[i]), x, y))
                return tips[i];
        return null;
    }
    function select(tip) {
        if (tip.toggle)
            active = false;
        else
            selected = tip;
    }
    function openGuide() {
        selected = null;
        guideRequested();
    }
    readonly property rect guideBounds: {
        var update = revision;
        var button = headerGuide || guide;
        var p = button.mapToItem(help, 0, 0);
        return Qt.rect(p.x, p.y, button.width, button.height);
    }
    readonly property rect selectedBounds: selected ? bounds(selected) : Qt.rect(0, 0, 0, 0)
    readonly property point popupPosition: {
        var r = selectedBounds, w = popup.width, h = popup.height, m = Theme.margin;
        var candidates = [
            [r.x + r.width + 12, r.y], [r.x - w - 12, r.y],
            [r.x, r.y + r.height + 12], [r.x, r.y - h - 12],
            [m, m], [width - w - m, m], [m, height - h - m], [width - w - m, height - h - m]
        ];
        var best = Qt.point(m, m), score = Infinity;
        candidates.forEach(function(candidate) {
            var x = Math.max(m, Math.min(width - w - m, candidate[0]));
            var y = Math.max(m, Math.min(height - h - m, candidate[1]));
            var box = Qt.rect(x, y, w, h);
            var cost = overlap(box, r) * 10 + overlap(box, guideBounds) * 4;
            tips.filter(function(tip) { return tip.toggle; }).forEach(function(tip) { cost += overlap(box, bounds(tip)) * 10; });
            cost += (Math.abs(x + w / 2 - r.x - r.width / 2)
                + Math.abs(y + h / 2 - r.y - r.height / 2)) * 0.01;
            if (cost < score) { score = cost; best = Qt.point(x, y); }
        });
        return best;
    }

    property real pulse: 0.4
    SequentialAnimation on pulse {
        running: help.visible
        loops: Animation.Infinite
        NumberAnimation { to: 1; duration: 850; easing.type: Easing.InOutSine }
        NumberAnimation { to: 0.4; duration: 850; easing.type: Easing.InOutSine }
    }
    Repeater {
        model: help.tips
        Item {
            objectName: "helpHighlight"
            required property var modelData
            readonly property rect area: help.bounds(modelData)
            readonly property real frameHeight: modelData.frameHeight || area.height + 4
            x: Math.max(4, area.x - 2)
            y: Math.max(4, area.y + (area.height - frameHeight) / 2)
            width: Math.max(0, Math.min(help.width - 4, area.x + area.width + 2) - x)
            height: Math.max(0, Math.min(help.height - 4, area.y + (area.height + frameHeight) / 2) - y)
            visible: !modelData.toggle && area.width > 0 && area.height > 0
            Rectangle {
                anchors.fill: parent
                color: "transparent"
                radius: Theme.radius
                border.width: 4
                border.color: Theme.accentBright
                opacity: help.selected === modelData ? 0.2 : help.pulse * 0.12
            }
            Rectangle {
                anchors.fill: parent
                color: "transparent"
                radius: Theme.radius
                border.width: 2
                border.color: Theme.accentBright
                opacity: help.selected === modelData ? 1 : help.pulse
            }
        }
    }
    MouseArea {
        anchors.fill: parent
        property var pressedTip
        property bool pressedGuide
        property var scroll
        property real startY
        property real initialY
        property bool dragged
        onPressed: function(mouse) {
            pressedGuide = help.headerGuide && help.contains(help.guideBounds, mouse.x, mouse.y);
            pressedTip = help.at(mouse.x, mouse.y);
            startY = mouse.y;
            dragged = false;
            scroll = pressedTip ? pressedTip.target.parent : null;
            while (scroll && !(scroll instanceof Flickable)) scroll = scroll.parent;
            initialY = scroll ? scroll.contentY : 0;
        }
        onPositionChanged: function(mouse) {
            if (!pressed || !scroll) return;
            var dy = mouse.y - startY;
            if (Math.abs(dy) > 8) dragged = true;
            if (dragged) {
                help.selected = null;
                scroll.contentY = Math.max(scroll.originY,
                    Math.min(scroll.originY + Math.max(0, scroll.contentHeight - scroll.height), initialY - dy));
                help.refresh();
            }
        }
        onReleased: function(mouse) {
            if (pressedGuide && help.contains(help.guideBounds, mouse.x, mouse.y))
                help.openGuide();
            else if (!dragged && pressedTip)
                help.select(pressedTip);
        }
    }

    GuideButton {
        id: guide
        objectName: "helpGuide"
        controller: help
        visible: !help.headerGuide
        width: 112
        height: 48
        x: help.selected && help.overlap(help.selectedBounds, Qt.rect(help.width - width - Theme.margin, help.height - height - Theme.margin, width, height)) > 0 ? Theme.margin : help.width - width - Theme.margin
        y: help.height - height - Theme.margin
    }
    Rectangle {
        id: popup
        objectName: "helpPopup"
        visible: help.selected !== null
        x: help.popupPosition.x
        y: help.popupPosition.y
        width: Math.min(336, help.width - 2 * Theme.margin)
        height: popupContent.implicitHeight + 24
        color: Theme.panelRaised
        radius: Theme.radius
        border.color: Theme.accentBright
        border.width: 1
        MouseArea { anchors.fill: parent }
        ColumnLayout {
            id: popupContent
            anchors.fill: parent
            anchors.margins: 12
            spacing: 8
            RowLayout {
                Layout.fillWidth: true
                Label {
                    Layout.fillWidth: true
                    text: help.selected ? I18n.tr(help.selected.title) : ""
                    color: Theme.accentBright
                    font.pixelSize: 22
                    font.weight: Font.DemiBold
                    wrapMode: Text.WordWrap
                }
                LabButton {
                    objectName: "helpPopupClose"
                    Layout.preferredWidth: 48
                    Layout.preferredHeight: 48
                    text: "\u00d7"
                    font.pixelSize: 30
                    padding: 4
                    Accessible.name: I18n.tr("Close explanation")
                    background: null
                    onClicked: help.selected = null
                }
            }
            Label {
                Layout.fillWidth: true
                text: help.selected ? I18n.tr(help.selected.text) : ""
                wrapMode: Text.WordWrap
                color: Theme.text
                font.pixelSize: 20
            }
        }
    }
}
