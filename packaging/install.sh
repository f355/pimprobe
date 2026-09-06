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
payload=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
destination=/userdata/pimprobe
vendor=/root/app
unit=/etc/systemd/system/pimprobe-service.service
restart=ask
yes=false
check=false
uninstall=false
for arg in "$@"; do
    case "$arg" in
        --uninstall) uninstall=true ;;
        --yes) yes=true ;;
        --restart) restart=yes ;;
        --no-restart) restart=no ;;
        --check-target) check=true ;;
        *) echo "Unknown option: $arg" >&2; exit 1 ;;
    esac
done
fail() { echo "$*" >&2; exit 1; }
[ "$(uname -s)" = Linux ] && [ "$(uname -m)" = aarch64 ] || fail 'This installer requires Linux ARM64.'
[ "$(id -u)" = 0 ] || fail 'Run this installer as root.'
[ "$uninstall" = false ] || [ "$check" = false ] || fail '--uninstall cannot be combined with --check-target.'
for command in systemctl flock pgrep readlink; do
    command -v "$command" >/dev/null 2>&1 || fail "Required command missing: $command"
done
[ -x "$vendor/CNC_Lab" ] || fail 'Supported NestPad application not found in /root/app.'
if [ "$uninstall" = false ]; then
    [ -x /usr/bin/qmlscene ] && [ -d /usr/qml/QtQuick/Controls/Basic ] || fail 'Required Qt Quick runtime is missing.'
    for file in "$payload/app/bin/pimprobe-service" "$payload/app/lib/"*.so; do
        report=$(LD_LIBRARY_PATH="$vendor" ldd "$file" 2>&1) || fail "Cannot load $file: $report"
        case "$report" in *'not found'*) fail "Missing runtime dependency for $file: $report" ;; esac
    done
fi
[ -d /userdata ] && [ -d /etc/systemd/system ] || fail 'Expected machine filesystem layout is missing.'
for name in libpimprobeproxyplugin.so libpimprobelauncherplugin.so; do
    link=$vendor/$name
    if [ -e "$link" ] || [ -L "$link" ]; then
        [ -L "$link" ] && [ "$(readlink "$link")" = "$destination/lib/$name" ] || fail "Refusing to replace an unexpected plugin: $link"
    fi
done
if [ "$check" = true ]; then
    echo 'Target layout and runtime dependencies OK. Plugin compatibility is limited to supported NestPad firmware.'
    exit 0
fi
echo 'The machine must be idle with the spindle stopped.'
if [ "$uninstall" = true ]; then
    echo 'Uninstall permanently deletes PIMProbe settings, configuration and backups.'
fi
if [ "$yes" = false ]; then
    if [ "$uninstall" = true ]; then
        printf 'Remove PIMProbe and all its data? [y/N] '
    else
        printf 'Install PIMProbe now? [y/N] '
    fi
    read -r answer || exit 1
    case "$answer" in y|Y|yes) ;; *) echo 'Cancelled.'; exit 0 ;; esac
fi
if [ "$restart" = ask ]; then
    if [ "$yes" = true ]; then
        restart=no
    else
        printf 'Restart CNC_Lab to finish now? [y/N; otherwise reboot later] '
        read -r answer || exit 1
        case "$answer" in y|Y|yes) restart=yes ;; *) restart=no ;; esac
    fi
fi
# Serialize installation and removal with a directory lock.
exec 8</userdata
flock -n 8 || fail 'Another installation or uninstall is running.'
stage=''
backup=''
changed=false
cleanup() {
    status=$?
    trap - EXIT
    if [ -n "$stage" ]; then rm -rf "$stage"; fi
    if [ "$status" != 0 ] && [ "$changed" = true ]; then
        echo "Operation stopped after changes. Backup (if installing): $backup. Do not use probing until repaired." >&2
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
if [ "$uninstall" = false ]; then
    stage=$(mktemp -d /userdata/pimprobe-stage.XXXXXX)
    cp -a "$payload/app/." "$stage/"
    chown -R root:root "$stage"
    if [ -d "$destination" ]; then
        # Carry saved configuration into the staged installation.
        for file in config.json settings.json; do
            if [ -f "$destination/$file" ]; then cp -p "$destination/$file" "$stage/$file"; fi
        done
    fi
    mkdir -p /userdata/backups
    backup=$(mktemp -d /userdata/backups/pimprobe-install.XXXXXX)
    if [ -e "$unit" ]; then cp -a "$unit" "$backup/"; fi
    for name in libpimprobeproxyplugin.so libpimprobelauncherplugin.so; do
        if [ -L "$vendor/$name" ]; then cp -a "$vendor/$name" "$backup/"; fi
    done
fi
# Stop only our UI. The vendor application remains running unless restart was requested.
if [ -f /run/pimprobe-ui.pid ]; then
    pid=$(cat /run/pimprobe-ui.pid)
    if [ -r "/proc/$pid/cmdline" ]; then
        case "$(tr '\0' ' ' <"/proc/$pid/cmdline")" in
            *'/userdata/pimprobe/bin/pimprobe-ui'*)
                kill "$pid"
                for attempt in 1 2 3 4 5; do
                    if ! kill -0 "$pid" 2>/dev/null; then break; fi
                    sleep 1
                done
                if kill -0 "$pid" 2>/dev/null; then fail 'Probing UI did not exit.'; fi
                ;;
            *) fail 'Unexpected process in the probing UI PID file.' ;;
        esac
    fi
