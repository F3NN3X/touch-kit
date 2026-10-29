// Pointer-drag helpers. Touches arrive as synthetic PointerEvents (see touch/bridge.ts):
// setPointerCapture throws for them, and move/up bubble to the element that got pointerdown.

import type { PointerEvent } from "react";

/** Window px per design px; the stage is designed at 1920 wide. */
export const getScale = (): number => window.innerWidth / 1920;

type Handler = (ev: globalThis.PointerEvent) => void;

/** Follows one pointer from the pointerdown on `e.currentTarget` until it lifts or cancels. */
export function track(e: PointerEvent<HTMLElement>, move?: Handler, up?: Handler): void {
  const el = e.currentTarget;
  const id = e.pointerId;
  try {
    el.setPointerCapture(id);
  } catch {
    // synthetic pointer ids can't be captured; the bridge already routes them to this element
  }
  const onMove: Handler = (ev) => {
    if (ev.pointerId === id) move?.(ev);
  };
  const end: Handler = (ev) => {
    if (ev.pointerId !== id) return;
    el.removeEventListener("pointermove", onMove);
    el.removeEventListener("pointerup", end);
    el.removeEventListener("pointercancel", end);
    up?.(ev);
  };
  el.addEventListener("pointermove", onMove);
  el.addEventListener("pointerup", end);
  el.addEventListener("pointercancel", end);
}

/** Slide behaviour: applies `apply` at the press point, then on every move. */
export function trackJump(e: PointerEvent<HTMLElement>, apply: Handler): void {
  apply(e.nativeEvent);
  track(e, apply);
}
