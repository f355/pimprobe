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

import QtQuick
import "Api.js" as Api

QtObject {
    property bool pending: false
    property int generation: 0
    property var request: null

    function send(method, url, body, done) {
        if (pending)
            return false;
        pending = true;
        var serial = ++generation;
        request = Api.request(method, url, body, function (reply) {
            if (serial !== generation)
                return;
            pending = false;
            request = null;
            done(reply);
        });
        return true;
    }

    function cancel() {
        ++generation;
        var previous = request;
        request = null;
        pending = false;
        if (previous)
            previous.abort();
    }

    Component.onDestruction: cancel()
}
