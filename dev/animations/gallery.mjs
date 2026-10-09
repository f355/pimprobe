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

import {readFile,writeFile,mkdir} from 'node:fs/promises';

const output = new URL('../../build/ui-review/',import.meta.url);
const clips = JSON.parse(await readFile(new URL('../../ui/animations/manifest.json',import.meta.url),'utf8'));
function title(key) {
    const sides = /^(outside|inside)-(-?\d)-(-?\d)$/.exec(key);
    if (sides) return sides[1]+' '+['X','Y'].map((axis,i) => Number(sides[i+2]) ? axis+(Number(sides[i+2]) > 0 ? '+' : '−') : '').filter(Boolean).join(' / ');
    return key.replaceAll('-',' ').replace('Negative',' toward Y−');
}
const sections = Object.keys(clips).map((key,index) => `<section><h2>${String(index+1).padStart(2,'0')} · ${title(key)}</h2><canvas width="320" height="176" data-clip="${key}"></canvas><p></p></section>`).join('\n');
await mkdir(output,{recursive:true});
await writeFile(new URL('motion-loops.html',output),`<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Probing motion previews</title><style>
*{box-sizing:border-box}body{margin:0;padding:24px;background:#202425;color:#f0f3f3;font:16px system-ui,sans-serif}
h1{font-size:24px;font-weight:500;margin:0 0 24px}main{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:32px 24px;max-width:1200px;margin:auto}
section{min-width:0}h2{font-size:18px;font-weight:500;margin:0 0 8px;text-transform:capitalize}canvas{display:block;width:100%;aspect-ratio:320/176}p{font-size:14px;color:#b1bdc0;margin:8px 0 0}
@media(max-width:360px){body{padding:16px}main{grid-template-columns:1fr}}
</style></head><body><h1>Probing motion previews</h1><main>${sections}</main><script>
const clips=${JSON.stringify(clips)};
const observer=new IntersectionObserver(entries=>entries.forEach(entry=>{
    const canvas=entry.target;
    canvas.playing=entry.isIntersecting;
    if(canvas.playing&&!canvas.loaded){
        canvas.loaded=true;
        const key=canvas.dataset.clip,metadata=clips[key],image=new Image();
        image.onload=()=>{
            const context=canvas.getContext('2d'),columns=image.width/metadata.width;
            function draw(time){
                if(canvas.playing){
                    const frame=Math.floor(time/1000*metadata.rate)%metadata.frames;
                    context.drawImage(image,frame%columns*metadata.width,Math.floor(frame/columns)*metadata.height,metadata.width,metadata.height,0,0,320,176);
                    canvas.nextElementSibling.textContent=['Position','Measure','Finish'][metadata.phases[frame]];
                }
                requestAnimationFrame(draw);
            }
            requestAnimationFrame(draw);
        };
        image.src='../../ui/animations/'+key+'.png';
    }
}));
document.querySelectorAll('canvas').forEach(canvas=>observer.observe(canvas));
</script></body></html>`);
console.log(new URL('motion-loops.html',output).pathname);
