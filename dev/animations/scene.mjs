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

const key = new URLSearchParams(location.search).get('operation') || 'outside--1--1';
const renderer = new THREE.WebGLRenderer({antialias:true, preserveDrawingBuffer:true});
renderer.setSize(320, 176);
renderer.setClearColor('#202425');
document.body.append(renderer.domElement);
const scene = new THREE.Scene();
scene.add(new THREE.HemisphereLight('#ecf5f0', '#293634', 2.2));
const light = new THREE.DirectionalLight('#ffffff', 3);
light.position.set(-10, -8, 22);
scene.add(light);
const camera = new THREE.OrthographicCamera(-15.27, 15.27, 8.4, -8.4, 0.1, 100);
camera.up.set(0, 0, 1);
camera.position.set(18, -24, 28);
camera.lookAt(0, 0, 4.5);

const stockMaterial = new THREE.MeshStandardMaterial({color:'#526c62', roughness:0.62,
    metalness:0.15, transparent:true, opacity:0.78, depthWrite:false});
const bedMaterial = new THREE.MeshStandardMaterial({color:'#343e40', roughness:0.9});
const bodyMaterial = new THREE.MeshStandardMaterial({color:'#b7c7c6', metalness:0.45, roughness:0.42});
const shaftMaterial = new THREE.MeshStandardMaterial({color:'#f2f5f4', metalness:0.3, roughness:0.35});
const ballMaterial = new THREE.MeshStandardMaterial({color:'#e46864', roughness:0.3, metalness:0.1});
const stock = new THREE.Group();
scene.add(stock);

function mesh(geometry, material, position, parent = scene) {
    const item = new THREE.Mesh(geometry, material);
    item.position.set(...position);
    parent.add(item);
    return item;
}
function box(size, position, parent = stock) {
    const item = mesh(new THREE.BoxGeometry(...size), stockMaterial, position, parent);
    const edges = new THREE.LineSegments(new THREE.EdgesGeometry(item.geometry),
        new THREE.LineBasicMaterial({color:'#a5bdb3', transparent:true, opacity:0.65}));
    item.add(edges);
    return item;
}
function cylinder(radius, length, position, axis, material = stockMaterial, parent = stock) {
    const item = mesh(new THREE.CylinderGeometry(radius, radius, length, 48), material, position, parent);
    item.rotation.set(axis === 'z' ? Math.PI / 2 : 0, 0, axis === 'x' ? Math.PI / 2 : 0);
    return item;
}
mesh(new THREE.BoxGeometry(24, 19, 0.6), bedMaterial, [0, 0, -0.4]);
const bedLines = [];
for (let x = -10; x <= 10; x += 4) bedLines.push(x,-9,-0.08, x,9,-0.08);
const bedGeometry = new THREE.BufferGeometry();
bedGeometry.setAttribute('position', new THREE.Float32BufferAttribute(bedLines, 3));
scene.add(new THREE.LineSegments(bedGeometry, new THREE.LineBasicMaterial({color:'#506062'})));

const axesOrigin = new THREE.Vector3(-9,-6,0.1);
for (const [name,direction,color] of [
    ['X',new THREE.Vector3(1,0,0),'#e9a29b'],
    ['Y',new THREE.Vector3(0,1,0),'#a9d6b7'],
    ['Z',new THREE.Vector3(0,0,1),'#a1c8e6']
]) {
    scene.add(new THREE.ArrowHelper(direction,axesOrigin,2.3,color,0.45,0.3));
    const label = document.createElement('canvas');
    label.width = label.height = 64;
    const context = label.getContext('2d');
    context.fillStyle = color; context.font = 'bold 48px sans-serif';
    context.textAlign = 'center'; context.textBaseline = 'middle';
    context.fillText(name,32,32);
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({map:new THREE.CanvasTexture(label),depthTest:false}));
    sprite.position.copy(axesOrigin).addScaledVector(direction,3.1);
    sprite.scale.set(1.5,1.5,1);
    scene.add(sprite);
}

const probe = new THREE.Group();
scene.add(probe);
mesh(new THREE.SphereGeometry(0.6, 20, 12), ballMaterial, [0,0,0], probe);
cylinder(0.17, 4.6, [0,0,2.8], 'z', shaftMaterial, probe);
cylinder(0.85, 1.4, [0,0,5.8], 'z', bodyMaterial, probe);
cylinder(0.86, 0.25, [0,0,6.15], 'z',
    new THREE.MeshStandardMaterial({color:'#439e7e', roughness:0.6}), probe);

