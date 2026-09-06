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
    signal selected(string feature)
    columns: 3
    rowSpacing: 8
    columnSpacing: 8
    Repeater {
        model: [
            {
                feature: "boss",
                label: "Boss"
            },
            {
                feature: "block",
                label: "Block"
            },
            {
                feature: "x-ridge",
                label: "X ridge"
            },
            {
                feature: "hole",
                label: "Hole"
            },
            {
                feature: "pocket",
                label: "Pocket"
            },
            {
                feature: "x-valley",
                label: "X valley"
            },
            {
                feature: "z",
                label: "Z-"
            },
            {
                feature: "y-ridge",
                label: "Y ridge"
            },
            {
                feature: "y-valley",
                label: "Y valley"
            }
        ]
        CenterProbeButton {
            required property var modelData
            Layout.preferredWidth: 108
            Layout.preferredHeight: 108
            feature: modelData.feature
            label: modelData.label
            onClicked: grid.selected(feature)
        }
    }
}
