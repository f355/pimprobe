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

QtObject {
    property int headerHeight: 64
    property int buttonHeight: 56
    property int textSize: 20
    property int radius: 8
    property int margin: 16
    property int columnWidth: 352
    property int groupSpacing: 16
    property color page: "#202425"
    property color header: "#282E30"
    property color panel: "#282E30"
    property color panelRaised: "#343C3E"
    property color control: "#343C3E"
    property color pressed: "#4B585B"
    property color divider: "#586568"
    property color text: "#F0F3F3"
    property color textMuted: "#B1BDC0"
    property color primaryText: "#FFFFFF"
    property color accent: "#28785F"
    property color accentPressed: "#205F4C"
    property color accentBright: "#7ED5B1"
    property color accentWash: "#344D44"
    property color danger: "#FF9292"
    property color warning: "#E9BD62"
    property color field: "#282E30"
    property color fieldBorder: "#586568"
    property color stock: "#586B64"
    property color stockEdge: "#A2B9B0"
}
