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

import {
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
  type PointerEvent,
  type MouseEvent,
  type KeyboardEvent,
  type SyntheticEvent,
} from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { useTranslation } from "../i18n";
import {
  contains,
  intersection,
  popupPosition,
  targetSelector,
  visibleRect,
  type HelpTarget,
  type Rect,
} from "./geometry";
import "./help.css";

export function helpProps(
  title: string,
  text: string,
): { "data-help-title": string; "data-help": string } {
  return { "data-help-title": title, "data-help": text };
}

export interface HelpProviderProps {
  children: ReactNode;
  active: boolean;
  onActiveChange: (active: boolean) => void;
  onMoreInformation?: (target: HTMLElement) => void;
}

type Gesture = {
  element: HTMLElement;
  x: number;
  y: number;
  dragged: boolean;
  scroll: HTMLElement | null;
  left: number;
  top: number;
};

function scrollParent(
  element: HTMLElement,
  root: HTMLElement,
): HTMLElement | null {
  for (
    let parent: HTMLElement | null = element;
    parent;
    parent = parent.parentElement
  ) {
    const style = getComputedStyle(parent);
    if (
      (/(auto|scroll)/.test(style.overflowY) &&
        parent.scrollHeight > parent.clientHeight) ||
      (/(auto|scroll)/.test(style.overflowX) &&
        parent.scrollWidth > parent.clientWidth)
    )
      return parent;
    if (parent === root) break;
  }
  return null;
}

