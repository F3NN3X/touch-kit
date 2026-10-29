// Webview errors: written to the app log (Rust side) and kept for the status strip, so a failure
// shows on the panel itself instead of only in devtools, which is off outside dev mode.

import { invoke } from "@tauri-apps/api/core";
import { useSyncExternalStore } from "react";

let last: string | null = null;
const listeners = new Set<() => void>();

function message(err: unknown): string {
  if (err instanceof Error) return err.message;
  return typeof err === "string" ? err : JSON.stringify(err);
}

/** Logs `err` and shows it in the status strip. `context` says what failed. */
export function reportError(context: string, err: unknown): void {
  last = `${context}: ${message(err)}`;
  listeners.forEach((l) => l());
  const stack = err instanceof Error && err.stack ? `\n${err.stack}` : "";
  // If IPC itself is what failed, this fails too; the strip still shows it.
  invoke("webview_error", { text: last + stack }).catch(() => {});
}

let started = false;

/** Reports uncaught errors and unhandled promise rejections. */
export function startErrorReporting(): void {
  if (started) return;
  started = true;
  window.addEventListener("error", (e) => reportError("uncaught", e.error ?? e.message));
  window.addEventListener("unhandledrejection", (e) => reportError("unhandled rejection", e.reason));
}

/** The most recent webview error, or null. */
export function useLastError(): string | null {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    () => last,
  );
}
