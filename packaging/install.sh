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
root=/userdata/cnc
destination=$root/apps/pimprobe
legacy=/userdata/pimprobe
previous_destination=$destination
if [ ! -d "$destination" ] && [ -d "$legacy" ]; then previous_destination=$legacy; fi
vendor=/root/app
unit=/etc/systemd/system/pimprobe-service.service
plugin_hook=/etc/systemd/system/nestpad.service.d/pimprobe.conf
restart=yes
yes=true
uninstall=false
for arg in "$@"; do
    case "$arg" in
        --uninstall) uninstall=true ;;
        --yes|-y) yes=true ;;
        --confirm) yes=false ;;
        --restart|-r) restart=yes ;;
        --no-restart) restart=no ;;
        *) echo "Unknown option: $arg" >&2; exit 1 ;;
    esac
done
fail() { echo "$*" >&2; exit 1; }
[ "$(uname -s)" = Linux ] && [ "$(uname -m)" = aarch64 ] || fail 'This installer requires Linux ARM64.'
[ "$(id -u)" = 0 ] || fail 'Run this installer as root.'
for command in systemctl flock readlink wget; do
    command -v "$command" >/dev/null 2>&1 || fail "Required command missing: $command"
done
[ -x "$vendor/CNC_Lab" ] || fail 'Supported NestPad application not found in /root/app.'
systemctl is-active --quiet nestpad.service || fail 'Open the stock UI before installing or removing PIMProbe.'
if [ "$uninstall" = false ]; then
    sh "$payload/app/bin/pimprobe-ui" --check
    for file in "$payload/app/bin/pimprobe-service" "$payload/app/lib/"*.so; do
        report=$(LD_LIBRARY_PATH="$vendor${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" ldd "$file" 2>&1) || fail "Cannot load $file: $report"
        case "$report" in *'not found'*) fail "Missing runtime dependency for $file: $report" ;; esac
    done
fi
[ -d /userdata ] && [ -d /etc/systemd/system ] || fail 'Expected machine filesystem layout is missing.'
for name in libpimprobeproxyplugin.so libpimprobelauncherplugin.so; do
    link=$vendor/$name
    if [ -e "$link" ] || [ -L "$link" ]; then
        [ -L "$link" ] || fail "Refusing to replace an unexpected plugin: $link"
        case "$(readlink "$link")" in
            "$destination/lib/$name"|"$legacy/lib/$name") ;;
            *) fail "Refusing to replace an unexpected plugin: $link" ;;
        esac
    fi
done
echo 'The machine must be idle with the spindle stopped.'
if [ "$uninstall" = true ]; then
    echo 'Saved probing settings and logs will be kept.'
fi
if [ "$yes" = false ]; then
    if [ "$uninstall" = true ]; then
        printf 'Remove PIMProbe? [y/N] '
    else
        printf 'Install PIMProbe now? [y/N] '
    fi
    read -r answer || exit 1
    case "$answer" in y|Y|yes) ;; *) echo 'Cancelled.'; exit 0 ;; esac
fi
# Serialize installation and removal with a directory lock.
exec 8</userdata
flock -n 8 || fail 'Another installation or uninstall is running.'
stage=''
rollback=''
service_was_active=false
service_stopped=false
unit_was_enabled=false
installation_started=false
destination_installed=false
plugins_changed=false
unit_changed=false

stop_pimprobe_ui() {
    [ -f /run/pimprobe-ui.pid ] || return 0
    pid=$(cat /run/pimprobe-ui.pid)
    if [ -r "/proc/$pid/cmdline" ]; then
        case "$(tr '\0' ' ' <"/proc/$pid/cmdline")" in
            *'/userdata/pimprobe/bin/pimprobe-ui'*|*'/userdata/cnc/apps/pimprobe/bin/pimprobe-ui'*)
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
}

rollback_installation() {
    systemctl stop pimprobe-service >/dev/null 2>&1 || true
    if [ "$plugins_changed" = true ]; then
        for name in libpimprobeproxyplugin.so libpimprobelauncherplugin.so; do
            rm -f "$vendor/$name"
            if [ -L "$rollback/$name" ]; then cp -a "$rollback/$name" "$vendor/$name"; fi
        done
        rm -f "$plugin_hook"
        if [ -f "$rollback/nestpad-pimprobe.conf" ]; then
            cp -p "$rollback/nestpad-pimprobe.conf" "$plugin_hook"
        fi
    fi
    if [ "$unit_changed" = true ]; then
        rm -rf "$unit" "$unit.d" /etc/systemd/system/multi-user.target.wants/pimprobe-service.service
        if [ -f "$rollback/$(basename "$unit")" ]; then
            cp -a "$rollback/$(basename "$unit")" "$unit"
        fi
        if [ -d "$rollback/$(basename "$unit").d" ]; then
            cp -a "$rollback/$(basename "$unit").d" "$unit.d"
        fi
    fi
    if [ -d "$rollback/installation" ]; then
        rm -rf "$destination"
        mv "$rollback/installation" "$previous_destination"
    elif [ "$destination_installed" = true ]; then
        rm -rf "$destination"
    fi
    systemctl daemon-reload >/dev/null 2>&1 || true
    if [ "$unit_was_enabled" = true ]; then
        systemctl enable pimprobe-service >/dev/null 2>&1 || true
    else
        systemctl disable pimprobe-service >/dev/null 2>&1 || true
    fi
}

