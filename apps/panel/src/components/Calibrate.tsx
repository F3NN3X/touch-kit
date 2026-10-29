// Calibration on the panel itself (started from settings). The Rust side maps this panel without
// its calibration meanwhile, so a finger's px convert back to the raw touch: u = x / (width − 1).

import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useEffect, useRef, useState } from "react";
import { onDeviceMessage } from "../touch/bridge";
import { reportError } from "../touch/errors";
import { C, FONT_MONO } from "../kit/tokens";

interface Start {
  index: number;
  name: string;
  width_px: number;
  height_px: number;
}

interface CalPoint {
  touch: [number, number];
  panel: [number, number];
}

/** Targets at 10 % insets, clockwise from top-left, as fractions of the panel. */
const TARGETS: [number, number][] = [
  [0.1, 0.1],
  [0.9, 0.1],
  [0.9, 0.9],
  [0.1, 0.9],
];

/** Below this RMS error (px) the fit is good; above BAD it is probably a missed target. */
const GOOD_PX = 6;
const BAD_PX = 20;

type Phase = { kind: "collect"; points: CalPoint[] } | { kind: "review"; points: CalPoint[]; error: number };

export function Calibrate() {
  const [start, setStart] = useState<Start | null>(null);
  const [phase, setPhase] = useState<Phase>({ kind: "collect", points: [] });
  const live = useRef<{ start: Start; phase: Phase } | null>(null);
  live.current = start && { start, phase };

  useEffect(() => {
    // This window only: the global `listen` also hears events sent to the other panels.
    const un = getCurrentWebviewWindow().listen<Start>("calibrate", (e) => {
      setStart(e.payload);
      setPhase({ kind: "collect", points: [] });
    });
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  }, []);

  useEffect(
    () =>
      onDeviceMessage((m) => {
        const cur = live.current;
        if (!cur || cur.phase.kind !== "collect" || "status" in m || m.type !== "Pointer" || m.kind !== "Down") return;
        const { width_px: w, height_px: h } = cur.start;
        const n = cur.phase.points.length;
        const [fx, fy] = TARGETS[n];
        const point: CalPoint = { touch: [m.x / (w - 1), m.y / (h - 1)], panel: [fx * (w - 1), fy * (h - 1)] };
        const points = [...cur.phase.points, point];
        if (points.length < TARGETS.length) return setPhase({ kind: "collect", points });
        invoke<number>("calibrate_check", { points })
          .then((error) => setPhase({ kind: "review", points, error }))
          .catch((e) => {
            reportError("calibration", e);
            setPhase({ kind: "collect", points: [] });
          });
      }),
    [],
  );

  if (!start) return null;
  const dpr = window.devicePixelRatio || 1;
  const { width_px: w, height_px: h } = start;
  const close = () => setStart(null);
  const cancel = () => {
    invoke("calibrate_cancel", { index: start.index }).catch((e) => reportError("calibration", e));
    close();
  };
  const save = (points: CalPoint[]) => {
    invoke("calibrate_save", { index: start.index, points })
      .then(close)
      .catch((e) => reportError("saving calibration", e));
  };

  return (
    <div style={{ position: "absolute", inset: 0, background: "rgba(5,5,6,.94)", zIndex: 1000, color: C.text }}>
      {phase.kind === "collect" &&
        TARGETS.map(([fx, fy], i) => {
          const done = i < phase.points.length;
          const current = i === phase.points.length;
          return (
            <div
              key={i}
              style={{
                position: "absolute",
                left: (fx * (w - 1)) / dpr - 22,
                top: (fy * (h - 1)) / dpr - 22,
                width: 44,
                height: 44,
                opacity: current ? 1 : done ? 0.35 : 0.15,
              }}
            >
              <div style={{ position: "absolute", left: 21, top: 0, width: 2, height: 44, background: current ? C.accent : C.text4 }} />
              <div style={{ position: "absolute", top: 21, left: 0, height: 2, width: 44, background: current ? C.accent : C.text4 }} />
              {current && (
                <div
                  style={{
                    position: "absolute",
                    inset: 0,
                    borderRadius: "50%",
                    border: `2px solid ${C.accent}`,
                    animation: "rip 1.4s ease-out infinite",
                  }}
                />
              )}
            </div>
          );
        })}
      <div
        style={{
          position: "absolute",
          inset: 0,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          gap: 14,
          pointerEvents: "none",
        }}
      >
        <div style={{ fontSize: 30, fontWeight: 600 }}>Calibrating “{start.name}”</div>
        {phase.kind === "collect" ? (
          <div style={{ fontFamily: FONT_MONO, fontSize: 14, color: C.text3 }}>
            tap the centre of the green target · {phase.points.length + 1} of {TARGETS.length}
          </div>
        ) : (
          <Review error={phase.error} />
        )}
      </div>
      <div style={{ position: "absolute", left: 0, right: 0, bottom: 28, display: "flex", justifyContent: "center", gap: 14 }}>
        {phase.kind === "review" && (
          <>
            <BigButton label="Save" tone="accent" onClick={() => save(phase.points)} />
            <BigButton label="Again" onClick={() => setPhase({ kind: "collect", points: [] })} />
          </>
        )}
        <BigButton label="Cancel" onClick={cancel} />
      </div>
    </div>
  );
}

function Review({ error }: { error: number }) {
  const [text, color] =
    error <= GOOD_PX
      ? ["Good fit", C.accent]
      : error <= BAD_PX
        ? ["Usable, but a tap was a little off", C.amber]
        : ["Probably a missed target. Try again", C.red];
  return (
    <div style={{ textAlign: "center" }}>
      <div style={{ fontSize: 18, color }}>{text}</div>
      <div style={{ fontFamily: FONT_MONO, fontSize: 13, color: C.text4, marginTop: 4 }}>
        average error {error.toFixed(1)} px
      </div>
    </div>
  );
}

interface BigButtonProps {
  label: string;
  onClick: () => void;
  tone?: "accent";
}

function BigButton({ label, onClick, tone }: BigButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      style={{
        minWidth: 160,
        height: 64,
        borderRadius: 14,
        border: "none",
        fontSize: 18,
        fontWeight: 600,
        fontFamily: "inherit",
        background: tone === "accent" ? C.accent : C.control,
        color: tone === "accent" ? C.accentInk : C.text,
        boxShadow: tone === "accent" ? "none" : `inset 0 0 0 1px ${C.controlRing}`,
      }}
    >
      {label}
    </button>
  );
}
