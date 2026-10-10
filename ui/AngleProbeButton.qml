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
import "controls"
import "ProbePages.js" as Pages
import "ProbeDrawing.js" as Draw

ProbeButton {
    id: control
    required property string feature
    readonly property bool slope: feature.indexOf("z-") === 0
    readonly property bool xFace: feature.indexOf("x-") === 0
    readonly property bool negative: feature.indexOf("minus") >= 0
    diagramLabel: I18n.tr(Pages.angleName(feature))
    helpTitle: diagramLabel
    helpText: slope
        ? I18n.tr("Start above the first point. Touch Z down twice, moving in the named positive axis between points, then return to start.")
        : I18n.tr("Start beside the face at measuring height. Touch twice in this direction, moving along the other positive axis between points, then return to start.")
    contentItem: ProbeDiagram {
        function stock(c, points) {
            c.fillStyle = Theme.stock;
            c.strokeStyle = Theme.stockEdge;
            c.lineWidth = 1.5;
            c.beginPath();
            points.forEach(function(p, index) {
                if (index) c.lineTo(p[0], p[1]); else c.moveTo(p[0], p[1]);
            });
            c.closePath(); c.fill(); c.stroke();
        }
        function approach(c, fromX, fromY, toX, toY, start) {
            c.strokeStyle = Theme.text;
            c.lineWidth = 2.5;
            Draw.arrow(c, fromX, fromY, toX, toY);
            if (start) Draw.crosshair(c, fromX, fromY);
        }
        function traverse(c, fromX, fromY, toX, toY) {
            c.save(); c.setLineDash([3, 4]);
            c.strokeStyle = Theme.textMuted; c.lineWidth = 1.5;
            Draw.arrow(c, fromX, fromY, toX, toY);
            c.restore();
        }
        onPaint: {
            var c = getContext("2d");
            c.reset();
            c.scale(width / 160, height / 90);
            if (control.slope) {
                stock(c, [[10, 7], [150, 7], [150, 83], [10, 83]]);
                var alongX = control.feature === "z-x";
                Draw.referencePlane(c, alongX ? 14 : 63, alongX ? 29 : 11,
                    alongX ? 146 : 97, alongX ? 61 : 79, Theme.axisZ);
                var first = alongX ? [34, 45] : [80, 67];
                var second = alongX ? [126, 45] : [80, 23];
                Draw.referenceDashes(c, alongX ? 0 : 80, alongX ? 45 : 0,
                    alongX ? 160 : 80, alongX ? 45 : 90, [Theme.axisZ]);
                Draw.downwardTouch(c, first[0], first[1], true, Theme.text);
                Draw.downwardTouch(c, second[0], second[1], false, Theme.text);
            } else if (control.xFace) {
                if (control.negative) { c.translate(160, 0); c.scale(-1, 1); }
                function edge(y) { return 94 + (45 - y) * 0.18; }
                stock(c, [[edge(5), 5], [152, 5], [152, 85], [edge(85), 85]]);
                var referenceX = edge(0);
                Draw.referenceDashes(c, referenceX, 3, referenceX, 87, [Theme.textMuted]);
                c.strokeStyle = Theme.axisX; c.lineWidth = 3;
                c.beginPath(); c.arc(referenceX, 0, 64, Math.PI / 2, Math.PI / 2 + Math.atan(0.18)); c.stroke();
                approach(c, 28, 72, edge(72) - 5, 72, true);
                approach(c, 28, 18, edge(18) - 5, 18, false);
                traverse(c, 28, 56, 28, 32);
            } else {
                if (!control.negative) { c.translate(0, 90); c.scale(1, -1); }
                function edgeY(x) { return 46 + (x - 80) * 0.16; }
                stock(c, [[9, edgeY(9)], [151, edgeY(151)], [151, 85], [9, 85]]);
                var referenceY = edgeY(160);
                Draw.referenceDashes(c, 5, referenceY, 155, referenceY, [Theme.textMuted]);
                c.strokeStyle = Theme.axisY; c.lineWidth = 3;
                c.beginPath(); c.arc(160, referenceY, 80, Math.PI, Math.PI + Math.atan(0.16)); c.stroke();
                approach(c, 28, 13, 28, edgeY(28) - 5, true);
                approach(c, 130, 13, 130, edgeY(130) - 5, false);
                traverse(c, 47, 13, 113, 13);
            }
        }
    }
}
