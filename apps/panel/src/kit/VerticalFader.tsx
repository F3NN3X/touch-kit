import { trackJump } from "./drag";
import { C, caption, clamp, ring } from "./tokens";

interface VerticalFaderProps {
  name: string;
  /** Channel colour of the fill. */
  color: string;
  /** 0–100 */
  value: number;
  onChange: (value: number) => void;
}

export function VerticalFader({ name, color, value, onChange }: VerticalFaderProps) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6, alignItems: "center", width: 52 }}>
      <div
        onPointerDown={(e) => {
          const r = e.currentTarget.getBoundingClientRect();
          trackJump(e, (ev) => onChange(Math.round(clamp(((r.bottom - ev.clientY) / r.height) * 100, 0, 100))));
        }}
        style={{
          position: "relative",
          flex: 1,
          width: 52,
          borderRadius: 14,
          background: C.well,
          boxShadow: ring(C.panelRing),
          overflow: "hidden",
          touchAction: "none",
        }}
      >
        <div style={{ position: "absolute", left: 0, right: 0, bottom: 0, height: `${value}%`, background: color }} />
        <div
          style={{
            position: "absolute",
            left: 12,
            right: 12,
            bottom: `calc(${value}% - 2px)`,
            height: 4,
            borderRadius: 2,
            background: C.text,
          }}
        />
      </div>
      <span style={{ fontSize: 12, fontWeight: 500 }}>{name}</span>
      <span style={{ ...caption, marginTop: -4 }}>{value}</span>
    </div>
  );
}
