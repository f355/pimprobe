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
import "controls"
import QtQuick.Controls
import QtQuick.Layouts
import "HelpPages.js" as HelpPages
import "ProbePages.js" as Pages

Pane {
    id: page
    required property var client
    property bool showHeader: true
    property bool compactLayout: false
    property Style style
    onStyleChanged: if (style)
        Theme.style = style
    property Component headerControls: Component {
        ProbeSwitch {
            id: probeSwitch
            implicitWidth: 158
            text: page.actuatorRequestPending || page.machineState.actuatorPending ? I18n.tr('Moving') : I18n.tr(page.probeState((page.machineState.status || {}).probeActuator, (page.machineState.status || {}).probeActuatorKnown))
            checked: (page.machineState.status || {}).probeActuatorKnown === true && (page.machineState.status || {}).probeActuator !== 0
            enabled: page.machineState.connected && (page.machineState.status || {}).mode === "Ready" && (page.machineState.status || {}).probeActuatorKnown && !page.actuatorRequestPending && !page.machineState.actuatorPending && !page.interactionLocked()
            font.family: page.uiFontFamily
            font.pixelSize: 18
            contentItem: Label {
                leftPadding: probeSwitch.indicator.width + probeSwitch.spacing
                text: probeSwitch.text
                color: page.probeLabelColor()
                font: probeSwitch.font
                verticalAlignment: Text.AlignVCenter
            }
            onClicked: {
                var status = page.machineState.status || {};
                page.setProbeExtended(status.probeActuator === 0);
            }
        }
    }
    property Component settingsContribution
    property bool updateAvailable: false
    signal leaveRequested
    signal exitCancelled
    signal alarmRequested
    readonly property bool busy: interactionLocked()
    readonly property var topPage: {
        var views = [rotaryDialog, confirmDialog, repeatabilityDialog, historyDialog, settingsPage, utilitiesPage, helpPage, wcsPicker];
        var top = null;
        for (var i = 0; i < views.length; ++i)
            if (views[i].opened && (!top || views[i].z > top.z))
                top = views[i];
        return top;
    }
    readonly property bool subpageOpen: topPage !== null
    readonly property string helpTitle: I18n.tr(["Outside help", "Inside help", "Center help", "Rotary help", "Settings help"][helpIndex])
    readonly property string pageTitle: topPage === wcsPicker ? I18n.tr('Work coordinates')
        : topPage === helpPage ? helpTitle
        : topPage === historyDialog ? I18n.tr('Probe history')
        : topPage === repeatabilityDialog ? I18n.tr('Probe repeatability')
        : topPage === confirmDialog ? confirmDialog.description
        : topPage === rotaryDialog ? rotaryDialog.title
        : topPage === settingsPage ? I18n.tr('Probe settings')
        : topPage === utilitiesPage ? I18n.tr('Utilities') : I18n.tr('Probing')
    property int helpIndex: tabs.currentIndex
    onVisibleChanged: if (!visible)
        exitDialog.close()
    padding: 0
    topPadding: showHeader ? Theme.headerHeight : 0
    property ProbeSettings settings: ProbeSettings {
        client: page.client
    }
    NumericEditor {
        id: numericEditor
    }
    ServiceRequest {
        id: wcsRequest
        client: page.client
    }
    ServiceRequest {
        id: actuatorRequest
        client: page.client
    }
    Component.onCompleted: {
        settings.load();
        observeState();
    }
    property string uiFontFamily: "sans-serif"
    property string monoFontFamily: "monospace"
    property var machineState: ({
            "connected": false
        })
    property string requestError: "Starting service"
    property string actionError: ""
    readonly property var droCoordinates: (machineState.coordinates || {})[droReference.probeSelected ? "probe" : "spindle"] || ({})
    readonly property bool actuatorRequestPending: actuatorRequest.pending
    property bool exitAfterRetract: false

    function coordinate(position, index) {
        return position && position.length > index ? Number(position[index]).toFixed(3) : "--.---";
    }

    function probeState(value, known) {
        if (!known)
            return "Unknown";
        if (value === 0)
            return "Retracted";
        if (value === 1)
            return "Extended";
        if (value === 2)
            return "Intermediate";
        if (value === 3)
            return "Moving";
        return "Unknown";
    }

    function currentWcs() {
        var reported = Number((machineState.status || {}).wcs);
        return reported >= 54 && reported <= 59 ? reported : 54;
    }

    function probeFullyExtended() {
        var status = machineState.status || {};
        return status.probeActuatorKnown === true && status.probeActuator === 1;
    }

    function probeAvailable() {
        return machineState.connected === true && probeFullyExtended();
    }

    function activePageAvailable() {
        return tabs.currentIndex === 4 ? machineState.connected === true : probeAvailable();
    }

    function probeLabelColor() {
        if (actuatorRequestPending || machineState.actuatorPending)
            return Theme.textMuted;
        var status = machineState.status || {};
        if (status.probeActuatorKnown && status.probeActuator === 0)
            return Theme.danger;
        if (status.probeActuatorKnown && status.probeActuator === 1)
            return Theme.accentBright;
        return Theme.textMuted;
    }

    function requestExit() {
        var status = machineState.status || {};
        if (status.probeActuatorKnown && status.probeActuator !== 0) {
            actionError = "";
            exitDialog.open();
            return;
        }
        page.leaveRequested();
    }

    function retractAndExit() {
        exitAfterRetract = true;
        setProbeExtended(false);
    }

    function chooseWcs(wcs) {
        if (wcsRequest.pending)
            return;
        wcsPicker.errorText = "";
        wcsRequest.send("wcs.select", {
            wcs: wcs
        }, function (reply) {
            if (reply.ok) {
                page.refreshState();
                wcsPicker.close();
            } else
                wcsPicker.errorText = reply.error;
        });
    }

    function openRoutineReview(family, selection) {
        numericEditor.cancel();
        confirmDialog.showRoutine(Pages.routine(settings.values, family, selection, currentWcs()));
    }

    function controlsLocked() {
        return machineState.contactActive || machineState.recoveryFailed;
    }

    function interactionLocked() {
        return controlsLocked() || confirmDialog.phase === "running" || confirmDialog.busy || repeatabilityDialog.phase === "running" || rotaryDialog.phase === "running" || rotaryDialog.busy;
    }

    function setProbeExtended(extended) {
        if (actuatorRequest.pending)
            return;
        actionError = "";
        actionErrorTimer.stop();
        actuatorRequest.send("probe.set", {
            extended: extended
        }, function (reply) {
            if (!reply.ok) {
                exitAfterRetract = false;
                actionError = reply.error;
                actionErrorTimer.restart();
            }
            refreshState();
        });
    }

    function refreshState() {
        client.refresh();
    }

    function observeState() {
        machineState = client.state;
        requestError = client.error || "";
        if (!activePageAvailable())
            numericEditor.cancel();
        if (!machineState.connected)
            settingsPage.cancelEdit();
        if (exitAfterRetract && (machineState.status || {}).probeActuatorKnown && (machineState.status || {}).probeActuator === 0) {
            exitAfterRetract = false;
            exitDialog.close();
            leaveRequested();
        }
    }

    function requestBack() {
        if (busy)
            return;
        if (topPage)
            topPage.close();
        else
            requestExit();
    }
    function openHelp() {
        helpPage.open();
    }
    function openWcs() {
        wcsPicker.open();
    }
    function openSettings() {
        if (busy)
            return;
        numericEditor.cancel();
        while (topPage)
            topPage.close();
        if (compactLayout)
            settingsPage.open();
        else
            tabs.currentIndex = 4;
    }
    function openUtilities() {
        if (busy)
            return;
        numericEditor.cancel();
        settingsPage.cancelEdit();
        utilitiesPage.open();
    }

    Connections {
        target: page.client
        function onStateChanged() {
            page.observeState();
        }
        function onErrorChanged() {
            page.requestError = page.client.error || "";
        }
    }

    font.family: uiFontFamily
    palette.windowText: Theme.text
    background: Rectangle {
        color: Theme.page
    }

    property Item headerBar: Rectangle {
        parent: page
        width: page.width
        height: page.showHeader ? Theme.headerHeight : 0
        visible: page.showHeader && !page.subpageOpen
        implicitHeight: Theme.headerHeight
        color: Theme.header

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 8
            anchors.rightMargin: 16
            spacing: 8

            BackButton {
                id: backButton
                Layout.preferredWidth: 48
                Layout.fillHeight: true
                onClicked: page.requestExit()
            }

            Item {
                Layout.fillWidth: true
            }

            RowLayout {
                Layout.fillHeight: true
                spacing: 4
                Repeater {
                    model: ["X", "Y", "Z"]
                    ColumnLayout {
                        id: headerCoordinate
                        required property string modelData
                        required property int index
                        Layout.preferredWidth: 136
                        Layout.fillHeight: true
                        spacing: 0

                        Label {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            text: headerCoordinate.modelData + " " + page.coordinate(page.droCoordinates.workPosition, headerCoordinate.index)
                            color: Theme.text
                            font.family: page.monoFontFamily
                            font.pixelSize: 22
                            horizontalAlignment: Text.AlignRight
                            verticalAlignment: Text.AlignBottom
                        }
                        Label {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            text: page.coordinate(page.droCoordinates.machinePosition, headerCoordinate.index)
                            color: Theme.textMuted
                            font.family: page.monoFontFamily
                            font.pixelSize: 16
                            horizontalAlignment: Text.AlignRight
                            verticalAlignment: Text.AlignTop
                        }
                    }
                }
            }

            LabButton {
                Layout.preferredWidth: 80
                Layout.fillHeight: true
                background: null
                padding: 8
                text: "G" + page.currentWcs()
                font.family: page.uiFontFamily
                font.pixelSize: 22
                font.bold: true
                textColor: Theme.accentBright
                onClicked: wcsPicker.open()
            }

            CoordinateReferenceButton {
                id: droReference
                objectName: "droReference"
                Layout.preferredWidth: 96
                Layout.preferredHeight: 56
                font.family: page.uiFontFamily
            }

            LabButton {
                id: helpButton
                Layout.preferredWidth: 48
                Layout.preferredHeight: 48
                Layout.rightMargin: 0
                text: "?"
                font.family: page.uiFontFamily
                font.pixelSize: 25
                font.bold: true
                padding: 4
                contentItem: Label {
                    text: helpButton.text
                    color: Theme.text
                    font: helpButton.font
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                background: Rectangle {
                    color: helpButton.down ? Theme.pressed : Theme.panel
                    border.color: Theme.divider
                    border.width: 1
                    radius: width / 2
                }
                onClicked: helpPage.open()
            }
        }
    }

    PageView {
        id: wcsPicker
        onClosed: wcsRequest.cancel()
        property string errorText: ""
        onOpening: errorText = ""
        parent: page
        x: 0
        y: 0
        width: parent.width
        height: parent.height
        padding: 0

        background: Rectangle {
            color: Theme.page
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            PageHeader {
                Layout.fillWidth: true
                Layout.preferredHeight: Theme.headerHeight
                visible: page.showHeader
                title: I18n.tr('Work coordinates')
                uiFont: page.uiFontFamily
                onBack: wcsPicker.close()
            }

            Label {
                Layout.fillWidth: true
                Layout.leftMargin: 24
                visible: wcsPicker.errorText.length > 0
                text: I18n.tr(wcsPicker.errorText)
                color: Theme.danger
                font.pixelSize: 18
            }

            GridLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.margins: 16
                columns: 3
                columnSpacing: 12
                rowSpacing: 12

                Repeater {
                    model: [54, 55, 56, 57, 58, 59]
                    LabButton {
                        required property int modelData
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        text: "G" + modelData
                        font.family: page.uiFontFamily
                        font.pixelSize: 30
                        font.bold: modelData === page.currentWcs()
                        selected: modelData === page.currentWcs()
                        enabled: !wcsRequest.pending
                        onClicked: page.chooseWcs(modelData)
                    }
                }
            }
        }
    }

    PageView {
        id: helpPage
        onOpening: {
            page.helpIndex = settingsPage.opened ? 4 : tabs.currentIndex;
            helpScroll.reset();
        }
        parent: page
        x: 0
        y: 0
        width: parent.width
        height: parent.height
        padding: 0

        background: Rectangle {
            color: Theme.page
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            PageHeader {
                Layout.fillWidth: true
                Layout.preferredHeight: Theme.headerHeight
                visible: page.showHeader
                title: page.helpTitle
                uiFont: page.uiFontFamily
                onBack: helpPage.close()
            }

            HelpDocument {
                id: helpScroll
                Layout.fillWidth: true
                Layout.fillHeight: true
                sections: HelpPages.forLanguage(I18n.language)[page.helpIndex]
                uiFont: page.uiFontFamily
            }
        }
    }

    ProbeFlow {
        id: confirmDialog
        parent: page
        showHeader: page.showHeader
        onAlarmRequested: page.alarmRequested()
        client: page.client
        uiFont: page.uiFontFamily
        codeFont: page.monoFontFamily
        onSettingChanged: function (key, value) {
            page.settings.setValue(key, value);
        }
    }

    RotaryFlow {
        id: rotaryDialog
        onAlarmRequested: page.alarmRequested()
        parent: page
        showHeader: page.showHeader
        client: page.client
        uiFont: page.uiFontFamily
        codeFont: page.monoFontFamily
    }

    RepeatabilityFlow {
        id: repeatabilityDialog
        parent: page
        showHeader: page.showHeader
        client: page.client
        uiFont: page.uiFontFamily
        codeFont: page.monoFontFamily
    }

    HistoryFlow {
        id: historyDialog
        parent: page
        showHeader: page.showHeader
        client: page.client
        uiFont: page.uiFontFamily
        onResultRequested: function (opened) {
            var details = function () { return historyDialog.detail(opened.entry); };
            historyDialog.close();
            if (opened.entry.category === "rotary")
                rotaryDialog.showHistory(opened, details);
            else
                confirmDialog.showHistory(opened, details);
        }
    }

    ProbeSettingsPage {
        id: settingsPage
        parent: page
        settings: page.settings
        settingsContribution: page.settingsContribution
        editable: page.settings.loaded && page.machineState.connected && !page.busy
        showHeader: page.showHeader
        font.family: page.uiFontFamily
        onUtilitiesRequested: page.openUtilities()
    }

    PageView {
        id: utilitiesPage
        parent: page
        x: 0
        y: 0
        width: parent.width
        height: parent.height
        padding: 0

        background: Rectangle {
            color: Theme.page
        }

        UtilitiesPanel {
            anchors.fill: parent
            uiFont: page.uiFontFamily
            showHeader: page.showHeader
            client: page.client
            onClosed: utilitiesPage.close()
            onRepeatabilityRequested: repeatabilityDialog.showCheck()
            onHistoryRequested: historyDialog.showHistory()
        }
    }

    TouchDialog {
        id: exitDialog
        objectName: "exitDialog"
        font.family: page.uiFontFamily
        title: I18n.tr('Probe extended')
        message: page.exitAfterRetract ? I18n.tr('Retracting probe...') : I18n.tr('Retract the probe before leaving?')
        errorText: page.actionError
        busy: page.exitAfterRetract
        closeOnAccept: false
        alternateText: I18n.tr('Leave extended')
        acceptText: I18n.tr('Retract and exit')
        onRejected: page.exitCancelled()
        // Keep the prompt visible until retraction is confirmed.
        onAlternate: page.leaveRequested()
        onAccepted: page.retractAndExit()
    }

    Timer {
        id: actionErrorTimer
        interval: 3000
        onTriggered: page.actionError = ""
    }

    ColumnLayout {
        anchors.fill: parent
        visible: !page.subpageOpen
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            visible: page.settings.error.length > 0
            Label {
                Layout.fillWidth: true
                text: I18n.tr(page.settings.error)
                color: Theme.danger
                font.pixelSize: 18
            }
            LabButton {
                text: I18n.tr('Retry')
                primary: true
                onClicked: page.settings.loaded ? page.settings.save() : page.settings.load()
            }
        }

        MessageStrip {
            Layout.fillWidth: true
            Layout.leftMargin: 16
            Layout.rightMargin: 16
            visible: !page.machineState.connected || page.actionError.length > 0
            text: page.actionError || page.requestError ? I18n.tr(page.actionError || page.requestError) : I18n.tr('Controller disconnected')
            textColor: Theme.danger
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 56
            spacing: 8
            Loader {
                visible: page.showHeader
                Layout.leftMargin: Theme.margin
                Layout.preferredWidth: 140
                Layout.preferredHeight: 56
                sourceComponent: page.headerControls
            }
            TabBar {
                id: tabs
                onCurrentIndexChanged: numericEditor.cancel()
                Layout.fillWidth: true
                Layout.preferredHeight: 56
                enabled: !page.interactionLocked()
                background: Rectangle {
                    color: Theme.page
                }
                Repeater {
                    model: page.compactLayout ? ["Outside", "Inside", "Center", "Rotary"] : ["Outside", "Inside", "Center", "Rotary", "Settings"]
                    LabTabButton {
                        required property string modelData
                        implicitHeight: 56
                        height: tabs.height
                        text: I18n.tr(modelData)
                        notification: modelData === "Settings" && page.updateAvailable
                        font.family: page.uiFontFamily
                    }
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            enabled: page.settings.loaded && page.activePageAvailable() && !page.interactionLocked()
            opacity: page.activePageAvailable() ? 1 : 0.35

            StackLayout {
                anchors.fill: parent
                currentIndex: tabs.currentIndex

                ProbePanel {
                    settings: page.settings
                    editor: numericEditor
                    parameters: Pages.outside
                    ProbeGrid {
                        anchors.centerIn: parent
                        width: parent.width
                        height: Math.min(340, parent.height)
                        inside: false
                        onSelected: function (routine) {
                            page.openRoutineReview("outside", routine);
                        }
                    }
                }
                ProbePanel {
                    settings: page.settings
                    editor: numericEditor
                    parameters: Pages.inside
                    ProbeGrid {
                        anchors.centerIn: parent
                        width: parent.width
                        height: Math.min(340, parent.height)
                        inside: true
                        onSelected: function (routine) {
                            page.openRoutineReview("inside", routine);
                        }
                    }
                }
                ProbePanel {
                    settings: page.settings
                    editor: numericEditor
                    parameters: Pages.center
                    CenterGrid {
                        anchors.centerIn: parent
                        width: parent.width
                        height: Math.min(340, parent.height)
                        onSelected: function (feature) {
                            page.openRoutineReview("center", feature);
                        }
                    }
                }
                ProbePanel {
                    settings: page.settings
                    editor: numericEditor
                    parameters: Pages.rotary
                    GridLayout {
                        anchors.fill: parent
                        columns: 2
                        rowSpacing: 8
                        columnSpacing: 8
                        RotaryDiagram {
                            objectName: "rotaryCalibrationButton"
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            Layout.preferredHeight: 162
                            onClicked: {
                                numericEditor.cancel();
                                rotaryDialog.showCalibration(Pages.rotaryConfig(page.settings.values));
                            }
                        }
                        RotaryLevelButton {
                            objectName: "verticalLevelButton"
                            vertical: true
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            Layout.preferredWidth: 166
                            Layout.preferredHeight: 162
                            onClicked: {
                                numericEditor.cancel();
                                rotaryDialog.showCalibration(Pages.rotaryConfig(page.settings.values, "vertical"));
                            }
                        }
                        RotaryLevelButton {
                            objectName: "horizontalLevelButton"
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            Layout.preferredWidth: 166
                            Layout.preferredHeight: 162
                            onClicked: {
                                numericEditor.cancel();
                                rotaryDialog.showCalibration(Pages.rotaryConfig(page.settings.values, "horizontal"));
                            }
                        }
                        RotaryLevelButton {
                            objectName: "verticalNegativeLevelButton"
                            vertical: true
                            negativeY: true
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            Layout.preferredWidth: 166
                            Layout.preferredHeight: 162
                            onClicked: {
                                numericEditor.cancel();
                                rotaryDialog.showCalibration(Pages.rotaryConfig(page.settings.values, "verticalNegative"));
                            }
                        }
                    }
                }
                ProbeSetupPanel {
                    settings: page.settings
                    editor: numericEditor
                    settingsContribution: page.settingsContribution
                    onUtilitiesRequested: page.openUtilities()
                }
            }
            NumericKeypad {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.margins: Theme.margin
                width: Theme.columnWidth
                visible: numericEditor.target !== null
                onKeyPressed: function (key) {
                    numericEditor.typeKey(key);
                }
                onAccepted: numericEditor.accept()
            }
        }
    }
}
