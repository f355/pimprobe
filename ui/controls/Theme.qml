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
    readonly property Style defaults: Style {}
    property Style style: defaults
    readonly property int headerHeight: (style || defaults).headerHeight
    readonly property int buttonHeight: (style || defaults).buttonHeight
    readonly property int textSize: (style || defaults).textSize
    readonly property int radius: (style || defaults).radius
    readonly property int margin: (style || defaults).margin
    readonly property int columnWidth: (style || defaults).columnWidth
    readonly property int groupSpacing: (style || defaults).groupSpacing
    readonly property color page: (style || defaults).page
    readonly property color header: (style || defaults).header
    readonly property color panel: (style || defaults).panel
    readonly property color panelRaised: (style || defaults).panelRaised
    readonly property color control: (style || defaults).control
    readonly property color pressed: (style || defaults).pressed
    readonly property color divider: (style || defaults).divider
    readonly property color text: (style || defaults).text
    readonly property color primaryText: (style || defaults).primaryText
    readonly property color textMuted: (style || defaults).textMuted
    readonly property color accent: (style || defaults).accent
    readonly property color accentPressed: (style || defaults).accentPressed
    readonly property color accentBright: (style || defaults).accentBright
    readonly property color accentWash: (style || defaults).accentWash
    readonly property color danger: (style || defaults).danger
    readonly property color warning: (style || defaults).warning
    readonly property color field: (style || defaults).field
    readonly property color fieldBorder: (style || defaults).fieldBorder
    readonly property color stock: (style || defaults).stock
    readonly property color stockEdge: (style || defaults).stockEdge
    readonly property color axisX: (style || defaults).axisX
    readonly property color axisY: (style || defaults).axisY
    readonly property color axisZ: (style || defaults).axisZ
}
