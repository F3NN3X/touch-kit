// App settings from the Rust config (`<app config dir>/config.json`), hot-reloaded.
// `gestures` are the same thresholds touch-core's recogniser uses, so UI and Rust agree.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSyncExternalStore } from "react";
import { setClickSlop, setPredictionLead } from "./bridge";
import { reportError } from "./errors";

export interface GestureThresholds {
  slop_px: number;
  long_press_ms: number;
  long_press_repeat_ms: number;
  double_tap_gap_ms: number;
  double_tap_slop_px: number;
  swipe_min_px: number;
  flick_min_px: number;
  flick_min_speed: number;
}

export type Action =
  | { action: "toggle_debug" }
  | { action: "page"; to: "next" | "prev" }
  | { action: "emit"; name: string }
  | { action: "log" };

export interface UiConfig {
  prediction_ms: number;
  dev_mode: boolean;
  touch_dots: boolean;
  debug: boolean;
  gestures: GestureThresholds;
  actions: Record<string, Action>;
  /** Problems with the config file as last read. With an error, the values above are the previous config's. */
  issues: ConfigIssue[];
}

export interface ConfigIssue {
  level: "error" | "warning";
  message: string;
}

/** Spec defaults (Touch Kit SPEC §2), used until the Rust config arrives. */
export const defaultUiConfig: UiConfig = {
  prediction_ms: 16,
  dev_mode: false,
  touch_dots: false,
  debug: true,
  gestures: {
    slop_px: 12,
    long_press_ms: 500,
    long_press_repeat_ms: 150,
    double_tap_gap_ms: 280,
    double_tap_slop_px: 24,
    swipe_min_px: 60,
    flick_min_px: 10,
    flick_min_speed: 0.5,
  },
  actions: {},
  issues: [],
};

let current = defaultUiConfig;
const listeners = new Set<() => void>();

function set(next: UiConfig) {
  current = next;
  setPredictionLead(next.prediction_ms);
  setClickSlop(next.gestures.slop_px);
  listeners.forEach((l) => l());
}

function subscribe(l: () => void): () => void {
  listeners.add(l);
  return () => listeners.delete(l);
}

let started = false;

export function startSettings(): void {
  if (started) return;
  started = true;
  invoke<UiConfig>("ui_config")
    .then((c) => set(c))
    .catch((e) => reportError("loading config", e));
  listen<UiConfig>("ui-config", (e) => set(e.payload)).catch((e) => reportError("config reload", e));
}

export const getSettings = (): UiConfig => current;

export function useSettings(): UiConfig {
  return useSyncExternalStore(subscribe, getSettings);
}
