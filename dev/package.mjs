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
import { execFileSync } from 'node:child_process';
import { chmodSync, copyFileSync, cpSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const options = { service: join(root, 'target/linux/release/pimprobe-service'), plugins: join(root, 'plugins/build-arm64') };
for (let i = 2; i < process.argv.length; ++i) {
    const arg = process.argv[i];
    if (arg === '--help') {
        console.log('Usage: node dev/package.mjs [--output FILE] [--service ARM64_BINARY] [--plugins BUILD_DIR]');
        process.exit(0);
    }
    if (!['--output', '--service', '--plugins'].includes(arg) || !process.argv[i + 1])
        throw new Error('Unknown or incomplete argument: ' + arg);
    options[arg.slice(2)] = resolve(process.argv[++i]);
}

const version = execFileSync('git', ['describe', '--always', '--dirty'], { cwd: root, encoding: 'utf8' }).trim();
const output = options.output || join(root, 'dist', `pimprobe-${version}.run`);
const scratch = mkdtempSync(join(tmpdir(), 'pimprobe-package-'));
const payload = join(scratch, 'payload');
function copy(source, destination) {
    const target = join(payload, destination);
    mkdirSync(dirname(target), { recursive: true });
    copyFileSync(source, target);
    chmodSync(target, 0o644);
}
function binary(source, destination) {
    const header = readFileSync(source);
    if (header.subarray(0, 4).toString('hex') !== '7f454c46' || header[4] !== 2 || header[5] !== 1 || header.readUInt16LE(18) !== 183)
        throw new Error(`${source}: expected a Linux ARM64 ELF file; build the machine artifacts first`);
    copy(source, destination);
    chmodSync(join(payload, destination), 0o755);
}
try {
    binary(options.service, 'app/bin/pimprobe-service');
    for (const name of ['libpimprobeproxyplugin.so', 'libpimprobelauncherplugin.so'])
        binary(join(options.plugins, name), 'app/lib/' + name);
    cpSync(join(root, 'ui'), join(payload, 'app/ui'), {
        recursive: true,
        filter: source => !source.split('/').some(part => part === 'tests' || part === '.DS_Store'),
    });
    copy(join(root, 'packaging/pimprobe-ui'), 'app/bin/pimprobe-ui');
    chmodSync(join(payload, 'app/bin/pimprobe-ui'), 0o755);
    copy(join(root, 'packaging/config.example.json'), 'app/config.json');
    copy(join(root, 'packaging/pimprobe-service.service'), 'pimprobe-service.service');
    copy(join(root, 'packaging/install.sh'), 'install.sh');
    for (const name of ['LICENSE.md', 'README.md']) {
        copy(join(root, name), name);
        copy(join(root, name), 'app/' + name);
    }
    for (const destination of ['docs', 'app/docs'])
        cpSync(join(root, 'docs'), join(payload, destination), { recursive: true });
    writeFileSync(join(payload, 'app/VERSION'), version + '\n');
    const archive = join(scratch, 'payload.tar.gz');
    execFileSync('tar', ['--no-xattrs', '--no-acls', '-czf', archive, '-C', payload, '.'], { env: { ...process.env, COPYFILE_DISABLE: '1' } });
    const data = readFileSync(archive);
    let header = readFileSync(join(root, 'packaging/self-extract.sh'), 'utf8')
        .replace('@SHA256@', createHash('sha256').update(data).digest('hex'));
    const offset = String(Buffer.byteLength(header) + 1).padStart(12, '0');
    header = header.replace('@OFFSET0000@', offset);
    mkdirSync(dirname(output), { recursive: true });
    const temporary = output + '.tmp';
    writeFileSync(temporary, Buffer.concat([Buffer.from(header), data]), { mode: 0o755 });
    renameSync(temporary, output);
    console.log(`${output} (${data.length} bytes compressed; ${version})`);
} finally {
    rmSync(scratch, { recursive: true, force: true });
}
