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

#include <QCoreApplication>
#include <QJsonArray>
#include <QJsonObject>
#include <QPluginLoader>
#include <QDebug>

int main(int argc, char **argv) {
    QCoreApplication app(argc, argv);
    if (argc != 3) {
        return 2;
    }
    const QStringList names = {QStringLiteral("PimProbeProxyPlugin"),
                               QStringLiteral("PimProbeLauncherPlugin")};
    for (int i = 0; i < names.size(); ++i) {
        QPluginLoader loader(QString::fromLocal8Bit(argv[i + 1]));
        const auto metadata = loader.metaData();
        if (metadata.value("IID").toString() !=
                QStringLiteral("com.elephantrobotics.CNC.CNC_Lab.PagePluginInterface/1.0") ||
            metadata.value("className").toString() != names[i] ||
            metadata.value("MetaData").toObject().value("Keys").toArray() !=
                QJsonArray{names[i]}) {
            qCritical() << "Unexpected plugin metadata:" << metadata;
            return 1;
        }
    }
    return 0;
}
