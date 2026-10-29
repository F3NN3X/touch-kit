import { getScale, trackJump } from "./drag";
import { C, clamp, ring } from "./tokens";

const SEGMENTS = 24;
const HOT_FROM = 19;
const PAD = 9;

interface LedFaderProps {
  /** 0–100 */
  value: number;
  onChange: (value: number) => void;
}

export function LedFader({ value, onChange }: LedFaderProps) {
  const lit = Math.round((value / 100) * SEGMENTS);
  return (
    <div
      onPointerDown={(e) => {
        const r = e.currentTarget.getBoundingClientRect();
        const pad = PAD * getScale();
        trackJump(e, (ev) => onChange(Math.round(clamp(((ev.clientX - r.left - pad) / (r.width - 2 * pad)) * 100, 0, 100))));
      }}
      style={{
        height: 52,
        borderRadius: 14,
        background: C.well,
        boxShadow: ring(C.panelRing),
        padding: PAD,
        boxSizing: "border-box",
        display: "grid",
        gridTemplateColumns: `repeat(${SEGMENTS}, minmax(0,1fr))`,
        gap: 3,
        touchAction: "none",
      }}
    >
      {Array.from({ length: SEGMENTS }, (_, i) => (
        <div key={i} style={{ borderRadius: 3, background: i < lit ? (i < HOT_FROM ? C.accent : C.amber) : C.ledOff }} />
      ))}
    </div>
  );
}
