// The settings window's view of the Rust config commands (src-tauri/src/settings.rs).

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import type { ConfigIssue, UiConfig } from "../touch/settings";
import type { Status } from "../touch/types";

export interface MonitorMatch {
  name?: string | null;
  position?: [number, number] | null;
}

export interface PanelEntry {
  name: string;
  device: { vid: number; pid: number; port: string | null; iface: number | null };
  monitor: MonitorMatch | null;
  panel: {
    width_px: number;
    height_px: number;
    rotation: "0" | "90" | "180" | "270";
    flip_x: boolean;
    flip_y: boolean;
    calibration: unknown | null;
  };
}

export interface AppConfig {
  panels: PanelEntry[];
  gestures: Record<string, number>;
  dev_mode: boolean;
  prediction_ms: number;
  touch_dots: boolean;
  debug: boolean;
  testing: { enabled: boolean; virtual_panels: number; replay: string };
}

/** JSON as written in the file. */
export type Json = null | boolean | number | string | Json[] | { [k: string]: Json };

export interface ConfigView {
  file: { [k: string]: Json } | null;
  file_error: string | null;
  effective: AppConfig;
  defaults: AppConfig;
}

export interface PanelStatus {
  index: number;
  name: string;
  is_virtual: boolean;
  monitor: string | null;
  touch: Status | null;
}

export interface Monitor {
  name: string | null;
  position: [number, number];
  size: [number, number];
}

/** Writes one key (`null` = back to default). Rejects with the reasons if the file wouldn't apply. */
export function setKey(key: string, value: Json): Promise<void> {
  return invoke("config_set", { key, value });
}

/** The value at a dotted path in the file, or undefined if the file doesn't set it. */
export function fileValue(file: ConfigView["file"], key: string): Json | undefined {
  let node: Json | undefined = file;
  for (const part of key.split(".")) {
    if (node === null || typeof node !== "object") return undefined;
    node = Array.isArray(node) ? node[Number(part)] : node[part];
  }
  return node;
}

/** The config, re-read whenever the Rust side reloads it; plus the issues that came with it. */
export function useConfig(): { view: ConfigView | null; issues: ConfigIssue[]; refresh: () => void } {
  const [view, setView] = useState<ConfigView | null>(null);
  const [issues, setIssues] = useState<ConfigIssue[]>([]);
  const refresh = useCallback(() => {
    invoke<ConfigView>("config_get").then(setView).catch(() => {});
    invoke<UiConfig>("ui_config").then((c) => setIssues(c.issues)).catch(() => {});
  }, []);
  useEffect(() => {
    refresh();
    const un = listen<UiConfig>("ui-config", (e) => {
      setIssues(e.payload.issues);
      invoke<ConfigView>("config_get").then(setView).catch(() => {});
    });
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  }, [refresh]);
  return { view, issues, refresh };
}

/** Polls the running panels' status (cheap; settings is open only occasionally). */
export function usePanelStatus(): PanelStatus[] {
  const [status, setStatus] = useState<PanelStatus[]>([]);
  useEffect(() => {
    const tick = () => invoke<PanelStatus[]>("panels_status").then(setStatus).catch(() => {});
    tick();
    const t = setInterval(tick, 1000);
    return () => clearInterval(t);
  }, []);
  return status;
}

export function useMonitors(): Monitor[] {
  const [ms, setMs] = useState<Monitor[]>([]);
  useEffect(() => {
    invoke<Monitor[]>("monitors").then(setMs).catch(() => {});
  }, []);
  return ms;
}
