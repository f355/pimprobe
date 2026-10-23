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

export interface Rect {
  left: number;
  top: number;
  width: number;
  height: number;
}
export interface HelpTarget {
  element: HTMLElement;
  rect: Rect;
}
export const targetSelector = '[data-help]:not([data-help=""])';

export function intersection(a: Rect, b: Rect): Rect {
  const left = Math.max(a.left, b.left),
    top = Math.max(a.top, b.top);
  return {
    left,
    top,
    width: Math.max(0, Math.min(a.left + a.width, b.left + b.width) - left),
    height: Math.max(0, Math.min(a.top + a.height, b.top + b.height) - top),
  };
}

export function visibleRect(element: HTMLElement, viewport: Rect): Rect | null {
  if (!element.isConnected || !element.getClientRects().length) return null;
  let rect = intersection(element.getBoundingClientRect(), viewport);
  for (
    let parent: HTMLElement | null = element;
    parent;
    parent = parent.parentElement
  ) {
    const style = getComputedStyle(parent);
    if (
      style.visibility === "hidden" ||
      style.visibility === "collapse" ||
      style.display === "none" ||
      Number(style.opacity) === 0
    )
      return null;
    if (parent === element) continue;
    const bounds = parent.getBoundingClientRect();
    const clip = {
      left: viewport.left,
      top: viewport.top,
      width: viewport.width,
      height: viewport.height,
    };
    if (/(auto|scroll|hidden|clip)/.test(style.overflowX)) {
      clip.left = bounds.left + parent.clientLeft;
      clip.width = parent.clientWidth;
    }
    if (/(auto|scroll|hidden|clip)/.test(style.overflowY)) {
      clip.top = bounds.top + parent.clientTop;
      clip.height = parent.clientHeight;
    }
    rect = intersection(rect, clip);
  }
  return rect.width > 0 && rect.height > 0 ? rect : null;
}

export function contains(rect: Rect, x: number, y: number): boolean {
  return (
    x >= rect.left &&
    x < rect.left + rect.width &&
    y >= rect.top &&
    y < rect.top + rect.height
  );
}

export function popupPosition(
  target: Rect,
  width: number,
  height: number,
  viewport: Rect,
): Rect {
  const margin = 8,
    gap = 12;
  const candidates = [
    [target.left + target.width + gap, target.top],
    [target.left - width - gap, target.top],
    [target.left, target.top + target.height + gap],
    [target.left, target.top - height - gap],
    [viewport.left + margin, viewport.top + margin],
    [
      viewport.left + viewport.width - width - margin,
      viewport.top + viewport.height - height - margin,
    ],
  ];
  let best = {
      left: viewport.left + margin,
      top: viewport.top + margin,
      width,
      height,
    },
    score = Infinity;
  for (const [x, y] of candidates) {
    const rect = {
      left: Math.max(
        viewport.left + margin,
        Math.min(viewport.left + viewport.width - width - margin, x),
      ),
      top: Math.max(
        viewport.top + margin,
        Math.min(viewport.top + viewport.height - height - margin, y),
      ),
      width,
      height,
    };
    const overlap = intersection(rect, target);
    const cost =
      overlap.width * overlap.height * 10 +
      (Math.abs(rect.left + width / 2 - target.left - target.width / 2) +
        Math.abs(rect.top + height / 2 - target.top - target.height / 2)) *
        0.01;
    if (cost < score) {
      best = rect;
      score = cost;
    }
  }
  return best;
}
