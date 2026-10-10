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

.pragma library

function arrow(context, fromX, fromY, toX, toY) {
    var angle = Math.atan2(toY - fromY, toX - fromX);
    var head = 6;
    context.beginPath();
    context.moveTo(fromX, fromY);
    context.lineTo(toX, toY);
    context.moveTo(toX, toY);
    context.lineTo(toX - head * Math.cos(angle - Math.PI / 5),
                   toY - head * Math.sin(angle - Math.PI / 5));
    context.moveTo(toX, toY);
    context.lineTo(toX - head * Math.cos(angle + Math.PI / 5),
                   toY - head * Math.sin(angle + Math.PI / 5));
    context.stroke();
}

function crosshair(context, x, y) {
    context.lineWidth = 2;
    context.beginPath();
    context.arc(x, y, 7, 0, Math.PI * 2);
    context.stroke();
    context.beginPath();
    context.moveTo(x - 10, y);
    context.lineTo(x + 10, y);
    context.moveTo(x, y - 10);
    context.lineTo(x, y + 10);
    context.stroke();
}

function referencePoint(context, x, y, colors) {
    context.save();
    colors.forEach(function(color, index) {
        context.fillStyle = color;
        context.beginPath();
        context.moveTo(x, y);
        context.arc(x, y, 4.5, -Math.PI / 2 + index * Math.PI * 2 / colors.length,
            -Math.PI / 2 + (index + 1) * Math.PI * 2 / colors.length);
        context.closePath();
        context.fill();
    });
    context.restore();
}

function cornerPoint(context, x, y, xSide, ySide, xColor, yColor) {
    context.save();
    context.translate(x, y);
    context.scale(xSide, ySide);
    // X follows the vertical edge, Y the horizontal edge.
    context.rotate(3 * Math.PI / 4);
    referencePoint(context, 0, 0, [xColor, yColor]);
    context.restore();
}

function referenceLine(context, x1, y1, x2, y2, color) {
    context.save();
    context.strokeStyle = color;
    context.lineWidth = 3;
    context.beginPath();
    context.moveTo(x1, y1);
    context.lineTo(x2, y2);
    context.stroke();
    context.restore();
}

function referenceDashes(context, x1, y1, x2, y2, colors, gap) {
    context.save();
    var dx = x2 - x1, dy = y2 - y1;
    var length = Math.sqrt(dx * dx + dy * dy);
    var angle = Math.atan2(dy, dx);
    var spacing = 6 + (gap === undefined ? 6 : gap);
    context.lineWidth = 2.5;
    context.lineCap = "butt";
    for (var distance = 0, dash = 0; distance < length; distance += spacing, ++dash) {
        var end = Math.min(distance + 6, length);
        context.strokeStyle = colors[dash % colors.length];
        context.beginPath();
        context.moveTo(x1 + distance * Math.cos(angle), y1 + distance * Math.sin(angle));
        context.lineTo(x1 + end * Math.cos(angle), y1 + end * Math.sin(angle));
        context.stroke();
    }
    context.restore();
}

function referencePlane(context, left, top, right, bottom, color) {
    context.save();
    context.beginPath();
    context.rect(left, top, right - left, bottom - top);
    context.clip();
    context.fillStyle = Qt.rgba(color.r, color.g, color.b, 0.12);
    context.fillRect(left, top, right - left, bottom - top);
    context.strokeStyle = color;
    context.lineWidth = 1;
    for (var offset = top - bottom; offset < right - left; offset += 12) {
        context.beginPath();
        context.moveTo(left + offset, bottom);
        context.lineTo(left + offset + bottom - top, top);
        context.stroke();
    }
    context.restore();
}

function downwardTouch(context, x, y, start, color) {
    context.save();
    context.strokeStyle = color;
    context.lineWidth = 2;
    context.beginPath();
    context.arc(x, y, 9, 0, Math.PI * 2);
    context.moveTo(x - 4, y - 4); context.lineTo(x + 4, y + 4);
    context.moveTo(x - 4, y + 4); context.lineTo(x + 4, y - 4);
    if (start) {
        context.moveTo(x - 15, y); context.lineTo(x - 10, y);
        context.moveTo(x + 10, y); context.lineTo(x + 15, y);
        context.moveTo(x, y - 15); context.lineTo(x, y - 10);
        context.moveTo(x, y + 10); context.lineTo(x, y + 15);
    }
    context.stroke();
    context.restore();
}
