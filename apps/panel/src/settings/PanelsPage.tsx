import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { Status } from "../touch/types";
import { fileValue, useMonitors, usePanelStatus, type ConfigView, type Json, type PanelEntry } from "./api";
import { Button, NumberField, Row, Select, TextField, Toggle, useSave } from "./controls";
import { SetupBar } from "./SetupBar";

const ROTATIONS = ["0", "90", "180", "270"].map((r) => ({ value: r, label: `${r}°` }));

export function touchText(s: Status | null | undefined): { text: string; tone: string } {
  if (!s) return { text: "waiting", tone: "text-zinc-400" };
  switch (s.status) {
    case "Connected":
      return { text: "touch connected", tone: "text-emerald-300" };
    case "Disconnected":
      return { text: "touch device not found", tone: "text-red-300" };
    case "Error":
      return { text: `touch error · ${s.message}`, tone: "text-amber-300" };
  }
}

type PairResult =
  | { result: "paired"; index: number; port: string }
  | { result: "cancelled"; index: number }
  | { result: "timed_out"; index: number }
  | { result: "failed"; index: number; message: string };

function pairText(r: PairResult): string {
  switch (r.result) {
    case "paired":
      return `Paired with the touch device at USB port ${shortPort(r.port)}.`;
    case "cancelled":
      return "Pairing cancelled.";
    case "timed_out":
      return "No touch within 30 s; pairing stopped.";
    case "failed":
      return `Pairing failed: ${r.message}`;
  }
}

/** Pairing state per panel: running, or the last result's message. */
function usePairing(): {
  pairing: number | null;
  message: { index: number; text: string } | null;
  start: (i: number) => void;
  cancel: () => void;
} {
  const [pairing, setPairing] = useState<number | null>(null);
  const [message, setMessage] = useState<{ index: number; text: string } | null>(null);
  useEffect(() => {
    const un = listen<PairResult>("pair-result", (e) => {
      setPairing(null);
      setMessage({ index: e.payload.index, text: pairText(e.payload) });
    });
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  }, []);
  const start = (i: number) => {
    setMessage(null);
    setPairing(i);
    invoke("pair_start", { index: i }).catch((e) => {
      setPairing(null);
      setMessage({ index: i, text: String(e) });
    });
  };
  return { pairing, message, start, cancel: () => invoke("pair_cancel").catch(() => {}) };
}

/** The tail of a USB location, which is what tells two ports apart (`…USBROOT(0)-4.2` → `4.2`). */
const shortPort = (p: string) => p.slice(p.lastIndexOf("-") + 1);

