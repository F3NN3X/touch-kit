import { useState } from "react";
import { useConfig } from "./api";
import { SaveContext, useSaveWithErrors } from "./controls";
import { DriverPage } from "./DriverPage";
import { GeneralPage } from "./GeneralPage";
import { GesturesPage } from "./GesturesPage";
import { PanelsPage } from "./PanelsPage";
import { TestingPage } from "./TestingPage";

const PAGES = [
  { id: "panels", label: "Panels", Page: PanelsPage },
  { id: "general", label: "General", Page: GeneralPage },
  { id: "gestures", label: "Gestures", Page: GesturesPage },
  { id: "driver", label: "Driver", Page: DriverPage },
  { id: "testing", label: "Testing", Page: TestingPage },
] as const;

type PageId = (typeof PAGES)[number]["id"];

/** `#settings/<page>` opens on that page (the app uses `#settings/driver`). */
const initialPage = (): PageId => PAGES.find((p) => p.id === window.location.hash.split("/")[1])?.id ?? "panels";

/** The settings window: a normal window on the main monitor, opened from the tray. */
export function Settings() {
  const [page, setPage] = useState(initialPage);
  const { view, issues } = useConfig();
  const { save, errors, clear } = useSaveWithErrors();
  const { Page } = PAGES.find((p) => p.id === page)!;

  return (
    <SaveContext.Provider value={save}>
      <div className="flex h-full bg-zinc-950 text-zinc-100">
        <nav className="w-48 shrink-0 border-r border-white/5 p-4">
          <div className="mb-6 px-2 text-sm font-semibold">Touch Kit</div>
          {PAGES.map((p) => (
            <button
              key={p.id}
              type="button"
              onClick={() => setPage(p.id)}
              className={`mb-1 block w-full rounded-md px-2 py-1.5 text-left text-sm ${
                page === p.id ? "bg-white/10 text-zinc-100" : "text-zinc-400 hover:bg-white/5 hover:text-zinc-200"
              }`}
            >
              {p.label}
            </button>
          ))}
        </nav>
        <main className="min-w-0 flex-1 overflow-y-auto px-10 py-8">
          <h1 className="mb-6 text-xl font-semibold">{PAGES.find((p) => p.id === page)!.label}</h1>
          {errors.length > 0 && (
            <div className="mb-6 rounded-lg bg-red-500/10 px-4 py-3 text-sm text-red-200 ring-1 ring-red-400/30">
              <div className="flex items-start justify-between gap-4">
                <div>
                  <div className="font-medium">Not saved: the change would make the config invalid.</div>
                  <ul className="mt-1 list-disc pl-5">
                    {errors.map((e) => (
                      <li key={e}>{e}</li>
                    ))}
                  </ul>
                </div>
                <button type="button" onClick={clear} className="text-red-200/70 hover:text-red-100">
                  Dismiss
                </button>
              </div>
            </div>
          )}
          {issues.length > 0 && (
            <div className="mb-6 rounded-lg bg-amber-500/10 px-4 py-3 text-sm text-amber-100 ring-1 ring-amber-400/30">
              <div className="font-medium">The config file has problems</div>
              <ul className="mt-1 list-disc pl-5">
                {issues.map((i) => (
                  <li key={i.message}>
                    {i.level === "error" ? "Not applied: " : ""}
                    {i.message}
                  </li>
                ))}
              </ul>
            </div>
          )}
          {view?.file_error && (
            <p className="mb-6 text-sm text-red-300">
              The config file can't be read ({view.file_error}). Fix or delete it; settings can't edit it until then.
            </p>
          )}
          {view ? <Page view={view} /> : <p className="text-sm text-zinc-500">Loading…</p>}
        </main>
      </div>
    </SaveContext.Provider>
  );
}
