import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useEffect, useState } from "react";
import { C, FONT_MONO } from "../kit/tokens";

/**
 * One monitor's setup window (`#setup/<radius>`): a target `radius` (fraction of the screen) right of
 * the centre. Each monitor's target sits at a different distance, which is how the Rust side tells
 * which monitor a tap was on (src-tauri/src/setup.rs). Turns into a confirmation once bound.
 */
export function SetupScreen({ radius }: { radius: number }) {
  const [bound, setBound] = useState<string | null>(null);
  useEffect(() => {
    // This window only: the global `listen` also hears events sent to the other setup windows.
    const un = getCurrentWebviewWindow().listen<string>("setup-bound", (e) => setBound(e.payload));
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  }, []);

  return (
    <div style={{ position: "absolute", inset: 0, background: C.bg, color: C.text, overflow: "hidden" }}>
      <div
        style={{
          position: "absolute",
          left: `${(0.5 + radius) * 100}%`,
          top: "50%",
          width: 120,
          height: 120,
          margin: -60,
        }}
      >
        {bound === null && (
          <div
            style={{
              position: "absolute",
              inset: 0,
              borderRadius: "50%",
              border: `3px solid ${C.accent}`,
              animation: "rip 1.6s ease-out infinite",
            }}
          />
        )}
        <div style={{ position: "absolute", inset: 36, borderRadius: "50%", background: bound ? C.text4 : C.accent }} />
      </div>
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: 32,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 8,
        }}
      >
        <div style={{ fontSize: 40, fontWeight: 600 }}>{bound ? "Set up" : "Tap the circle"}</div>
        <div style={{ fontFamily: FONT_MONO, fontSize: 15, color: C.text3 }}>
          {bound ? `this screen's touch is ready (${bound})` : "on each screen whose touch you want to use"}
        </div>
      </div>
    </div>
  );
}
