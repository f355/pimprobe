#!/usr/bin/env node
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

import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

export function releaseNotes(subjects, stable) {
    const selected = stable
        ? subjects.filter(subject => /^\[(bugfix|improvement|feature)\]/.test(subject))
        : subjects;
    return selected.length ? selected.map(subject => `- ${subject}`).join("\n") : "No changes listed.";
}

function main() {
    const mode = process.argv[2];
    const range = process.argv[3] || "HEAD";
    if (mode !== "stable" && mode !== "development")
        throw new Error("Usage: release-notes.mjs stable|development [GIT_RANGE]");
    const output = execFileSync("git", ["log", "--no-merges", "--format=%s", range], { encoding: "utf8" });
    const subjects = output.split("\n").filter(Boolean);
    process.stdout.write(releaseNotes(subjects, mode === "stable") + "\n");
}

if (process.argv[1] && fileURLToPath(import.meta.url) === fileURLToPath(new URL(`file://${process.argv[1]}`)))
    main();
