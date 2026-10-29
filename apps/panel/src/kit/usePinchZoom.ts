// Two-finger pinch + one-finger pan + wheel zoom (SPEC §2 Pinch, §7). Each finger's events reach
// the surface by bubbling, so the handlers see every pointer stream.

import { useRef, useState, type PointerEvent, type RefObject, type WheelEvent } from "react";
import { getSettings } from "../touch/settings";
import { getScale } from "./drag";
import { clamp } from "./tokens";

const MIN_ZOOM = 1;
const MAX_ZOOM = 4;

interface View {
  zoom: number;
  px: number;
  py: number;
}

interface Point {
  x: number;
  y: number;
}

const IDLE: View = { zoom: 1, px: 0, py: 0 };

export interface PinchZoom {
  zoom: number;
  transform: string;
  pinching: boolean;
  reset: () => void;
  /** The transformed content; its size bounds the pan. */
  contentRef: RefObject<HTMLDivElement | null>;
  surface: {
    onPointerDown: (e: PointerEvent<HTMLElement>) => void;
    onPointerMove: (e: PointerEvent<HTMLElement>) => void;
    onPointerUp: (e: PointerEvent<HTMLElement>) => void;
    onPointerCancel: (e: PointerEvent<HTMLElement>) => void;
    onWheel: (e: WheelEvent<HTMLElement>) => void;
  };
}

export function usePinchZoom(): PinchZoom {
  const [view, setView] = useState<View>(IDLE);
  const [pinching, setPinching] = useState(false);
  const viewRef = useRef(view);
  const contentRef = useRef<HTMLDivElement>(null);
  const pts = useRef(new Map<number, Point>());
  const pinch = useRef<{ d: number; z: number } | null>(null);
  const pan = useRef<{ x: number; y: number; px: number; py: number } | null>(null);
  const lastDown = useRef<{ t: number; x: number; y: number } | null>(null);

  const commit = (v: View) => {
    viewRef.current = v;
    setView(v);
  };
  const fit = (zoom: number, px: number, py: number): View => {
    const mx = ((zoom - 1) * (contentRef.current?.offsetWidth ?? 0)) / 2;
    const my = ((zoom - 1) * (contentRef.current?.offsetHeight ?? 0)) / 2;
    return { zoom, px: clamp(px, -mx, mx), py: clamp(py, -my, my) };
  };
  const dist = () => {
    const [a, b] = [...pts.current.values()];
    return Math.hypot(a.x - b.x, a.y - b.y);
  };

  const onPointerDown = (e: PointerEvent<HTMLElement>) => {
    try {
      e.currentTarget.setPointerCapture(e.pointerId);
    } catch {
      // synthetic pointer ids can't be captured; the bridge already routes them here
    }
    pts.current.set(e.pointerId, { x: e.clientX, y: e.clientY });
    if (pts.current.size === 2) {
      pinch.current = { d: dist() || 1, z: viewRef.current.zoom };
      pan.current = null;
      lastDown.current = null;
    } else if (pts.current.size === 1) {
      const v = viewRef.current;
      pan.current = { x: e.clientX, y: e.clientY, px: v.px, py: v.py };
      const g = getSettings().gestures;
      const now = performance.now();
      const prev = lastDown.current;
      if (prev && now - prev.t < g.double_tap_gap_ms && Math.hypot(e.clientX - prev.x, e.clientY - prev.y) / getScale() <= g.double_tap_slop_px) {
        commit(IDLE);
        lastDown.current = null;
      } else {
        lastDown.current = { t: now, x: e.clientX, y: e.clientY };
      }
    }
  };

  const onPointerMove = (e: PointerEvent<HTMLElement>) => {
    if (!pts.current.has(e.pointerId)) return;
    pts.current.set(e.pointerId, { x: e.clientX, y: e.clientY });
    const v = viewRef.current;
    if (pts.current.size === 2 && pinch.current) {
      const z = clamp((pinch.current.z * dist()) / pinch.current.d, MIN_ZOOM, MAX_ZOOM);
      setPinching(true);
      commit(fit(z, v.px, v.py));
    } else if (pts.current.size === 1 && pan.current && v.zoom > 1) {
      const scale = getScale();
      setPinching(true);
      commit(fit(v.zoom, pan.current.px + (e.clientX - pan.current.x) / scale, pan.current.py + (e.clientY - pan.current.y) / scale));
    }
  };

  const onPointerUp = (e: PointerEvent<HTMLElement>) => {
    pts.current.delete(e.pointerId);
    if (pts.current.size < 2) pinch.current = null;
    if (pts.current.size === 1) {
      const [p] = [...pts.current.values()];
      pan.current = { x: p.x, y: p.y, px: viewRef.current.px, py: viewRef.current.py };
    }
    if (pts.current.size === 0) {
      pan.current = null;
      setPinching(false);
    }
  };

  const onWheel = (e: WheelEvent<HTMLElement>) => {
    const v = viewRef.current;
    commit(fit(clamp(v.zoom * Math.exp(-e.deltaY * 0.003), MIN_ZOOM, MAX_ZOOM), v.px, v.py));
  };

  return {
    zoom: view.zoom,
    transform: `translate(${view.px.toFixed(1)}px, ${view.py.toFixed(1)}px) scale(${view.zoom.toFixed(3)})`,
    pinching,
    reset: () => commit(IDLE),
    contentRef,
    surface: { onPointerDown, onPointerMove, onPointerUp, onPointerCancel: onPointerUp, onWheel },
  };
}
