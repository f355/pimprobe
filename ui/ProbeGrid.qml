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

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

GridLayout {
    id: grid
    required property bool inside
    signal selected(var routine)
    columns: 3
    rowSpacing: 8
    columnSpacing: 8

    Repeater {
        model: 9
        Loader {
            id: cell
            required property int index
            Layout.preferredWidth: 108
            Layout.preferredHeight: 108
            readonly property int direction: grid.inside ? 1 : -1
            readonly property var routine: ({
                    x: (index % 3 - 1) * direction,
                    y: (1 - Math.floor(index / 3)) * direction,
                    z: index === 4
                })
            sourceComponent: grid.inside ? insideButton : outsideButton
            Component {
                id: outsideButton
                OutsideProbeButton {
                    xApproach: cell.routine.x
                    yApproach: cell.routine.y
                    zApproach: cell.routine.z
                    onClicked: grid.selected(cell.routine)
                }
            }
            Component {
                id: insideButton
                InsideProbeButton {
                    xApproach: cell.routine.x
                    yApproach: cell.routine.y
                    zApproach: cell.routine.z
                    onClicked: grid.selected(cell.routine)
                }
            }
        }
    }
}
