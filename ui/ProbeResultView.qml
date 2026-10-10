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
import "ProbePages.js" as Pages

RowLayout {
    id: view
    required property var flow
    required property NumericEditor editor
    readonly property bool editing: editor.target !== null
    spacing: Theme.groupSpacing

    Item {
        Layout.preferredWidth: Theme.columnWidth
        Layout.fillHeight: true
        ColumnLayout {
            anchors.fill: parent
            visible: !view.editing
            spacing: 12
            Label {
                text: I18n.tr(Pages.featureName(view.flow.routine))
                Layout.fillWidth: true
                font.pixelSize: 24
                wrapMode: Text.WordWrap
                color: Theme.text
            }
            Repeater {
                model: view.flow.dimensions
                RowLayout {
                    required property var modelData
                    Layout.fillWidth: true
                    spacing: 12
                    Label {
                        text: I18n.tr('%1 · mm', [I18n.tr(parent.modelData.label)])
                        Layout.fillWidth: true
                        font.pixelSize: 18
                        color: Theme.textMuted
                    }
                    Label {
                        text: parent.modelData.value.toFixed(3)
                        font.family: view.flow.codeFont
                        font.pixelSize: 26
                        color: Theme.text
                    }
                }
            }
            Loader {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumHeight: 48
                sourceComponent: view.flow.routine.family === "center" ? centerDiagram : view.flow.routine.family === "inside" ? insideDiagram : outsideDiagram
                Component {
                    id: centerDiagram
                    CenterProbeButton {
                        feature: view.flow.routine.z ? "z" : view.flow.routine.feature
                        label: ""
                        enabled: false
                    }
                }
                Component {
                    id: insideDiagram
                    InsideProbeButton {
                        xApproach: view.flow.routine.x || 0
                        yApproach: view.flow.routine.y || 0
                        zApproach: view.flow.routine.z || false
                        enabled: false
                    }
                }
                Component {
                    id: outsideDiagram
                    OutsideProbeButton {
                        xApproach: view.flow.routine.x || 0
                        yApproach: view.flow.routine.y || 0
                        zApproach: view.flow.routine.z || false
                        enabled: false
                    }
                }
            }
        }
        NumericKeypad {
            anchors.fill: parent
            visible: view.editing
            onKeyPressed: function (key) {
                view.editor.typeKey(key);
            }
            onAccepted: view.editor.accept()
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
        spacing: 8
        RowLayout {
            Layout.fillWidth: true
            ConfirmationLabel {
                id: heading
                heading: I18n.tr('Work zero')
                Layout.fillWidth: true
            }
            MenuButton {
                objectName: "resultWcs"
                text: "G" + view.flow.routine.wcs
                enabled: !view.flow.busy && view.flow.completedID.length > 0
                Layout.preferredWidth: 112
                Layout.preferredHeight: 56
                onClicked: wcsPicker.open()
                Accessible.name: I18n.tr('Choose result work coordinates')
            }
        }
        ScrollView {
            id: scroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth
            clip: true
            ColumnLayout {
                width: scroll.availableWidth
                spacing: 8
                RowLayout {
                    Layout.fillWidth: true
                    Label {
                        Layout.fillWidth: true
                        text: I18n.tr('Measured · G53 · mm')
                        color: Theme.textMuted
                        font.pixelSize: 18
                    }
                    Label {
                        Layout.preferredWidth: 112
                        text: I18n.tr('Offset · mm')
                        color: Theme.textMuted
                        font.pixelSize: 18
                        wrapMode: Text.WordWrap
                        horizontalAlignment: Text.AlignRight
                    }
                }
                Repeater {
                    model: view.flow.measuredAxes
                    RowLayout {
                        id: axis
                        required property int modelData
                        Layout.fillWidth: true
                        spacing: 12
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 0
                            Label {
                                text: ["X", "Y", "Z"][axis.modelData] + " " + view.flow.measuredPosition(axis.modelData).toFixed(3)
                                Layout.fillWidth: true
                                font.family: view.flow.codeFont
                                font.pixelSize: 28
                                color: Theme.text
                            }
                            Label {
                                text: "G" + view.flow.routine.wcs + " " + view.flow.measuredWorkPosition(axis.modelData).toFixed(3)
                                Layout.fillWidth: true
                                color: Theme.textMuted
                                font.family: view.flow.codeFont
                                font.pixelSize: 16
                            }
                        }
                        NumberField {
                            objectName: "resultOffset" + axis.modelData
                            editor: view.editor
                            enabled: !view.flow.busy && view.flow.completedID.length > 0
                            minimum: -1000
                            maximum: 1000
                            value: view.flow.offsets[axis.modelData]
                            Layout.preferredWidth: 112
                            Layout.preferredHeight: 56
                            Accessible.name: I18n.tr('%1 origin offset', [["X", "Y", "Z"][axis.modelData]])
                            onCommitted: function (value) {
                                view.flow.setOffset(axis.modelData, value);
                            }
                        }
                    }
                }
                RowLayout {
                    visible: !view.flow.historical && view.flow.routine.family === "inside" && !view.flow.routine.z
                    Layout.fillWidth: true
                    Label {
                        Layout.fillWidth: true
                        text: I18n.tr('Safe Z lift · mm')
                        color: Theme.text
                        font.pixelSize: 20
                    }
                    NumberField {
                        editor: view.editor
                        minimum: 0.1
                        maximum: 1000
                        value: view.flow.safeZOffset
                        Layout.preferredWidth: 112
                        Layout.preferredHeight: 56
                        onCommitted: function (value) {
                            view.flow.safeZOffset = value;
                            view.flow.settingChanged("safeZOffset", value);
                        }
                    }
                }
            }
        }
    }
    WorkCoordinatePicker {
        id: wcsPicker
        objectName: "resultWcsPicker"
        currentWcs: view.flow.routine.wcs || 54
        title: I18n.tr('Save result to work coordinates')
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
        function onWorkZeroSaved() {
            heading.confirm("Work zero set");
        }
        function onRoutineChanged() {
            heading.clear();
        }
    }
}
