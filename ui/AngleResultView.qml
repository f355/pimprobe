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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "controls"
import "ProbePages.js" as Pages

RowLayout {
    id: view
    required property var flow
    readonly property var measurement: flow.angle
    spacing: Theme.groupSpacing
    ColumnLayout {
        Layout.preferredWidth: Theme.columnWidth
        Layout.fillHeight: true
        spacing: 12
        Label {
            text: I18n.tr(Pages.angleName(view.flow.routine.feature))
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: Theme.text
            font.pixelSize: 24
        }
        Label {
            text: view.measurement ? view.measurement.degrees.toFixed(4) + "°" : "—"
            color: Theme.accentBright
            font.family: view.flow.codeFont
            font.pixelSize: 42
        }
        Label {
            text: view.measurement ? I18n.tr("Change %1 mm over %2 mm", [view.measurement.difference.toFixed(3), view.measurement.spacing.toFixed(3)]) : ""
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: Theme.textMuted
            font.pixelSize: 20
        }
        HelpTip {
            title: I18n.tr("Measured angle")
            text: I18n.tr("Angle of the face relative to the machine axes. Positive X/Y angles turn counterclockwise; positive Z slopes rise along the named axis.")
        }
        Item { Layout.fillHeight: true }
    }
    Rectangle { Layout.preferredWidth: 1; Layout.fillHeight: true; color: Theme.divider }
    ColumnLayout {
        Layout.fillWidth: true
        Layout.fillHeight: true
        spacing: 12
        RowLayout {
            Layout.fillWidth: true
            ConfirmationLabel {
                id: confirmation
                heading: I18n.tr("Work coordinates")
                Layout.fillWidth: true
            }
            MenuButton {
                text: "G" + view.flow.routine.wcs
                Layout.preferredWidth: 112
                enabled: !view.flow.busy && view.flow.completedID.length > 0
                onClicked: wcsPicker.open()
                helpTitle: I18n.tr("Choose result work coordinates")
                helpText: I18n.tr("Choose which work coordinate system receives this measured angle.")
            }
        }
        Label {
            text: I18n.tr("Contacts · G53 · mm")
            font.pixelSize: 18
            color: Theme.textMuted
        }
        Repeater {
            model: view.measurement ? view.measurement.touches : []
            ColumnLayout {
                required property var modelData
                required property int index
                Layout.fillWidth: true
                spacing: 4
                Label {
                    text: I18n.tr("Touch %1", [parent.index + 1])
                    color: Theme.textMuted
                    font.pixelSize: 18
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    Repeater {
                        model: ["X", "Y", "Z"]
                        Label {
                            required property string modelData
                            required property int index
                            text: modelData + " " + parent.parent.modelData[index].toFixed(3)
                            Layout.fillWidth: true
                            font.family: view.flow.codeFont
                            font.pixelSize: 18
                            color: Theme.text
                        }
                    }
                }
            }
        }
        HelpTip {
            title: I18n.tr("Contacts · G53 · mm")
            text: I18n.tr("Machine coordinates of the two fine touches. Their difference gives the face angle; probe offsets and ball diameter cancel.")
        }
        Item { Layout.fillHeight: true }
    }
    WorkCoordinatePicker {
        id: wcsPicker
        currentWcs: view.flow.routine.wcs || 54
        title: I18n.tr("Save result to work coordinates")
        onSelected: function(wcs) { view.flow.resultWcsRequested(wcs); }
    }
    Connections {
        target: view.flow
        function onClosed() { wcsPicker.close(); confirmation.clear(); }
        function onRotationSaved() { confirmation.confirm("X/Y rotation set"); }
        function onRoutineChanged() { confirmation.clear(); }
    }
}
