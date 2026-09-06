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

#pragma once

#include <QByteArray>
#include <QLocalServer>
#include <QObject>
#include <QString>

class QLocalSocket;

namespace pimprobe {

bool connectControllerOutput(QObject *source, QObject *receiver);

class ControllerBridge : public QObject {
    Q_OBJECT

public:
    explicit ControllerBridge(QObject *parent = nullptr);

    bool listen(const QString &path);
    QString errorString() const;
    QString serverName() const;

public slots:
    void publishControllerData(const QByteArray &data);
    void publishRawData(const QByteArray &data);

signals:
    void queuedCommandReceived(const QByteArray &command);
    void realtimeCommandReceived(const QByteArray &command);

private:
    void acceptConnections();
    void readClientData();
    void disconnectClient();
    void writeFrame(char kind, const QByteArray &payload);

    static constexpr quint32 maxFrameSize_ = 1024 * 1024;
    static constexpr quint32 maxQueuedCommandSize_ = 4096;
    static constexpr quint32 maxRealtimeCommandSize_ = 64;
    static constexpr qint64 maxPendingOutput_ = 256 * 1024;
    QLocalServer server_;
    QLocalSocket *client_ = nullptr;
    QByteArray input_;
    QByteArray rawInput_;
};

}  // namespace pimprobe