fi
if systemctl is-active --quiet pimprobe-service; then systemctl stop pimprobe-service; fi
changed=true
if [ "$uninstall" = true ]; then
    if [ -e "$unit" ] || [ -L /etc/systemd/system/multi-user.target.wants/pimprobe-service.service ]; then
        systemctl disable pimprobe-service
    fi
    rm -f "$vendor/libpimprobeproxyplugin.so" "$vendor/libpimprobelauncherplugin.so"
    rm -rf "$unit" "$unit.d" /etc/systemd/system/multi-user.target.wants/pimprobe-service.service
    rm -rf "$destination" /root/.config/pimprobe /root/.local/share/pimprobe /root/.cache/pimprobe
    rm -rf /userdata/backups/pimprobe-* /userdata/pimprobe-stage.* /userdata/pimprobe-update.*
    rm -f /run/pimprobe-controller.sock /run/pimprobe-ui.pid /run/pimprobe-ui.lock /run/pimprobe-install.lock
    systemctl daemon-reload
    echo 'PIMProbe and its data removed. Installer files and shared system journals were left intact.'
else
    if [ -e "$destination" ]; then mv "$destination" "$backup/installation"; fi
    mv "$stage" "$destination"
    for name in libpimprobeproxyplugin.so libpimprobelauncherplugin.so; do
        ln -sfn "$destination/lib/$name" "$vendor/$name"
    done
    cp "$payload/pimprobe-service.service" "$unit"
    systemctl daemon-reload
    systemctl enable pimprobe-service
    echo "Installed $(cat "$destination/VERSION"). Backup: $backup"
fi
if [ "$restart" = no ]; then
    if [ "$uninstall" = true ]; then
        echo 'Reboot to unload the plugins and remove the button from CNC_Lab.'
        exit 0
    fi
    echo 'Probing service is stopped. Reboot the machine before using PIMProbe.'
    exit 0
fi
# Start CNC_Lab directly; S99nestworks also starts wpa_supplicant.
for pid in $(pgrep -x CNC_Lab || true); do
    [ "$(readlink "/proc/$pid/exe")" = "$vendor/CNC_Lab" ] || fail 'Unexpected CNC_Lab executable.'
    kill "$pid"
done
for attempt in 1 2 3 4 5 6 7 8 9 10; do
    if ! pgrep -x CNC_Lab >/dev/null; then break; fi
    sleep 1
done
if pgrep -x CNC_Lab >/dev/null; then fail 'CNC_Lab did not exit; reboot before using probing.'; fi
log=/tmp/cnc-lab.log
if [ "$uninstall" = false ]; then log=$destination/cnc-lab.log; fi
(
    # Vendor login profiles reference optional, unset environment variables.
    set +u
    . /etc/profile
    set -u
    export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/var/run}
    export QT_QPA_PLATFORM=${QT_QPA_PLATFORM:-wayland}
    cd "$vendor"
    nohup ./CNC_Lab >"$log" 2>&1 </dev/null 8>&- &
)
if [ "$uninstall" = false ]; then systemctl start pimprobe-service; fi
sleep 1
pgrep -x CNC_Lab >/dev/null || fail "CNC_Lab did not start. Check $log."
if [ "$uninstall" = true ]; then
    echo 'CNC_Lab restarted without PIMProbe.'
    exit 0
fi
systemctl is-active --quiet pimprobe-service || fail 'Probing service did not start. Check journalctl -u pimprobe-service.'
echo 'CNC_Lab restarted. Use its probe button to open the UI.'
