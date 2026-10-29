// Per-control gesture recogniser (SPEC §2): tap / double tap / long press / swipe, with the
// thresholds read live from the settings. Touch-core's own recogniser (Rust) is separate.

import { useEffect, useRef, useState, type PointerEvent } from "react";
import { getSettings } from "../touch/settings";
import { getScale, track } from "./drag";
import { swipeDir, type SwipeDir } from "./swipe";

export type { SwipeDir };

export interface GestureHandlers {
  /** Pointer went down; `x`/`y` are design px inside the element. */
  onPress?: (at: { x: number; y: number }) => void;
  onTap?: () => void;
  /** When set, a tap waits `double_tap_gap_ms` for a second one before firing. */
  onDoubleTap?: () => void;
  onLongPress?: () => void;
  onSwipe?: (dir: SwipeDir) => void;
}

export function useGesture(handlers: GestureHandlers) {
  const h = useRef(handlers);
  h.current = handlers;
  const [holding, setHolding] = useState(false);
  const longTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const tapTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const lastTap = useRef({ x: 0, y: 0 });

  useEffect(
    () => () => {
      clearTimeout(longTimer.current);
      clearTimeout(tapTimer.current);
    },
    [],
  );

  const onPointerDown = (e: PointerEvent<HTMLElement>) => {
    const s = getSettings().gestures;
    const scale = getScale();
    const r = e.currentTarget.getBoundingClientRect();
    const startX = e.clientX;
    const startY = e.clientY;
    const t0 = performance.now();
    let dx = 0;
    let dy = 0;
    let moved = false;
    let longFired = false;

    setHolding(true);
    h.current.onPress?.({ x: (startX - r.left) / scale, y: (startY - r.top) / scale });
    clearTimeout(longTimer.current);
    longTimer.current = setTimeout(() => {
      longFired = true;
      clearTimeout(tapTimer.current);
      tapTimer.current = undefined;
      setHolding(false);
      h.current.onLongPress?.();
    }, s.long_press_ms);

    track(
      e,
      (ev) => {
        dx = (ev.clientX - startX) / scale;
        dy = (ev.clientY - startY) / scale;
        if (!moved && Math.hypot(dx, dy) > s.slop_px) {
          moved = true;
          clearTimeout(longTimer.current);
          setHolding(false);
        }
      },
      () => {
        clearTimeout(longTimer.current);
        setHolding(false);
        if (longFired) return;
        if (moved) {
          const dir = swipeDir(dx, dy, performance.now() - t0, s);
          if (dir) h.current.onSwipe?.(dir);
          return;
        }
        if (!h.current.onDoubleTap) {
          h.current.onTap?.();
          return;
        }
        const pending = tapTimer.current !== undefined;
        const near = Math.hypot(startX - lastTap.current.x, startY - lastTap.current.y) / scale <= s.double_tap_slop_px;
        clearTimeout(tapTimer.current);
        tapTimer.current = undefined;
        if (pending && near) {
          h.current.onDoubleTap();
          return;
        }
        if (pending) h.current.onTap?.();
        lastTap.current = { x: startX, y: startY };
        tapTimer.current = setTimeout(() => {
          tapTimer.current = undefined;
          h.current.onTap?.();
        }, s.double_tap_gap_ms);
      },
    );
  };

  return { onPointerDown, holding };
}
