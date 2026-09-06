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

function measuredPoint(context, x, y) {
    context.fillStyle = "#017B44";
    context.beginPath();
    context.arc(x, y, 4, 0, Math.PI * 2);
    context.fill();
}
