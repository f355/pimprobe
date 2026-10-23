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

import { useEffect, useRef } from "react";
import manifest from "../assets/animations/manifest.json";
import "./diagrams.css";

export interface RoutineDiagramProps {
  family: string;
  feature?: string;
  x?: number;
  y?: number;
  z?: boolean;
}

export interface MotionIllustrationProps {
  clip: string;
  onPhase?: (phase: number) => void;
}

type Context = CanvasRenderingContext2D;
type Point = readonly [number, number];
type Palette = ReturnType<typeof palette>;

function palette(canvas: HTMLCanvasElement) {
  const style = getComputedStyle(canvas);
  const color = (name: string, fallback: string) =>
    style.getPropertyValue(`--pp-${name}`).trim() || fallback;
  return {
    stock: color("stock", "#586B64"),
    stockEdge: color("stock-edge", "#A2B9B0"),
    x: color("x", "#F18B8B"),
    y: color("y", "#7ED5B1"),
    z: color("z", "#88B8F1"),
    text: color("text", "#F0F3F3"),
    muted: color("muted", "#B1BDC0"),
  };
}

function arrow(
  c: Context,
  fromX: number,
  fromY: number,
  toX: number,
  toY: number,
) {
  const angle = Math.atan2(toY - fromY, toX - fromX);
  c.beginPath();
  c.moveTo(fromX, fromY);
  c.lineTo(toX, toY);
  for (const side of [-1, 1]) {
    c.moveTo(toX, toY);
    c.lineTo(
      toX - 6 * Math.cos(angle + (side * Math.PI) / 5),
      toY - 6 * Math.sin(angle + (side * Math.PI) / 5),
    );
  }
  c.stroke();
}

function crosshair(c: Context, x: number, y: number) {
  c.lineWidth = 2;
  c.beginPath();
  c.arc(x, y, 7, 0, Math.PI * 2);
  c.stroke();
  c.beginPath();
  c.moveTo(x - 10, y);
  c.lineTo(x + 10, y);
  c.moveTo(x, y - 10);
  c.lineTo(x, y + 10);
  c.stroke();
}

function referencePoint(c: Context, x: number, y: number, colors: string[]) {
  c.save();
  colors.forEach((color, index) => {
    c.fillStyle = color;
    c.beginPath();
    c.moveTo(x, y);
    c.arc(
      x,
      y,
      4.5,
      -Math.PI / 2 + (index * Math.PI * 2) / colors.length,
      -Math.PI / 2 + ((index + 1) * Math.PI * 2) / colors.length,
    );
    c.closePath();
    c.fill();
  });
  c.restore();
}

function cornerPoint(
  c: Context,
  x: number,
  y: number,
  xSide: number,
  ySide: number,
  p: Palette,
) {
  c.save();
  c.translate(x, y);
  c.scale(xSide, ySide);
  // X follows the vertical edge, Y the horizontal edge.
  c.rotate((3 * Math.PI) / 4);
  referencePoint(c, 0, 0, [p.x, p.y]);
  c.restore();
}

function referenceLine(
  c: Context,
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  color: string,
) {
  c.save();
  c.strokeStyle = color;
  c.lineWidth = 3;
  c.beginPath();
  c.moveTo(x1, y1);
  c.lineTo(x2, y2);
  c.stroke();
  c.restore();
}

function referenceDashes(
  c: Context,
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  colors: string[],
  gap = 6,
) {
  c.save();
  const length = Math.hypot(x2 - x1, y2 - y1);
  const angle = Math.atan2(y2 - y1, x2 - x1);
  c.lineWidth = 2.5;
  c.lineCap = "butt";
  for (
    let distance = 0, dash = 0;
    distance < length;
    distance += 6 + gap, ++dash
  ) {
    const end = Math.min(distance + 6, length);
    c.strokeStyle = colors[dash % colors.length];
    c.beginPath();
    c.moveTo(x1 + distance * Math.cos(angle), y1 + distance * Math.sin(angle));
    c.lineTo(x1 + end * Math.cos(angle), y1 + end * Math.sin(angle));
    c.stroke();
  }
  c.restore();
}

