import { useEffect, useRef, useState, type PointerEvent } from "react";
import { getSettings } from "../touch/settings";
import { track } from "./drag";
import { Icon } from "./Icon";
import { toast } from "./toast";
import { C, ring } from "./tokens";

interface Face {
  icon: string;
  label: string;
  /** Toast shown when a hold switches *to* this face. */
  toast: string;
}

interface HoldToggleProps {
  /** `true` shows the alert face (e.g. muted). */
  on: boolean;
  onChange: (on: boolean) => void;
  off: Face;
  onFace: Face;
}

/**
 * A tile that reacts on touch (pressed look within a frame) and switches only after a full long
 * press. Progress fills bottom-up over `long_press_ms` in the colour of the state it will switch to;
 * an early release drains it and changes nothing (Touch Kit spec §2, §3).
 */
export function HoldToggle({ on, onChange, off, onFace }: HoldToggleProps) {
  const [pressed, setPressed] = useState(false);
  const [holding, setHolding] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const holdMs = getSettings().gestures.long_press_ms;

  useEffect(() => () => clearTimeout(timer.current), []);

  const onPointerDown = (e: PointerEvent<HTMLElement>) => {
    setPressed(true);
    setHolding(true);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      setHolding(false);
      onChange(!on);
      toast((on ? off : onFace).toast);
    }, holdMs);
    track(e, undefined, () => {
      clearTimeout(timer.current);
      setPressed(false);
      setHolding(false);
    });
  };

  const face = on ? onFace : off;
  const fill = on ? "rgba(52,211,153,.28)" : "rgba(248,113,113,.28)";
  return (
    <div
      onPointerDown={onPointerDown}
      style={{
        position: "relative",
        height: 80,
        borderRadius: 14,
        overflow: "hidden",
        touchAction: "none",
        // Pressed wins over the alert face too (spec §2).
        background: pressed ? C.pressed : on ? "#2a1215" : C.control,
        boxShadow: pressed ? ring(C.pressedRing) : on ? ring(C.red, 1.5) : ring(C.controlRing),
        color: on && !pressed ? C.red : C.text,
        transform: pressed ? "scale(.94)" : "none",
        transition: "transform .12s, background .12s",
      }}
    >
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          bottom: 0,
          top: 0,
          background: fill,
          // Fills bottom-up as a transform, not a height change: no layout per frame.
          transformOrigin: "bottom",
          transform: holding ? "scaleY(1)" : "scaleY(0)",
          transition: holding ? `transform ${holdMs}ms linear` : "transform .2s ease-out",
        }}
      />
      <div
        style={{
          position: "relative",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          gap: 5,
        }}
      >
        <Icon name={face.icon} size={26} />
        <span style={{ fontSize: 12, fontWeight: on ? 600 : 500 }}>{face.label}</span>
      </div>
    </div>
  );
}
