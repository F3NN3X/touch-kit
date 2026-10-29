import { useState } from "react";
import { Icon } from "../kit/Icon";
import { Panel } from "../kit/Panel";
import { C, FONT_MONO, caption, ring } from "../kit/tokens";
import { useGesture } from "../kit/useGesture";
import { useSettings } from "../touch/settings";

const ICON: Record<string, string> = {
  Tap: "touch_app",
  "Double tap": "ads_click",
  "Long press": "front_hand",
  "Swipe left": "swipe_left",
  "Swipe right": "swipe_right",
  "Swipe up": "swipe_up",
  "Swipe down": "swipe_down",
};

const RING_LEN = 364.4;
const HISTORY = 4;

interface Ripple {
  id: number;
  x: number;
  y: number;
}

let rippleId = 0;

export function GesturePanel() {
  const longMs = useSettings().gestures.long_press_ms;
  const [gesture, setGesture] = useState<string | null>(null);
  const [count, setCount] = useState(0);
  const [log, setLog] = useState<string[]>([]);
  const [ripples, setRipples] = useState<Ripple[]>([]);

  const fire = (name: string) => {
    setGesture(name);
    setCount((n) => n + 1);
    setLog((l) => [name, ...l].slice(0, HISTORY));
  };
  const { onPointerDown, holding } = useGesture({
    onPress: ({ x, y }) => {
      const id = ++rippleId;
      setRipples((r) => [...r, { id, x, y }]);
      setTimeout(() => setRipples((r) => r.filter((q) => q.id !== id)), 700);
    },
    onTap: () => fire("Tap"),
    onDoubleTap: () => fire("Double tap"),
    onLongPress: () => fire("Long press"),
    onSwipe: (dir) => fire(`Swipe ${dir}`),
  });

  return (
    <Panel index="02" title="Gesture pad" badgeIcon="gesture" badge="all">
      <div
        onPointerDown={onPointerDown}
        style={{
          position: "relative",
          flex: 1,
          minHeight: 0,
          borderRadius: 14,
          background: `radial-gradient(circle, ${C.panelRing} 1px, transparent 1.4px) 0 0 / 20px 20px, ${C.well}`,
          boxShadow: ring(C.panelRing),
          overflow: "hidden",
          touchAction: "none",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          gap: 14,
        }}
      >
        <div style={{ position: "absolute", inset: 0, pointerEvents: "none" }}>
          {ripples.map((q) => (
            <span
              key={q.id}
              style={{
                position: "absolute",
                left: q.x - 60,
                top: q.y - 60,
                width: 120,
                height: 120,
                borderRadius: "50%",
                background: "rgba(52,211,153,.22)",
                animation: "rip .65s ease-out forwards",
              }}
            />
          ))}
        </div>
        <div
          style={{
            position: "relative",
            width: 132,
            height: 132,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            pointerEvents: "none",
          }}
        >
          <svg width="132" height="132" viewBox="0 0 132 132" style={{ position: "absolute", inset: 0 }}>
            <circle cx="66" cy="66" r="58" style={{ fill: C.panel, stroke: C.panelRing, strokeWidth: 3 }} />
            <circle
              cx="66"
              cy="66"
              r="58"
              transform="rotate(-90 66 66)"
              style={{
                fill: "none",
                stroke: C.accent,
                strokeWidth: 4,
                strokeLinecap: "round",
                strokeDasharray: RING_LEN,
                strokeDashoffset: holding ? 0 : RING_LEN,
                transition: holding ? `stroke-dashoffset ${longMs}ms linear` : "stroke-dashoffset .15s",
              }}
            />
          </svg>
          <Icon
            key={count}
            name={gesture ? ICON[gesture] : "touch_app"}
            size={46}
            color={gesture ? C.accent : C.text5}
            style={{ position: "relative", animation: count ? "pop .35s cubic-bezier(.2,.9,.3,1.4)" : "none" }}
          />
        </div>
        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 4, pointerEvents: "none" }}>
          <span style={{ fontSize: 22, fontWeight: 600, letterSpacing: "-.01em" }}>{gesture ?? "Try it"}</span>
          <span style={{ fontFamily: FONT_MONO, fontSize: 11, color: C.text4 }}>tap · double tap · hold · swipe</span>
        </div>
      </div>
      <div style={{ display: "flex", gap: 6, height: 32, overflow: "hidden" }}>
        {log.length === 0 && (
          <span style={{ display: "flex", alignItems: "center", ...caption, fontSize: 11, color: C.text5 }}>
            History shows here
          </span>
        )}
        {log.map((name, i) => (
          <span
            key={`${i}-${name}`}
            style={{
              flex: "none",
              display: "flex",
              alignItems: "center",
              gap: 5,
              padding: "0 10px",
              borderRadius: 10,
              background: C.control,
              fontSize: 12,
              color: C.text2,
              opacity: 1 - i * 0.2,
            }}
          >
            <Icon name={ICON[name]} size={16} />
            {name}
          </span>
        ))}
      </div>
    </Panel>
  );
}
