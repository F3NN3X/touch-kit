// Touch Kit design tokens (SPEC §9). Everything in kit/ and screens/ takes its colours from here.

import type { CSSProperties } from "react";

export const C = {
  bg: "#050506",
  panel: "#131316",
  panelRing: "#222228",
  control: "#1c1c21",
  controlRing: "#2a2a31",
  pressed: "#2a2a31",
  pressedRing: "#3a3a44",
  well: "#0c0c0e",
  track: "#26262c",
  ledOff: "#1f1f25",
  text: "#ededf0",
  text2: "#c9c9cf",
  text3: "#a1a1aa",
  text4: "#8b8b94",
  text5: "#5a5a63",
  accent: "#34d399",
  accentInk: "#04140d",
  amber: "#f59e0b",
  red: "#f87171",
} as const;

export const FONT_MONO = "'Geist Mono Variable', monospace";
export const EASE = "cubic-bezier(.2,.8,.2,1)";

/** Inset ring, the design's replacement for a border. */
export const ring = (color: string, width = 1): string => `inset 0 0 0 ${width}px ${color}`;

/** The small grey mono caption used under controls. */
export const caption: CSSProperties = { fontFamily: FONT_MONO, fontSize: 10.5, color: C.text4 };

export const clamp = (v: number, lo: number, hi: number): number => Math.max(lo, Math.min(hi, v));
