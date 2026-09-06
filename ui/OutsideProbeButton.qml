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

    required property int xApproach
    required property int yApproach
    property bool zApproach: false
    diagramLabel: zApproach ? "Z surface" : "X " + xApproach + ", Y " + yApproach

    contentItem: ProbeDiagram {
        id: diagram

        onPaint: {
            var context = getContext("2d");
            context.reset();

            var margin = 5;
            var edgeX = width * 0.38;
            var edgeY = height * 0.38;
            var left = control.xApproach > 0 ? edgeX : margin;
            var right = control.xApproach < 0 ? width - edgeX : width - margin;
            var top = control.yApproach < 0 ? edgeY : margin;
            var bottom = control.yApproach > 0 ? height - edgeY : height - margin;

            context.fillStyle = "#A9A9A9";
            context.fillRect(left, top, right - left, bottom - top);
            context.strokeStyle = "#777777";
            context.lineWidth = 1;
            context.strokeRect(left, top, right - left, bottom - top);

            var targetX = control.xApproach > 0 ? left : control.xApproach < 0 ? right : width / 2;
            var targetY = control.yApproach < 0 ? top : control.yApproach > 0 ? bottom : height / 2;

            if (control.zApproach) {
                targetX = width / 2;
                targetY = height / 2;
            }

            var probeX = (left + right) / 2;
            var probeY = (top + bottom) / 2;

            context.strokeStyle = "#343434";
            context.lineWidth = 3;
            if (control.xApproach > 0)
                Draw.arrow(context, margin, probeY, left - 3, probeY);
            else if (control.xApproach < 0)
                Draw.arrow(context, width - margin, probeY, right + 3, probeY);
            if (control.yApproach < 0)
                Draw.arrow(context, probeX, margin, probeX, top - 3);
            else if (control.yApproach > 0)
                Draw.arrow(context, probeX, height - margin, probeX, bottom + 3);

            context.strokeStyle = "#E8E8E8";
            Draw.crosshair(context, probeX, probeY);

            Draw.measuredPoint(context, targetX, targetY);
        }
    }
}
