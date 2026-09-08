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

import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const revision = '10dc7658f13c38a474cde201bb09a4617267545b';
const baseUrl = `https://raw.githubusercontent.com/vercel/geist-font/${revision}`;
const files = [
    ['Geist-Regular.otf', 'fonts/Geist/otf/Geist-Regular.otf', '63eed3b8f533234e2ae120fae23e79c92d8dda96bccce4147480c62a2fbddba5'],
    ['Geist-Medium.otf', 'fonts/Geist/otf/Geist-Medium.otf', '877ce6187b69f063920dc67d516948a4901fd8f9456f47ab22e7a67cf6a95742'],
    ['Geist-Bold.otf', 'fonts/Geist/otf/Geist-Bold.otf', 'b23edd02fa88c86701214cd0aa90d43f63798d4eb4b1bc1c52fbf834ff30d113'],
    ['OFL.txt', 'OFL.txt', 'c683bfbcc7e087f5d37a54ef628f10387c451a83ddc459b151403a164ac46c90'],
];

function digest(data) {
    return createHash('sha256').update(data).digest('hex');
}

export async function fetchFonts(output) {
    mkdirSync(output, { recursive: true });
    for (const [name, source, expected] of files) {
        const destination = join(output, name);
        try {
            if (digest(readFileSync(destination)) === expected)
                continue;
        } catch (error) {
            if (error.code !== 'ENOENT')
                throw error;
        }
        const temporary = destination + '.tmp';
        try {
            const response = await fetch(`${baseUrl}/${source}`);
            if (!response.ok)
                throw new Error(`Unable to fetch ${name}: HTTP ${response.status}`);
            const data = Buffer.from(await response.arrayBuffer());
            if (digest(data) !== expected)
                throw new Error(`Checksum mismatch for ${name}`);
            writeFileSync(temporary, data);
            renameSync(temporary, destination);
        } finally {
            rmSync(temporary, { force: true });
        }
    }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
    let output = fileURLToPath(new URL('../ui/fonts', import.meta.url));
    if (process.argv.length > 2) {
        if (process.argv.length !== 4 || process.argv[2] !== '--output')
            throw new Error('Usage: node dev/fetch-fonts.mjs [--output DIRECTORY]');
        output = resolve(process.argv[3]);
    }
    await fetchFonts(output);
}
