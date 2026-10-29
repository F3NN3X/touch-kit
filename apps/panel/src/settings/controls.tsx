// Mouse-first form controls for the settings window. Every control writes through `save`, which
// reports a refused write (the Rust side validates first) in the window's banner.

import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { setKey, type Json } from "./api";

type Save = (key: string, value: Json) => Promise<boolean>;

export const SaveContext = createContext<Save>(async () => false);

/** `save(key, value)`: true if written; on refusal the reasons show in the banner. */
export const useSave = (): Save => useContext(SaveContext);

export function useSaveWithErrors(): { save: Save; errors: string[]; clear: () => void } {
  const [errors, setErrors] = useState<string[]>([]);
  const save: Save = async (key, value) => {
    try {
      await setKey(key, value);
      setErrors([]);
      return true;
    } catch (e) {
      setErrors(Array.isArray(e) ? e.map(String) : [String(e)]);
      return false;
    }
  };
  return { save, errors, clear: () => setErrors([]) };
}

interface RowProps {
  label: string;
  hint?: string;
  /** The file sets this key: show a reset to the default. */
  isSet?: boolean;
  onReset?: () => void;
  children: ReactNode;
}

export function Row({ label, hint, isSet, onReset, children }: RowProps) {
  return (
    <div className="flex items-center gap-6 border-b border-white/5 py-3 last:border-b-0">
      <div className="min-w-0 flex-1">
        <div className="text-sm text-zinc-100">{label}</div>
        {hint && <div className="mt-0.5 text-xs text-zinc-500">{hint}</div>}
      </div>
      <div className="flex shrink-0 items-center gap-2">
        {children}
        <button
          type="button"
          onClick={onReset}
          disabled={!isSet}
          title="Back to default"
          className="w-14 text-xs text-zinc-500 hover:text-zinc-200 disabled:invisible"
        >
          reset
        </button>
      </div>
    </div>
  );
}

interface ToggleProps {
  on: boolean;
  onChange: (on: boolean) => void;
}

export function Toggle({ on, onChange }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      onClick={() => onChange(!on)}
      className={`relative h-6 w-11 rounded-full transition-colors ${on ? "bg-emerald-400" : "bg-zinc-700"}`}
    >
      <span
        className={`absolute top-0.5 left-0 h-5 w-5 rounded-full bg-white transition-transform ${on ? "translate-x-5" : "translate-x-0.5"}`}
      />
    </button>
  );
}

interface NumberFieldProps {
  value: number;
  onCommit: (v: number) => void;
  min?: number;
  max?: number;
  step?: number;
  unit?: string;
}

/** Commits on Enter or blur, not per keystroke: each commit is a file write and a reload. */
export function NumberField({ value, onCommit, min, max, step = 1, unit }: NumberFieldProps) {
  const [text, setText] = useState(String(value));
  useEffect(() => setText(String(value)), [value]);
  const commit = () => {
    const v = Number(text);
    if (text.trim() === "" || !Number.isFinite(v)) return setText(String(value));
    if (v !== value) onCommit(v);
  };
  return (
    <span className="flex items-center gap-1.5">
      <input
        type="number"
        value={text}
        min={min}
        max={max}
        step={step}
        onChange={(e) => setText(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
        className="w-24 rounded-md bg-zinc-900 px-2 py-1 text-right font-mono text-sm text-zinc-100 ring-1 ring-white/10 focus:ring-emerald-400 focus:outline-none"
      />
      {unit && <span className="w-8 text-xs text-zinc-500">{unit}</span>}
    </span>
  );
}

interface TextFieldProps {
  value: string;
  onCommit: (v: string) => void;
}

export function TextField({ value, onCommit }: TextFieldProps) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  return (
    <input
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={() => text.trim() && text !== value && onCommit(text.trim())}
      onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
      className="w-56 rounded-md bg-zinc-900 px-2 py-1 text-sm text-zinc-100 ring-1 ring-white/10 focus:ring-emerald-400 focus:outline-none"
    />
  );
}

interface SelectProps {
  value: string;
  options: { value: string; label: string }[];
  onChange: (v: string) => void;
}

export function Select({ value, options, onChange }: SelectProps) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="w-56 rounded-md bg-zinc-900 px-2 py-1 text-sm text-zinc-100 ring-1 ring-white/10 focus:ring-emerald-400 focus:outline-none"
    >
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

interface ButtonProps {
  onClick: () => void;
  children: ReactNode;
  tone?: "default" | "danger";
  disabled?: boolean;
}

export function Button({ onClick, children, tone = "default", disabled }: ButtonProps) {
  const color =
    tone === "danger" ? "text-red-300 ring-red-400/30 hover:bg-red-500/10" : "text-zinc-100 ring-white/10 hover:bg-white/5";
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={`rounded-md px-3 py-1.5 text-sm ring-1 disabled:opacity-40 ${color}`}
    >
      {children}
    </button>
  );
}

export function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="mb-8">
      <h2 className="mb-2 text-xs font-medium tracking-wide text-zinc-500 uppercase">{title}</h2>
      <div className="rounded-xl bg-zinc-900/60 px-4 ring-1 ring-white/5">{children}</div>
    </section>
  );
}
