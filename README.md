# PIMProbe - Previously Impossible Manual Probing for the Nestworks C500

**WARNING: The author(s) assume absolutely no responsibility for any damage and/or disappointment that might occur as a
result of using this software. Exercise caution, double-check your parameters and keep your hand on the emergency stop at all times.**

**This software is NOT endorsed by or affiliated with Nestworks and/or Elephant Robotics.**

## Quickstart

Download the installer from the [latest stable release](https://github.com/f355/pimprobe/releases/latest)
or the [rolling development release](https://github.com/f355/pimprobe/releases/tag/dev),
rename it to `pimprobe.run` and copy it to a USB drive.

Make sure the machine is idle with the spindle stopped.
Connect a USB keyboard to the port on the right side of the machine.
Press `Ctrl-Alt-T`, a terminal window will open on the NestPad.

Have your USB drive ready. Type the following in the terminal exactly, press Enter,
then swap the keyboard for the USB drive within 20 seconds, and wait for the NestPad UI to restart.

```sh
sleep 20; sh /mnt/udisk/pimprobe.run -y -r
```

To uninstall, including settings, follow the same procedure, just use a different command:

```sh
sleep 20; sh /mnt/udisk/pimprobe.run --uninstall -y -r
```

To get new versions as they are released, head to the Settings tab and press "Check for updates".

## Documentation

[Operator guide](docs/README.md): routines, settings and work zero, with pictures.

## Development

Requirements: stable Rust, Qt 6 with Qt Quick Controls and Qt Test, CMake,
and Node.js. The development scripts locate Homebrew Qt and rustup on macOS.

Run `./dev/preview.sh` for an 800 x 480 preview with simulated stock.
UI tests use an isolated mock service and temporary settings.

Operator text lives in `docs/`; standalone links to shared pages are expanded
into the contextual help during previews and packaging. Screenshots sit inside
`guide-only` comment blocks. Refresh them with
`./dev/test-ui.sh dev/screenshots`.

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
node --test dev/test-assets.mjs
./dev/test-ui.sh
cmake -S plugins -B build/plugins -DCMAKE_PREFIX_PATH=/opt/homebrew
cmake --build build/plugins
ctest --test-dir build/plugins --output-on-failure
```

## Source

- `crates/probe-core`: planning, guarded motion, double touches, coordinate
  compensation, review scripts and execution.
- `crates/controller`: Unix socket protocol, GRBL records, connection and
  machine state.
- `crates/probe-app`: application operations, settings, history, review tokens
  and exclusive access to the machine.
- `crates/service`: standalone HTTP API, file storage and software updates.
- `plugins`: C++ controller proxy and launcher.
- `ui`: QML interface.
- `docs`: operator guide and contextual help sources.
- `packaging`: installer, service unit and launch scripts.

The controller proxy exposes `/run/pimprobe-controller.sock` through CNC_Lab's
controller objects. The launcher adds a button that opens the QML UI.
The service runs independently and reconnects when the proxy becomes available.
The socket framing is documented in
[`docs/developer/controller-bridge.md`](docs/developer/controller-bridge.md).

See [embedding](docs/developer/embedding.md) for the Rust host interfaces and
the reusable QML page.

Parameter ranges are defined in `crates/probe-core/src/parameters.rs` and
exposed at `/api/v1/settings/schema`. Deploy the UI and service together
when changing this contract.

## ARM64 Builds

Build the Linux service:

```sh
docker build --platform linux/arm64 -t pimprobe-rust-builder packaging
docker run --rm --platform linux/arm64 -v "$PWD:/src" pimprobe-rust-builder
```

Output: `target/linux/release/pimprobe-service`.

Build the plugins:

```sh
docker build --platform linux/arm64 -t pimprobe-qt-builder plugins
docker run --rm --platform linux/arm64 \
  -v "$PWD/plugins:/src" pimprobe-qt-builder \
  cmake -S /src -B /src/build-arm64
docker run --rm --platform linux/arm64 \
  -v "$PWD/plugins:/src" pimprobe-qt-builder \
  cmake --build /src/build-arm64
```

Output: `plugins/build-arm64/libpimprobe{proxy,launcher}plugin.so`.
Both builds use Debian Bookworm's runtime baseline.

Linux plugins resolve the NestPad ABI through CNC_Lab at load time. Set
`PIMPROBE_ABSTRACT_PLUGIN` to a local copy of `libabstractplugin.so` when
configuring CMake to also check their imports against that library's exports.

## Releases

CI runs the Rust, plugin, UI and installer tests and builds the ARM64 installer.
Successful pushes to `main` update the rolling `dev` prerelease, available on the
[releases page](https://github.com/f355/pimprobe/releases/tag/dev). Stable tags use
`YYYY.MM.N`, with `N` starting at zero each month. Their release notes include
commit subjects beginning with `[bugfix]`, `[improvement]`, or `[feature]`.

## Installation

Package the built ARM64 artifacts:

```sh
node dev/package.mjs
```

Output: `dist/pimprobe-<git-version>.run`. Rebuild the artifacts before
packaging a release. Override input/output paths with `--service PATH`,
`--plugins BUILD_DIR` and `--output FILE`. CI supplies `--version` and
`--commit` so installed builds retain both their release name and source commit.

Replace `<machine-host>` with the machine's IP address or hostname. Transfer the
installer and run it as root with the machine idle and spindle stopped:

```sh
scp dist/pimprobe-<git-version>.run root@<machine-host>:/userdata/
ssh -t root@<machine-host> 'sh /userdata/pimprobe-<git-version>.run'
```

The installer checks the archive, machine layout and runtime dependencies, and
preserves configuration and settings during upgrades.

Reboot later to activate the installation, or use `-r` to restart CNC_Lab immediately.

```sh
sh installer.run --extract ./unpacked    # Extract to a new directory
sh installer.run -y                      # Install; reboot later
sh installer.run -y -r                   # Install and restart CNC_Lab
sh installer.run --uninstall             # Remove PIMProbe and its data
sh installer.run --uninstall -y -r
```

`-y` (`--yes`) confirms that you have made the machine idle with its spindle stopped.
Obtain installers from a trusted source; the checksum detects corruption.

Uninstall removes PIMProbe's service, plugins, UI, settings and probing logs.
The installer file remains available for reinstallation.

## Configuration

The machine installation is in `/userdata/pimprobe`. Service options are in
`config.json`, and probing settings are in `settings.json`.

For local use, settings default to `$XDG_CONFIG_HOME/pimprobe/settings.json`
or `$HOME/.config/pimprobe/settings.json`. Override this with `--settings FILE`.

The HTTP listener is `127.0.0.1:8137`. Use an SSH tunnel for remote access.

## Installer Tests

These tests require the built ARM64 artifacts:

```sh
node --test dev/test-installer.mjs
node dev/package.mjs --output dist/pimprobe-test.run
docker run --rm --platform linux/arm64 \
  -v "$PWD/dist/pimprobe-test.run:/package.run:ro" \
  -v "$PWD/dev/test-install-container.sh:/test.sh:ro" \
  -v "$PWD/packaging/config.example.json:/expected-config.json:ro" \
  pimprobe-rust-builder sh /test.sh
```

The fixture writes to machine installation paths with simulated system
services. Run it only in a disposable container.

## Safety

Secure the stock and check the full probe route against clamps, stock,
the tool magazine and other obstacles. Travel-limit checks cover the
machine envelope, not fixture geometry.

Positioning uses G38.3; contact releases use G1. Touching the probe while
idle does not stop the machine. Use the physical emergency stop when needed.

## License

Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>.

Licensed under [GPLv3 or later](LICENSE.md). Third-party dependencies
retain their own licenses.

### Linking Exception

The following permission applies only to code in `plugins/src/`.

Additional permission under GNU GPL version 3 section 7

If you modify this Program, or any covered work, by linking or combining
it with the CNC_Lab application, libabstractplugin.so, or the vendor
libraries required by their plugin interfaces (or modified versions of
those components), containing parts covered by the proprietary license
terms of Nestworks and/or Elephant Robotics, the licensors of this
Program grant you additional permission to convey the resulting work.

This permission does not change the license terms of those vendor
components or grant permission to redistribute them. All other GNU GPL
obligations for the covered work remain in effect.
