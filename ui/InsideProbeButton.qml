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
            var left = width * 0.30;
            var right = width * 0.70;
            var top = height * 0.30;
            var bottom = height * 0.70;
            var centerX = width / 2;
            var centerY = height / 2;
            var probeX = control.zApproach ? centerX : centerX - control.xApproach * 8;
            var probeY = control.zApproach ? centerY : centerY + control.yApproach * 8;

            context.fillStyle = Theme.panelRaised;
            context.fillRect(margin, margin, width - 2 * margin, height - 2 * margin);
            context.fillStyle = Theme.stock;
            if (control.zApproach) {
                context.fillRect(left, top, right - left, bottom - top);
            } else {
                if (control.xApproach < 0)
                    context.fillRect(margin, margin, left - margin, height - 2 * margin);
                else if (control.xApproach > 0)
                    context.fillRect(right, margin, width - right - margin, height - 2 * margin);
                if (control.yApproach > 0)
                    context.fillRect(margin, margin, width - 2 * margin, top - margin);
                else if (control.yApproach < 0)
                    context.fillRect(margin, bottom, width - 2 * margin, height - bottom - margin);
            }
            context.strokeStyle = Theme.stockEdge;
            context.lineWidth = 1;
            context.beginPath();
            if (control.zApproach) {
                context.rect(left, top, right - left, bottom - top);
            } else {
                if (control.xApproach < 0) {
                    context.moveTo(left, control.yApproach > 0 ? top : margin);
                    context.lineTo(left, control.yApproach < 0 ? bottom : height - margin);
                } else if (control.xApproach > 0) {
                    context.moveTo(right, control.yApproach > 0 ? top : margin);
                    context.lineTo(right, control.yApproach < 0 ? bottom : height - margin);
                }
                if (control.yApproach > 0) {
                    context.moveTo(control.xApproach < 0 ? left : margin, top);
                    context.lineTo(control.xApproach > 0 ? right : width - margin, top);
                } else if (control.yApproach < 0) {
                    context.moveTo(control.xApproach < 0 ? left : margin, bottom);
                    context.lineTo(control.xApproach > 0 ? right : width - margin, bottom);
                }
            }
            context.stroke();

            var targetX = control.xApproach < 0 ? left : control.xApproach > 0 ? right : centerX;
            var targetY = control.yApproach > 0 ? top : control.yApproach < 0 ? bottom : centerY;

            context.strokeStyle = Theme.text;
            context.lineWidth = 3;
            if (control.xApproach < 0)
                Draw.arrow(context, probeX - 5, probeY, left + 3, probeY);
            else if (control.xApproach > 0)
                Draw.arrow(context, probeX + 5, probeY, right - 3, probeY);
            if (control.yApproach > 0)
                Draw.arrow(context, probeX, probeY - 5, probeX, top + 3);
            else if (control.yApproach < 0)
                Draw.arrow(context, probeX, probeY + 5, probeX, bottom - 3);

            context.strokeStyle = Theme.text;
            Draw.crosshair(context, probeX, probeY);

            if (control.zApproach) {
                targetX = centerX;
                targetY = centerY;
            }
            Draw.measuredPoint(context, targetX, targetY, Theme.accentBright);
        }
    }
}
