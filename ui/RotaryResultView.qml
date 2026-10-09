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
import QtQuick.Controls
import QtQuick.Layouts
import "controls"

RowLayout {
    id: view
    required property var flow
    spacing: 16
    ColumnLayout {
        Layout.preferredWidth: Theme.columnWidth
        Layout.fillHeight: true
        spacing: 8
        Label {
            text: view.flow.leveling ? "Surface alignment" : "Rotary axis"
            font.pixelSize: 24
            color: Theme.text
        }
        Loader {
            Layout.fillWidth: true
            Layout.fillHeight: true
            sourceComponent: view.flow.leveling ? surfaceDiagram : axisDiagram
            Component {
                id: axisDiagram
                RotaryDiagram {
                    enabled: false
                    caption: ""
                }
            }
            Component {
                id: surfaceDiagram
                RotaryLevelButton {
                    vertical: view.flow.operation !== "horizontal"
                    negativeY: view.flow.operation === "verticalNegative"
                    enabled: false
                    caption: ""
                }
            }
        }
    }
    Rectangle {
        Layout.preferredWidth: 1
        Layout.fillHeight: true
        color: Theme.divider
    }
    ColumnLayout {
        Layout.fillWidth: true
        Layout.fillHeight: true
        spacing: 12
        RowLayout {
            Layout.fillWidth: true
            ConfirmationLabel {
                id: heading
                heading: "Work zero · " + view.flow.zeroAxes
                Layout.fillWidth: true
            }
            MenuButton {
                objectName: "rotaryResultWcs"
                text: "G" + view.flow.result.wcs
                enabled: !view.flow.busy && view.flow.reviewID.length > 0
                Layout.preferredWidth: 112
                onClicked: wcsPicker.open()
                Accessible.name: "Choose result work coordinates"
            }
        }
        ScrollView {
            id: details
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth
            clip: true
            ColumnLayout {
                width: details.availableWidth
                spacing: 8
                Label {
                    text: (view.flow.leveling ? "Touches" : "Axis centers") + " · G53 · mm"
                    color: Theme.textMuted
                    font.pixelSize: 18
                }
                GridLayout {
                    Layout.fillWidth: true
                    columns: 3
                    columnSpacing: 12
                    rowSpacing: 8
                    Repeater {
                        model: ["X", "Y", "Z"]
                        Label {
                            required property string modelData
                            text: modelData
                            Layout.fillWidth: true
                            Layout.preferredWidth: 1
                            color: Theme.textMuted
                            font.pixelSize: 18
                        }
                    }
                    Repeater {
                        model: view.flow.leveling ? ((view.flow.result.level || {}).touches || []) : (view.flow.result.stations || [])
                        RowLayout {
                            id: station
                            required property var modelData
                            required property int index
                            readonly property var point: view.flow.leveling ? modelData : modelData.center
                            Layout.columnSpan: 3
                            Layout.fillWidth: true
                            spacing: 12
                            Repeater {
                                model: [0, 1, 2]
                                Label {
                                    required property int modelData
                                    text: Number(station.point[modelData]).toFixed(3)
                                    Layout.fillWidth: true
                                    Layout.preferredWidth: 1
                                    color: Theme.text
                                    font.family: view.flow.codeFont
                                    font.pixelSize: 20
                                }
                            }
                        }
                    }
                }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 1
                    color: Theme.divider
                }
                Repeater {
                    model: view.flow.leveling ? [
                        {
                            label: "A correction",
                            value: (view.flow.result.level || {}).correction
                        },
                        {
                            label: "Remaining tilt",
                            value: (view.flow.result.level || {}).residual
                        }
                    ] : [
                        {
                            label: "Axis angle in X/Y",
                            value: view.flow.result.xyAngle
                        },
                        {
                            label: "Axis angle in X/Z",
                            value: view.flow.result.xzAngle
                        }
                    ]
                    RowLayout {
                        required property var modelData
                        Layout.fillWidth: true
                        Label {
                            text: parent.modelData.label
                            Layout.fillWidth: true
                            color: Theme.text
                            font.pixelSize: 20
                        }
                        Label {
                            text: Number(parent.modelData.value || 0).toFixed(4) + "°"
                            color: Theme.text
                            font.family: view.flow.codeFont
                            font.pixelSize: 24
                        }
                    }
                }
                Label {
                    text: view.flow.leveling ? "Zero uses the measured surface and A angle." : "Zero uses the first axis center."
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    color: Theme.textMuted
                    font.pixelSize: 18
                }
            }
        }
    }
    WorkCoordinatePicker {
        id: wcsPicker
        currentWcs: view.flow.result.wcs || 54
        title: "Save result to work coordinates"
        onSelected: function (wcs) {
            view.flow.resultWcsRequested(wcs);
        }
    }
    Connections {
        target: view.flow
        function onClosed() {
            wcsPicker.close();
            heading.clear();
        }
        function onCalibrationSaved(message) {
            heading.confirm(message);
        }
        function onResultWcsRequested() {
            heading.clear();
        }
    }
}
