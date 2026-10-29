// Driver page: live driver state per touch device, guided Zadig install, automated uninstall.

import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { Button, Section } from "./controls";

interface DriverDevice {
  vid: number;
  pid: number;
  port: string;
  product: string | null;
  driver: string | null;
}

interface DriverView {
  ids: [number, number][];
  devices: DriverDevice[];
  windows: boolean;
}

interface UninstallResult {
  changed: boolean;
  log: string;
}

const hex = (n: number) => n.toString(16).toUpperCase().padStart(4, "0");
const shortPort = (p: string) => p.slice(p.lastIndexOf("-") + 1);
const isWinUsb = (d: DriverDevice) => d.driver?.toLowerCase() === "winusb";

/** Re-read every 2 s, so the state flips the moment Zadig (or the uninstall) finishes. */
function useDrivers(): { view: DriverView | null; error: string | null } {
  const [view, setView] = useState<DriverView | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const tick = () =>
      invoke<DriverView>("driver_status")
        .then((v) => {
          setView(v);
          setError(null);
        })
        .catch((e) => setError(String(e)));
    tick();
    const t = setInterval(tick, 2000);
    return () => clearInterval(t);
  }, []);
  return { view, error };
}

export function DriverPage() {
  const { view, error } = useDrivers();
  if (error) return <p className="text-sm text-red-300">Couldn't list USB devices: {error}</p>;
  if (!view) return <p className="text-sm text-zinc-500">Looking for devices…</p>;
  if (!view.windows) {
    return (
      <Section title="Linux">
        <p className="py-3 text-sm text-zinc-300">
          No driver change is needed on Linux: the app detaches the kernel driver while it runs. It needs access to the
          device (root, or a udev rule for it).
        </p>
      </Section>
    );
  }
  if (view.ids.length === 0) {
    return (
      <Section title="Touchscreens">
        <p className="py-3 text-sm text-zinc-400">
          No touchscreen found. Plug in the display's touch USB cable; it shows up here by itself, with the steps to
          set it up.
        </p>
      </Section>
    );
  }
  return (
    <>
      {view.ids.map(([vid, pid]) => {
        const devices = view.devices.filter((d) => d.vid === vid && d.pid === pid);
        const id = `${hex(vid)}:${hex(pid)}`;
        const needsSetup = devices.some((d) => !isWinUsb(d));
        return (
          <div key={id}>
            <Section title={`Touchscreen ${id}`}>
              {devices.length === 0 && (
                <p className="py-3 text-sm text-zinc-400">
                  None connected. Plug in its touch USB cable; it shows up here by itself.
                </p>
              )}
              {devices.map((d) => (
                <DeviceRow key={d.port} device={d} />
              ))}
            </Section>
            {needsSetup && <ZadigSteps vid={vid} pid={pid} product={devices.find((d) => !isWinUsb(d))?.product ?? null} />}
          </div>
        );
      })}
    </>
  );
}

function DeviceRow({ device: d }: { device: DriverDevice }) {
  const ready = isWinUsb(d);
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<{ ok: boolean; text: string; log?: string } | null>(null);

  const uninstall = () => {
    setConfirm(false);
    setBusy(true);
    setResult(null);
    invoke<UninstallResult>("driver_uninstall", { vid: d.vid, pid: d.pid })
      .then((r) =>
        setResult({
          ok: true,
          text: r.changed ? "Given back to Windows. Its touch works there again." : "Nothing was bound to WinUSB.",
          log: r.log,
        }),
      )
      .catch((e) => setResult({ ok: false, text: String(e) }))
      .finally(() => setBusy(false));
  };

  return (
    <div className="border-b border-white/5 py-3 last:border-b-0">
      <div className="flex items-center gap-4">
        <div className="min-w-0 flex-1">
          <div className="text-sm text-zinc-100">
            {d.product ?? "Unknown device"} <span className="text-zinc-500">· USB port {shortPort(d.port)}</span>
          </div>
          <div className={`mt-0.5 text-xs ${ready ? "text-emerald-300" : "text-amber-300"}`}>
            {ready
              ? "WinUSB · ready: this app reads it, Windows doesn't"
              : `${d.driver ?? "no driver"} · Windows owns it: needs the WinUSB driver (steps below)`}
          </div>
        </div>
        {ready &&
          (busy ? (
            <span className="text-xs text-zinc-400">Waiting for the Windows admin prompt…</span>
          ) : confirm ? (
            <>
              <Button onClick={() => setConfirm(false)}>Keep</Button>
              <Button tone="danger" onClick={uninstall}>
                Give back to Windows
              </Button>
            </>
          ) : (
            <Button tone="danger" onClick={() => setConfirm(true)}>
              Uninstall…
            </Button>
          ))}
      </div>
      {confirm && (
        <p className="mt-3 rounded-md bg-red-500/10 px-3 py-2 text-xs text-red-100 ring-1 ring-red-400/30">
          Windows takes this panel back: touches move the cursor and click on that screen again, and this app stops
          receiving them. Windows asks for admin permission once. The certificate Zadig added for {hex(d.vid)}:
          {hex(d.pid)} is removed too. You can install it again with the steps below.
        </p>
      )}
      {result && (
        <div className={`mt-3 text-xs ${result.ok ? "text-emerald-200" : "text-red-200"}`}>
          <div className="whitespace-pre-wrap">{result.text}</div>
          {result.log && (
            <details className="mt-1 text-zinc-400">
              <summary className="cursor-pointer">Log</summary>
              <pre className="mt-1 max-h-48 overflow-auto rounded bg-black/40 p-2 font-mono text-[11px]">{result.log}</pre>
            </details>
          )}
        </div>
      )}
    </div>
  );
}

interface ZadigStepsProps {
  vid: number;
  pid: number;
  product: string | null;
}

function ZadigSteps({ vid, pid, product }: ZadigStepsProps) {
  return (
    <Section title="Install the WinUSB driver with Zadig">
      <ol className="list-decimal space-y-3 py-4 pl-5 text-sm text-zinc-300 marker:text-zinc-500">
        <li>
          Download Zadig from its official site.{" "}
          <Button onClick={() => invoke("open_zadig").catch(() => {})}>Open zadig.akeo.ie</Button>
        </li>
        <li>
          Run it. Choose <b className="text-zinc-100">Options → List All Devices</b>, and untick{" "}
          <b className="text-zinc-100">Ignore Hubs or Composite Parents</b>.
        </li>
        <li>
          In the drop-down pick <b className="text-zinc-100">{product ?? "the touch panel"}</b>. Then check the{" "}
          <b className="text-zinc-100">USB ID</b> fields read exactly:
          <div className="my-2 inline-flex gap-2 font-mono text-2xl text-zinc-100">
            <span className="rounded bg-zinc-800 px-3 py-1">{hex(vid)}</span>
            <span className="rounded bg-zinc-800 px-3 py-1">{hex(pid)}</span>
          </div>
          <div className="text-xs text-amber-200">
            Picking your keyboard or mouse here instead is the classic mistake. If the IDs differ, pick again.
          </div>
        </li>
        <li>
          Right of the arrow, set the target driver to <b className="text-zinc-100">WinUSB</b> and click{" "}
          <b className="text-zinc-100">Replace Driver</b> (or Install Driver).
        </li>
        <li>
          Come back here: the device above turns green by itself and the panel starts taking touch. Windows touch on that
          screen stops; that's the point.
        </li>
      </ol>
    </Section>
  );
}
