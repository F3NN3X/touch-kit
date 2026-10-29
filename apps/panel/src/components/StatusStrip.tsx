import { useEffect, useRef, useState } from "react";
import { useDebugVisible } from "../touch/actions";
import { onDeviceMessage } from "../touch/bridge";
import { onLatency, type LatencySummary, type Percentiles } from "../touch/latency";
import { useLastError } from "../touch/errors";
import { useSettings, type ConfigIssue } from "../touch/settings";
import { isStatus, type Gesture, type Status } from "../touch/types";
import { C, FONT_MONO } from "../kit/tokens";

const DOT: Record<Status["status"] | "Waiting", string> = {
  Waiting: C.text5,
  Connected: C.accent,
  Disconnected: C.red,
  Error: C.amber,
};

function describe(g: Gesture): string {
  switch (g.type) {
    case "Swipe":
      return `Swipe ${g.dir} ×${g.fingers}`;
    case "Pinch":
      return `Pinch ${g.scale.toFixed(2)}`;
    case "DragMove":
      return `Drag ${Math.round(g.dx)},${Math.round(g.dy)}`;
    case "LongPressRepeat":
      return `LongPress repeat ${g.n}`;
    default:
      return g.type;
  }
}

function statusText(s: Status | null): string {
  if (!s) return "touch waiting";
  switch (s.status) {
    case "Connected":
      return `touch connected · ${s.report_len} B reports`;
    case "Disconnected":
      return "touch disconnected · waiting for the USB device";
    case "Error":
      return `touch error · ${s.message}`;
  }
}

/** The one problem worth the strip's space: a webview error, else the first config error, else warning. */
function problem(webview: string | null, issues: ConfigIssue[]): { text: string; color: string } | null {
  if (webview) return { text: webview, color: C.red };
  const first = issues.find((i) => i.level === "error") ?? issues[0];
  if (!first) return null;
  const more = issues.length > 1 ? ` (+${issues.length - 1} more in the log)` : "";
  return first.level === "error"
    ? { text: `config not applied · ${first.message}${more}`, color: C.red }
    : { text: `config · ${first.message}${more}`, color: C.amber };
}

const pair = (p: Percentiles) => `${p.p50.toFixed(1)}/${p.p95.toFixed(1)}`;

/** Bottom band of the stage: connection state always, our latency and threshold numbers in debug. */
export function StatusStrip() {
  const settings = useSettings();
  const webviewError = useLastError();
  const debug = useDebugVisible();
  const [status, setStatus] = useState<Status | null>(null);
  const [contacts, setContacts] = useState(0);
  const [last, setLast] = useState("—");
  const [latency, setLatency] = useState<LatencySummary | null>(null);
  const live = useRef(new Set<number>());

  useEffect(() => onLatency(setLatency), []);

  useEffect(
    () =>
      onDeviceMessage((m) => {
        if (isStatus(m)) {
          setStatus(m);
          if (m.status !== "Connected") {
            live.current.clear();
            setContacts(0);
          }
          return;
        }
        if (m.type === "Gesture") {
          setLast(describe(m.gesture));
          return;
        }
        if (m.type !== "Pointer" || m.kind === "Move") return;
        if (m.kind === "Down") live.current.add(m.id);
        else live.current.delete(m.id);
        setContacts(live.current.size);
      }),
    [],
  );

  const g = settings.gestures;
  const issue = problem(webviewError, settings.issues);
  const touchColor = status?.status === "Disconnected" ? C.red : status?.status === "Error" ? C.amber : undefined;
  const stats = latency
    ? `ipc ${pair(latency.ipc)} · to-frame ${pair(latency.frame)} · frame ${pair(latency.frameInterval)} ms`
    : "ipc – · to-frame – · frame – ms";

  return (
    <div
      style={{
        position: "absolute",
        left: 24,
        right: 24,
        bottom: 0,
        height: 24,
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: 16,
        fontFamily: FONT_MONO,
        fontSize: 10.5,
        color: C.text4,
        pointerEvents: "none",
        whiteSpace: "nowrap",
      }}
    >
      <span style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0, flex: "none", maxWidth: "60%" }}>
        <span style={{ width: 6, height: 6, borderRadius: 3, flex: "none", background: DOT[status?.status ?? "Waiting"] }} />
        <span style={{ flex: "none", color: touchColor }}>{statusText(status)}</span>
        {issue && (
          <span style={{ color: issue.color, overflow: "hidden", textOverflow: "ellipsis" }}>· {issue.text}</span>
        )}
      </span>
      {debug && (
        /* Shrinks on narrow screens (ellipsis) so it never runs into the touch status. */
        <span style={{ color: C.text5, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis" }}>
          contacts {contacts} · last {last} · {stats} · predict {settings.prediction_ms} ms · tap {g.slop_px}px · hold{" "}
          {g.long_press_ms}ms · dbl {g.double_tap_gap_ms}ms
        </span>
      )}
    </div>
  );
}