cleanup() {
    status=$?
    trap - EXIT
    if [ "$status" != 0 ] && [ "$uninstall" = false ] &&
        { [ "$installation_started" = true ] || [ "$service_stopped" = true ]; }; then
        if [ "$installation_started" = true ]; then rollback_installation; fi
        if [ "$service_stopped" = true ] && [ "$service_was_active" = true ]; then
            systemctl start pimprobe-service >/dev/null 2>&1 || true
        fi
        echo 'Installation failed; the previous PIMProbe installation was restored.' >&2
    elif [ "$status" != 0 ] && [ "$uninstall" = true ]; then
        echo 'Uninstall did not finish. Reboot before using PIMProbe.' >&2
    fi
    if [ -n "$stage" ]; then rm -rf "$stage"; fi
    if [ -n "$rollback" ]; then rm -rf "$rollback"; fi
    sync
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
if [ "$uninstall" = false ]; then
    mkdir -p "$root/apps" "$root/backups"
    stage=$(mktemp -d "$root/apps/pimprobe-stage.XXXXXX")
    cp -a "$payload/app/." "$stage/"
    chown -R root:root "$stage"
    rollback=$(mktemp -d "$root/backups/pimprobe-install.XXXXXX")
    if [ -e "$unit" ]; then cp -a "$unit" "$rollback/"; fi
    if [ -d "$unit.d" ]; then cp -a "$unit.d" "$rollback/"; fi
    for name in libpimprobeproxyplugin.so libpimprobelauncherplugin.so; do
        if [ -L "$vendor/$name" ]; then cp -a "$vendor/$name" "$rollback/"; fi
    done
    if [ -f "$plugin_hook" ]; then cp -p "$plugin_hook" "$rollback/nestpad-pimprobe.conf"; fi
fi
if [ -L /etc/systemd/system/multi-user.target.wants/pimprobe-service.service ]; then unit_was_enabled=true; fi
if systemctl is-active --quiet pimprobe-service; then
    service_was_active=true
    systemctl stop pimprobe-service
    service_stopped=true
fi
if [ "$restart" = no ] || [ "$uninstall" = true ]; then stop_pimprobe_ui; fi
sh "$payload/migrate-data.sh" /userdata /root
if [ ! -e "$root/config/pimprobe-service.json" ]; then
    temporary=$(mktemp "$root/config/pimprobe-service.json.XXXXXX")
    cp "$payload/app/config.json" "$temporary"
    sync
    mv "$temporary" "$root/config/pimprobe-service.json"
    sync
fi
if [ "$uninstall" = true ]; then
    if [ -e "$unit" ] || [ -L /etc/systemd/system/multi-user.target.wants/pimprobe-service.service ]; then
        systemctl disable pimprobe-service
    fi
    rm -f "$vendor/libpimprobeproxyplugin.so" "$vendor/libpimprobelauncherplugin.so"
    rm -f "$plugin_hook"
    rm -rf "$unit" "$unit.d" /etc/systemd/system/multi-user.target.wants/pimprobe-service.service
    rm -rf "$destination"
    if [ -d "$legacy" ]; then
        backup=$(mktemp -d "$root/backups/pimprobe-legacy.XXXXXX")
        mv "$legacy" "$backup/app"
    fi
    rm -f /run/pimprobe-controller.sock /run/pimprobe-ui.pid /run/pimprobe-ui.lock \
        /run/pimprobe-update-*.status
    rm -rf /run/pimprobe-browser
    systemctl daemon-reload
    echo 'PIMProbe removed. Saved probing settings and logs were kept.'
else
    installation_started=true
    if [ -e "$previous_destination" ]; then mv "$previous_destination" "$rollback/installation"; fi
    mv "$stage" "$destination"
    stage=''
    destination_installed=true
    plugins_changed=true
    for name in libpimprobeproxyplugin.so libpimprobelauncherplugin.so; do
        ln -sfn "$destination/lib/$name" "$vendor/$name"
    done
    mkdir -p "$(dirname "$plugin_hook")"
    cp "$payload/nestpad-pimprobe.conf" "$plugin_hook"
    unit_changed=true
    cp "$payload/pimprobe-service.service" "$unit"
    systemctl daemon-reload
    systemctl enable pimprobe-service
    if [ "$previous_destination" = "$legacy" ]; then
        backup=$(mktemp -d "$root/backups/pimprobe-legacy.XXXXXX")
        if [ -d "$rollback/installation" ]; then
            cp -a "$rollback/installation" "$backup/app"
        fi
        if [ -d /userdata/pimprobe-data ]; then
            mv /userdata/pimprobe-data "$backup/logs"
        fi
        sync
    fi
    echo "Installed $(cat "$destination/VERSION")."
fi
if [ "$restart" = no ]; then
    if [ "$uninstall" = true ]; then
        echo 'Reboot to unload the plugins and remove the button from CNC_Lab.'
        exit 0
    fi
    echo 'Probing service is stopped. Reboot the machine before using PIMProbe.'
    exit 0
fi
stop_pimprobe_ui
if [ "$uninstall" = false ]; then
    systemctl start pimprobe-service
    systemctl is-active --quiet pimprobe-service || fail 'Probing service did not start. Check journalctl -u pimprobe-service.'
fi
if [ -n "$stage" ]; then rm -rf "$stage"; stage=''; fi
if [ -n "$rollback" ]; then rm -rf "$rollback"; rollback=''; fi
sync
trap - EXIT
if [ "$uninstall" = true ]; then
    echo 'PIMProbe removed; restarting CNC_Lab.'
else
    echo 'Probing service started; restarting CNC_Lab.'
fi
systemctl restart nestpad.service
