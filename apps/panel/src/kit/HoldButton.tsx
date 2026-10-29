import { useEffect, useRef, useState, type PointerEvent } from "react";
import { getSettings } from "../touch/settings";
import { track } from "./drag";
import { Icon } from "./Icon";
import { toast } from "./toast";
import { C, ring } from "./tokens";

const SAVED_MS = 1600;

interface HoldButtonProps {
  icon: string;
  label: string;
  doneLabel: string;
  doneToast: string;
}

/** Fill grows over `long_press_ms` while held; an early release drains it and never fires. */
export function HoldButton({ icon, label, doneLabel, doneToast }: HoldButtonProps) {
  const [holding, setHolding] = useState(false);
  const [done, setDone] = useState(false);
  const holdTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const doneTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(
    () => () => {
      clearTimeout(holdTimer.current);
      clearTimeout(doneTimer.current);
    },
    [],
  );

  const onPointerDown = (e: PointerEvent<HTMLElement>) => {
    setHolding(true);
    setDone(false);
    clearTimeout(holdTimer.current);
    holdTimer.current = setTimeout(() => {
      setHolding(false);
      setDone(true);
      toast(doneToast);
      clearTimeout(doneTimer.current);
      doneTimer.current = setTimeout(() => setDone(false), SAVED_MS);
    }, getSettings().gestures.long_press_ms);
    track(e, undefined, () => {
      clearTimeout(holdTimer.current);
      setHolding(false);
    });
  };

  return (
    <div
      onPointerDown={onPointerDown}
      style={{
        position: "relative",
        height: 64,
        borderRadius: 14,
        background: C.control,
        boxShadow: ring(C.controlRing),
        overflow: "hidden",
        touchAction: "none",
      }}
    >
      <div
        style={{
          position: "absolute",
          left: 0,
          top: 0,
          bottom: 0,
          width: "100%",
          background: "rgba(52,211,153,.28)",
          // Grows left to right (spec §3) as a transform, not a width change: no layout per frame.
          transformOrigin: "left",
          transform: holding ? "scaleX(1)" : "scaleX(0)",
          transition: holding ? `transform ${getSettings().gestures.long_press_ms}ms linear` : "transform .2s ease-out",
        }}
      />
      <div
        style={{
          position: "relative",
          height: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          gap: 7,
          fontSize: 13,
          fontWeight: 500,
        }}
      >
        <Icon name={done ? "check" : icon} size={20} color={C.accent} />
        {done ? doneLabel : label}
      </div>
    </div>
  );
}
