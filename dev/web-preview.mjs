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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

import { spawn, execFileSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import { createServer } from 'node:net';
import { fetchFonts } from './fetch-fonts.mjs';

const root=fileURLToPath(new URL('..',import.meta.url));
const directory=resolve(root,'build/web-preview');
mkdirSync(directory,{recursive:true});
for (const port of [8137, 8140]) {
    const server=createServer();
    await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(port,'127.0.0.1',()=>server.close(resolve));});
}
await fetchFonts(resolve(root,'web/assets/fonts'));
const binary=execFileSync(resolve(root,'dev/build-service.sh'),{cwd:root,encoding:'utf8',stdio:['ignore','pipe','inherit']}).trim();
const service=spawn(binary,['--mock','--listen','127.0.0.1:8137','--settings',resolve(directory,'settings.json'),'--web-root',resolve(root,'web/dist')],{cwd:root,stdio:'inherit'});
const ui=spawn(process.execPath,[resolve(root,'web/node_modules/vite/bin/vite.js'),'--host','127.0.0.1','--strictPort','--open','/?preview'],{cwd:resolve(root,'web'),stdio:'inherit'});
let stopping=false;
function stop(){if(stopping)return;stopping=true;service.kill('SIGTERM');ui.kill('SIGTERM');}
process.on('SIGINT',stop);process.on('SIGTERM',stop);
service.on('exit',stop);ui.on('exit',stop);