export function HelpProvider({
  children,
  active,
  onActiveChange,
  onMoreInformation,
}: HelpProviderProps) {
  const { t } = useTranslation();
  const root = useRef<HTMLDivElement>(null);
  const popup = useRef<HTMLDivElement>(null);
  const gesture = useRef<Gesture | null>(null);
  const requestedTarget = useRef<HTMLElement | null>(null);
  const [targets, setTargets] = useState<HelpTarget[]>([]);
  const [selected, setSelected] = useState<HTMLElement | null>(null);
  const [modal, setModal] = useState<HTMLDialogElement | null>(null);
  const [viewport, setViewport] = useState<Rect>({
    left: 0,
    top: 0,
    width: 0,
    height: 0,
  });
  const [size, setSize] = useState({ width: 336, height: 240 });
  const titleId = useId(),
    textId = useId();
  const chosen = active
    ? targets.find((item) => item.element === selected)
    : undefined;

  useLayoutEffect(() => {
    setSelected(requestedTarget.current);
    requestedTarget.current = null;
    gesture.current = null;
    if (!active || !root.current) return;
    const scope = root.current;
    let frame = 0;
    function refresh() {
      const visual = window.visualViewport;
      const area = intersection(scope.getBoundingClientRect(), {
        left: visual?.offsetLeft || 0,
        top: visual?.offsetTop || 0,
        width: visual?.width || window.innerWidth,
        height: visual?.height || window.innerHeight,
      });
      setViewport((previous) =>
        JSON.stringify(previous) === JSON.stringify(area) ? previous : area,
      );
      // A native modal is in the top layer; its help must be in that layer too.
      const dialogs = [
        ...scope.querySelectorAll<HTMLDialogElement>("dialog:modal"),
      ];
      const top = dialogs.at(-1) || null;
      setModal(top);
      const found = [
        ...(top || scope).querySelectorAll<HTMLElement>(targetSelector),
      ]
        .filter((element) => element.dataset.help?.trim())
        .flatMap((element) => {
          const rect = visibleRect(element, area);
          return rect ? [{ element, rect }] : [];
        });
      setTargets((previous) =>
        previous.length === found.length &&
        previous.every(
          (item, index) =>
            item.element === found[index].element &&
            JSON.stringify(item.rect) === JSON.stringify(found[index].rect),
        )
          ? previous
          : found,
      );
      setSelected((previous) =>
        previous && found.some((item) => item.element === previous)
          ? previous
          : null,
      );
    }
    function schedule() {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(refresh);
    }
    const observer = new MutationObserver(schedule);
    observer.observe(scope, {
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: [
        "data-help",
        "data-help-title",
        "class",
        "style",
        "hidden",
        "open",
      ],
    });
    const resize = new ResizeObserver(schedule);
    resize.observe(scope);
    window.addEventListener("resize", schedule);
    window.addEventListener("scroll", schedule, true);
    visualViewport?.addEventListener("resize", schedule);
    visualViewport?.addEventListener("scroll", schedule);
    const timer = window.setInterval(refresh, 250);
    refresh();
    return () => {
      observer.disconnect();
      resize.disconnect();
      cancelAnimationFrame(frame);
      clearInterval(timer);
      window.removeEventListener("resize", schedule);
      window.removeEventListener("scroll", schedule, true);
      visualViewport?.removeEventListener("resize", schedule);
      visualViewport?.removeEventListener("scroll", schedule);
    };
  }, [active]);

  useLayoutEffect(() => {
    if (!chosen || !popup.current) return;
    function measure() {
      const bounds = popup.current?.getBoundingClientRect();
      if (bounds)
        setSize((previous) =>
          previous.width === bounds.width && previous.height === bounds.height
            ? previous
            : { width: bounds.width, height: bounds.height },
        );
    }
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(popup.current);
    return () => observer.disconnect();
  }, [chosen?.element, modal, viewport.width, viewport.height, t]);

  function isPanel(event: SyntheticEvent): boolean {
    return (
      event.target instanceof Element &&
      !!event.target.closest("[data-pp-help-panel]")
    );
  }
  function closeExplanation() {
    const element = chosen?.element;
    setSelected(null);
    const target = element?.matches(
      "button, input, select, textarea, [tabindex]",
    )
      ? element
      : element?.querySelector<HTMLElement>(
          "input, select, textarea, button, [tabindex]",
        );
    target?.focus({ preventScroll: true });
  }
  function at(event: {
    target: EventTarget;
    clientX?: number;
    clientY?: number;
  }): HelpTarget | undefined {
    if (
      event.target instanceof Element &&
      event.target.closest("[data-help-bypass]")
    )
      return undefined;
    const scope = modal || root.current;
    if (!scope) return undefined;
    const element =
      event.target instanceof Element
        ? event.target.closest<HTMLElement>(targetSelector)
        : null;
    // A scroll can move targets before the highlight snapshot refreshes.
    if (element && scope.contains(element)) {
      const rect = visibleRect(element, viewport);
      if (rect) return { element, rect };
    }
    if (event.clientX === undefined || event.clientY === undefined)
      return undefined;
    // Disabled form controls can be skipped by browser hit testing.
    for (const element of [
      ...scope.querySelectorAll<HTMLElement>(targetSelector),
    ].reverse()) {
      const rect = visibleRect(element, viewport);
      if (rect && contains(rect, event.clientX, event.clientY))
        return { element, rect };
    }
    return undefined;
  }
  function pointerDown(event: PointerEvent<HTMLDivElement>) {
    if (!active || isPanel(event)) return;
    const target = at(event);
    if (!target) return;
    event.stopPropagation();
    if (event.pointerType === "mouse") event.preventDefault();
    const scroll = scrollParent(target.element, root.current!);
    gesture.current = {
      element: target.element,
      x: event.clientX,
      y: event.clientY,
      dragged: false,
      scroll,
      left: scroll?.scrollLeft || 0,
      top: scroll?.scrollTop || 0,
    };
  }
  function pointerMove(event: PointerEvent<HTMLDivElement>) {
    if (!active || !gesture.current) return;
    const current = gesture.current;
    event.stopPropagation();
    if (Math.hypot(event.clientX - current.x, event.clientY - current.y) > 8)
      current.dragged = true;
    if (current.dragged) {
      setSelected(null);
      if (event.pointerType === "mouse" && current.scroll) {
        current.scroll.scrollLeft = current.left - (event.clientX - current.x);
        current.scroll.scrollTop = current.top - (event.clientY - current.y);
      }
    }
  }
  function pointerUp(event: PointerEvent<HTMLDivElement>) {
    if (!active || !gesture.current) return;
    event.stopPropagation();
    const current = gesture.current;
    gesture.current = null;
    if (!current.dragged && at(event)?.element === current.element)
      setSelected(current.element);
  }
  function blockAction(event: MouseEvent<HTMLDivElement>) {
    if (!active || isPanel(event)) return;
    if (
      event.target instanceof Element &&
      event.target.closest("[data-help-bypass]")
    )
      return;
    const target = at(event);
    const control =
      event.target instanceof Element &&
      event.target.closest(
        "button, input, select, textarea, a, summary, [role='button']",
      );
    if (!target && !control) return;
    event.preventDefault();
    event.stopPropagation();
    if (target && event.type === "click" && event.detail === 0)
      setSelected(target.element);
  }
  function keyboard(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === "F1") {
      const element =
        event.target instanceof Element
          ? event.target.closest<HTMLElement>(targetSelector)
          : null;
      if (
        !element ||
        isPanel(event) ||
        element.closest(".pp-help-root") !== root.current
      )
        return;
      event.preventDefault();
      event.stopPropagation();
      if (event.type === "keydown") {
        if (active) setSelected(element);
        else {
          requestedTarget.current = element;
          onActiveChange(true);
        }
      }
      return;
    }
    if (!active) return;
    if (isPanel(event)) {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        closeExplanation();
      }
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      if (selected) closeExplanation();
      else onActiveChange(false);
    } else if (event.key !== "Tab" && at(event)) {
      event.preventDefault();
      event.stopPropagation();
      if (
        event.type === "keydown" &&
        (event.key === "Enter" || event.key === " ")
      )
        setSelected(at(event)!.element);
    }
  }

  const position = chosen
    ? popupPosition(chosen.rect, size.width, size.height, viewport)
    : null;
  const layer = active && (
    <div className="pp-help-layer" aria-live="polite">
      {targets.map((target, index) => (
        <div
          key={index}
          aria-hidden="true"
          className={`pp-help-highlight${target.element === selected ? " pp-help-chosen" : ""}`}
          style={{
            left: target.rect.left,
            top: target.rect.top,
            width: target.rect.width,
            height: target.rect.height,
          }}
        />
      ))}
      {chosen && position && (
        <div
          ref={popup}
          className="pp-help-popup"
          data-pp-help-panel=""
          role="dialog"
          aria-modal="false"
          aria-labelledby={titleId}
          aria-describedby={textId}
          style={{
            left: position.left,
            top: position.top,
            maxWidth: Math.max(0, viewport.width - 16),
            maxHeight: Math.max(0, viewport.height - 16),
          }}
        >
          <div className="pp-help-popup-header">
            <h2 id={titleId}>{t(chosen.element.dataset.helpTitle || "")}</h2>
            <button
              type="button"
              className="pp-help-icon"
              aria-label={t("Close explanation")}
              title={t("Close explanation")}
              onClick={closeExplanation}
            >
              <X aria-hidden="true" size={28} />
            </button>
          </div>
          <div className="pp-help-popup-text" id={textId}>
            {t(chosen.element.dataset.help || "")}
          </div>
          {onMoreInformation && chosen.element.dataset.helpGuide && (
            <button
              type="button"
              className="pp-help-more"
              onClick={() => onMoreInformation(chosen.element)}
            >
              {t("More information")}
            </button>
          )}
        </div>
      )}
    </div>
  );

  return (
    <div
      ref={root}
      className={`pp-help-root${active ? " pp-help-active" : ""}`}
      onPointerDownCapture={pointerDown}
      onPointerMoveCapture={pointerMove}
      onPointerUpCapture={pointerUp}
      onPointerCancelCapture={() => {
        gesture.current = null;
        setSelected(null);
      }}
      onClickCapture={blockAction}
      onDoubleClickCapture={blockAction}
      onContextMenuCapture={blockAction}
      onMouseDownCapture={blockAction}
      onMouseUpCapture={blockAction}
      onTouchStartCapture={(event) => {
        if (active && !isPanel(event) && at(event)) event.stopPropagation();
      }}
      onFocusCapture={(event) => {
        if (active && !isPanel(event) && at(event)) event.stopPropagation();
      }}
      onKeyDownCapture={keyboard}
      onKeyUpCapture={keyboard}
    >
      {children}
      {modal && active ? createPortal(layer, modal) : layer}
    </div>
  );
}
