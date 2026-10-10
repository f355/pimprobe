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

import {createServer} from 'node:http';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {resolve,extname} from 'node:path';
import {chromium} from 'playwright';

const root = fileURLToPath(new URL('./',import.meta.url));
const output = fileURLToPath(new URL('../../ui/animations/',import.meta.url));
const stills = fileURLToPath(new URL('../../build/ui-review/motion-stills/',import.meta.url));
const server = createServer(async (request,response) => {
    const path = resolve(root,'.' + new URL(request.url,'http://localhost').pathname);
    if (!path.startsWith(root)) { response.writeHead(403).end(); return; }
    try {
        const body = await readFile(path);
        response.setHeader('Content-Type',extname(path) === '.html' ? 'text/html' : 'text/javascript');
        response.end(body);
    } catch { response.writeHead(404).end(); }
});
await new Promise(resolve => server.listen(0,'127.0.0.1',resolve));
const browser = await chromium.launch({channel:process.env.BROWSER || 'chrome',headless:true,
    args:['--enable-unsafe-swiftshader']});
const page = await browser.newPage({viewport:{width:320,height:176}});
const errors = [];
page.on('pageerror',error => errors.push(error.message));
page.on('console',message => {
    if (message.type() === 'error' || message.text().startsWith('THREE.'))
        errors.push(message.text());
});
const operations = ['z-surface','z-pocket'];
for (const family of ['outside','inside'])
    for (const x of [-1,0,1]) for (const y of [-1,0,1]) if (x || y)
        operations.push(`${family}-${x}-${y}`);
for (const feature of ['boss','block','hole','pocket','x-ridge','x-valley','y-ridge','y-valley'])
    operations.push('center-'+feature);
for (const operation of ['axis','horizontal','vertical','verticalNegative'])
    operations.push('rotary-'+operation);
for (const feature of ['x-minus','x-plus','y-minus','y-plus','z-x','z-y'])
    operations.push('angle-'+feature);
await mkdir(output,{recursive:true});
await mkdir(stills,{recursive:true});
const metadata = {};
try {
    for (const operation of operations.filter(key => !process.argv[2] || key === process.argv[2])) {
        errors.length = 0;
        await page.goto(`http://127.0.0.1:${server.address().port}/scene.html?operation=${operation}`);
        await page.waitForFunction(() => typeof window.renderFrame === 'function');
        const result = await page.evaluate(() => {
            const frames = 64, columns = 6, width = 320, height = 176;
            const atlas = document.createElement('canvas');
            atlas.width = columns*width; atlas.height = Math.ceil(frames/columns)*height;
            const context = atlas.getContext('2d');
            const phases = [];
            for (let frame=0; frame<frames; frame++) {
                phases.push(window.renderFrame(frame/(frames-1)));
                if (Math.abs(window.probeTop.x) > 0.96 || Math.abs(window.probeTop.y) > 0.96)
                    throw new Error('Probe body is outside the frame at '+frame);
                context.drawImage(window.canvas,(frame%columns)*width,Math.floor(frame/columns)*height);
            }
            const samples = [0,20,40,54,60].map(frame => {
                const pixels = context.getImageData((frame%columns)*width,Math.floor(frame/columns)*height,width,height).data;
                let hash = 0, colored = 0;
                for (let i=0; i<pixels.length; i+=4) {
                    hash = ((hash * 31) + pixels[i] + pixels[i+1] + pixels[i+2]) | 0;
                    if (pixels[i] > 45 || pixels[i+1] > 45 || pixels[i+2] > 45) colored++;
                }
                return {hash,colored};
            });
            return {data:atlas.toDataURL('image/png').split(',')[1],phases,samples};
        });
        if (errors.length) throw new Error(errors.join('\n'));
        if (result.samples.some(sample => sample.colored < 3000)) throw new Error(operation+': blank scene');
        if (new Set(result.samples.slice(0,3).map(sample => sample.hash)).size < 2)
            throw new Error(operation+': no movement');
        if (result.samples[3].hash === result.samples[4].hash)
            throw new Error(operation+': the result highlight does not pulse');
        const data = Buffer.from(result.data,'base64');
        await writeFile(resolve(output,operation+'.png'),data);
        metadata[operation] = {frames:64,width:320,height:176,rate:8,phases:result.phases};
        await page.evaluate(() => window.renderFrame(0.9));
        await page.screenshot({path:resolve(stills,operation+'.png')});
        console.log(operation,Math.round(data.length/1024)+' KiB');
    }
    if (!process.argv[2]) {
        const entries = Object.entries(metadata).map(([key,clip]) => JSON.stringify(key)+': '+JSON.stringify(clip));
        await writeFile(resolve(output,'manifest.json'),'{\n'+entries.map(entry => '  '+entry).join(',\n')+'\n}\n');
        const header = (await readFile(resolve(root,'../license-header.txt'),'utf8')).trimEnd()
            .split('\n').map(line => '//'+(line ? ' '+line : '')).join('\n');
        await writeFile(resolve(output,'Animations.js'),header+'\n\n.pragma library\n\n// Generated by dev/animations/render.mjs.\nvar clips = {\n'+entries.map(entry => '    '+entry).join(',\n')+'\n};\n');
    }
} finally {
    await browser.close();
    server.close();
}
