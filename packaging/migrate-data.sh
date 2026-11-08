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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.

set -eu
userdata=${1:-/userdata}
home=${2:-/root}
root=$userdata/cnc
mkdir -p "$root/config" "$root/logs" "$root/cache" "$root/backups" "$root/apps"

copy_setting() {
    source=$1
    target=$2
    if [ -f "$source" ] && [ ! -e "$target" ]; then
        temporary=$(mktemp "$target.XXXXXX")
        cp -p "$source" "$temporary"
        sync
        mv "$temporary" "$target"
        sync
    fi
}

copy_setting "$userdata/pimprobe/settings.json" "$root/config/probing.json"
copy_setting "$home/.config/pimprobe/settings.json" "$root/config/probing.json"
copy_setting "$userdata/pimprobe/config.json" "$root/config/pimprobe-service.json"

if [ ! -e "$root/logs/pimprobe" ]; then
    source=$userdata/pimprobe-data
    if [ ! -d "$source" ]; then source=$home/.local/share/pimprobe; fi
    if [ -d "$source" ]; then
        temporary=$(mktemp -d "$root/logs/pimprobe-migrate.XXXXXX")
        cp -a "$source/." "$temporary/"
        sync
        mv "$temporary" "$root/logs/pimprobe"
        sync
    fi
fi
