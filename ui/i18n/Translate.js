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

.pragma library
.import "Catalog.js" as Catalog

var templates = {};

function translate(language, source, args) {
    var dictionary = Catalog.languages[language] || {};
    var text = dictionary[source] || source;
    return String(text).replace(/%([1-9]\d*)/g, function(match, number) {
        return args && args[number - 1] !== undefined ? String(args[number - 1]) : match;
    });
}

function patterns(language) {
    if (templates[language]) return templates[language];
    var dictionary = Catalog.languages[language] || {};
    templates[language] = Object.keys(dictionary).filter(function(source) {
        return /%[1-9]\d*/.test(source);
    }).sort(function(a, b) { return b.length - a.length; }).map(function(source) {
        var groups = [];
        var pattern = "^";
        var offset = 0;
        var placeholder = /%([1-9]\d*)/g;
        var match;
        function escape(text) { return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"); }
        while ((match = placeholder.exec(source)) !== null) {
            pattern += escape(source.slice(offset, match.index));
            var group = groups.indexOf(Number(match[1]));
            if (group >= 0) {
                pattern += "\\" + (group + 1) + "(?:)";
            } else {
                groups.push(Number(match[1]));
                pattern += "(.+?)";
            }
            offset = placeholder.lastIndex;
        }
        pattern += escape(source.slice(offset)) + "$";
        return {source:source, groups:groups, expression:new RegExp(pattern)};
    });
    return templates[language];
}

// Match formatted service messages while preserving machine coordinates and codes.
function message(language, text) {
    if (!text || language === "en") return text;
    var translated = translate(language, text, []);
    if (translated !== text) return translated;
    if (text.indexOf("\n") >= 0)
        return text.split("\n").map(function(line) { return message(language, line); }).join("\n");
    var candidates = patterns(language);
    for (var i = 0; i < candidates.length; ++i) {
        var candidate = candidates[i];
        var match = candidate.expression.exec(text);
        if (!match) continue;
        var args = [];
        candidate.groups.forEach(function(number, index) {
            args[number - 1] = message(language, match[index + 1]);
        });
        return translate(language, candidate.source, args);
    }
    return text;
}
