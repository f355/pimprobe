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

#include "launcher_button.h"

#include <QHBoxLayout>
#include <QMainWindow>
#include <QPushButton>
#include <QSignalSpy>
#include <QTest>
#include <QWidget>

class LauncherButtonTest : public QObject {
    Q_OBJECT

private slots:
    void installsOnceInVendorTopBar();
    void invokesLaunchAction();
    void matchesResizableWifi();
};

static QHBoxLayout *makeTopBar(QMainWindow &window) {
    auto *central = new QWidget(&window);
    auto *layout = new QHBoxLayout(central);
    layout->setObjectName("horizontalLayout");
    auto *navigation = new QWidget(central);
    auto *navigationLayout = new QHBoxLayout(navigation);
    navigationLayout->setObjectName("horizontalLayout_5");
    navigationLayout->addWidget(new QPushButton("Back", navigation));
    navigationLayout->addWidget(new QPushButton("Home", navigation));
    layout->addWidget(navigation);
    layout->addStretch();
    auto *wifi = new QPushButton("Wi-Fi", central);
    wifi->setObjectName("wifi_pb");
    wifi->setFixedSize(60, 70);
    layout->addWidget(wifi);
    window.setCentralWidget(central);
    return layout;
}

void LauncherButtonTest::installsOnceInVendorTopBar() {
    QMainWindow window;
    QHBoxLayout *layout = makeTopBar(window);

    QPushButton *first = pimprobe::installLauncherButton(&window, [] {});
    QPushButton *second = pimprobe::installLauncherButton(&window, [] {});

    QVERIFY(first != nullptr);
    QCOMPARE(second, first);
    QCOMPARE(first->objectName(), QString("pimprobe_launcher_pb"));
    QCOMPARE(first->size(), QSize(60, 70));
    QCOMPARE(layout->indexOf(first) + 1,
             layout->indexOf(window.findChild<QPushButton *>("wifi_pb")));
    QVERIFY(first->text().isEmpty());
    QVERIFY(!first->icon().isNull());
    const auto image = first->icon().pixmap(QSize(40, 50)).toImage();
    QCOMPARE(image.pixelColor(image.width() / 2, 2), QColor(Qt::white));
    QCOMPARE(image.pixelColor(image.width() / 2, image.height() * 37 / 50),
             QColor("#793438"));
    QVERIFY(first->grab().save("launcher-button.png"));
    QCOMPARE(window.findChildren<QPushButton *>("pimprobe_launcher_pb").size(), 1);
}

void LauncherButtonTest::invokesLaunchAction() {
    QMainWindow window;
    makeTopBar(window);
    int launches = 0;
    QPushButton *button = pimprobe::installLauncherButton(
        &window, [&launches] { ++launches; });

    QVERIFY(button != nullptr);
    QTest::mouseClick(button, Qt::LeftButton);
    QCOMPARE(launches, 1);
}

void LauncherButtonTest::matchesResizableWifi() {
    QMainWindow window;
    auto *layout = makeTopBar(window);
    auto *wifi = window.findChild<QPushButton *>("wifi_pb");
    wifi->setMinimumWidth(0);
    wifi->setMaximumWidth(QWIDGETSIZE_MAX);
    wifi->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
    auto *button = pimprobe::installLauncherButton(&window, [] {});
    window.resize(800, 100);
    window.show();
    layout->activate();
    QCOMPARE(button->height(), wifi->height());
    QVERIFY(qAbs(button->width() - wifi->width()) <= 1);
    window.resize(1000, 100);
    layout->activate();
    QCOMPARE(button->height(), wifi->height());
    QVERIFY(qAbs(button->width() - wifi->width()) <= 1);
}

QTEST_MAIN(LauncherButtonTest)
#include "launcher_button_test.moc"
