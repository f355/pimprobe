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

#include "controller_bridge.h"

#include <QDataStream>
#include <QLocalSocket>

namespace pimprobe {

bool connectControllerOutput(QObject *source, QObject *receiver) {
    const auto raw = QObject::connect(
        source, SIGNAL(rawDataReceived(QByteArray)), receiver,
        SLOT(publishRawData(QByteArray)));
    const auto robot = QObject::connect(
        source, SIGNAL(robotinfoSignal(QByteArray)), receiver,
        SLOT(publishControllerData(QByteArray)));
    const auto other = QObject::connect(
        source, SIGNAL(otherinfoSignal(QByteArray)), receiver,
        SLOT(publishControllerData(QByteArray)));
    return raw && robot && other;
}

ControllerBridge::ControllerBridge(QObject *parent) : QObject(parent) {
    server_.setSocketOptions(QLocalServer::UserAccessOption);
    connect(&server_, &QLocalServer::newConnection, this,
            &ControllerBridge::acceptConnections);
}

bool ControllerBridge::listen(const QString &path) {
    return server_.listen(path);
}

QString ControllerBridge::errorString() const { return server_.errorString(); }

QString ControllerBridge::serverName() const { return server_.serverName(); }

void ControllerBridge::acceptConnections() {
    while (server_.hasPendingConnections()) {
        QLocalSocket *candidate = server_.nextPendingConnection();
        if (client_ != nullptr) {
            candidate->disconnectFromServer();
            candidate->deleteLater();
            continue;
        }
        client_ = candidate;
        input_.clear();
        connect(client_, &QLocalSocket::readyRead, this,
                &ControllerBridge::readClientData);
        connect(client_, &QLocalSocket::disconnected, this,
                &ControllerBridge::disconnectClient);
    }
}

void ControllerBridge::readClientData() {
    input_.append(client_->readAll());
    while (input_.size() >= 5) {
        const char kind = input_.at(0);
        QDataStream stream(input_.mid(1, 4));
        stream.setByteOrder(QDataStream::BigEndian);
        quint32 size = 0;
        stream >> size;
        if (size > maxFrameSize_) {
            client_->abort();
            return;
        }
        if ((kind == 'Q' && size > maxQueuedCommandSize_) ||
            (kind == 'R' && size > maxRealtimeCommandSize_) ||
            (kind != 'Q' && kind != 'R')) {
            client_->abort();
            return;
        }
        if (input_.size() < 5 + static_cast<qsizetype>(size)) {
            return;
        }
        const QByteArray payload = input_.mid(5, size);
        input_.remove(0, 5 + size);
        if (kind == 'Q') {
            emit queuedCommandReceived(payload);
        } else if (kind == 'R') {
            emit realtimeCommandReceived(payload);
        }
    }
}

void ControllerBridge::publishControllerData(const QByteArray &data) {
    writeFrame('D', data);
}

void ControllerBridge::publishRawData(const QByteArray &data) {
    rawInput_.append(data);
    for (;;) {
        const qsizetype start = rawInput_.indexOf('<');
        if (start < 0) {
            rawInput_.clear();
            return;
        }
        if (start > 0) {
            rawInput_.remove(0, start);
        }

        const qsizetype end = rawInput_.indexOf('>', 1);
        const qsizetype nestedStart = rawInput_.indexOf('<', 1);
        if (nestedStart >= 0 && (end < 0 || nestedStart < end)) {
            rawInput_.remove(0, nestedStart);
            continue;
        }
        if (end < 0) {
            if (rawInput_.size() > 8192) {
                rawInput_.clear();
            }
            return;
        }

        const QByteArray candidate = rawInput_.left(end + 1);
        rawInput_.remove(0, end + 1);
        bool printable = true;
        for (char byte : candidate) {
            if (byte < 0x20 || byte > 0x7e) {
                printable = false;
                break;
            }
        }
        if (printable && candidate.contains("|MPos:")) {
            publishControllerData(candidate);
        }
    }
}

void ControllerBridge::writeFrame(char kind, const QByteArray &payload) {
    if (client_ == nullptr || payload.size() > maxFrameSize_) {
        return;
    }
    QByteArray frame;
    QDataStream stream(&frame, QIODevice::WriteOnly);
    stream.setByteOrder(QDataStream::BigEndian);
    stream << static_cast<quint8>(kind)
           << static_cast<quint32>(payload.size());
    frame.append(payload);
    if (client_->bytesToWrite() + frame.size() > maxPendingOutput_ ||
        client_->write(frame) != frame.size()) {
        client_->abort();
    }
}

void ControllerBridge::disconnectClient() {
    client_->deleteLater();
    client_ = nullptr;
    input_.clear();
}

}  // namespace pimprobe