function referencePlane(
  c: Context,
  left: number,
  top: number,
  right: number,
  bottom: number,
  color: string,
) {
  c.save();
  c.beginPath();
  c.rect(left, top, right - left, bottom - top);
  c.clip();
  c.fillStyle = color;
  c.globalAlpha = 0.12;
  c.fillRect(left, top, right - left, bottom - top);
  c.globalAlpha = 1;
  c.strokeStyle = color;
  c.lineWidth = 1;
  for (let offset = top - bottom; offset < right - left; offset += 12) {
    c.beginPath();
    c.moveTo(left + offset, bottom);
    c.lineTo(left + offset + bottom - top, top);
    c.stroke();
  }
  c.restore();
}

function downwardTouch(
  c: Context,
  x: number,
  y: number,
  start: boolean,
  color: string,
) {
  c.save();
  c.strokeStyle = color;
  c.lineWidth = 2;
  c.beginPath();
  c.arc(x, y, 9, 0, Math.PI * 2);
  c.moveTo(x - 4, y - 4);
  c.lineTo(x + 4, y + 4);
  c.moveTo(x - 4, y + 4);
  c.lineTo(x + 4, y - 4);
  if (start) {
    c.moveTo(x - 15, y);
    c.lineTo(x - 10, y);
    c.moveTo(x + 10, y);
    c.lineTo(x + 15, y);
    c.moveTo(x, y - 15);
    c.lineTo(x, y - 10);
    c.moveTo(x, y + 10);
    c.lineTo(x, y + 15);
  }
  c.stroke();
  c.restore();
}

function outside(
  c: Context,
  width: number,
  height: number,
  p: Palette,
  x: number,
  y: number,
  z: boolean,
) {
  const margin = 5;
  const left = x > 0 ? width * 0.38 : margin;
  const right = x < 0 ? width * 0.62 : width - margin;
  const top = y < 0 ? height * 0.38 : margin;
  const bottom = y > 0 ? height * 0.62 : height - margin;
  c.fillStyle = p.stock;
  c.fillRect(left, top, right - left, bottom - top);
  c.strokeStyle = p.stockEdge;
  c.lineWidth = 1;
  c.strokeRect(left, top, right - left, bottom - top);
  const targetX = x > 0 ? left : x < 0 ? right : width / 2;
  const targetY = y < 0 ? top : y > 0 ? bottom : height / 2;
  if (z) referencePlane(c, left, top, right, bottom, p.z);
  else if (x && y) cornerPoint(c, targetX, targetY, x, -y, p);
  else if (x) referenceLine(c, targetX, top, targetX, bottom, p.x);
  else referenceLine(c, left, targetY, right, targetY, p.y);

  const probeX = (left + right) / 2;
  const probeY = (top + bottom) / 2;
  c.strokeStyle = p.text;
  c.lineWidth = 3;
  if (x > 0) arrow(c, margin, probeY, left - 3, probeY);
  else if (x < 0) arrow(c, width - margin, probeY, right + 3, probeY);
  if (y < 0) arrow(c, probeX, margin, probeX, top - 3);
  else if (y > 0) arrow(c, probeX, height - margin, probeX, bottom + 3);
  crosshair(c, probeX, probeY);
}

function inside(
  c: Context,
  width: number,
  height: number,
  p: Palette,
  x: number,
  y: number,
) {
  const margin = 5;
  const left = width * 0.3,
    right = width * 0.7;
  const top = height * 0.3,
    bottom = height * 0.7;
  const probeX = width / 2 - x * 14;
  const probeY = height / 2 + y * 14;
  c.fillStyle = p.stock;
  if (x < 0) c.fillRect(margin, margin, left - margin, height - 2 * margin);
  else if (x > 0)
    c.fillRect(right, margin, width - right - margin, height - 2 * margin);
  if (y > 0) c.fillRect(margin, margin, width - 2 * margin, top - margin);
  else if (y < 0)
    c.fillRect(margin, bottom, width - 2 * margin, height - bottom - margin);
  c.strokeStyle = p.stockEdge;
  c.lineWidth = 1;
  c.beginPath();
  if (x < 0) {
    c.moveTo(left, y > 0 ? top : margin);
    c.lineTo(left, y < 0 ? bottom : height - margin);
  } else if (x > 0) {
    c.moveTo(right, y > 0 ? top : margin);
    c.lineTo(right, y < 0 ? bottom : height - margin);
  }
  if (y > 0) {
    c.moveTo(x < 0 ? left : margin, top);
    c.lineTo(x > 0 ? right : width - margin, top);
  } else if (y < 0) {
    c.moveTo(x < 0 ? left : margin, bottom);
    c.lineTo(x > 0 ? right : width - margin, bottom);
  }
  c.stroke();
  const targetX = x < 0 ? left : x > 0 ? right : width / 2;
  const targetY = y > 0 ? top : y < 0 ? bottom : height / 2;
  if (x && y) cornerPoint(c, targetX, targetY, -x, y, p);
  else if (x) referenceLine(c, targetX, margin, targetX, height - margin, p.x);
  else referenceLine(c, margin, targetY, width - margin, targetY, p.y);
  c.strokeStyle = p.text;
  c.lineWidth = 3;
  if (x < 0) arrow(c, probeX - 5, probeY, left + 2, probeY);
  else if (x > 0) arrow(c, probeX + 5, probeY, right - 2, probeY);
  if (y > 0) arrow(c, probeX, probeY - 5, probeX, top + 2);
  else if (y < 0) arrow(c, probeX, probeY + 5, probeX, bottom - 2);
  crosshair(c, probeX, probeY);
}

