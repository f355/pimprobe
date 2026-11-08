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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {test} from 'node:test';

test('upgrade keeps legacy calibration and logs without replacing shared settings', () => {
  const root = mkdtempSync(join(tmpdir(), 'pimprobe-migration-'));
  const userdata = join(root, 'userdata'), home = join(root, 'root');
  const run = () => execFileSync('sh', ['packaging/migrate-data.sh', userdata, home]);
  try {
    mkdirSync(join(userdata, 'pimprobe'), {recursive:true});
    mkdirSync(join(userdata, 'pimprobe-data', 'runs'), {recursive:true});
    writeFileSync(join(userdata, 'pimprobe', 'settings.json'), '{"probeDiameter":1.829,"fineFeed":40}\n');
    writeFileSync(join(userdata, 'pimprobe', 'config.json'), '{"listenAddress":"127.0.0.1:8137"}\n');
    writeFileSync(join(userdata, 'pimprobe-data', 'runs', 'old.jsonl'), '{"x":1}\n');
    run();
    const settings = join(userdata, 'cnc/config/probing.json');
    assert.equal(readFileSync(settings, 'utf8'), '{"probeDiameter":1.829,"fineFeed":40}\n');
    assert.equal(readFileSync(join(userdata, 'cnc/logs/pimprobe/runs/old.jsonl'), 'utf8'), '{"x":1}\n');
    assert.equal(readFileSync(join(userdata, 'cnc/config/pimprobe-service.json'), 'utf8'), '{"listenAddress":"127.0.0.1:8137"}\n');
    writeFileSync(settings, '{"probeDiameter":1.835}\n');
    writeFileSync(join(userdata, 'cnc/logs/pimprobe/runs/new.jsonl'), '{"x":2}\n');
    run();
    assert.equal(readFileSync(settings, 'utf8'), '{"probeDiameter":1.835}\n');
    assert.equal(readFileSync(join(userdata, 'cnc/logs/pimprobe/runs/new.jsonl'), 'utf8'), '{"x":2}\n');
    assert.equal(readFileSync(join(userdata, 'pimprobe', 'settings.json'), 'utf8'), '{"probeDiameter":1.829,"fineFeed":40}\n');
  } finally { rmSync(root, {recursive:true, force:true}); }
});