const moves = [];
let position = [0,0,5.4,0];
function move(to, phase, duration = 0.65, contact = false) {
    moves.push({from:position.slice(), to:to.slice(), phase, duration, contact});
    position = to.slice();
}
function touch(axis, value) {
    const point = position.slice();
    point[axis] = value;
    move(point, 1, 0.75, true);
    const backoff = point.slice();
    backoff[axis] -= Math.sign(value - moves.at(-1).from[axis]) * 0.7;
    move(backoff, 1, 0.2);
    move(point, 1, 0.4, true);
    move(backoff, 1, 0.2);
}
function pocket(round = false, valley = '') {
    if (round) {
        const shape = new THREE.Shape();
        shape.moveTo(-8,-7); shape.lineTo(8,-7); shape.lineTo(8,7); shape.lineTo(-8,7); shape.closePath();
        const hole = new THREE.Path();
        hole.absarc(0,0,5,0,Math.PI*2,true);
        shape.holes.push(hole);
        const item = mesh(new THREE.ExtrudeGeometry(shape,{depth:3,bevelEnabled:false,curveSegments:40}),
            stockMaterial,[0,0,0],stock);
        item.add(new THREE.LineSegments(new THREE.EdgesGeometry(item.geometry,30),
            new THREE.LineBasicMaterial({color:'#8fa99e',transparent:true,opacity:0.35})));
    } else {
        if (valley !== 'y') {
            box([3,16,3],[-6.5,0,1.5]); box([3,16,3],[6.5,0,1.5]);
        }
        if (valley !== 'x') {
            box([10,3,3],[0,-6.5,1.5]); box([10,3,3],[0,6.5,1.5]);
        }
    }
}

if (key === 'z-surface' || key === 'z-pocket') {
    const internal = key === 'z-pocket';
    if (internal) {
        pocket();
        box([10,10,0.3],[0,0,-0.15]);
        position = [0,0,2.1,0];
    } else box([12,10,3],[0,0,1.5]);
    const start = position.slice();
    move(position,0,0.6);
    touch(2,internal ? 0.6 : 3.6);
    move(start,2);
} else if (key.startsWith('outside') || key.startsWith('inside')) {
    const [,xs,ys] = /^(outside|inside)-(-?\d)-(-?\d)$/.exec(key).slice(1);
    const internal = key.startsWith('inside');
    const x = Number(xs) * (internal ? 1 : -1), y = Number(ys) * (internal ? 1 : -1);
    if (internal) pocket(); else box([10,10,3],[0,0,1.5]);
    const start = internal ? [0,0,1.5,0] : [x * 2.8,y * 2.8,5.4,0];
    position = start.slice();
    move(start,0,0.55);
    const sides = [[0,x],[1,y]].filter(([,direction]) => direction);
    for (const [index, [axis, direction]] of sides.entries()) {
        const last = index === sides.length - 1;
        if (!direction) continue;
        if (!internal) {
            const beyond = start.slice(); beyond[axis] = direction * 7.6;
            move(beyond,0);
            move([beyond[0],beyond[1],1.5,0],0);
        }
        touch(axis,direction * (internal ? 4.4 : 5.6));
        if (internal) {
            move(start, last ? 2 : 1);
        } else {
            move([position[0],position[1],5.4,0],last ? 2 : 0);
            if (!last) move(start,0);
        }
    }
    if (!internal) move([x ? x*5 : start[0], y ? y*5 : start[1],5.4,0],2);
} else if (key.startsWith('center-')) {
    const feature = key.slice(7);
    const internal = ['hole','pocket','x-valley','y-valley'].includes(feature);
    const x = !feature.startsWith('y-'), y = !feature.startsWith('x-');
    if (internal) pocket(feature === 'hole', feature.endsWith('valley') ? feature[0] : '');
    else if (feature === 'boss') cylinder(5,3,[0,0,1.5],'z');
    else box([feature === 'x-ridge' ? 6 : 10,feature === 'y-ridge' ? 6 : 10,3],[0,0,1.5]);
    const start = [0.8,-0.6,internal ? 1.5 : 5.4,0];
    position = start.slice(); move(start,0,0.55);
    for (const axis of [0,1]) {
        if (!(axis === 0 ? x : y)) continue;
        const edge = feature.endsWith('ridge') ? 3 : 5;
        for (const direction of [-1,1]) {
            if (!internal) {
                move([position[0],position[1],5.4,0],0);
                const beyond = position.slice(); beyond[axis] = direction * 7.6;
                move(beyond,0);
                move([position[0],position[1],1.5,0],0);
            }
            const round = feature === 'boss' || feature === 'hole';
            const surface = round ? Math.sqrt(edge * edge - position[1-axis] ** 2) : edge;
            touch(axis, direction * (surface + (internal ? -0.6 : 0.6)));
        }
        if (!internal) move([position[0],position[1],5.4,0],axis === 0 && y ? 0 : 2);
        const centered = position.slice(); centered[axis] = 0;
        move(centered,axis === 0 && y ? 1 : 2);
    }
} else if (key.startsWith('rotary-')) {
    stock.position.z = 3;
    cylinder(3.4,2,[-7,0,0],'x');
    cylinder(2,14,[1,0,0],'x');
    const operation = key.slice(7);
    if (operation === 'axis') {
        mesh(new THREE.BoxGeometry(10,0.18,0.08),
            new THREE.MeshStandardMaterial({color:'#e5bd75',roughness:0.7}),[1,0,2],stock);
        position = [-3,0,7,0]; move(position,0,0.5);
        for (const x of [-3,3]) {
            move([x,0,7,0],0);
            touch(2,5.6);
            move([x,0,7,0],0);
            for (const side of [-1,1]) {
                for (let i=1; i<=6; i++) {
                    const a = side * Math.PI/2 * i/6;
                    move([x,3.8*Math.sin(a),3+3.8*Math.cos(a),a],1,0.15);
                }
                touch(1,side*2.6);
                for (let i=5; i>=0; i--) {
                    const a = side * Math.PI/2 * i/6;
                    move([x,3.8*Math.sin(a),3+3.8*Math.cos(a),a],1,0.15);
                }
            }
        }
        move([-3,0,7,0],2);
    } else {
        // The face turns around X; the probe remains vertical.
        stock.remove(stock.children.at(-1));
        const vertical = operation !== 'horizontal';
        const side = operation === 'verticalNegative' ? -1 : 1;
        // Lean the upper edge away from the vertical probe shaft.
        const initialAngle = vertical ? -side * 0.12 : 0.12;
        box([12,vertical ? 2 : 8,vertical ? 7 : 2],[1,0,0]);
        stock.rotation.x = initialAngle;
        position = vertical ? [0,-side*4,5.2,initialAngle] : [0,-2.5,7,initialAngle];
        move(position,0,0.55);
        for (const angle of [initialAngle,0]) {
            move([position[0],position[1],position[2],angle],1,0.75);
            for (const point of [-1,1]) {
                if (vertical) {
                    move([0,-side*4,3 + point*1.8,angle],0);
                    // The ball center stays one radius outside the tilted face.
                    touch(1,(-side*1.6 - point*1.8*Math.sin(angle)) / Math.cos(angle));
                } else {
                    move([0,point*2.5,7,angle],0);
                    touch(2,4.6 + Math.sin(angle)*point*2.5);
                    move([0,point*2.5,7,angle],0);
                }
            }
        }
        move(vertical ? [0,-side*4,5.2,0] : [0,-2.5,7,0],2);
    }
}
move(position,2,0.9);
// Keep the ball and body in the fixed camera throughout the loop.
camera.updateMatrixWorld();
let extent = 0;
for (const step of moves) {
    for (const point of [step.from,step.to]) {
        for (const z of [-0.6,6.5]) for (const dx of [-0.9,0.9]) for (const dy of [-0.9,0.9]) {
            const projected = new THREE.Vector3(point[0]+dx,point[1]+dy,point[2]+z).project(camera);
            extent = Math.max(extent,Math.abs(projected.x),Math.abs(projected.y));
        }
    }
}
const fit = Math.max(1,extent / 0.92);
camera.left *= fit; camera.right *= fit; camera.top *= fit; camera.bottom *= fit;
camera.updateProjectionMatrix();
const total = moves.reduce((time,step) => time + step.duration, 0);
const marker = mesh(new THREE.SphereGeometry(0.24,12,8),
    new THREE.MeshBasicMaterial({color:'#82deb5',depthTest:false}),[0,0,0]);