function center(
  c: Context,
  width: number,
  height: number,
  p: Palette,
  feature: string,
) {
  const cx = width / 2,
    cy = height / 2;
  const internal =
    feature === "hole" || feature === "pocket" || feature.includes("valley");
  const round = feature === "boss" || feature === "hole";
  const onlyX = feature.startsWith("x-"),
    onlyY = feature.startsWith("y-");
  const z = feature === "z";
  const radius = Math.min(
    width * 0.3,
    height * (feature === "boss" || feature === "block" ? 0.25 : 0.35),
  );
  let left = cx - radius,
    right = cx + radius,
    top = cy - radius,
    bottom = cy + radius;
  if (onlyX) {
    left = cx - width * 0.18;
    right = cx + width * 0.18;
    top = internal ? 0 : 3;
    bottom = internal ? height : height - 3;
  }
  if (onlyY) {
    left = internal ? 0 : 3;
    right = internal ? width : width - 3;
    top = cy - height * 0.23;
    bottom = cy + height * 0.23;
  }
  c.fillStyle = p.stock;
  if (internal) {
    c.fillRect(0, 0, width, height);
    // The opening shows the surrounding control's background.
    c.globalCompositeOperation = "destination-out";
  }
  c.beginPath();
  if (round) c.arc(cx, cy, radius, 0, Math.PI * 2);
  else c.rect(left, top, right - left, bottom - top);
  c.fill();
  c.globalCompositeOperation = "source-over";
  c.strokeStyle = p.stockEdge;
  c.lineWidth = 1;
  if (!onlyX && !onlyY) c.stroke();
  else {
    c.beginPath();
    if (onlyX) {
      c.moveTo(left, top);
      c.lineTo(left, bottom);
      c.moveTo(right, top);
      c.lineTo(right, bottom);
    } else {
      c.moveTo(left, top);
      c.lineTo(right, top);
      c.moveTo(left, bottom);
      c.lineTo(right, bottom);
    }
    c.stroke();
  }
  if (z) referencePlane(c, left, top, right, bottom, p.z);
  else if (onlyX) referenceDashes(c, cx, 0, cx, height, [p.x]);
  else if (onlyY) referenceDashes(c, 0, cy, width, cy, [p.y]);
  c.strokeStyle = p.text;
  c.lineWidth = 2.5;
  if (!z && !onlyY) {
    arrow(c, internal ? cx - 8 : 3, cy, left + (internal ? 2 : -2), cy);
    arrow(
      c,
      internal ? cx + 8 : width - 3,
      cy,
      right + (internal ? -2 : 2),
      cy,
    );
  }
  if (!z && !onlyX) {
    arrow(c, cx, internal ? cy - 8 : 2, cx, top + (internal ? 2 : -2));
    arrow(
      c,
      cx,
      internal ? cy + 8 : height - 2,
      cx,
      bottom + (internal ? -2 : 2),
    );
  }
  crosshair(c, cx, cy);
  if (!z && !onlyX && !onlyY) referencePoint(c, cx, cy, [p.x, p.y]);
}

function stock(c: Context, points: Point[], p: Palette) {
  c.fillStyle = p.stock;
  c.strokeStyle = p.stockEdge;
  c.lineWidth = 1.5;
  c.beginPath();
  points.forEach(([x, y], index) => (index ? c.lineTo(x, y) : c.moveTo(x, y)));
  c.closePath();
  c.fill();
  c.stroke();
}

