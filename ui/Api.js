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

.pragma library

function request(method, url, body, done) {
    var xhr = new XMLHttpRequest()
    xhr.onreadystatechange = function() {
        if (xhr.readyState !== XMLHttpRequest.DONE) return
        var response = {ok: xhr.status >= 200 && xhr.status < 300,
                        status: xhr.status, data: null, error: ""}
        if (xhr.responseText.length) {
            try { response.data = JSON.parse(xhr.responseText) }
            catch (error) {
                response.error = response.ok ? "Invalid service response" : xhr.responseText.trim()
                response.ok = false
            }
        }
        if (response.data && response.data.error === true) {
            response.ok = false
            response.error = response.data.message || "Service error"
        }
        if (!response.ok && !response.error)
            response.error = response.data && response.data.message
                           ? response.data.message : "Service unavailable"
        done(response)
    }
    xhr.open(method, url)
    if (body !== null) xhr.setRequestHeader("Content-Type", "application/json")
    xhr.send(body === null ? undefined : JSON.stringify(body))
    return xhr
}

// Consume complete NDJSON records as they arrive, preserving partial lines.
function stream(url, body, onEvent, onFinished) {
    var xhr = new XMLHttpRequest()
    var consumed = 0
    var finished = false
    function finish(error) {
        if (finished) return
        finished = true
        onFinished(error)
    }
    xhr.onreadystatechange = function() {
        if (finished || (xhr.readyState !== XMLHttpRequest.LOADING && xhr.readyState !== XMLHttpRequest.DONE)) return
        var text = xhr.responseText
        var end
        while ((end = text.indexOf("\n", consumed)) !== -1) {
            var line = text.substring(consumed, end)
            consumed = end + 1
            if (!line.length) continue
            var event
            try {
                event = JSON.parse(line)
                if (event.error === true) {
                    finish(event.message || "Service error")
                    xhr.abort()
                    return
                }
                onEvent(event)
            }
            catch (error) {
                finish("Invalid routine response")
                xhr.abort()
                return
            }
        }
        if (xhr.readyState === XMLHttpRequest.DONE) {
            var tail = text.substring(consumed).trim()
            if (xhr.status !== 200) {
                var message = "Service unavailable"
                try { message = JSON.parse(text).message || message } catch (_) {}
                finish(message)
            }
            else if (tail.length) {
                try {
                    var envelope = JSON.parse(tail)
                    finish(envelope.error === true ? envelope.message || "Service error"
                                                   : "Incomplete routine response")
                } catch (_) {
                    finish("Incomplete routine response")
                }
            }
            else finish("")
        }
    }
    xhr.open("POST", url)
    xhr.setRequestHeader("Content-Type", "application/json")
    xhr.send(JSON.stringify(body))
    return xhr
}
