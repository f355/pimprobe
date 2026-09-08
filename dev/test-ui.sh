#!/bin/sh
# PIMProbe - touch probing for the Nestworks C500.
# Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

set -eu
repo=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/pimprobe-ui-test.XXXXXX")
mock_pid=""
ui_pid=""
cleanup() {
    trap - EXIT INT TERM
    [ -z "$ui_pid" ] || kill "$ui_pid" 2>/dev/null || true
    [ -z "$ui_pid" ] || wait "$ui_pid" 2>/dev/null || true
    [ -z "$mock_pid" ] || kill "$mock_pid" 2>/dev/null || true
    [ -z "$mock_pid" ] || wait "$mock_pid" 2>/dev/null || true
    rm -rf "$scratch"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
cd "$repo"
node dev/build-help.mjs
node dev/fetch-fonts.mjs
if nc -z 127.0.0.1 18137 >/dev/null 2>&1; then
    echo 'Refusing to use occupied UI-test port 18137' >&2
    exit 1
fi
mock_bin=$("$repo/dev/build-service.sh")
HOME="$scratch" XDG_CONFIG_HOME="$scratch" "$mock_bin" --mock --listen 127.0.0.1:18137 --ready-token qml-test --settings "$scratch/settings.json" >"$scratch/mock.log" 2>&1 &
mock_pid=$!
ready=false
for attempt in 1 2 3 4 5 6 7 8 9 10; do
    response=$(curl -fsS http://127.0.0.1:18137/api/v1/mock/ready 2>/dev/null || true)
    if [ "$response" = qml-test ] && kill -0 "$mock_pid" 2>/dev/null; then ready=true; break; fi
    kill -0 "$mock_pid" 2>/dev/null || break
    sleep .1
done
if [ "$ready" != true ]; then cat "$scratch/mock.log" >&2; exit 1; fi
qt_prefix=${QT_DECLARATIVE_PREFIX:-$(brew --prefix qtdeclarative)}
QT_QPA_PLATFORM=offscreen QT_QUICK_CONTROLS_STYLE=Basic "${QMLTESTRUNNER:-$qt_prefix/bin/qmltestrunner}" -input "${1:-ui/tests}" &
ui_pid=$!
wait "$ui_pid"
