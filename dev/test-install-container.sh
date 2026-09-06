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

# Run only in a disposable container: these fixtures deliberately use real install paths.
set -eu
[ -f /.dockerenv ] || [ -f /run/.containerenv ] || { echo 'Run this test in Docker.' >&2; exit 1; }
mkdir -p /root/app /usr/qml/QtQuick/Controls/Basic /userdata /etc/systemd/system /test-bin
touch /root/app/libabstractplugin.so
printf '#!/bin/sh\ntouch /tmp/vendor-started\n' >/root/app/CNC_Lab
printf '#!/bin/sh\nexit 0\n' >/usr/bin/qmlscene
chmod +x /root/app/CNC_Lab /usr/bin/qmlscene
cat >/test-bin/systemctl <<'STUB'
#!/bin/sh
echo "$*" >>/tmp/systemctl.log
case "$1" in
    is-active) [ -e /tmp/service-active ] ;;
    start) touch /tmp/service-active ;;
    stop) rm -f /tmp/service-active ;;
esac
STUB
cat >/test-bin/ldd <<'STUB'
#!/bin/sh
if [ -f /tmp/missing-library ]; then echo 'libexample.so => not found'; else echo 'Runtime dependencies OK'; fi
STUB
printf '#!/bin/sh\n[ -f /tmp/vendor-started ]\n' >/test-bin/pgrep
chmod +x /test-bin/*
export PATH=/test-bin:$PATH
sh /package.run --check
sh /package.run --check-target
# A failed preflight must leave the existing installation usable.
mkdir -p /userdata/pimprobe
printf 'old version\n' >/userdata/pimprobe/VERSION
touch /tmp/missing-library
if sh /package.run --yes --no-restart; then exit 1; fi
test "$(cat /userdata/pimprobe/VERSION)" = 'old version'
rm /tmp/missing-library
rm -rf /userdata/pimprobe
sh /package.run --yes --no-restart
test -x /userdata/pimprobe/bin/pimprobe-service
test -f /userdata/pimprobe/ui/Main.qml
test "$(readlink /root/app/libpimprobeproxyplugin.so)" = /userdata/pimprobe/lib/libpimprobeproxyplugin.so
test -f /etc/systemd/system/pimprobe-service.service
cmp /userdata/pimprobe/config.json /expected-config.json
printf '{"coarseFeed":42}\n' >/userdata/pimprobe/settings.json
printf '{"listenAddress":"127.0.0.1:8137","travelLimits":[210,235,123]}\n' >/userdata/pimprobe/config.json
cp /userdata/pimprobe/settings.json /tmp/expected-settings
cp /userdata/pimprobe/config.json /tmp/expected-config
printf 'obsolete UI\n' >/userdata/pimprobe/ui/Old.qml
touch /tmp/service-active
sh /package.run --yes --no-restart
cmp /userdata/pimprobe/settings.json /tmp/expected-settings
cmp /userdata/pimprobe/config.json /tmp/expected-config
backup=$(find /userdata/backups -path '*/installation/ui/Old.qml')
test -n "$backup"
test "$(cat "$backup")" = 'obsolete UI'
test ! -e /tmp/service-active
grep -q '^stop pimprobe-service$' /tmp/systemctl.log
grep -q '^enable pimprobe-service$' /tmp/systemctl.log
# Conflicting plugin ownership must not get overwritten.
rm /root/app/libpimprobeproxyplugin.so
printf 'unrelated plugin\n' >/root/app/libpimprobeproxyplugin.so
if sh /package.run --yes --no-restart; then exit 1; fi
test "$(cat /root/app/libpimprobeproxyplugin.so)" = 'unrelated plugin'
rm /root/app/libpimprobeproxyplugin.so
# Match the vendor profile's use of optional environment variables.
printf '\n: "$OPTIONAL_VENDOR_VARIABLE"\n' >>/etc/profile
sh /package.run --yes --restart
test -e /tmp/vendor-started
test -e /tmp/service-active
cmp /userdata/pimprobe/settings.json /tmp/expected-settings
# Uninstall must work even when the UI runtime is broken, and preserve unrelated data.
mkdir -p /userdata/backups/other-app /root/.config/pimprobe
touch /userdata/backups/other-app/keep /root/.config/pimprobe/settings.json
touch /run/pimprobe-controller.sock /run/pimprobe-ui.lock
touch /tmp/missing-library
rm /usr/bin/qmlscene
printf 'n\n' | sh /package.run --uninstall --no-restart
test -d /userdata/pimprobe
sh /package.run --uninstall --yes --no-restart
test ! -e /userdata/pimprobe
test ! -e /root/.config/pimprobe
test ! -L /root/app/libpimprobeproxyplugin.so
test ! -L /root/app/libpimprobelauncherplugin.so
test ! -e /etc/systemd/system/pimprobe-service.service
test ! -e /run/pimprobe-controller.sock
test ! -e /run/pimprobe-ui.lock
test -z "$(find /userdata/backups -name 'pimprobe-*')"
test -f /userdata/backups/other-app/keep
test -x /root/app/CNC_Lab
test ! -e /tmp/service-active
sh /package.run --uninstall --yes --no-restart
# The same package can install again after complete removal.
rm /tmp/missing-library /tmp/vendor-started
printf '#!/bin/sh\nexit 0\n' >/usr/bin/qmlscene
chmod +x /usr/bin/qmlscene
sh /package.run --yes --no-restart
cmp /userdata/pimprobe/config.json /expected-config.json
sh /package.run --uninstall --yes --restart
test -f /tmp/vendor-started
test ! -e /tmp/service-active
test ! -e /userdata/pimprobe
echo 'Install, upgrade, uninstall, repeat uninstall and reinstall OK.'
