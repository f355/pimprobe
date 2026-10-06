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

#include <QJsonDocument>
#include <QJsonObject>
#include <QQmlComponent>
#include <QQmlContext>
#include <QQmlEngine>
#include <QQmlExpression>
#include <QQuickItem>
#include <QQuickWindow>
#include <QTest>

class HistoryTransportTest : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantMap reply MEMBER reply)
    QVariantMap reply;

private slots:
    void historyReply_data() {
        QTest::addColumn<QByteArray>("json");
        QTest::addColumn<int>("count");
        QTest::addColumn<QString>("error");
        QTest::newRow("populated") << QByteArray(R"({"ok":true,"data":[
            {"label":"Z surface","timestampMs":1000,"status":"success",
             "config":{"wcs":54,"diameter":2,"depth":1,"z":true,
                       "positioningFeed":100,"coarseFeed":50,"fineFeed":10},
             "result":{"machinePoint":[null,null,-12.5],"spans":[null,null]}},
            {"label":"X edge","timestampMs":2000,"status":"failed","error":"No contact"}
        ]})") << 2 << QString();
        QTest::newRow("empty") << QByteArray(R"({"ok":true,"data":[]})") << 0 << QString();
        QTest::newRow("failed") << QByteArray(R"({"ok":false,"error":"Offline"})") << 0 << QString("Offline");
        QTest::newRow("malformed") << QByteArray(R"({"ok":true,"data":{}})") << 0 << QString("Could not read probing history");
    }

    void historyReply() {
        QFETCH(QByteArray, json);
        QFETCH(int, count);
        QFETCH(QString, error);
        // Match native clients converting JSON replies to Qt containers.
        reply = QJsonDocument::fromJson(json).object().toVariantMap();
        QQmlEngine engine;
        engine.rootContext()->setContextProperty("transport", this);
        QQmlComponent component(&engine);
        component.setData(R"(
            import QtQuick
            import "."
            HistoryFlow {
                client: QtObject {
                    function request(operation, body, done) {
                        if (operation !== "history.get") throw new Error(operation);
                        done(transport.reply);
                        return {abort: function() {}};
                    }
                }
            }
        )", QUrl::fromLocalFile(QStringLiteral(PIMPROBE_UI_DIR "/transport-test.qml")));
        QScopedPointer<QObject> flow(component.create());
        QVERIFY2(flow, qPrintable(component.errorString()));
        QQuickWindow window;
        window.resize(800, 480);
        auto *item = qobject_cast<QQuickItem *>(flow.data());
        QVERIFY(item);
        item->setParentItem(window.contentItem());
        window.show();
        QVERIFY(QMetaObject::invokeMethod(flow.data(), "showHistory"));
        auto evaluate = [&](const QString &expression) {
            QQmlExpression result(engine.rootContext(), flow.data(), expression);
            const QVariant value = result.evaluate();
            if (result.hasError()) qFatal("%s", qPrintable(result.error().toString()));
            return value;
        };
        QCOMPARE(evaluate("entries.length").toInt(), count);
        QCOMPARE(flow->property("errorText").toString(), error);
        QCOMPARE(flow->property("selectedIndex").toInt(), count ? 0 : -1);
        if (count) {
            QCOMPARE(evaluate("selected.label").toString(), QString("Z surface"));
            QVERIFY(evaluate("detail(selected)").toString().contains("Z  -12.500"));
            flow->setProperty("selectedIndex", 1);
            QCOMPARE(evaluate("selected.label").toString(), QString("X edge"));
            QVERIFY(evaluate("detail(selected)").toString().contains("Error: No contact"));
        } else if (error.isEmpty()) {
            bool emptyMessage = false;
            for (QObject *child : flow->findChildren<QObject *>()) {
                if (child->property("text").toString() == "No probing attempts yet")
                    emptyMessage = child->property("visible").toBool();
            }
            QVERIFY(emptyMessage);
        }
    }
};

QTEST_MAIN(HistoryTransportTest)
#include "history_transport_test.moc"
