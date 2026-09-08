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

repo_dir=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
mock_log=$(mktemp "${TMPDIR:-/tmp}/pimprobe-mock-log.XXXXXX")
mock_token="pimprobe-preview-$$-$(date +%s)"
mock_pid=""
ui_pid=""

cleanup() {
	trap - EXIT INT TERM
	[ -z "$ui_pid" ] || kill "$ui_pid" 2>/dev/null || true
	[ -z "$ui_pid" ] || wait "$ui_pid" 2>/dev/null || true
	[ -z "$mock_pid" ] || kill "$mock_pid" 2>/dev/null || true
	[ -z "$mock_pid" ] || wait "$mock_pid" 2>/dev/null || true
	rm -f "$mock_log"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

qt_prefix=${QT_DECLARATIVE_PREFIX:-$(brew --prefix qtdeclarative)}
qml=${QML:-$qt_prefix/bin/qml}

cd "$repo_dir"
node dev/build-help.mjs
node dev/fetch-fonts.mjs
if nc -z 127.0.0.1 8137 >/dev/null 2>&1; then
	echo "Probing preview refused: port 8137 is already in use" >&2
	exit 1
fi
mock_bin=$("$repo_dir/dev/build-service.sh")
"$mock_bin" --mock --listen 127.0.0.1:8137 --ready-token "$mock_token" >"$mock_log" 2>&1 &
mock_pid=$!

ready=false
for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
	response=$(curl --silent --fail http://127.0.0.1:8137/api/v1/mock/ready 2>/dev/null || true)
	if [ "$response" = "$mock_token" ] && kill -0 "$mock_pid" 2>/dev/null; then
		ready=true
		break
	fi
	if ! kill -0 "$mock_pid" 2>/dev/null; then
		break
	fi
	sleep .05
done

if [ "$ready" != true ]; then
	cat "$mock_log" >&2
	exit 1
fi

QT_QUICK_CONTROLS_STYLE=Basic "$qml" "$repo_dir/ui/Preview.qml" &
ui_pid=$!
wait "$ui_pid"
