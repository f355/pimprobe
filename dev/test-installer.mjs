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

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

const root = new URL('../', import.meta.url).pathname;
function run(command, args, expected = 0) {
    const result = spawnSync(command, args, { cwd: root, encoding: 'utf8' });
    assert.equal(result.status, expected, result.stdout + result.stderr);
    return result;
}

test('self-extracting package preserves its payload and rejects corruption', () => {
    const temp = mkdtempSync(join(tmpdir(), 'pimprobe-installer-'));
    try {
        const installer = join(temp, 'installer with spaces.run');
        const commit = '0123456789abcdef0123456789abcdef01234567';
        run('node', ['dev/package.mjs', '--output', installer, '--version', '2026.09.01', '--commit', commit], 1);
        run('node', ['dev/package.mjs', '--output', installer, '--version', '2026.09.0', '--commit', commit]);
        run('sh', [installer, '--check']);
        const extracted = join(temp, 'unpacked');
        run('sh', [installer, '--extract', extracted]);
        assert.equal(readFileSync(join(extracted, 'app/VERSION'), 'utf8'), '2026.09.0\n');
        assert.equal(readFileSync(join(extracted, 'app/COMMIT'), 'utf8'), commit + '\n');
        for (const [source, target] of [
            ['target/linux/release/pimprobe-service', 'app/bin/pimprobe-service'],
            ['plugins/build-arm64/libpimprobeproxyplugin.so', 'app/lib/libpimprobeproxyplugin.so'],
            ['plugins/build-arm64/libpimprobelauncherplugin.so', 'app/lib/libpimprobelauncherplugin.so'],
            ['ui/ProbeFlow.qml', 'app/ui/ProbeFlow.qml'],
            ['LICENSE.md', 'app/LICENSE.md'],
            ['README.md', 'app/README.md'],
        ]) assert.deepEqual(readFileSync(join(extracted, target)), readFileSync(join(root, source)));
        for (const name of ['Geist-Regular.otf', 'Geist-Medium.otf', 'Geist-Bold.otf'])
            assert.ok(readFileSync(join(extracted, 'app/ui/fonts', name)).length > 10000, `${name} must be packaged`);
        assert.match(readFileSync(join(extracted, 'app/ui/fonts/OFL.txt'), 'utf8'), /SIL OPEN FONT LICENSE Version 1\.1/);
        assert.ok(existsSync(join(extracted, 'install.sh')));
        assert.ok(existsSync(join(extracted, 'app/VERSION')));
        run('sh', [installer, '--extract', extracted], 1);
        const corrupt = readFileSync(installer);
        corrupt[corrupt.length - 20] ^= 1;
        writeFileSync(installer, corrupt);
        run('sh', [installer, '--check'], 1);
        const rejected = join(temp, 'rejected');
        run('sh', [installer, '--extract', rejected], 1);
        assert.equal(existsSync(rejected), false, 'corrupt archive must be rejected before extraction');
    } finally {
        rmSync(temp, { recursive: true, force: true });
    }
});
