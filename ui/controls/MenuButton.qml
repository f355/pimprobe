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

LabButton {
    id: control
    property bool expanded: false
    property int textAlignment: Text.AlignHCenter

    contentItem: Item {
        implicitWidth: label.implicitWidth + 24
        implicitHeight: label.implicitHeight
        Label {
            id: label
            anchors.fill: parent
            anchors.rightMargin: 24
            text: control.text
            font: control.font
            color: control.enabled ? control.textColor : Theme.textMuted
            horizontalAlignment: control.textAlignment
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }
        Canvas {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            width: 12
            height: 8
            rotation: control.expanded ? 180 : 0
            property color stroke: label.color
            onStrokeChanged: requestPaint()
            onPaint: {
                var context = getContext("2d");
                context.reset();
                context.strokeStyle = stroke;
                context.lineWidth = 2;
                context.beginPath();
                context.moveTo(1, 1);
                context.lineTo(6, 6);
                context.lineTo(11, 1);
                context.stroke();
            }
        }
    }
}
