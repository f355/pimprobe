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
//
// Additional permission under GNU GPL version 3 section 7
//
// If you modify this Program, or any covered work, by linking or combining
// it with the CNC_Lab application, libabstractplugin.so, or the vendor
// libraries required by their plugin interfaces (or modified versions of
// those components), containing parts covered by the proprietary license
// terms of Nestworks and/or Elephant Robotics, the licensors of this
// Program grant you additional permission to convey the resulting work.
//
// This permission does not change the license terms of those vendor
// components or grant permission to redistribute them. All other GNU GPL
// obligations for the covered work remain in effect.

#include "pimprobe_proxy_plugin.h"

#include "controller_bridge.h"
#include "nestpad_serial_abi.h"
#include <QDebug>
#include <QTimer>

namespace pimprobe {
PimProbeProxyPlugin::PimProbeProxyPlugin() {
    QTimer::singleShot(0, this, &PimProbeProxyPlugin::startControllerBridge);
}

void PimProbeProxyPlugin::init(QWidget *widget, QMainWindow *mainWindow) {
    AbstractPlugin::init(widget, mainWindow);
    startControllerBridge();
}
void PimProbeProxyPlugin::initPlugin() { startControllerBridge(); }
QString PimProbeProxyPlugin::pluginName() const {
    return QStringLiteral("PimProbeProxyPlugin");
}
QByteArray PimProbeProxyPlugin::saveState() const { return {}; }
bool PimProbeProxyPlugin::restoreState(QByteArray &) { return true; }
void PimProbeProxyPlugin::cleanup() {
    stopped_ = true;
    delete bridge_;
    bridge_ = nullptr;
    AbstractPlugin::cleanup();
}

void PimProbeProxyPlugin::startControllerBridge() {
    if (stopped_ || bridge_) {
        return;
    }
    auto *manager = SerialThreadManager::getInstance();
    if (!manager) {
        qWarning() << "PimProbe controller manager is unavailable";
        return;
    }
    auto *bridge = new ControllerBridge(this);
    connect(bridge, &ControllerBridge::queuedCommandReceived, manager,
            [manager](const QByteArray &command) { manager->addWaiteTeam(command); });
    connect(bridge, &ControllerBridge::realtimeCommandReceived, manager,
            [manager](const QByteArray &command) { manager->addRawRealtime(command); });
    if (!connectControllerOutput(manager, bridge)) {
        qWarning() << "PimProbe could not subscribe to controller records";
        delete bridge;
        return;
    }
    const auto path = QStringLiteral("/run/pimprobe-controller.sock");
    if (!bridge->listen(path)) {
        qWarning() << "PimProbe controller bridge could not listen on"
                   << path << bridge->errorString();
        delete bridge;
        return;
    }
    bridge_ = bridge;
}
}  // namespace pimprobe
