import { getScale, track } from "./drag";
import { C, clamp, ring } from "./tokens";

const FULL_RANGE_PX = 160;

interface KnobProps {
  /** 0–1 */
  value: number;
  onChange: (value: number) => void;
}

/** Vertical drag, up = increase, 160 design px = full range. No jump on press. */
export function Knob({ value, onChange }: KnobProps) {
  return (
    <div
      onPointerDown={(e) => {
        const startY = e.clientY;
        const base = value;
        const scale = getScale();
        track(e, (ev) => onChange(clamp(base - (ev.clientY - startY) / scale / FULL_RANGE_PX, 0, 1)));
      }}
      style={{ position: "relative", width: 120, height: 120, touchAction: "none" }}
    >
      <svg width="120" height="120" viewBox="0 0 120 120" style={{ position: "absolute", inset: 0 }}>
        <circle
          cx="60"
          cy="60"
          r="52"
          transform="rotate(135 60 60)"
          style={{ fill: "none", stroke: C.track, strokeWidth: 7, strokeLinecap: "round", strokeDasharray: "245 400" }}
        />
        <circle
          cx="60"
          cy="60"
          r="52"
          transform="rotate(135 60 60)"
          style={{
            fill: "none",
            stroke: C.accent,
            strokeWidth: 7,
            strokeLinecap: "round",
            strokeDasharray: `${(245 * value).toFixed(1)} 400`,
          }}
        />
      </svg>
      <div
        style={{
          position: "absolute",
          left: 22,
          top: 22,
          width: 76,
          height: 76,
          borderRadius: 38,
          background: C.control,
          boxShadow: `${ring(C.controlRing)}, 0 4px 14px rgba(0,0,0,.5)`,
          transform: `rotate(${Math.round(-135 + value * 270)}deg)`,
        }}
      >
        <div style={{ position: "absolute", left: 36, top: 8, width: 4, height: 16, borderRadius: 2, background: C.text }} />
      </div>
    </div>
  );
}
