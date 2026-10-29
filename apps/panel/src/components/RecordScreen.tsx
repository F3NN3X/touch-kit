import { useEffect, useState } from "react";
import { C, FONT_MONO } from "../kit/tokens";

/** The report's recording prompt (`#record/<seconds>`), on each touch display while it records. */
export function RecordScreen({ seconds }: { seconds: number }) {
  const [left, setLeft] = useState(seconds);
  useEffect(() => {
    const t = setInterval(() => setLeft((s) => Math.max(0, s - 1)), 1000);
    return () => clearInterval(t);
  }, []);

  return (
    <div
      style={{
        position: "absolute",
        inset: 0,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: 14,
        background: C.bg,
        boxShadow: `inset 0 0 0 6px ${C.accent}`,
        color: C.text,
        textAlign: "center",
        padding: 24,
      }}
    >
      <div style={{ fontFamily: FONT_MONO, fontSize: 64, fontWeight: 600, color: C.accent }}>{left}</div>
      <div style={{ fontSize: 30, fontWeight: 600 }}>Touch this screen</div>
      <div style={{ fontSize: 16, color: C.text3, maxWidth: 900 }}>
        Tap each corner, swipe in every direction, pinch, hold a finger down — then put down as many fingers as fit.
      </div>
    </div>
  );
}
