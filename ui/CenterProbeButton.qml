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
import "ProbeDrawing.js" as Draw

ProbeButton {
    id: control
    required property string feature
    required property string label
    diagramLabel: label

    contentItem: Item {
        ProbeDiagram {
            id: diagram
            anchors.fill: parent
            anchors.bottomMargin: 20
            onPaint: {
                var c = getContext("2d");
                c.reset();
                var cx = width / 2, cy = height / 2;
                var f = control.feature;
                var internal = f === "hole" || f === "pocket" || f.indexOf("valley") >= 0;
                var round = f === "boss" || f === "hole";
                var onlyX = f.indexOf("x-") === 0, onlyY = f.indexOf("y-") === 0;
                var z = f === "z";
                var left = cx - 23, right = cx + 23, top = cy - 23, bottom = cy + 23;
                if (onlyX) {
                    left = cx - 17;
                    right = cx + 17;
                    top = internal ? 0 : 3;
                    bottom = internal ? height : height - 3;
                }
                if (onlyY) {
                    left = internal ? 0 : 3;
                    right = internal ? width : width - 3;
                    top = cy - 15;
                    bottom = cy + 15;
                }
                c.fillStyle = internal ? "#A9A9A9" : "#E1E1E1";
                c.fillRect(0, 0, width, height);
                c.fillStyle = internal ? "#E1E1E1" : "#A9A9A9";
                c.strokeStyle = "#777777";
                c.lineWidth = 1;
                c.beginPath();
                if (round)
                    c.arc(cx, cy, 23, 0, Math.PI * 2);
                else
                    c.rect(left, top, right - left, bottom - top);
                c.fill();
                if (!onlyX && !onlyY)
                    c.stroke();
                else {
                    c.beginPath();
                    if (onlyX) {
                        c.moveTo(left, top);
                        c.lineTo(left, bottom);
                        c.moveTo(right, top);
                        c.lineTo(right, bottom);
                    } else {
                        c.moveTo(left, top);
                        c.lineTo(right, top);
                        c.moveTo(left, bottom);
                        c.lineTo(right, bottom);
                    }
                    c.stroke();
                }
                c.strokeStyle = "#343434";
                c.lineWidth = 2.5;
                if (!z && !onlyY) {
                    Draw.arrow(c, internal ? cx - 8 : 3, cy, left + (internal ? 2 : -2), cy);
                    Draw.arrow(c, internal ? cx + 8 : width - 3, cy, right + (internal ? -2 : 2), cy);
                }
                if (!z && !onlyX) {
                    Draw.arrow(c, cx, internal ? cy - 8 : 2, cx, top + (internal ? 2 : -2));
                    Draw.arrow(c, cx, internal ? cy + 8 : height - 2, cx, bottom + (internal ? -2 : 2));
                }
                c.strokeStyle = "#777777";
                Draw.crosshair(c, cx, cy);
                Draw.measuredPoint(c, cx, cy);
            }
        }
        Label {
            anchors.bottom: parent.bottom
            anchors.horizontalCenter: parent.horizontalCenter
            text: control.label
            color: "#202020"
            font.pixelSize: 13
        }
    }
}
