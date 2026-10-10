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
import "controls"
import QtQuick.Controls
import QtQuick.Layouts

Popup {
    id: flow
    required property var client
    required property ProbeSettings settings
    property string uiFont: "sans-serif"
    property string phase: "loading"
    readonly property bool development: settings.values.developmentUpdates === true
    readonly property bool automaticChecks: settings.values.automaticUpdateChecks !== false
    readonly property bool updateAvailable: available !== null
    property bool startupChecked: false
    property string currentVersion: ""
    property var available: null
    property string errorText: ""
    property string operationId: ""
    property int statusTicks: 0

    parent: Overlay.overlay
    x: 0
    y: 0
    width: parent ? parent.width : 800
    height: parent ? parent.height : 480
    padding: 0
    modal: true
    closePolicy: Popup.NoAutoClose
    font.family: uiFont
    background: Rectangle { color: Theme.page }

    ServiceRequest { client: flow.client; id: checkRequest }
    ServiceRequest { client: flow.client; id: installRequest }
    ServiceRequest { client: flow.client; id: statusRequest }
    Timer {
        id: statusTimer
        interval: 1000
        repeat: true
        onTriggered: flow.pollStatus()
    }
    Connections {
        target: flow.settings
        function onLoadedChanged() { flow.checkOnStartup(); }
    }
    Component.onCompleted: checkOnStartup()

    function checkOnStartup() {
        if (startupChecked || !settings.loaded) return;
        startupChecked = true;
        if (automaticChecks) reload();
    }

    function show() {
        open();
        reload();
    }
    function reload() {
        checkRequest.cancel();
        phase = "loading";
        errorText = "";
        available = null;
        operationId = "";
        checkRequest.send("updates.check", {development:development}, function(reply) {
            if (!reply.ok || !reply.data) {
                phase = "error";
                errorText = reply.error || "Invalid update response";
                return;
            }
            currentVersion = reply.data.currentVersion || "unknown";
            available = reply.data.available || null;
            phase = available ? "available" : "current";
        });
    }
    function install() {
        if (!available || installRequest.pending) return;
        phase = "installing";
        errorText = "";
        statusTicks = 0;
        installRequest.send("updates.install", { token: available.token }, function(reply) {
            if (!reply.ok || !reply.data || !reply.data.operationId) {
                phase = "error";
                errorText = reply.error || "The updater did not return an operation ID";
                return;
            }
            operationId = reply.data.operationId;
            statusTimer.start();
        });
    }
    function pollStatus() {
        if (!operationId) return;
        if (++statusTicks > 90) {
            statusRequest.cancel();
            statusTimer.stop();
            phase = "error";
            errorText = "The installer did not report completion";
            return;
        }
        if (statusRequest.pending) return;
        statusRequest.send("updates.status", {id:operationId}, function(reply) {
            if (!reply.ok || !reply.data) return;
            if (reply.data.state === "failed") {
                statusTimer.stop();
                phase = "error";
                errorText = reply.data.message || "Installation failed";
            }
        });
    }
    onClosed: {
        installRequest.cancel();
        statusRequest.cancel();
        statusTimer.stop();
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        PageHeader {
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.headerHeight
            title: I18n.tr('Software update')
            uiFont: flow.uiFont
            backEnabled: flow.phase !== "installing"
            onBack: flow.close()
        }
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 16
            spacing: 12
            RowLayout {
                Layout.fillWidth: true
                spacing: 12
                Label {
                    Layout.fillWidth: true
                    text: I18n.tr('Installed: %1', [flow.currentVersion === "unknown" ? I18n.tr('unknown') : flow.currentVersion || "—"])
                    color: Theme.textMuted
                    font.pixelSize: 18
                    wrapMode: Text.WordWrap
                }
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: 16
                ProbeSwitch {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 48
                    text: I18n.tr('Check automatically')
                    helpText: I18n.tr("Check the selected update channel at each startup. A marker appears on Settings when an update is available.")
                    font.pixelSize: 20
                    checked: flow.automaticChecks
                    enabled: flow.settings.loaded && flow.phase !== "installing"
                    onClicked: flow.settings.setValue("automaticUpdateChecks", checked)
                }
                ProbeSwitch {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 48
                    text: I18n.tr('Development releases')
                    helpText: I18n.tr("Choose development builds instead of regular releases. The selected channel is saved and checked at startup.")
                    font.pixelSize: 20
                    checked: flow.development
                    enabled: flow.settings.loaded && flow.phase !== "installing"
                    onClicked: {
                        flow.settings.setValue("developmentUpdates", checked);
                        flow.reload();
                    }
                }
            }
            Label {
                Layout.fillWidth: true
                Layout.fillHeight: flow.phase !== "available"
                visible: flow.phase !== "available"
                text: flow.phase === "loading" ? I18n.tr('Checking for updates...')
                    : flow.phase === "current" ? (flow.development ? I18n.tr('You have the latest development build.') : I18n.tr('You have the latest release.'))
                    : flow.phase === "installing" ? I18n.tr('Installing update. The interface will restart.')
                    : I18n.tr(flow.errorText)
                color: flow.phase === "error" ? Theme.danger : Theme.text
                font.pixelSize: 22
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
            Label {
                Layout.fillWidth: true
                visible: flow.phase === "available"
                text: flow.available ? (flow.available.name || flow.available.version) : ""
                color: Theme.text
                font.pixelSize: 24
                font.bold: true
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                visible: flow.phase === "available"
                clip: true
                TextArea {
                    readOnly: true
                    textFormat: TextEdit.MarkdownText
                    text: flow.available ? flow.available.notes : ""
                    color: Theme.text
                    font.pixelSize: 18
                    wrapMode: TextEdit.WordWrap
                    padding: 16
                    background: Rectangle { color: Theme.panel; radius: 8 }
                }
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: 12
                visible: flow.phase === "available" || flow.phase === "error"
                Item { Layout.fillWidth: true }
                LabButton {
                    visible: flow.phase === "error"
                    text: I18n.tr('Try again')
                    helpText: I18n.tr("Check GitHub again for the latest version on the selected channel.")
                    Layout.preferredWidth: 232
                    Layout.preferredHeight: 64
                    onClicked: flow.reload()
                }
                LabButton {
                    visible: flow.phase === "available"
                    text: I18n.tr('Install update')
                    primary: true
                    Layout.preferredWidth: 232
                    Layout.preferredHeight: 64
                    onClicked: installConfirmation.open()
                }
            }
        }
    }

    TouchDialog {
        id: installConfirmation
        title: I18n.tr('Install update')
        message: I18n.tr('Make sure the machine is idle and the spindle is stopped. Install this update now?')
        acceptText: I18n.tr('Install')
        font.family: flow.uiFont
        onAccepted: flow.install()
    }
}
