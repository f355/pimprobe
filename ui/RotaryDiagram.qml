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
import "ProbeDrawing.js" as Draw

ProbeButton {
    diagramLabel: "Calibrate rotary axis"
    caption: "Axis center"
    contentItem: ProbeDiagram {
        onPaint: {
            var c = getContext("2d");
            c.reset();
            var cy = height / 2;
            var chuckLeft = width * 0.06, chuckRight = width * 0.23;
            var rodRight = width * 0.94;
            var rodTop = cy - height * 0.09, rodBottom = cy + height * 0.09;
            c.fillStyle = Theme.stock;
            c.strokeStyle = Theme.stockEdge;
            c.lineWidth = 2;
            c.fillRect(chuckRight, rodTop, rodRight - chuckRight, rodBottom - rodTop);
            c.strokeRect(chuckRight, rodTop, rodRight - chuckRight, rodBottom - rodTop);
            [-1, 1].forEach(function(side) {
                c.save();
                c.translate(chuckRight, cy);
                c.scale(1, side);
                c.beginPath();
                c.moveTo(0, -height * 0.28);
                c.lineTo(width * 0.065, -height * 0.28);
                c.lineTo(width * 0.065, -height * 0.20);
                c.lineTo(width * 0.13, -height * 0.20);
                c.lineTo(width * 0.13, -height * 0.08);
                c.lineTo(0, -height * 0.08);
                c.closePath();
                c.fill();
                c.stroke();
                c.restore();
            });
            c.fillRect(chuckLeft, height * 0.15, chuckRight - chuckLeft, height * 0.70);
            c.strokeRect(chuckLeft, height * 0.15, chuckRight - chuckLeft, height * 0.70);

            var first = width * 0.43, second = width * 0.77;
            [first, second].forEach(function(x) {
                c.strokeStyle = Theme.text;
                c.lineWidth = 3;
                Draw.arrow(c, x, height * 0.12, x, rodTop - 4);
                Draw.arrow(c, x, height * 0.88, x, rodBottom + 4);
            });
            c.strokeStyle = Theme.text;
            Draw.crosshair(c, first, cy);
            c.translate(first, cy);
            c.scale(0.6, 0.6);
            c.strokeStyle = Theme.accentBright;
            Draw.crosshair(c, 0, 0);
        }
    }
}