export function PanelsPage({ view }: { view: ConfigView }) {
  const save = useSave();
  const status = usePanelStatus();
  const monitors = useMonitors();
  const [confirmRemove, setConfirmRemove] = useState<number | null>(null);
  const pair = usePairing();
  const [calMessage, setCalMessage] = useState<{ index: number; text: string } | null>(null);
  const calibrate = (i: number) => {
    setCalMessage(null);
    invoke("calibrate_start", { index: i })
      .then(() => setCalMessage({ index: i, text: "Tap the four targets on the panel." }))
      .catch((e) => setCalMessage({ index: i, text: String(e) }));
  };
  useEffect(() => {
    const un = listen<number>("calibrate-end", (e) => setCalMessage((m) => (m?.index === e.payload ? null : m)));
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  }, []);
  const panels = view.effective.panels;
  const filePanels = Array.isArray(view.file?.panels) ? (view.file.panels as Json[]) : null;
  // Without a `panels` key the config is one default panel; writing `[{}]` first means the same.
  const base = (): Json[] => filePanels ?? panels.map(() => ({}));

  const setPanel = async (i: number, key: string, value: Json) => {
    if (!filePanels && !(await save("panels", base()))) return;
    await save(`panels.${i}.${key}`, value);
  };
  const isSet = (i: number, key: string) => fileValue(view.file, `panels.${i}.${key}`) !== undefined;

  const add = () => save("panels", [...base(), { name: `Panel ${panels.length + 1}` }]);
  const remove = (i: number) => {
    setConfirmRemove(null);
    save("panels", base().filter((_, j) => j !== i));
  };

  const monitorOptions = [
    { value: "", label: "Auto (by size)" },
    ...monitors.map((m) => ({
      value: m.name ?? "",
      label: `${m.name ?? "?"} · ${m.size[0]}×${m.size[1]}`,
    })),
  ];

  return (
    <>
      <SetupBar />
      {panels.map((p: PanelEntry, i) => {
        const st = status.find((s) => s.index === i);
        const touch = st ? touchText(st.touch) : { text: "not running · see the issues above", tone: "text-amber-300" };
        const port = st?.touch?.status === "Connected" ? st.touch.port : null;
        return (
          <section key={i} className="mb-8 rounded-xl bg-zinc-900/60 ring-1 ring-white/5">
            <header className="flex items-center gap-3 border-b border-white/5 px-4 py-3">
              <div className="min-w-0 flex-1">
                <div className="text-base font-medium text-zinc-100">{p.name}</div>
                <div className="mt-0.5 text-xs">
                  <span className={touch.tone}>{touch.text}</span>
                  {st?.monitor && <span className="text-zinc-500"> · on {st.monitor}</span>}
                  {port && <span className="text-zinc-500"> · USB port {shortPort(port)}</span>}
                </div>
              </div>
              <Button onClick={() => invoke("identify", { index: i })} disabled={!st}>
                Identify
              </Button>
              {confirmRemove === i ? (
                <Button tone="danger" onClick={() => remove(i)}>
                  Click again to remove
                </Button>
              ) : (
                <Button tone="danger" onClick={() => setConfirmRemove(i)}>
                  Remove
                </Button>
              )}
            </header>
            <div className="px-4">
              <Row
                label="Pair"
                hint={
                  pair.pairing === i
                    ? "Touch the screen showing “Touch this screen”…"
                    : pair.message?.index === i
                      ? pair.message.text
                      : "Find this panel's touch device by touching its screen. Pins the USB port."
                }
              >
                {pair.pairing === i ? (
                  <Button onClick={pair.cancel}>Cancel</Button>
                ) : (
                  <Button onClick={() => pair.start(i)} disabled={!st || pair.pairing !== null}>
                    Pair…
                  </Button>
                )}
              </Row>
              <Row
                label="Calibration"
                hint={
                  calMessage?.index === i
                    ? calMessage.text
                    : p.panel.calibration
                      ? "Calibrated. Recalibrate if touches land beside where you tap."
                      : "Not calibrated: touches map by size. Calibrate if they land beside where you tap."
                }
                isSet={isSet(i, "panel.calibration")}
                onReset={() => setPanel(i, "panel.calibration", null)}
              >
                <Button onClick={() => calibrate(i)} disabled={!st || st.touch?.status !== "Connected"}>
                  Calibrate…
                </Button>
              </Row>
              <Row label="Name" isSet={isSet(i, "name")} onReset={() => setPanel(i, "name", null)}>
                <TextField value={p.name} onCommit={(v) => setPanel(i, "name", v)} />
              </Row>
              <Row
                label="Monitor"
                hint="Auto takes the first free monitor of the panel's size. Changing it moves the window."
                isSet={isSet(i, "monitor")}
                onReset={() => setPanel(i, "monitor", null)}
              >
                <Select
                  value={p.monitor?.name ?? ""}
                  options={monitorOptions}
                  onChange={(v) => setPanel(i, "monitor", v ? { name: v } : null)}
                />
              </Row>
              <Row
                label="USB port"
                hint={
                  p.device.port
                    ? `Pinned to ${shortPort(p.device.port)}. Unpin to take the first free touchscreen.`
                    : "Not pinned: takes the first free touchscreen. Set up panels and Pair pin it."
                }
                isSet={isSet(i, "device.port")}
                onReset={() => setPanel(i, "device.port", null)}
              >
                {p.device.port ? (
                  <Button onClick={() => setPanel(i, "device.port", null)}>Unpin</Button>
                ) : (
                  port && <Button onClick={() => setPanel(i, "device.port", port)}>Pin to {shortPort(port)}</Button>
                )}
              </Row>
              <Row label="Rotation" hint="Clockwise, applied to touches." isSet={isSet(i, "panel.rotation")} onReset={() => setPanel(i, "panel.rotation", null)}>
                <Select value={p.panel.rotation} options={ROTATIONS} onChange={(v) => setPanel(i, "panel.rotation", v)} />
              </Row>
              <Row label="Mirror horizontally" isSet={isSet(i, "panel.flip_x")} onReset={() => setPanel(i, "panel.flip_x", null)}>
                <Toggle on={p.panel.flip_x} onChange={(v) => setPanel(i, "panel.flip_x", v)} />
              </Row>
              <Row label="Mirror vertically" isSet={isSet(i, "panel.flip_y")} onReset={() => setPanel(i, "panel.flip_y", null)}>
                <Toggle on={p.panel.flip_y} onChange={(v) => setPanel(i, "panel.flip_y", v)} />
              </Row>
              <Row label="Width" hint="Panel pixels touches map to." isSet={isSet(i, "panel.width_px")} onReset={() => setPanel(i, "panel.width_px", null)}>
                <NumberField value={p.panel.width_px} min={1} unit="px" onCommit={(v) => setPanel(i, "panel.width_px", v)} />
              </Row>
              <Row label="Height" isSet={isSet(i, "panel.height_px")} onReset={() => setPanel(i, "panel.height_px", null)}>
                <NumberField value={p.panel.height_px} min={1} unit="px" onCommit={(v) => setPanel(i, "panel.height_px", v)} />
              </Row>
            </div>
          </section>
        );
      })}
      <Button onClick={add}>Add panel</Button>
    </>
  );
}
