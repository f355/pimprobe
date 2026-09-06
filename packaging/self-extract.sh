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
offset=@OFFSET0000@
while [ "${offset#0}" != "$offset" ]; do offset=${offset#0}; done
digest=@SHA256@
usage() {
    cat <<'HELP'
PIMProbe offline installer for NestPad (Linux ARM64).
Run only with the machine idle and the spindle stopped.

  sh installer.run                     Interactive installation
  sh installer.run --yes --no-restart   Install; activate after a reboot
  sh installer.run --yes --restart      Install and restart CNC_Lab now
  sh installer.run --uninstall          Remove PIMProbe, settings and backups
  sh installer.run --check              Verify the embedded archive only
  sh installer.run --check-target       Verify archive and machine compatibility
  sh installer.run --extract DIRECTORY  Extract into a new directory; do not install

--yes confirms that the machine is idle. Without --yes, changes ask first.
--uninstall supports --yes, --restart and --no-restart. It leaves this installer
and shared system journals intact. Without a restart, reboot to unload plugins.
HELP
}
case "${1:-}" in
    --help|-h) usage; exit 0 ;;
    --check|--check-target) [ "$#" = 1 ] || { usage; exit 1; } ;;
    --extract) [ "$#" = 2 ] || { usage; exit 1; } ;;
esac
scratch=$(mktemp -d "${TMPDIR:-/tmp}/pimprobe-unpack.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
tail -c "+$offset" "$0" >"$scratch/payload.tar.gz"
if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$scratch/payload.tar.gz")
else
    actual=$(shasum -a 256 "$scratch/payload.tar.gz")
fi
[ "${actual%% *}" = "$digest" ] || { echo 'Payload checksum mismatch; nothing installed.' >&2; exit 1; }
if [ "${1:-}" = --check ]; then
    echo 'Payload checksum OK.'
    exit 0
fi
if [ "${1:-}" = --extract ]; then
    mkdir -- "$2"
    tar -xzf "$scratch/payload.tar.gz" -C "$2"
    echo "Extracted to $2"
    exit 0
fi
mkdir "$scratch/files"
tar -xzf "$scratch/payload.tar.gz" -C "$scratch/files"
sh "$scratch/files/install.sh" "$@"
exit 0
