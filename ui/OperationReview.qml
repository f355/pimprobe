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

pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "controls"
import "ReviewPlan.js" as ReviewPlan
import "controls/HelpText.js" as HelpText

RowLayout {
    id: review
    required property var config
    property bool rotary: false
    property var draft: ({})
    readonly property var plan: ReviewPlan.describe(config, rotary)
    spacing: Theme.groupSpacing
    onConfigChanged: {
        editor.cancel();
        draft = Object.assign({}, config);
        parameterScroll.contentY = 0;
    }
    onVisibleChanged: if (!visible) editor.cancel()

    NumericEditor { id: editor }

    function setValue(key, value) {
        var updated = Object.assign({}, draft);
        updated[key] = value;
        draft = updated;
    }

    function parameters() {
        if (editor.target)
            editor.accept();
        return editor.target ? null : Object.assign({}, draft);
    }

    Item {
        Layout.preferredWidth: Theme.columnWidth
        Layout.fillHeight: true
        ColumnLayout {
            anchors.fill: parent
            spacing: 8
            visible: !editor.target
            MotionIllustration {
                id: illustration
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumHeight: 140
                clip: review.plan.clip
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 4
                Repeater {
                    model: review.plan.phases
                    RowLayout {
                        id: step
                        required property string modelData
                        required property int index
                        readonly property bool active: illustration.phase === index
                        Layout.fillWidth: true
                        Layout.preferredHeight: 40
                        spacing: 10
                        Rectangle {
                            Layout.preferredWidth: 24
                            Layout.preferredHeight: 24
                            radius: 12
                            color: step.active ? Theme.accent : Theme.panelRaised
                            Label {
                                anchors.centerIn: parent
                                text: step.index + 1
                                font.pixelSize: 16
                                color: Theme.text
                            }
                        }
                        Label {
                            Layout.fillWidth: true
                            text: I18n.tr(step.modelData)
                            font.pixelSize: 18
                            wrapMode: Text.WordWrap
                            color: step.active ? Theme.text : Theme.textMuted
                        }
                    }
                }
            }
        }
        NumericKeypad {
            anchors.fill: parent
            visible: editor.target !== null
            onKeyPressed: function(key) { editor.typeKey(key); }
            onAccepted: editor.accept()
        }
    }

    Flickable {
        id: parameterScroll
        objectName: "reviewOptions"
        Layout.fillWidth: true
        Layout.fillHeight: true
        clip: true
        contentWidth: width
        contentHeight: optionRows.implicitHeight
        flickableDirection: Flickable.VerticalFlick
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar {
            id: bar
            visible: parameterScroll.contentHeight > parameterScroll.height
            policy: ScrollBar.AlwaysOn
        }
        ColumnLayout {
            id: optionRows
            width: parameterScroll.width - (bar.visible ? 12 : 0)
            spacing: 12
            Label {
                text: I18n.tr('Options for this run')
                Layout.fillWidth: true
                font.pixelSize: 20
                color: Theme.text
            }
            Repeater {
                model: review.plan.options
                ColumnLayout {
                    id: option
                    required property var modelData
                    Layout.fillWidth: true
                    spacing: 8
                    Label {
                        visible: !!option.modelData.section
                        Layout.fillWidth: true
                        Layout.topMargin: 8
                        text: option.modelData.section ? I18n.tr(option.modelData.section) : ""
                        font.pixelSize: 16
                        color: Theme.textMuted
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 12
                        Label {
                            text: I18n.tr(option.modelData.label)
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            font.pixelSize: 20
                            color: Theme.text
                        }
                        NumberField {
                            objectName: "review-" + option.modelData.key
                            Accessible.name: I18n.tr(option.modelData.label)
                            helpText: HelpText.option(option.modelData.key, review.config.family)
                            Layout.preferredWidth: 112
                            Layout.preferredHeight: 56
                            editor: editor
                            value: Number(review.draft[option.modelData.key] || 0)
                            minimum: option.modelData.minimum
                            maximum: option.modelData.maximum
                            onCommitted: function(value) { review.setValue(option.modelData.key, value); }
                        }
                        Label {
                            text: I18n.tr(option.modelData.unit)
                            Layout.preferredWidth: 68
                            font.pixelSize: 16
                            color: Theme.textMuted
                        }
                    }
                }
            }
        }
    }
}
