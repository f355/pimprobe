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

pragma ComponentBehavior: Bound
import QtQuick
import "controls"
import "ProbeDrawing.js" as Draw

ProbeButton {
    id: button
    property bool vertical: false
    property bool negativeY: false
    diagramLabel: vertical ? (negativeY ? "Align vertical surface toward Y−" : "Align vertical surface toward Y+") : "Level horizontal surface"

    contentItem: ProbeDiagram {
        onPaint: {
            var c = getContext("2d");
            c.reset();
            var size = Math.min(width, height);
            var span = size * 0.7;
            var thickness = size * 0.17;
            var surface = size * 0.03;
            var halfSpacing = span * 0.25;
            var tilt = (button.vertical ? 1 : -1) * Math.PI / 12;
            c.translate(width / 2, height / 2);

            c.save();
            var chuckRadius = size * 0.43;
            c.globalAlpha = 0.18;
            c.strokeStyle = Theme.text;
            c.lineWidth = 1.5;
            c.beginPath();
            c.arc(0, 0, chuckRadius, 0, Math.PI * 2);
            c.stroke();
            for (var jaw = 0; jaw < 4; ++jaw) {
                c.save();
                c.rotate(jaw * Math.PI / 2);
                c.strokeRect(chuckRadius * 0.55, -size * 0.045,
                             chuckRadius * 0.4, size * 0.09);
                c.restore();
            }
            c.restore();

            if (button.negativeY)
                c.scale(-1, 1);
            if (button.vertical)
                c.rotate(-Math.PI / 2);

            c.save();
            c.translate(0, surface + thickness / 2);
            c.rotate(tilt);
            c.fillStyle = Theme.stock;
            c.strokeStyle = Theme.stockEdge;
            c.lineWidth = 2;
            c.fillRect(-span / 2, -thickness / 2, span, thickness);
            c.strokeRect(-span / 2, -thickness / 2, span, thickness);
            c.restore();

            c.save();
            c.setLineDash([2, 4]);
            c.strokeStyle = Theme.text;
            c.strokeRect(-span / 2, surface, span, thickness);
            c.restore();

            c.strokeStyle = Theme.text;
            c.lineWidth = 3;
            [-halfSpacing, halfSpacing].forEach(function(x) {
                var contact = surface + thickness / 2 + x * Math.tan(tilt)
                            - thickness / (2 * Math.cos(tilt));
                Draw.arrow(c, x, -size * 0.27, x, contact - 5);
            });
            Draw.crosshair(c, button.vertical ? halfSpacing : -halfSpacing, -size * 0.27);
            c.strokeStyle = Theme.accentBright;
            Draw.crosshair(c, 0, surface);

            var radius = size * 0.36;
            var start = Math.PI * (button.vertical ? 0.72 : 0.28);
            var end = Math.PI * (button.vertical ? 0.28 : 0.72);
            var beforeEnd = end + (button.vertical ? 0.12 : -0.12);
            c.strokeStyle = Theme.text;
            c.lineWidth = 2;
            c.beginPath();
            c.arc(0, 0, radius, start, end, button.vertical);
            c.stroke();
            Draw.arrow(c, radius * Math.cos(beforeEnd), radius * Math.sin(beforeEnd),
                       radius * Math.cos(end), radius * Math.sin(end));
        }
    }
}
