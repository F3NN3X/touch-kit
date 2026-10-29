import { C, FONT_MONO } from "../kit/tokens";

/** The pairing window (`#pair/<name>`): shown on the monitor being paired until a finger lands. */
export function PairScreen({ name }: { name: string }) {
  return (
    <div
      style={{
        position: "absolute",
        inset: 0,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: 18,
        background: C.bg,
        boxShadow: `inset 0 0 0 6px ${C.accent}`,
        color: C.text,
      }}
    >
      <div style={{ position: "relative", width: 120, height: 120 }}>
        <div
          style={{
            position: "absolute",
            inset: 0,
            borderRadius: "50%",
            border: `3px solid ${C.accent}`,
            animation: "rip 1.6s ease-out infinite",
          }}
        />
        <div style={{ position: "absolute", inset: 36, borderRadius: "50%", background: C.accent }} />
      </div>
      <div style={{ fontSize: 56, fontWeight: 600 }}>Touch this screen</div>
      <div style={{ fontFamily: FONT_MONO, fontSize: 16, color: C.text3 }}>to pair it with “{name}”</div>
    </div>
  );
}
