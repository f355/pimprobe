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
    property color page: "#2B2B2B"
    property color header: "#242424"
    property color panel: "#414141"
    property color panelRaised: "#3B3A3A"
    property color control: "#2F2E2E"
    property color pressed: "#797979"
    property color divider: "#7C7C7C"
    property color text: "#D8D8D8"
    property color textMuted: "#7C7C7C"
    property color accent: "#017B44"
    property color accentPressed: "#014A29"
    property color accentBright: "#11D76A"
    property color accentWash: "#33017B44"
    property color danger: "#FF6B6B"
    property color warning: "#FFB078"
    property color field: "#383737"
    property color fieldBorder: "#5E5E5E"
    property color stock: "#A9A9A9"
    property color stockEdge: "#777777"
}
