import { trackJump } from "./drag";
import { C, EASE, FONT_MONO, clamp } from "./tokens";

interface SteppedSliderProps {
  stops: string[];
  /** Index of the current stop. */
  value: number;
  onChange: (index: number) => void;
}

/** Snaps to the nearest stop while dragging, not only on release. */
export function SteppedSlider({ stops, value, onChange }: SteppedSliderProps) {
  const last = stops.length - 1;
  const pct = (i: number) => (i / last) * 100;
  return (
    <>
      <div
        onPointerDown={(e) => {
          const r = e.currentTarget.getBoundingClientRect();
          trackJump(e, (ev) => onChange(Math.round(clamp((ev.clientX - r.left) / r.width, 0, 1) * last)));
        }}
        style={{ position: "relative", height: 52, margin: "-6px 16px", touchAction: "none" }}
      >
        <div style={{ position: "absolute", left: 0, right: 0, top: 23, height: 6, borderRadius: 3, background: C.track }} />
        <div
          style={{
            position: "absolute",
            left: 0,
            top: 23,
            height: 6,
            borderRadius: 3,
            width: "100%",
            background: C.accent,
            // A clip, not a width change: no layout per frame, and the rounded ends stay round.
            clipPath: `inset(0 ${100 - pct(value)}% 0 0 round 3px)`,
            transition: "clip-path .15s",
          }}
        />
        {stops.map((s, i) => (
          <div
            key={s}
            style={{
              position: "absolute",
              top: 22,
              left: `${pct(i)}%`,
              width: 8,
              height: 8,
              marginLeft: -4,
              borderRadius: 4,
              background: i <= value ? C.accent : C.pressedRing,
            }}
          />
        ))}
        <div
          style={{
            position: "absolute",
            top: 10,
            left: `${pct(value)}%`,
            width: 32,
            height: 32,
            marginLeft: -16,
            borderRadius: 16,
            background: C.text,
            boxShadow: "0 2px 8px rgba(0,0,0,.5)",
            transition: `left .15s ${EASE}`,
          }}
        />
      </div>
      <div style={{ display: "flex", justifyContent: "space-between", fontFamily: FONT_MONO, fontSize: 10.5 }}>
        {stops.map((s, i) => (
          <span key={s} style={{ width: 32, textAlign: "center", color: i === value ? C.text : C.text4 }}>
            {s}
          </span>
        ))}
      </div>
    </>
  );
}
