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
if [ -n "${PIMPROBE_SERVICE:-}" ]; then
    [ -x "$PIMPROBE_SERVICE" ] || exit 1
    printf '%s\n' "$PIMPROBE_SERVICE"
    exit 0
fi
cd "$repo"
target=${CARGO_TARGET_DIR:-$repo/target}
case "$target" in /*) ;; *) target="$repo/$target" ;; esac
if ! command -v cargo >/dev/null 2>&1; then
    rustup_bin=$(command -v rustup || true)
    if [ -z "$rustup_bin" ]; then
        for candidate in "${CARGO_HOME:-$HOME/.cargo}/bin/rustup" /opt/homebrew/bin/rustup /usr/local/bin/rustup; do
            if [ -x "$candidate" ]; then
                rustup_bin=$candidate
                break
            fi
        done
    fi
    if [ -z "$rustup_bin" ]; then
        echo "Rust toolchain not found: install Cargo or rustup" >&2
        exit 1
    fi
    cargo_bin=$("$rustup_bin" which cargo)
    # Cargo invokes rustc by name, including when rustup is a Homebrew wrapper.
    PATH="$(dirname "$cargo_bin"):$PATH"
    export PATH
fi
cargo build --bin pimprobe-service --target-dir "$target" >&2
printf '%s\n' "$target/debug/pimprobe-service"