function angle(
  c: Context,
  width: number,
  height: number,
  p: Palette,
  feature: string,
) {
  c.scale(width / 160, height / 90);
  const approach = (
    fromX: number,
    fromY: number,
    toX: number,
    toY: number,
    start: boolean,
  ) => {
    c.strokeStyle = p.text;
    c.lineWidth = 2.5;
    arrow(c, fromX, fromY, toX, toY);
    if (start) crosshair(c, fromX, fromY);
  };
  const traverse = (fromX: number, fromY: number, toX: number, toY: number) => {
    c.save();
    c.setLineDash([3, 4]);
    c.strokeStyle = p.muted;
    c.lineWidth = 1.5;
    arrow(c, fromX, fromY, toX, toY);
    c.restore();
  };
  const negative = feature.includes("minus");
  if (feature.startsWith("z-")) {
    stock(
      c,
      [
        [10, 7],
        [150, 7],
        [150, 83],
        [10, 83],
      ],
      p,
    );
    const alongX = feature === "z-x";
    referencePlane(
      c,
      alongX ? 14 : 63,
      alongX ? 29 : 11,
      alongX ? 146 : 97,
      alongX ? 61 : 79,
      p.z,
    );
    referenceDashes(
      c,
      alongX ? 0 : 80,
      alongX ? 45 : 0,
      alongX ? 160 : 80,
      alongX ? 45 : 90,
      [p.z],
    );
    downwardTouch(c, alongX ? 34 : 80, alongX ? 45 : 67, true, p.text);
    downwardTouch(c, alongX ? 126 : 80, alongX ? 45 : 23, false, p.text);
  } else if (feature.startsWith("x-")) {
    if (negative) {
      c.translate(160, 0);
      c.scale(-1, 1);
    }
    const edge = (y: number) => 94 + (45 - y) * 0.18;
    stock(
      c,
      [
        [edge(5), 5],
        [152, 5],
        [152, 85],
        [edge(85), 85],
      ],
      p,
    );
    referenceDashes(c, edge(0), 3, edge(0), 87, [p.muted]);
    c.strokeStyle = p.x;
    c.lineWidth = 3;
    c.beginPath();
    c.arc(edge(0), 0, 64, Math.PI / 2, Math.PI / 2 + Math.atan(0.18));
    c.stroke();
    approach(28, 72, edge(72) - 5, 72, true);
    approach(28, 18, edge(18) - 5, 18, false);
    traverse(28, 56, 28, 32);
  } else {
    if (!negative) {
      c.translate(0, 90);
      c.scale(1, -1);
    }
    const edge = (x: number) => 46 + (x - 80) * 0.16;
    stock(
      c,
      [
        [9, edge(9)],
        [151, edge(151)],
        [151, 85],
        [9, 85],
      ],
      p,
    );
    referenceDashes(c, 5, edge(160), 155, edge(160), [p.muted]);
    c.strokeStyle = p.y;
    c.lineWidth = 3;
    c.beginPath();
    c.arc(160, edge(160), 80, Math.PI, Math.PI + Math.atan(0.16));
    c.stroke();
    approach(28, 13, 28, edge(28) - 5, true);
    approach(130, 13, 130, edge(130) - 5, false);
    traverse(47, 13, 113, 13);
  }
}

function rotaryAxis(c: Context, width: number, height: number, p: Palette) {
  const cy = height / 2;
  const chuckLeft = width * 0.06,
    chuckRight = width * 0.23;
  const rodRight = width * 0.94;
  const rodTop = cy - height * 0.09,
    rodBottom = cy + height * 0.09;
  c.fillStyle = p.stock;
  c.strokeStyle = p.stockEdge;
  c.lineWidth = 2;
  c.fillRect(chuckRight, rodTop, rodRight - chuckRight, rodBottom - rodTop);
  c.strokeRect(chuckRight, rodTop, rodRight - chuckRight, rodBottom - rodTop);
  for (const side of [-1, 1]) {
    c.save();
    c.translate(chuckRight, cy);
    c.scale(1, side);
    c.beginPath();
    c.moveTo(0, -height * 0.28);
    c.lineTo(width * 0.065, -height * 0.28);
    c.lineTo(width * 0.065, -height * 0.2);
    c.lineTo(width * 0.13, -height * 0.2);
    c.lineTo(width * 0.13, -height * 0.08);
    c.lineTo(0, -height * 0.08);
    c.closePath();
    c.fill();
    c.stroke();
    c.restore();
  }
  c.fillRect(chuckLeft, height * 0.15, chuckRight - chuckLeft, height * 0.7);
  c.strokeRect(chuckLeft, height * 0.15, chuckRight - chuckLeft, height * 0.7);
  referenceDashes(c, 0, cy, width, cy, [p.z, p.y], 0);
  const first = width * 0.43;
  for (const x of [first, width * 0.77]) {
    c.strokeStyle = p.text;
    c.lineWidth = 3;
    arrow(c, x, height * 0.12, x, rodTop - 4);
    arrow(c, x, height * 0.88, x, rodBottom + 4);
  }
  crosshair(c, first, cy);
  referencePoint(c, first, cy, [p.y, p.z]);
}

