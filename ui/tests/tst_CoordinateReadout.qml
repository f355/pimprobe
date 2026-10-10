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
import QtQuick.Controls
import QtTest
import "../controls"

TestCase {
    name: "CoordinateReadout"
    when: windowShown
    width: 800
    height: 100

    CoordinateReadout {
        id: readout
        width: implicitWidth
        height: 56
        monoFont: "DejaVu Sans Mono"
    }

    function axisPositions() {
        var positions = {};
        function visit(item) {
            if (item instanceof Label && /^[XYZ]( |$)/.test(item.text)) {
                var point = item.mapToItem(readout, 0, 0);
                positions[item.text[0]] = point.x + (item.horizontalAlignment === Text.AlignRight
                    ? item.width - item.rightPadding - item.paintedWidth : item.leftPadding);
            }
            (item.children || []).forEach(visit);
        }
        visit(readout);
        return positions;
    }

    function test_axis_letters_stay_fixed_as_coordinates_change() {
        verify(waitForPolish(readout));
        var initial = axisPositions();
        compare(Object.keys(initial).sort(), ["X", "Y", "Z"]);
        [
            [0, 0, 0], [9, -9, 99], [-99, 999, -999],
            [-999.999, -999.999, -999.999], [1, 10, 100]
        ].forEach(function(position) {
            readout.workPosition = position;
            readout.machinePosition = position;
            verify(waitForPolish(readout));
            var current = axisPositions();
            ["X", "Y", "Z"].forEach(function(axis) {
                fuzzyCompare(current[axis], initial[axis], 0.5, axis + " label moved");
            });
        });
    }
}
