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

#include "launcher_button.h"

#include <QHBoxLayout>
#include <QMainWindow>
#include <QPainter>
#include <QPointer>
#include <QPushButton>
#include <QWidget>

#include <utility>

namespace pimprobe {

namespace {
class LauncherButton final : public QPushButton {
public:
    explicit LauncherButton(QPushButton *reference)
        : QPushButton(reference->parentWidget()), reference_(reference) {
        setMinimumSize(reference->minimumSize());
        setMaximumSize(reference->maximumSize());
        setSizePolicy(reference->sizePolicy());
    }

    QSize sizeHint() const override {
        return reference_ ? reference_->sizeHint() : QPushButton::sizeHint();
    }
    QSize minimumSizeHint() const override {
        return reference_ ? reference_->minimumSizeHint() : QPushButton::minimumSizeHint();
    }

private:
    QPointer<QPushButton> reference_;
};
}

QPushButton *installLauncherButton(QMainWindow *window,
                                   std::function<void()> launch) {
    if (window == nullptr) {
        return nullptr;
    }

    if (auto *existing =
            window->findChild<QPushButton *>("pimprobe_launcher_pb")) {
        return existing;
    }

    auto *wifi = window->findChild<QPushButton *>("wifi_pb");
    if (wifi == nullptr || wifi->parentWidget() == nullptr) {
        return nullptr;
    }
    auto *layout = qobject_cast<QHBoxLayout *>(wifi->parentWidget()->layout());
    if (layout == nullptr || layout->indexOf(wifi) < 0) {
        return nullptr;
    }

    auto *button = new LauncherButton(wifi);
    button->setObjectName("pimprobe_launcher_pb");
    button->setAccessibleName("Probing");
    button->setToolTip("Probing");
    QPixmap icon(80, 100);
    icon.setDevicePixelRatio(2);
    icon.fill(Qt::transparent);
    {
        QPainter painter(&icon);
        painter.setRenderHint(QPainter::Antialiasing);
        painter.setPen(Qt::NoPen);
        painter.setBrush(Qt::white);
        painter.drawRect(QRectF(17, 0, 6, 34));
        painter.setBrush(QColor("#793438"));
        painter.drawEllipse(QPointF(20, 37), 10, 10);
    }
    button->setIcon(QIcon(icon));
    button->setIconSize(QSize(40, 50));
    button->setFocusPolicy(Qt::NoFocus);
    button->setStyleSheet(
        "QPushButton {"
        "background: #414141; border: none; border-radius: 10px;"
        "color: #D8D8D8; font-family: Inter; font-size: 18px;"
        "padding: 0px;"
        "}"
        "QPushButton:pressed { background: #797979; }");
    QObject::connect(button, &QPushButton::clicked, button,
                     [launch = std::move(launch)] { launch(); });
    const int wifiIndex = layout->indexOf(wifi);
    layout->insertWidget(wifiIndex, button, layout->stretch(wifiIndex));
    return button;
}

}  // namespace pimprobe
