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

import assert from "node:assert/strict";
import { readFileSync, readdirSync, existsSync, mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { compileHelp, renderPage } from "./build-help.mjs";

const root = new URL("../", import.meta.url);
const read = path => readFileSync(new URL(path, root), "utf8");

test("rustup discovery puts the actual compiler on PATH", () => {
    const scratch = mkdtempSync(join(tmpdir(), "pimprobe-toolchain-"));
    try {
        const bin = join(scratch, "bin");
        const toolchain = join(scratch, "toolchain");
        mkdirSync(bin);
        mkdirSync(toolchain);
        writeFileSync(join(bin, "rustup"), '#!/bin/sh\n[ "$*" = "which cargo" ] || exit 41\nprintf "%s/cargo\\n" "$TEST_TOOLCHAIN"\n', { mode: 0o755 });
        writeFileSync(join(toolchain, "cargo"), '#!/bin/sh\n[ "$(command -v rustc)" = "$TEST_TOOLCHAIN/rustc" ] || exit 42\n[ "$1" = build ] || exit 43\n', { mode: 0o755 });
        writeFileSync(join(toolchain, "rustc"), "#!/bin/sh\nexit 0\n", { mode: 0o755 });
        const result = spawnSync("/bin/sh", [new URL("dev/build-service.sh", root).pathname], {
            env: { PATH: `${bin}:/usr/bin:/bin`, HOME: scratch, TEST_TOOLCHAIN: toolchain },
            encoding: "utf8"
        });
        assert.equal(result.status, 0, result.stderr);
        assert.equal(result.stdout.trim(), new URL("target/debug/pimprobe-service", root).pathname);
    } finally {
        rmSync(scratch, { recursive: true, force: true });
    }
});

test("help bundle matches Markdown sources", () => {
    assert.equal(read("ui/HelpPages.js"), compileHelp());
    for (const name of ["outside", "inside", "center"]) {
        assert.ok(renderPage(name + ".md").includes(renderPage("results.md").trim()));
        assert.ok(renderPage(name + ".md").includes(renderPage("safety.md").trim()));
        assert.equal(read(`ui/help/images/${name}.svg`), read(`docs/images/${name}.svg`));
    }
});

test("shell scripts have valid syntax", () => {
    for (const path of ["dev/preview.sh", "dev/test-ui.sh", "dev/build-service.sh", "dev/test-install-container.sh", "packaging/pimprobe-ui", "packaging/install.sh", "packaging/self-extract.sh"]) {
        const result = spawnSync("sh", ["-n", new URL(path, root).pathname]);
        assert.equal(result.status, 0, result.stderr?.toString());
    }
});

test("operator guide links and images resolve", () => {
    for (const name of readdirSync(new URL("docs/", root)).filter(name => name.endsWith(".md"))) {
        const page = new URL("docs/" + name, root);
        for (const [, target] of readFileSync(page, "utf8").matchAll(/!?\[[^\]]*\]\(([^)]+)\)/g))
            assert.ok(existsSync(new URL(target, page)), name + ": " + target);
    }
});