function rotaryLevel(
  c: Context,
  width: number,
  height: number,
  p: Palette,
  feature: string,
) {
  const vertical = feature === "vertical" || feature === "verticalNegative";
  const size = Math.min(width, height);
  const span = size * 0.7,
    thickness = size * 0.17,
    surface = size * 0.03;
  const halfSpacing = span * 0.25;
  const tilt = ((vertical ? 1 : -1) * Math.PI) / 12;
  c.translate(width / 2, height / 2);
  c.save();
  const chuckRadius = size * 0.43;
  c.globalAlpha = 0.18;
  c.strokeStyle = p.text;
  c.lineWidth = 1.5;
  c.beginPath();
  c.arc(0, 0, chuckRadius, 0, Math.PI * 2);
  c.stroke();
  for (let jaw = 0; jaw < 4; ++jaw) {
    c.save();
    c.rotate((jaw * Math.PI) / 2);
    c.strokeRect(
      chuckRadius * 0.55,
      -size * 0.045,
      chuckRadius * 0.4,
      size * 0.09,
    );
    c.restore();
  }
  c.restore();
  if (feature === "verticalNegative") c.scale(-1, 1);
  if (vertical) c.rotate(-Math.PI / 2);
  c.save();
  c.translate(0, surface + thickness / 2);
  c.rotate(tilt);
  c.fillStyle = p.stock;
  c.strokeStyle = p.stockEdge;
  c.lineWidth = 2;
  c.fillRect(-span / 2, -thickness / 2, span, thickness);
  c.strokeRect(-span / 2, -thickness / 2, span, thickness);
  c.restore();
  c.save();
  c.setLineDash([2, 4]);
  c.strokeStyle = p.muted;
  c.strokeRect(-span / 2, surface, span, thickness);
  c.restore();
  referenceDashes(
    c,
    -span / 2 - size * 0.08,
    surface,
    span / 2 + size * 0.08,
    surface,
    [vertical ? p.y : p.z],
  );
  c.strokeStyle = p.text;
  c.lineWidth = 3;
  for (const x of [-halfSpacing, halfSpacing]) {
    const contact =
      surface +
      thickness / 2 +
      x * Math.tan(tilt) -
      thickness / (2 * Math.cos(tilt));
    arrow(c, x, -size * 0.27, x, contact - 5);
  }
  crosshair(c, -halfSpacing, -size * 0.27);
  const radius = size * 0.36;
  const start = Math.PI * (vertical ? 0.72 : 0.28);
  const end = Math.PI * (vertical ? 0.28 : 0.72);
  const beforeEnd = end + (vertical ? 0.12 : -0.12);
  c.strokeStyle = p.text;
  c.lineWidth = 2;
  c.beginPath();
  c.arc(0, 0, radius, start, end, vertical);
  c.stroke();
  arrow(
    c,
    radius * Math.cos(beforeEnd),
    radius * Math.sin(beforeEnd),
    radius * Math.cos(end),
    radius * Math.sin(end),
  );
}

