import { useEffect, useRef, useState } from "react";
import { drawPositions, onDeviceMessage } from "../touch/bridge";
import { useDebugVisible } from "../touch/actions";
import { recordFrameInterval } from "../touch/latency";
import { useSettings } from "../touch/settings";
import { isStatus } from "../touch/types";

const DOT = 48;

/**
 * SPEC §8: a ring follows each live contact, above everything. React renders only when contacts
 * come and go; positions are written straight to the DOM once per animation frame.
 */
export function TouchDots() {
  const settings = useSettings();
  const debug = useDebugVisible();
  const visible = settings.touch_dots;
  const running = visible || debug;
  const [ids, setIds] = useState<number[]>([]);
  const live = useRef(new Set<number>());
  const dots = useRef(new Map<number, HTMLDivElement>());

  useEffect(
    () =>
      onDeviceMessage((m) => {
        if (isStatus(m)) {
          if (m.status !== "Connected") {
            live.current.clear();
            setIds([]);
          }
          return;
        }
        if (m.type !== "Pointer" || m.kind === "Move") return;
        if (m.kind === "Down") live.current.add(m.id);
        else live.current.delete(m.id);
        setIds([...live.current]);
      }),
    [],
  );

  // The frame-interval stats in the status strip are recorded here, so this loop runs whenever a
  // contact is live and the strip or the dots are shown.
  useEffect(() => {
    if (!running || ids.length === 0) return;
    let raf = 0;
    let prev = 0;
    const frame = (ts: number) => {
      if (prev) recordFrameInterval(ts - prev);
      prev = ts;
      for (const [id, p] of drawPositions(performance.timeOrigin + ts)) {
        const el = dots.current.get(id);
        if (el) el.style.transform = `translate3d(${p.x - DOT / 2}px, ${p.y - DOT / 2}px, 0)`;
      }
      raf = requestAnimationFrame(frame);
    };
    raf = requestAnimationFrame(frame);
    return () => cancelAnimationFrame(raf);
  }, [running, ids]);

  if (!visible) return null;
  return (
    <div style={{ position: "absolute", inset: 0, pointerEvents: "none", zIndex: 50 }}>
      {ids.map((id) => (
        <div
          key={id}
          ref={(el) => {
            if (el) dots.current.set(id, el);
            else dots.current.delete(id);
          }}
          style={{
            position: "absolute",
            left: 0,
            top: 0,
            width: DOT,
            height: DOT,
            borderRadius: "50%",
            border: "2px solid rgba(255,255,255,.75)",
            background: "rgba(255,255,255,.12)",
            boxSizing: "border-box",
            transform: "translate3d(-100px, -100px, 0)",
            willChange: "transform",
          }}
        />
      ))}
    </div>
  );
}
