import { Icon } from "../kit/Icon";
import { Panel } from "../kit/Panel";
import { PinchSurface } from "../kit/PinchSurface";
import { C, FONT_MONO, caption, ring } from "../kit/tokens";
import { usePinchZoom } from "../kit/usePinchZoom";
import { usePress } from "../kit/usePress";

const BARS = Array.from({ length: 48 }, (_, i) =>
  Math.round(Math.min(96, 34 + 20 * Math.sin(i / 5) + 12 * Math.sin(i / 1.7 + 1) + i * 0.8)),
);
const GRID_COLORS = [C.control, C.control, C.control, C.track];

export function PinchPanel() {
  const pinch = usePinchZoom();
  const press = usePress();

  return (
    <Panel index="05" title="Pinch" badgeIcon="pinch" badge="zoom · pan">
      <PinchSurface
        pinch={pinch}
        overlay={
          <div style={{ position: "absolute", top: 12, left: 14, display: "flex", flexDirection: "column", gap: 2, pointerEvents: "none" }}>
            <span style={{ fontSize: 13, fontWeight: 500 }}>Viewers</span>
            <span style={caption}>last 90 min</span>
          </div>
        }
      >
        <div style={{ position: "absolute", inset: 0, display: "flex", flexDirection: "column", justifyContent: "space-between", pointerEvents: "none" }}>
          {GRID_COLORS.map((c, i) => (
            <div key={i} style={{ height: 1, background: c }} />
          ))}
        </div>
        <div style={{ position: "absolute", inset: 0, display: "flex", alignItems: "flex-end", gap: 2 }}>
          {BARS.map((h, i) => (
            <div key={i} style={{ flex: 1, height: `${h}%`, borderRadius: "2px 2px 0 0", background: "rgba(52,211,153,.6)" }} />
          ))}
        </div>
        <div
          style={{
            position: "absolute",
            left: 0,
            right: 0,
            bottom: -20,
            display: "flex",
            justifyContent: "space-between",
            fontFamily: FONT_MONO,
            fontSize: 10,
            color: C.text5,
          }}
        >
          <span>-90m</span>
          <span>-60m</span>
          <span>-30m</span>
          <span>now</span>
        </div>
      </PinchSurface>
      <div style={{ display: "flex", alignItems: "center", gap: 10, height: 44 }}>
        <span style={{ fontFamily: FONT_MONO, fontSize: 13, color: C.text, width: 40 }}>{pinch.zoom.toFixed(1)}×</span>
        <div style={{ flex: 1, height: 6, borderRadius: 3, background: C.track, overflow: "hidden" }}>
          <div
            style={{
              height: "100%",
              background: C.accent,
              // Scaled, not resized: no layout per frame (the track clips the ends).
              transformOrigin: "left",
              transform: `scaleX(${(pinch.zoom - 1) / 3})`,
              transition: "transform .15s",
            }}
          />
        </div>
        <div
          onPointerDown={press.onPointerDown}
          onClick={pinch.reset}
          style={{
            width: 44,
            height: 44,
            borderRadius: 12,
            background: C.control,
            boxShadow: ring(C.controlRing),
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            transform: press.pressed ? "scale(.92)" : "none",
            transition: "transform .12s",
          }}
        >
          <Icon name="fit_screen" size={20} />
        </div>
      </div>
    </Panel>
  );
}
