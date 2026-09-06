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
#include <QList>
#include <QMetaObject>
#include <QObject>
#include <QString>

class QMainWindow;
class QWidget;

namespace AllData::HomeData {
struct ToolInfo;
}

// ABI declarations for NestPad's libabstractplugin.so, built against Qt 6.9.1.
class AbstractPlugin : public QObject {
public:
    AbstractPlugin();
    ~AbstractPlugin() override;

    static const QMetaObject staticMetaObject;
    const QMetaObject *metaObject() const override;
    void *qt_metacast(const char *) override;
    int qt_metacall(QMetaObject::Call, int, void **) override;

    virtual void init(QWidget *widget, QMainWindow *mainWindow);
    virtual void initPlugin() = 0;
    virtual QString pluginName() const = 0;
    virtual QByteArray saveState() const = 0;
    virtual bool restoreState(QByteArray &state) = 0;
    virtual void cleanup();
    virtual void onAsyncToolInfoUpdated(
        const QList<AllData::HomeData::ToolInfo> &tools);
    virtual void onAcceptReadTool();
    virtual bool hasScanToolDialogOpen() const;
    virtual void triggerScanToolReRead();
    virtual void startPCRead();
    virtual void pcReadStep(int step, bool accepted,
                            const AllData::HomeData::ToolInfo &tool);
    virtual void closePCRead(bool accepted);
    virtual QList<AllData::HomeData::ToolInfo> scanToolCurrentInfos() const;

    AbstractPlugin *plugin();
    const AbstractPlugin *plugin() const;
    QWidget *widget() const;
    QMainWindow *mainWindow() const;

private:
    QMainWindow *mainWindow_;
    QWidget *widget_;
    QList<QMetaObject::Connection> connections_;
};

class PagePluginInterface : public AbstractPlugin {
public:
    ~PagePluginInterface() override = default;
};

#define NESTPAD_PAGE_PLUGIN_IID \
    "com.elephantrobotics.CNC.CNC_Lab.PagePluginInterface/1.0"
Q_DECLARE_INTERFACE(PagePluginInterface, NESTPAD_PAGE_PLUGIN_IID)

#if defined(__aarch64__)
static_assert(sizeof(AbstractPlugin) == 56,
              "NestPad AbstractPlugin ABI size changed");
#endif
