import { useSyncExternalStore } from "react";
import { Icon } from "./Icon";
import { C } from "./tokens";

const TOAST_MS = 1800;

interface ToastState {
  id: number;
  text: string;
}

let current: ToastState | null = null;
let seq = 0;
let timer: ReturnType<typeof setTimeout> | undefined;
const listeners = new Set<() => void>();

const emit = () => listeners.forEach((l) => l());

/** One toast at a time: a new one replaces the old and restarts the timer. */
export function toast(text: string): void {
  clearTimeout(timer);
  current = { id: ++seq, text };
  emit();
  timer = setTimeout(() => {
    current = null;
    emit();
  }, TOAST_MS);
}

function subscribe(l: () => void): () => void {
  listeners.add(l);
  return () => listeners.delete(l);
}

export function ToastHost() {
  const t = useSyncExternalStore(subscribe, () => current);
  if (!t) return null;
  return (
    <div
      key={t.id}
      style={{
        position: "absolute",
        left: "50%",
        top: 14,
        transform: "translateX(-50%)",
        display: "flex",
        alignItems: "center",
        gap: 8,
        height: 36,
        padding: "0 16px",
        borderRadius: 999,
        background: C.text,
        color: C.bg,
        fontSize: 13,
        fontWeight: 500,
        boxShadow: "0 8px 24px rgba(0,0,0,.5)",
        pointerEvents: "none",
        zIndex: 40,
      }}
    >
      <Icon name="check_circle" size={18} />
      {t.text}
    </div>
  );
}
