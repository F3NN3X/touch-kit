import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { Button } from "./controls";

type SetupResult = { result: "done"; bound: { port: string; monitor: string }[] } | { result: "failed"; message: string };

function resultText(r: SetupResult): string {
  if (r.result === "failed") return `Setup failed: ${r.message}`;
  if (r.bound.length === 0) return "Setup ended without a tap; nothing changed.";
  return `Set up ${r.bound.map((b) => b.monitor).join(", ")}. Calibrate a panel if touches land beside where you tap.`;
}

/** "Set up panels…": one tap per touch display binds its touchscreen and monitor (src-tauri/src/setup.rs). */
export function SetupBar() {
  const [running, setRunning] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  useEffect(() => {
    const un = listen<SetupResult>("setup-result", (e) => {
      setRunning(false);
      setMessage(resultText(e.payload));
    });
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  }, []);

  const start = () => {
    setMessage(null);
    setRunning(true);
    invoke<number>("setup_start")
      .then((n) =>
        setMessage(
          `A circle shows on ${n === 1 ? "your other screen" : `each of your ${n} other screens`}: tap it on every touch display, then Done.`,
        ),
      )
      .catch((e) => {
        setRunning(false);
        setMessage(String(e));
      });
  };

  return (
    <section className="mb-8 flex items-center gap-4 rounded-xl bg-zinc-900/60 px-4 py-3 ring-1 ring-white/5">
      <div className="min-w-0 flex-1">
        <div className="text-sm text-zinc-100">Set up panels</div>
        <div className="mt-0.5 text-xs text-zinc-400">
          {message ?? "Finds every touch display: tap the circle on each, and its touchscreen, monitor and size are set."}
        </div>
      </div>
      {running ? (
        <Button onClick={() => invoke("setup_finish").catch(() => {})}>Done</Button>
      ) : (
        <Button onClick={start}>Set up panels…</Button>
      )}
    </section>
  );
}
