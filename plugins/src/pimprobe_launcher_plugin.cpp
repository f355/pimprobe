// PIMProbe - touch probing for the Nestworks C500.
// Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
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
// This additional permission applies under GNU GPL version 2 or any later version.
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

#include "pimprobe_launcher_plugin.h"

#include "launcher_button.h"
#include <QApplication>
#include <QDebug>
#include <QLocalSocket>
#include <QMainWindow>
#include <QProcess>
#include <QPushButton>
#include <QSettings>
#include <QTimer>
#include <dlfcn.h>

namespace pimprobe {
PimProbeLauncherPlugin::PimProbeLauncherPlugin() {
    listenForPanelActivity();
    QTimer::singleShot(0, this, &PimProbeLauncherPlugin::tryInstallLauncher);
}
PimProbeLauncherPlugin::~PimProbeLauncherPlugin() { delete button_.data(); }
void PimProbeLauncherPlugin::init(QWidget *widget, QMainWindow *mainWindow) {
    AbstractPlugin::init(widget, mainWindow);
}
void PimProbeLauncherPlugin::initPlugin() {}
QString PimProbeLauncherPlugin::pluginName() const {
    return QStringLiteral("PimProbeLauncherPlugin");
}
QByteArray PimProbeLauncherPlugin::saveState() const { return {}; }
bool PimProbeLauncherPlugin::restoreState(QByteArray &) { return true; }
void PimProbeLauncherPlugin::cleanup() {
    stopped_ = true;
    panel_.close();
    delete button_.data();
    AbstractPlugin::cleanup();
}
void PimProbeLauncherPlugin::listenForPanelActivity() {
    // Newer CNC_Lab builds expose their screen saver through libabstractplugin.
    // Resolve it at runtime so the launcher still loads on older builds.
    static const auto find = reinterpret_cast<void *(*)()>(
        dlsym(RTLD_DEFAULT, "_ZN18ScreenSaverUseCase12FindInstanceEv"));
    static const auto sleeping = reinterpret_cast<bool (*)(const void *)>(
        dlsym(RTLD_DEFAULT, "_ZNK18ScreenSaverUseCase8sleepingEv"));
    static const auto wake = reinterpret_cast<void (*)(void *)>(
        dlsym(RTLD_DEFAULT, "_ZN18ScreenSaverUseCase4WakeEv"));
    static const auto activity = reinterpret_cast<void (*)(void *, bool)>(
        dlsym(RTLD_DEFAULT, "_ZN18ScreenSaverUseCase14RecordActivityEb"));
    if (!find || !sleeping || !wake || !activity) {
        return;
    }
    panel_.setSocketOptions(QLocalServer::UserAccessOption);
    connect(&panel_, &QLocalServer::newConnection, this, [this] {
        while (auto *socket = panel_.nextPendingConnection()) {
            connect(socket, &QLocalSocket::disconnected, socket, &QObject::deleteLater);
            const auto receive = [socket] {
                const QByteArray request = socket->read(1);
                if (request.isEmpty()) {
                    return;
                }
                if (auto *saver = find(); saver && (request == "A" || request == "W")) {
                    // Check on CNC_Lab's GUI thread so an ordinary activity notice
                    // cannot wake the panel just before a late, blind button press.
                    if (!sleeping(saver)) {
                        activity(saver, false);
                    } else if (request == "W") {
                        wake(saver);
                        activity(saver, false);
                    }
                    socket->write("1", 1);
                }
                socket->disconnectFromServer();
            };
            connect(socket, &QLocalSocket::readyRead, socket, receive);
            receive();
        }
    });
    if (!panel_.listen(QStringLiteral("/run/pimprobe-panel.sock"))) {
        qWarning() << "PimProbe could not listen for panel activity:" << panel_.errorString();
    }
}
void PimProbeLauncherPlugin::tryInstallLauncher() {
    if (stopped_ || button_) {
        return;
    }
    for (QWidget *topLevel : QApplication::topLevelWidgets()) {
        auto *window = qobject_cast<QMainWindow *>(topLevel);
        if (!window) {
            continue;
        }
        button_ = installLauncherButton(window, [] {
            const QSettings settings(QStringLiteral("/userdata/.nestpad/config/settings.ini"),
                                     QSettings::IniFormat);
            const bool chinese = settings.value(QStringLiteral("language")).toString()
                                 == QStringLiteral("\u4e2d\u6587");
            const QString language = chinese ? QStringLiteral("zh_CN") : QStringLiteral("en_US");
            if (!QProcess::startDetached(
                    QStringLiteral("/userdata/cnc/apps/pimprobe/bin/pimprobe-ui"),
                    {QStringLiteral("--language=") + language})) {
                qWarning() << "PimProbe launcher could not start UI";
            }
        });
        if (button_) {
            return;
        }
    }
    if (++installAttempts_ < 20) {
        QTimer::singleShot(250, this, &PimProbeLauncherPlugin::tryInstallLauncher);
    }
}
}  // namespace pimprobe