export function RoutineDiagram({
  family,
  feature = "",
  x = 0,
  y = 0,
  z = false,
}: RoutineDiagramProps) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const canvas = ref.current;
    const c = canvas?.getContext("2d");
    if (!canvas || !c) return;
    const paint = () => {
      const { width, height } = canvas.getBoundingClientRect();
      if (!width || !height) return;
      const ratio = window.devicePixelRatio || 1;
      canvas.width = Math.round(width * ratio);
      canvas.height = Math.round(height * ratio);
      c.scale(canvas.width / width, canvas.height / height);
      const p = palette(canvas);
      if (family === "center") center(c, width, height, p, z ? "z" : feature);
      else if (family === "angle") angle(c, width, height, p, feature);
      else if (family === "rotary") {
        if (!feature || feature === "axis") rotaryAxis(c, width, height, p);
        else rotaryLevel(c, width, height, p, feature);
      } else if (family === "inside" && !z)
        inside(c, width, height, p, Math.sign(x), Math.sign(y));
      else outside(c, width, height, p, Math.sign(x), Math.sign(y), z);
    };
    paint();
    const resize = new ResizeObserver(paint);
    resize.observe(canvas);
    // Repaint inherited colors when a theme or pressed control changes.
    const theme = new MutationObserver(paint);
    for (
      let element: Element | null = canvas;
      element;
      element = element.parentElement
    ) {
      theme.observe(element, {
        attributes: true,
        attributeFilter: ["class", "style"],
      });
    }
    window.addEventListener("resize", paint);
    return () => {
      resize.disconnect();
      theme.disconnect();
      window.removeEventListener("resize", paint);
    };
  }, [family, feature, x, y, z]);
  return (
    <canvas
      ref={ref}
      className="pp-routine-diagram"
      width={92}
      height={92}
      aria-hidden="true"
    />
  );
}

interface ClipMetadata {
  frames: number;
  width: number;
  height: number;
  rate: number;
  phases: number[];
}

const clips: Record<string, ClipMetadata> = manifest;

export function MotionIllustration({ clip, onPhase }: MotionIllustrationProps) {
  const ref = useRef<HTMLCanvasElement>(null);
  const phaseCallback = useRef(onPhase);
  useEffect(() => {
    phaseCallback.current = onPhase;
  }, [onPhase]);
  const metadata = Object.hasOwn(clips, clip) ? clips[clip] : undefined;
  useEffect(() => {
    const canvas = ref.current;
    const c = canvas?.getContext("2d");
    if (!canvas || !c) return;
    c.clearRect(0, 0, canvas.width, canvas.height);
    phaseCallback.current?.(0);
    if (!metadata) return;
    const image = new Image();
    let disposed = false;
    let loaded = false;
    let visible = true;
    let request = 0;
    let frame = -1;
    let phase = 0;
    let elapsed = 0;
    let previousTime: number | undefined;
    const draw = (next: number) => {
      if (next === frame) return;
      frame = next;
      const columns = Math.floor(image.naturalWidth / metadata.width);
      c.clearRect(0, 0, metadata.width, metadata.height);
      c.drawImage(
        image,
        (frame % columns) * metadata.width,
        Math.floor(frame / columns) * metadata.height,
        metadata.width,
        metadata.height,
        0,
        0,
        metadata.width,
        metadata.height,
      );
      const nextPhase = metadata.phases[frame] ?? 0;
      if (nextPhase !== phase) {
        phase = nextPhase;
        phaseCallback.current?.(phase);
      }
    };
    const tick = (time: number) => {
      if (disposed) return;
      if (previousTime !== undefined) elapsed += time - previousTime;
      previousTime = time;
      draw(Math.floor((elapsed * metadata.rate) / 1000) % metadata.frames);
      request = requestAnimationFrame(tick);
    };
    const updatePlayback = () => {
      cancelAnimationFrame(request);
      previousTime = undefined;
      if (loaded && visible && !document.hidden && !disposed)
        request = requestAnimationFrame(tick);
    };
    const observer = new IntersectionObserver(([entry]) => {
      visible = entry.isIntersecting;
      updatePlayback();
    });
    observer.observe(canvas);
    document.addEventListener("visibilitychange", updatePlayback);
    image.onload = () => {
      if (
        disposed ||
        image.naturalWidth < metadata.width ||
        image.naturalHeight < metadata.height
      )
        return;
      loaded = true;
      draw(0);
      updatePlayback();
    };
    image.src = `/media/animations/${clip}.png`;
    return () => {
      disposed = true;
      cancelAnimationFrame(request);
      observer.disconnect();
      document.removeEventListener("visibilitychange", updatePlayback);
      image.onload = null;
    };
  }, [clip, metadata]);
  return (
    <canvas
      ref={ref}
      className="pp-motion-illustration"
      width={metadata?.width ?? 320}
      height={metadata?.height ?? 176}
      aria-hidden="true"
    />
  );
}
