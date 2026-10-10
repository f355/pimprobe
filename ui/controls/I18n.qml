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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

pragma Singleton
import QtQuick
import "../i18n/Translate.js" as Translate

QtObject {
    property string systemLanguage: launchLanguage(Qt.application.arguments)
    property string language: resolve("", systemLanguage)

    function launchLanguage(commandLine) {
        for (var i = 0; i < commandLine.length; ++i) {
            if (commandLine[i].indexOf("--language=") === 0)
                return commandLine[i].slice(11);
        }
        return "";
    }

    function resolve(preference, systemLanguage) {
        var name = String(preference || systemLanguage || Qt.locale().name).replace(/-/g, "_").toLowerCase();
        if (name === "zh" || name.indexOf("zh_cn") === 0 || name.indexOf("zh_hans") === 0)
            return "zh_CN";
        if (name === "sv" || name.indexOf("sv_") === 0)
            return "sv";
        return "en";
    }

    function tr(source, args) {
        return args ? Translate.translate(language, source, args) : Translate.message(language, source || "");
    }
}
