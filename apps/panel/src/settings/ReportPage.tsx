// Report page: exports everything about the connected touch displays as one JSON file
// (src-tauri/src/report.rs), for supporting them in other apps.

import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { Button, Row, Section, Toggle } from "./controls";

const RECORD_SECONDS = 15;

export function ReportPage() {
  const [record, setRecord] = useState(true);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<{ ok: boolean; text: string } | null>(null);

  const create = () => {
    setBusy(true);
    setResult(null);
    invoke<string>("report_create", { recordSeconds: record ? RECORD_SECONDS : 0 })
      .then((path) => setResult({ ok: true, text: path }))
      .catch((e) => setResult({ ok: false, text: String(e) }))
      .finally(() => setBusy(false));
  };

  return (
    <>
      <Section title="Display report">
        <p className="py-3 text-sm text-zinc-300">
          One file with what's needed to support your touch displays in other apps: Windows version, monitors and
          scaling, each touchscreen's USB and HID details (report layout, ranges, physical size, contact count), and
          this app's settings. No serial numbers, user or computer names.
        </p>
        <Row
          label="Include a touch recording"
          hint={`${RECORD_SECONDS} s of real touches on every touch display (a prompt shows on each): the most useful part for checking how a display reports.`}
        >
          <Toggle on={record} onChange={setRecord} />
        </Row>
        <Row
          label="Create report"
          hint={
            busy
              ? record
                ? "Recording: touch every touch display as its prompt says…"
                : "Reading the devices…"
              : "The panels pause while the devices are read, then come back by themselves."
          }
        >
          <Button onClick={create} disabled={busy}>
            Create report…
          </Button>
        </Row>
      </Section>
      {result && (
        <Section title={result.ok ? "Saved" : "Failed"}>
          <p className={`py-3 text-sm break-all ${result.ok ? "text-zinc-200" : "text-red-300"}`}>{result.text}</p>
          {result.ok && (
            <Row label="Share it" hint="Opens a GitHub issue form; drag the file into it.">
              <Button onClick={() => invoke("open_report_issue").catch(() => {})}>Open issue form</Button>
            </Row>
          )}
        </Section>
      )}
    </>
  );
}
