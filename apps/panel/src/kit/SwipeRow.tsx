import { useState, type ReactNode } from "react";
import { getSettings } from "../touch/settings";
import { getScale, track } from "./drag";
import { Icon } from "./Icon";
import { C, EASE, clamp } from "./tokens";
import { usePress } from "./usePress";

const OPEN_LEFT = -140;
const OPEN_RIGHT = 90;
const MIN_X = -160;
const MAX_X = 110;
const OPEN_LEFT_AT = -70;
const OPEN_RIGHT_AT = 55;

interface SwipeRowProps {
  /** Resting offset: 0 closed, OPEN_LEFT or OPEN_RIGHT open. Owned by the parent so only one row stays open. */
  rest: number;
  onRest: (rest: number) => void;
  /** Called when a drag starts, so the parent can close the other rows. */
  onGrab: () => void;
  /** Revealed by dragging right (90 px wide). */
  leftAction: ReactNode;
  /** Revealed by dragging left (140 px wide). */
  rightActions: ReactNode;
  children: ReactNode;
}

interface ActionProps {
  background: string;
  color: string;
  icon: string;
  label: string;
  /** Fills the width of the action area on its own (pin) rather than sharing it. */
  wide?: boolean;
  onClick: () => void;
}

export function RowAction({ background, color, icon, label, wide, onClick }: ActionProps) {
  const press = usePress();
  return (
    <div
      onPointerDown={press.onPointerDown}
      onClick={onClick}
      style={{
        flex: wide ? undefined : 1,
        width: wide ? 90 : undefined,
        background,
        color,
        filter: press.pressed ? "brightness(.85)" : "none",
        display: "flex",
        flexDirection: wide ? "row" : "column",
        alignItems: "center",
        justifyContent: "center",
        gap: wide ? 5 : 2,
        fontSize: wide ? 12 : 11,
        fontWeight: 600,
      }}
    >
      <Icon name={icon} size={18} />
      {label}
    </div>
  );
}

/** Foreground follows the finger (clamped), then snaps open or closed. */
export function SwipeRow({ rest, onRest, onGrab, leftAction, rightActions, children }: SwipeRowProps) {
  const [drag, setDrag] = useState<number | null>(null);
  const x = drag ?? rest;

  return (
    <div style={{ position: "relative", height: 52, borderRadius: 12, overflow: "hidden", background: C.well }}>
      <div
        style={{ position: "absolute", left: 0, top: 0, bottom: 0, width: 90, display: "flex", opacity: x > 0 ? 1 : 0 }}
      >
        {leftAction}
      </div>
      <div
        style={{ position: "absolute", right: 0, top: 0, bottom: 0, width: 140, display: "flex", opacity: x < 0 ? 1 : 0 }}
      >
        {rightActions}
      </div>
      <div
        onPointerDown={(e) => {
          const startX = e.clientX;
          const scale = getScale();
          const slop = getSettings().gestures.slop_px;
          const base = rest;
          let moved = false;
          let cur = base;
          onGrab();
          setDrag(base);
          track(
            e,
            (ev) => {
              const d = (ev.clientX - startX) / scale;
              if (Math.abs(d) > slop) moved = true;
              cur = clamp(base + d, MIN_X, MAX_X);
              setDrag(cur);
            },
            () => {
              onRest(!moved ? 0 : cur < OPEN_LEFT_AT ? OPEN_LEFT : cur > OPEN_RIGHT_AT ? OPEN_RIGHT : 0);
              setDrag(null);
            },
          );
        }}
        style={{
          position: "absolute",
          inset: 0,
          background: C.control,
          transform: `translateX(${x}px)`,
          transition: drag === null ? `transform .28s ${EASE}` : "none",
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "0 14px",
          touchAction: "pan-y",
          minWidth: 0,
        }}
      >
        {children}
      </div>
    </div>
  );
}
