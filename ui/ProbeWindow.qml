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
import "HelpPages.js" as HelpPages
import "ProbePages.js" as Pages

ApplicationWindow {
    id: window
    property string serviceUrl: "http://127.0.0.1:8137/api/v1"
    property ProbeSettings settings: ProbeSettings {
        serviceUrl: window.serviceUrl
    }
    NumericEditor {
        id: numericEditor
    }
    ServiceRequest {
        id: wcsRequest
    }
    ServiceRequest {
        id: actuatorRequest
    }
    ServiceRequest {
        id: stateRequest
    }
    Component.onCompleted: settings.load()
    FontLoader { id: geistRegular; source: "fonts/Geist-Regular.otf" }
    FontLoader { source: "fonts/Geist-Medium.otf" }
    FontLoader { source: "fonts/Geist-Bold.otf" }
    readonly property var availableFonts: Qt.fontFamilies()
    property string uiFontFamily: geistRegular.name.length > 0 ? geistRegular.name
        : availableFonts.indexOf("DejaVu Sans") !== -1 ? "DejaVu Sans"
        : Qt.platform.os === "osx" ? "Helvetica Neue" : "sans-serif"
    property string monoFontFamily: availableFonts.indexOf("DejaVu Sans Mono") !== -1
        ? "DejaVu Sans Mono" : Qt.platform.os === "osx" ? "Menlo" : "monospace"
    property var machineState: ({
            "connected": false
        })
    property string requestError: "Starting service"
    property string actionError: ""
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
        return tabs.currentIndex === 3 ? machineState.connected === true : probeAvailable();
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
        Qt.quit();
    }

    function retractAndExit() {
        exitAfterRetract = true;
        setProbeExtended(false);
    }

    function chooseWcs(wcs) {
        if (wcsRequest.pending)
            return;
        wcsPicker.errorText = "";
        wcsRequest.send("POST", serviceUrl + "/wcs", {
            wcs: wcs
        }, function (reply) {
            if (reply.ok) {
                window.refreshState();
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
        return controlsLocked() || confirmDialog.phase === "running" || repeatabilityDialog.phase === "running";
    }

    function requestFailed(message) {
        numericEditor.cancel();
        requestError = message;
        machineState = {
            "connected": false
        };
    }

    function setProbeExtended(extended) {
        if (actuatorRequest.pending)
            return;
        actionError = "";
        actionErrorTimer.stop();
        actuatorRequest.send("POST", serviceUrl + "/probe-actuator", {
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
        stateRequest.send("GET", serviceUrl + "/state", null, function (reply) {
            if (!reply.ok || !reply.data) {
                requestFailed(reply.error || "Invalid service response");
                return;
            }
            machineState = reply.data;
            if (!window.activePageAvailable())
                numericEditor.cancel();
            if (window.exitAfterRetract && (machineState.status || {}).probeActuatorKnown && (machineState.status || {}).probeActuator === 0) {
                Qt.quit();
                return;
            }
            requestError = "";
        });
    }

    visible: true
    color: Theme.page
    title: "Probing"
    font.family: uiFontFamily
    palette.windowText: Theme.text

    header: Rectangle {
        implicitHeight: 70
        color: Theme.header

        RowLayout {
            anchors.fill: parent
            spacing: 8

            BackButton {
                id: backButton
                Layout.preferredWidth: 70
                Layout.fillHeight: true
                onClicked: window.requestExit()
            }

            ProbeSwitch {
                id: probeSwitch
                Layout.preferredWidth: 158
                text: window.actuatorRequestPending || window.machineState.actuatorPending ? "Moving" : window.probeState((window.machineState.status || {}).probeActuator, (window.machineState.status || {}).probeActuatorKnown)
                checked: (window.machineState.status || {}).probeActuatorKnown === true && (window.machineState.status || {}).probeActuator !== 0
                enabled: window.machineState.connected && (window.machineState.status || {}).mode === "Ready" && (window.machineState.status || {}).probeActuatorKnown && !window.actuatorRequestPending && !window.machineState.actuatorPending && !window.interactionLocked()
                font.family: window.uiFontFamily
                font.pixelSize: 15
                contentItem: Label {
                    leftPadding: probeSwitch.indicator.width + probeSwitch.spacing
                    text: probeSwitch.text
                    color: window.probeLabelColor()
                    font: probeSwitch.font
                    verticalAlignment: Text.AlignVCenter
                }
                onClicked: {
                    var status = window.machineState.status || {};
                    window.setProbeExtended(status.probeActuator === 0);
                }
            }

            Item {
                Layout.fillWidth: true
            }

            Repeater {
                model: ["X", "Y", "Z"]
                ColumnLayout {
                    id: headerCoordinate
                    required property string modelData
                    required property int index
                    Layout.preferredWidth: 112
                    Layout.fillHeight: true
                    spacing: 0

                    Label {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        text: headerCoordinate.modelData + " " + window.coordinate((window.machineState.status || {}).workPosition, headerCoordinate.index)
                        color: Theme.text
                        font.family: window.monoFontFamily
                        font.pixelSize: 22
                        horizontalAlignment: Text.AlignRight
                        verticalAlignment: Text.AlignBottom
                    }
                    Label {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        text: window.coordinate((window.machineState.status || {}).machinePosition, headerCoordinate.index)
                        color: Theme.textMuted
                        font.family: window.monoFontFamily
                        font.pixelSize: 13
                        horizontalAlignment: Text.AlignRight
                        verticalAlignment: Text.AlignTop
                    }
                }
            }

            LabButton {
                Layout.preferredWidth: 82
                Layout.fillHeight: true
                background: null
                text: "G" + window.currentWcs()
                font.family: window.uiFontFamily
                font.pixelSize: 22
                font.bold: true
                textColor: Theme.accentBright
                onClicked: wcsPicker.open()
            }

            LabButton {
                id: helpButton
                Layout.preferredWidth: 48
                Layout.preferredHeight: 48
                Layout.rightMargin: 14
                text: "?"
                font.family: window.uiFontFamily
                font.pixelSize: 25
                font.bold: true
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

    Popup {
        id: wcsPicker
        onClosed: wcsRequest.cancel()
        property string errorText: ""
        onOpened: errorText = ""
        parent: Overlay.overlay
        x: 0
        y: 0
        width: parent.width
        height: parent.height
        padding: 0
        modal: true
        closePolicy: Popup.NoAutoClose

        background: Rectangle {
            color: Theme.page
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 70
                color: Theme.header

                BackButton {
                    anchors.left: parent.left
                    width: 70
                    height: parent.height
                    onClicked: wcsPicker.close()
                }
            }

            Label {
                Layout.fillWidth: true
                Layout.leftMargin: 24
                visible: wcsPicker.errorText.length > 0
                text: wcsPicker.errorText
                color: Theme.danger
                font.pixelSize: 18
            }

            GridLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.margins: 24
                columns: 3
                columnSpacing: 18
                rowSpacing: 18

                Repeater {
                    model: [54, 55, 56, 57, 58, 59]
                    LabButton {
                        required property int modelData
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        text: "G" + modelData
                        font.family: window.uiFontFamily
                        font.pixelSize: 30
                        font.bold: modelData === window.currentWcs()
                        selected: modelData === window.currentWcs()
                        enabled: !wcsRequest.pending
                        onClicked: window.chooseWcs(modelData)
                    }
                }
            }
        }
    }

    Popup {
        id: helpPage
        onOpened: helpScroll.contentItem.contentY = 0
        parent: Overlay.overlay
        x: 0
        y: 0
        width: parent.width
        height: parent.height
        padding: 0
        modal: true
        closePolicy: Popup.NoAutoClose

        background: Rectangle {
            color: Theme.page
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 70
                color: Theme.header

                BackButton {
                    anchors.left: parent.left
                    width: 70
                    height: parent.height
                    onClicked: helpPage.close()
                }

                Label {
                    anchors.centerIn: parent
                    text: ["Outside", "Inside", "Center", "Settings"][tabs.currentIndex] + " help"
                    color: Theme.text
                    font.family: window.uiFontFamily
                    font.pixelSize: 24
                }
            }

            ScrollView {
                id: helpScroll
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                contentWidth: availableWidth
                TextArea {
                    readOnly: true
                    textFormat: TextEdit.MarkdownText
                    baseUrl: Qt.resolvedUrl("help/")
                    text: HelpPages.pages[tabs.currentIndex]
                    color: Theme.text
                    font.family: window.uiFontFamily
                    font.pixelSize: 19
                    wrapMode: TextEdit.WordWrap
                    padding: 28
                    background: null
                }
            }
        }
    }

    Timer {
        interval: 250
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: window.refreshState()
    }

    ProbeFlow {
        id: confirmDialog
        serviceUrl: window.serviceUrl
        uiFont: window.uiFontFamily
        codeFont: window.monoFontFamily
        onSettingChanged: function(key, value) { window.settings.setValue(key, value); }
    }

    RepeatabilityFlow {
        id: repeatabilityDialog
        serviceUrl: window.serviceUrl
        uiFont: window.uiFontFamily
        codeFont: window.monoFontFamily
    }

    UpdateFlow {
        id: updateDialog
        serviceUrl: window.serviceUrl
        uiFont: window.uiFontFamily
    }

    Dialog {
        id: exitDialog
        parent: Overlay.overlay
        anchors.centerIn: parent
        font.family: window.uiFontFamily
        width: 640
        height: 230
        modal: true
        title: "Probe extended"
        standardButtons: Dialog.NoButton
        closePolicy: Popup.NoAutoClose

        header: Label {
            text: exitDialog.title
            padding: 12
            elide: Text.ElideRight
            color: Theme.text
            font.family: window.uiFontFamily
            font.pixelSize: 19
            font.bold: true
            background: Rectangle {
                x: 1
                y: 1
                width: parent.width - 2
                height: parent.height - 1
                color: Theme.panelRaised
            }
        }

        background: Rectangle {
            color: Theme.panelRaised
            border.color: Theme.divider
            radius: 12
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 18
            spacing: 14

            Label {
                Layout.fillWidth: true
                Layout.fillHeight: true
                text: window.exitAfterRetract ? "Retracting probe..." : "Retract the probe before leaving?"
                horizontalAlignment: Text.AlignHCenter
                color: Theme.text
                font.family: window.uiFontFamily
                font.pixelSize: 19
                wrapMode: Text.WordWrap
                verticalAlignment: Text.AlignVCenter
            }
            Label {
                Layout.fillWidth: true
                visible: window.actionError.length > 0
                text: window.actionError
                color: Theme.danger
                font.pixelSize: 16
            }
            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 10
                LabButton {
                    text: "Cancel"
                    font.pixelSize: 22
                    Layout.preferredHeight: 56
                    enabled: !window.exitAfterRetract
                    onClicked: exitDialog.close()
                }
                LabButton {
                    text: "Leave extended"
                    font.pixelSize: 22
                    Layout.preferredHeight: 56
                    enabled: !window.exitAfterRetract
                    onClicked: Qt.quit()
                }
                LabButton {
                    text: "Retract and exit"
                    font.pixelSize: 22
                    Layout.preferredHeight: 56
                    enabled: !window.exitAfterRetract
                    primary: true
                    onClicked: window.retractAndExit()
                }
            }
        }
    }

    Timer {
        id: actionErrorTimer
        interval: 3000
        onTriggered: window.actionError = ""
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            visible: window.settings.error.length > 0
            Label {
                Layout.fillWidth: true
                text: window.settings.error
                color: Theme.danger
                font.pixelSize: 18
            }
            LabButton {
                text: "Retry"
                primary: true
                onClicked: window.settings.loaded ? window.settings.save() : window.settings.load()
            }
        }

        TabBar {
            id: tabs
            onCurrentIndexChanged: numericEditor.cancel()
            Layout.fillWidth: true
            Layout.preferredHeight: 42
            enabled: !window.interactionLocked()
            background: Rectangle { color: Theme.page }

            Repeater {
                model: ["Outside", "Inside", "Center", "Settings"]
                LabTabButton {
                    id: tabButton
                    required property string modelData
                    text: modelData
                    font.family: window.uiFontFamily
                    font.pixelSize: 20
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            enabled: window.settings.loaded && window.activePageAvailable() && !window.interactionLocked()
            opacity: window.activePageAvailable() ? 1 : 0.35

            StackLayout {
                anchors.fill: parent
                currentIndex: tabs.currentIndex

                ProbePanel {
                    settings: window.settings
                    editor: numericEditor
                    parameters: Pages.outside
                    roomy: true
                    ProbeGrid {
                        anchors.centerIn: parent
                        inside: false
                        onSelected: function (routine) {
                            window.openRoutineReview("outside", routine);
                        }
                    }
                }
                ProbePanel {
                    settings: window.settings
                    editor: numericEditor
                    parameters: Pages.inside
                    roomy: true
                    ProbeGrid {
                        anchors.centerIn: parent
                        inside: true
                        onSelected: function (routine) {
                            window.openRoutineReview("inside", routine);
                        }
                    }
                }
                ProbePanel {
                    settings: window.settings
                    editor: numericEditor
                    parameters: Pages.center
                    roomy: true
                    CenterGrid {
                        anchors.centerIn: parent
                        onSelected: function (feature) {
                            window.openRoutineReview("center", feature);
                        }
                    }
                }
                ProbePanel {
                    settings: window.settings
                    editor: numericEditor
                    parameters: Pages.setup
                    setup: true
                    ColumnLayout {
                        anchors.centerIn: parent
                        width: parent.width - 28
                        spacing: 14
                        LabButton {
                            text: "Probe repeatability"
                            Layout.fillWidth: true
                            Layout.preferredHeight: 56
                            font.pixelSize: 19
                            onClicked: {
                                numericEditor.cancel();
                                repeatabilityDialog.showCheck();
                            }
                        }
                        LabButton {
                            text: "Check for updates"
                            Layout.fillWidth: true
                            Layout.preferredHeight: 56
                            font.pixelSize: 19
                            primary: true
                            onClicked: {
                                numericEditor.cancel();
                                updateDialog.show();
                            }
                        }
                    }
                }
            }
            NumericKeypad {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.margins: 14
                width: 340
                visible: numericEditor.target !== null
                onKeyPressed: function (key) {
                    numericEditor.typeKey(key);
                }
                onAccepted: numericEditor.accept()
            }
        }
    }
}
