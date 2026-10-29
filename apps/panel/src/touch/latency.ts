// Per-move latency, in ms, split where we can observe it:
//   ipc   = webview arrival − Rust send (both wall clock; same machine)
//   frame = start of the animation frame that applies the move − arrival
// Display scan-out after that frame is outside our view, so `total` is a lower bound.

import { invoke } from "@tauri-apps/api/core";

export interface Percentiles {
  p50: number;
  p95: number;
}

export interface LatencySummary {
  n: number;
  ipc: Percentiles;
  frame: Percentiles;
  total: Percentiles;
  /** Time between drawn frames while a contact is live. */
  frameInterval: Percentiles;
}

const REPORT_EVERY_MS = 2000;

let ipc: number[] = [];
let frame: number[] = [];
let intervals: number[] = [];
const listeners = new Set<(s: LatencySummary) => void>();

export const wallNow = (): number => performance.timeOrigin + performance.now();

export function recordFrameInterval(ms: number): void {
  intervals.push(ms);
}

export function record(ipcMs: number, frameMs: number): void {
  ipc.push(ipcMs);
  frame.push(Math.max(0, frameMs));
}

function pct(values: number[]): Percentiles {
  const s = [...values].sort((a, b) => a - b);
  const at = (q: number) => s[Math.min(s.length - 1, Math.floor(q * s.length))] ?? 0;
  return { p50: at(0.5), p95: at(0.95) };
}

export function summarise(ipcMs: number[], frameMs: number[], intervalMs: number[]): LatencySummary {
  return {
    n: ipcMs.length,
    ipc: pct(ipcMs),
    frame: pct(frameMs),
    total: pct(ipcMs.map((v, i) => v + frameMs[i])),
    frameInterval: pct(intervalMs),
  };
}

export function onLatency(l: (s: LatencySummary) => void): () => void {
  listeners.add(l);
  return () => listeners.delete(l);
}

setInterval(() => {
  if (ipc.length === 0) return;
  const s = summarise(ipc, frame, intervals);
  ipc = [];
  frame = [];
  intervals = [];
  listeners.forEach((l) => l(s));
  const f = (p: Percentiles) => `${p.p50.toFixed(1)}/${p.p95.toFixed(1)}`;
  invoke("latency_report", {
    report: `n=${s.n} p50/p95 ms: ipc ${f(s.ipc)} · to-frame ${f(s.frame)} · total ${f(s.total)} · frame interval ${f(s.frameInterval)}`,
  }).catch(() => {});
}, REPORT_EVERY_MS);
