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

pragma Singleton
import QtQuick

QtObject {
    readonly property color page: "#2B2B2B"
    readonly property color header: "#242424"
    readonly property color panel: "#414141"
    readonly property color panelRaised: "#3B3A3A"
    readonly property color control: "#2F2E2E"
    readonly property color pressed: "#797979"
    readonly property color divider: "#7C7C7C"
    readonly property color text: "#D8D8D8"
    readonly property color textMuted: "#7C7C7C"
    readonly property color accent: "#017B44"
    readonly property color accentPressed: "#014A29"
    readonly property color accentBright: "#11D76A"
    readonly property color accentWash: "#33017B44"
    readonly property color danger: "#FF6B6B"
    readonly property color warning: "#FFB078"
    readonly property color field: "#383737"
    readonly property color fieldBorder: "#5E5E5E"
    readonly property color stock: "#A9A9A9"
    readonly property color stockEdge: "#777777"
}
