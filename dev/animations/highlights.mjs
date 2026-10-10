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

import * as THREE from './node_modules/three/build/three.module.js';

export const axisColors = ['#f18b8b','#7ed5b1','#88b8f1'];

export class ResultHighlight extends THREE.Group {
    constructor() {
        super();
        this.materials = [];
        this.visible = false;
    }

    material(color, opacity = 1) {
        const material = new THREE.MeshBasicMaterial({color, transparent:true, opacity,
            depthTest:false, depthWrite:false, side:THREE.DoubleSide});
        this.materials.push({material, opacity});
        return material;
    }

    stroke(points, color, radius = 0.075) {
        const path = new THREE.CurvePath();
        for (let i=1; i<points.length; i++)
            path.add(new THREE.LineCurve3(new THREE.Vector3(...points[i-1]),new THREE.Vector3(...points[i])));
        const line = new THREE.Mesh(new THREE.TubeGeometry(path,Math.max(16,points.length*4),radius,6,false),
            this.material(color));
        line.renderOrder = 11;
        this.add(line);
    }

    dashed(from, to, colors, gaps = true) {
        const start = new THREE.Vector3(...from), end = new THREE.Vector3(...to);
        const count = Math.ceil(start.distanceTo(end)/0.85);
        for (let i=0; i<count; i++) {
            const a = start.clone().lerp(end,i/count), b = start.clone().lerp(end,(i+(gaps ? 0.6 : 1))/count);
            this.stroke([a.toArray(),b.toArray()],colors[i%colors.length]);
        }
    }

    point(at, colors) {
        for (let i=0; i<colors.length; i++) {
            const dot = new THREE.Mesh(new THREE.SphereGeometry(0.38,16,12,
                i*Math.PI*2/colors.length,Math.PI*2/colors.length),this.material(colors[i]));
            dot.position.set(...at);
            dot.rotation.z = Math.PI/4;
            dot.renderOrder = 12;
            this.add(dot);
        }
        for (const axis of [0,1]) {
            const a = at.slice(), b = at.slice();
            a[axis] -= 1; b[axis] += 1;
            this.stroke([a,b],colors[axis%colors.length]);
        }
    }

    plane(corners, color, imaginary = false) {
        const geometry = new THREE.BufferGeometry();
        geometry.setAttribute('position',new THREE.Float32BufferAttribute(corners.flat(),3));
        geometry.setIndex([0,1,2,0,2,3]);
        const face = new THREE.Mesh(geometry,this.material(color,0.22));
        face.renderOrder = 10;
        this.add(face);
        for (let i=0; i<4; i++) {
            const a = corners[i], b = corners[(i+1)%4];
            if (imaginary) this.dashed(a,b,[color]);
            else this.stroke([a,b],color);
        }
    }

    angle(origin, along, normal, radians, color) {
        const start = new THREE.Vector3(...origin), direction = new THREE.Vector3(...along);
        const axis = new THREE.Vector3(...normal);
        const end = direction.clone().applyAxisAngle(axis,radians);
        for (const ray of [direction,end])
            this.stroke([start.toArray(),start.clone().addScaledVector(ray,6).toArray()],'#d3ddda',0.045);
        const arc = Array.from({length:17},(_,i) => start.clone()
            .addScaledVector(direction.clone().applyAxisAngle(axis,radians*i/16),4.8).toArray());
        this.stroke(arc,color,0.14);
        const geometry = new THREE.BufferGeometry();
        geometry.setAttribute('position',new THREE.Float32BufferAttribute([origin,...arc].flat(),3));
        geometry.setIndex(Array.from({length:16},(_,i) => [0,i+1,i+2]).flat());
        const wedge = new THREE.Mesh(geometry,this.material(color,0.35));
        wedge.renderOrder = 10;
        this.add(wedge);
    }

    animate(progress) {
        this.visible = progress > 0;
        const fade = THREE.MathUtils.smoothstep(progress,0,0.18);
        const pulse = 0.45 + 0.55*Math.sin(progress*Math.PI*2)**2;
        for (const {material,opacity} of this.materials) material.opacity = opacity*fade*pulse;
        return fade;
    }
}
