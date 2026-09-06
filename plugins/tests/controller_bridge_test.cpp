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

#include "controller_bridge.h"

#include <QDataStream>
#include <QLocalSocket>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QtTest>
#include <cstring>

#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

namespace {

class ControllerSignalSource : public QObject {
    Q_OBJECT

signals:
    void rawDataReceived(const QByteArray &data);
    void robotinfoSignal(const QByteArray &data);
    void otherinfoSignal(const QByteArray &data);
};

QByteArray frame(char kind, const QByteArray &payload) {
    QByteArray result;
    QDataStream stream(&result, QIODevice::WriteOnly);
    stream.setByteOrder(QDataStream::BigEndian);
    stream << static_cast<quint8>(kind)
           << static_cast<quint32>(payload.size());
    result.append(payload);
    return result;
}

class ControllerBridgeTest : public QObject {
    Q_OBJECT

private slots:
    void exchangesCommandsAndControllerData();
    void rejectsUnknownFrameType();
    void forwardsOnlyParsedControllerRecords();
    void replacesStaleSocketPath();
    void rejectsOversizedQueuedCommand();
    void disconnectsClientThatStopsReading();
};

void ControllerBridgeTest::exchangesCommandsAndControllerData() {
    QTemporaryDir directory;
    QVERIFY(directory.isValid());
    const QString path = directory.filePath(QStringLiteral("controller.sock"));

    pimprobe::ControllerBridge bridge;
    QVERIFY2(bridge.listen(path), qPrintable(bridge.errorString()));
    QSignalSpy queued(&bridge,
                      &pimprobe::ControllerBridge::queuedCommandReceived);

    QLocalSocket socket;
    socket.connectToServer(path);
    QVERIFY(socket.waitForConnected());
    QCOMPARE(socket.write(frame('Q', QByteArrayLiteral("$P\n"))), 8);
    QVERIFY(socket.waitForBytesWritten());
    QTRY_COMPARE(queued.size(), 1);
    QCOMPARE(queued.takeFirst().at(0).toByteArray(), QByteArrayLiteral("$P\n"));

    bridge.publishControllerData(
        QByteArrayLiteral("<Ready|MPos:1,2,3,0|PM:0>\r\n"));
    QTRY_VERIFY(socket.bytesAvailable() >= 5);
    const QByteArray header = socket.read(5);
    QCOMPARE(header.at(0), 'D');
    QDataStream sizeStream(header.mid(1));
    sizeStream.setByteOrder(QDataStream::BigEndian);
    quint32 size = 0;
    sizeStream >> size;
    QTRY_VERIFY(socket.bytesAvailable() >= size);
    QCOMPARE(socket.read(size),
             QByteArrayLiteral("<Ready|MPos:1,2,3,0|PM:0>\r\n"));
}

void ControllerBridgeTest::rejectsUnknownFrameType() {
    QTemporaryDir directory;
    QVERIFY(directory.isValid());
    pimprobe::ControllerBridge bridge;
    QVERIFY(bridge.listen(directory.filePath(QStringLiteral("controller.sock"))));

    QLocalSocket socket;
    socket.connectToServer(bridge.serverName());
    QVERIFY(socket.waitForConnected());
    socket.write(frame('?', QByteArrayLiteral("bad")));
    socket.flush();
    QTRY_COMPARE(socket.state(), QLocalSocket::UnconnectedState);
}

void ControllerBridgeTest::forwardsOnlyParsedControllerRecords() {
    QTemporaryDir directory;
    QVERIFY(directory.isValid());
    pimprobe::ControllerBridge bridge;
    QVERIFY(bridge.listen(directory.filePath(QStringLiteral("controller.sock"))));
    ControllerSignalSource source;
    QVERIFY(pimprobe::connectControllerOutput(&source, &bridge));

    QLocalSocket socket;
    socket.connectToServer(bridge.serverName());
    QVERIFY(socket.waitForConnected());
    QCoreApplication::processEvents();

    emit source.rawDataReceived(
        QByteArray::fromHex("1a9b") +
        QByteArrayLiteral("<Ready|MPos:1.000,2.000,3.000,0.000"));
    emit source.rawDataReceived(
        QByteArrayLiteral("|WPos:4.000,5.000,6.000,0.000>") +
        QByteArray::fromHex("00ff"));

    emit source.robotinfoSignal(QByteArrayLiteral("<Ready>\r\n"));
    emit source.otherinfoSignal(QByteArrayLiteral("$33=-55.872\r\n"));
    QTRY_VERIFY(socket.bytesAvailable() >= 10);
    QCOMPARE(socket.readAll(),
             frame('D', QByteArrayLiteral(
                            "<Ready|MPos:1.000,2.000,3.000,0.000|WPos:4.000,5.000,6.000,0.000>")) +
                 frame('D', QByteArrayLiteral("<Ready>\r\n")) +
                 frame('D', QByteArrayLiteral("$33=-55.872\r\n")));
}

void ControllerBridgeTest::replacesStaleSocketPath() {
    QTemporaryDir directory;
    QVERIFY(directory.isValid());
    const QByteArray path =
        directory.filePath(QStringLiteral("controller.sock")).toLocal8Bit();
    const int descriptor = ::socket(AF_UNIX, SOCK_STREAM, 0);
    QVERIFY(descriptor >= 0);
    sockaddr_un address{};
    address.sun_family = AF_UNIX;
    QVERIFY(path.size() < static_cast<int>(sizeof(address.sun_path)));
    std::memcpy(address.sun_path, path.constData(), path.size() + 1);
    QVERIFY(::bind(descriptor, reinterpret_cast<sockaddr *>(&address),
                   sizeof(address)) == 0);
    ::close(descriptor);

    pimprobe::ControllerBridge bridge;
    QVERIFY2(bridge.listen(QString::fromLocal8Bit(path)),
             qPrintable(bridge.errorString()));
}

void ControllerBridgeTest::rejectsOversizedQueuedCommand() {
    QTemporaryDir directory;
    QVERIFY(directory.isValid());
    pimprobe::ControllerBridge bridge;
    QVERIFY(bridge.listen(directory.filePath(QStringLiteral("controller.sock"))));
    QSignalSpy queued(&bridge,
                      &pimprobe::ControllerBridge::queuedCommandReceived);
    QLocalSocket socket;
    socket.connectToServer(bridge.serverName());
    QVERIFY(socket.waitForConnected());
    socket.write(frame('Q', QByteArray(4097, 'G')));
    QVERIFY(socket.waitForBytesWritten());
    QTest::qWait(50);
    QCOMPARE(queued.size(), 0);
    QCOMPARE(socket.state(), QLocalSocket::UnconnectedState);
}

void ControllerBridgeTest::disconnectsClientThatStopsReading() {
    QTemporaryDir directory;
    QVERIFY(directory.isValid());
    pimprobe::ControllerBridge bridge;
    QVERIFY(bridge.listen(directory.filePath(QStringLiteral("controller.sock"))));
    QLocalSocket socket;
    socket.setReadBufferSize(1);
    socket.connectToServer(bridge.serverName());
    QVERIFY(socket.waitForConnected());
    QCoreApplication::processEvents();

    const QByteArray record(65536, 'X');
    for (int i = 0; i < 128 && socket.state() == QLocalSocket::ConnectedState;
         ++i) {
        bridge.publishControllerData(record);
    }
    QTRY_COMPARE(socket.state(), QLocalSocket::UnconnectedState);
}

}  // namespace

QTEST_MAIN(ControllerBridgeTest)
#include "controller_bridge_test.moc"
