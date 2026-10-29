import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useEffect, useState } from "react";
import { C } from "../kit/tokens";

const SHOW_MS = 2500;

/** Settings' "Identify": this panel's name, big, over everything for a moment. */
export function Identify() {
  const [name, setName] = useState<string | null>(null);
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    // This window only: the global `listen` also hears events sent to the other panels.
    const un = getCurrentWebviewWindow().listen<string>("identify", (e) => {
      setName(e.payload);
      clearTimeout(timer);
      timer = setTimeout(() => setName(null), SHOW_MS);
    });
    return () => {
      clearTimeout(timer);
      un.then((f) => f()).catch(() => {});
    };
  }, []);
  if (name === null) return null;
  return (
    <div
      style={{
        position: "absolute",
        inset: 0,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "rgba(5,5,6,.82)",
        boxShadow: `inset 0 0 0 6px ${C.accent}`,
        color: C.text,
        fontSize: 96,
        fontWeight: 600,
        pointerEvents: "none",
        zIndex: 1000,
      }}
    >
      {name}
    </div>
  );
}