marker.renderOrder = 5;
const pathGeometry = new THREE.BufferGeometry();
pathGeometry.setAttribute('position',new THREE.Float32BufferAttribute(new Float32Array((moves.length+2)*3),3));
pathGeometry.attributes.position.setUsage(THREE.DynamicDrawUsage);
const trail = new THREE.Line(pathGeometry,new THREE.LineBasicMaterial({color:'#73bba3',transparent:true,opacity:0.5}));
scene.add(trail);

window.renderFrame = function(fraction) {
    let time = fraction * total, current = moves[0];
    const visited = [moves[0].from.slice(0,3)];
    for (const step of moves) {
        current = step;
        if (time <= step.duration) break;
        time -= step.duration;
        visited.push(step.to.slice(0,3));
    }
    const amount = Math.min(1,Math.max(0,time/current.duration));
    const ease = amount * amount * (3 - 2 * amount);
    const point = current.from.map((value,index) => value + (current.to[index]-value)*ease);
    probe.position.set(...point.slice(0,3));
    const top = probe.position.clone().add(new THREE.Vector3(0,0,6.35)).project(camera);
    window.probeTop = {x:top.x,y:top.y};
    if (key.startsWith('rotary-')) stock.rotation.x = (key === 'rotary-axis' ? -1 : 1) * point[3];
    marker.visible = current.contact && amount > 0.75;
    marker.position.copy(probe.position);
    ballMaterial.color.set(marker.visible ? '#91e3bd' : '#e46864');
    visited.push(point.slice(0,3));
    pathGeometry.setFromPoints(visited.map(point => new THREE.Vector3(...point)));
    pathGeometry.setDrawRange(0,visited.length);
    pathGeometry.computeBoundingSphere();
    renderer.render(scene,camera);
    return current.phase;
};
window.canvas = renderer.domElement;
window.renderFrame(0.12);
