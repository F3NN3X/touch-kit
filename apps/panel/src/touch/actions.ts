// Runs the config's gesture → action map on gestures recognised by touch-core (Rust).
// The UI kit's own per-control gestures (useGesture) are separate and don't go through here.

import { invoke } from "@tauri-apps/api/core";
import { useSyncExternalStore } from "react";
import { onDeviceMessage } from "./bridge";
import { getSettings, type Action } from "./settings";
import type { Gesture } from "./types";

/** Config key for a gesture: `Tap`, `LongPress`, `Swipe Left x2`, `ThreeFingerTap`, ... */
export function gestureKey(g: Gesture): string {
  return g.type === "Swipe" ? `Swipe ${g.dir} x${g.fingers}` : g.type;
}

let debugVisible: boolean | null = null;
const listeners = new Set<() => void>();

function setDebug(v: boolean) {
  debugVisible = v;
  listeners.forEach((l) => l());
}

/** Debug strip visibility: starts from the config, toggled by `toggle_debug`. */
export function useDebugVisible(): boolean {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    () => debugVisible ?? getSettings().debug,
  );
}

function run(key: string, a: Action) {
  switch (a.action) {
    case "toggle_debug":
      setDebug(!(debugVisible ?? getSettings().debug));
      break;
    case "page":
      window.dispatchEvent(new CustomEvent("kit:page", { detail: { to: a.to } }));
      break;
    case "emit":
      window.dispatchEvent(new CustomEvent(`kit:${a.name}`, { detail: { gesture: key } }));
      invoke("log_action", { text: `${key} → emit ${a.name}` }).catch(() => {});
      break;
    case "log":
      invoke("log_action", { text: key }).catch(() => {});
      break;
  }
}

let started = false;

export function startActions(): void {
  if (started) return;
  started = true;
  onDeviceMessage((m) => {
    if ("status" in m || m.type !== "Gesture") return;
    const key = gestureKey(m.gesture);
    const a = getSettings().actions[key];
    if (a) run(key, a);
  });
}
