// Receives touch-core messages over a Tauri Channel and turns them into DOM events.
//
// Each contact becomes a synthetic PointerEvent stream (pointerType "touch", pointerId
// TOUCH_POINTER_BASE + contact id), so UI code is written once against Pointer Events and works the
// same with the real mouse in dev mode. Panel px → CSS px (÷ devicePixelRatio; the window covers
// the panel monitor exactly). On down, `elementFromPoint` picks the target and keeps it for that
// contact: implicit capture, because `setPointerCapture` only accepts real pointers (call it in a
// try/catch). A lift within the slop also dispatches `click`, as a browser would.
// None of this is OS input: nothing leaves the webview.
//
// Rust forwards every move (125 Hz); moves are held per contact and dispatched once per animation
// frame, latest wins. Any other message flushes held moves first, keeping order. Every sample also
// feeds a per-contact Predictor; visuals that follow a finger draw from `drawPositions`.
// Rust gestures arrive as bubbling `tp-gesture` CustomEvents on the element under the gesture.
//
// The Rust half is touch-kit-tauri's `Bridge`; the app exposes a command (default `subscribe`)
// that hands it the Channel.

import { Channel, invoke } from "@tauri-apps/api/core";
import { defaultOptions, Predictor, type PredictorOptions } from "./predictor";
import { isStatus, type DeviceMsg, type Gesture, type PointerMsg } from "./types";

export const TOUCH_POINTER_BASE = 1000;

export interface BridgeOptions {
  /** The Tauri command that takes the Channel. */
  command: string;
  onError: (context: string, err: unknown) => void;
  /** Per applied move: Rust send → webview arrival, and arrival → the frame that applied it (ms). */
  onLatency?: (ipcMs: number, frameMs: number) => void;
}

let options: BridgeOptions = { command: "subscribe", onError: (context, err) => console.error(context, err) };

/** Changes how the bridge subscribes and reports; call before the first `onDeviceMessage`. */
export function configureBridge(o: Partial<BridgeOptions>): void {
  options = { ...options, ...o };
}

const wallNow = (): number => performance.timeOrigin + performance.now();

export type GestureDetail = { gesture: Gesture; x: number; y: number };

declare global {
  interface HTMLElementEventMap {
    "tp-gesture": CustomEvent<GestureDetail>;
  }
}

type Listener = (m: DeviceMsg) => void;
type UiMsg = DeviceMsg & { sent_ms: number };

interface Contact {
  target: Element | null;
  startX: number;
  startY: number;
  x: number;
  y: number;
  moved: boolean;
}

const listeners = new Set<Listener>();
const contacts = new Map<number, Contact>();
const heldMoves = new Map<number, { m: PointerMsg; ipc: number; arrived: number }>();
const predictors = new Map<number, Predictor>();
let predictorOptions: PredictorOptions = defaultOptions;
let slopPx = 12;
let primaryId: number | null = null;
let frameRequested = false;

/** Prediction lead (ms) for contacts that land afterwards. */
export function setPredictionLead(leadMs: number): void {
  predictorOptions = { ...defaultOptions, leadMs };
}

/** Movement (CSS px) beyond which a lift no longer produces a click. */
export function setClickSlop(px: number): void {
  slopPx = px;
}

/** Where to draw each live contact in the frame starting at `frameT` (wall ms), in CSS px. */
export function drawPositions(frameT: number): Map<number, { x: number; y: number }> {
  const out = new Map<number, { x: number; y: number }>();
  for (const [id, p] of predictors) {
    const at = p.at(frameT);
    if (at) out.set(id, at);
  }
  return out;
}

export const toCss = (x: number, y: number) => ({ x: x / window.devicePixelRatio, y: y / window.devicePixelRatio });

function track(m: PointerMsg, t: number) {
  if (m.kind === "Up") {
    predictors.delete(m.id);
    return;
  }
  let p = predictors.get(m.id);
  if (!p || m.kind === "Down") {
    p = new Predictor(predictorOptions);
    predictors.set(m.id, p);
  }
  p.add({ t, ...toCss(m.x, m.y) });
}

function firePointer(target: Element | null, type: string, id: number, x: number, y: number) {
  const down = type !== "pointerup" && type !== "pointercancel";
  target?.dispatchEvent(
    new PointerEvent(type, {
      bubbles: true,
      cancelable: true,
      composed: true,
      pointerId: TOUCH_POINTER_BASE + id,
      pointerType: "touch",
      isPrimary: id === primaryId,
      clientX: x,
      clientY: y,
      button: type === "pointermove" ? -1 : 0,
      buttons: down ? 1 : 0,
      pressure: down ? 0.5 : 0,
      width: 20,
      height: 20,
    }),
  );
}

function applyPointer(m: PointerMsg) {
  const { x, y } = toCss(m.x, m.y);
  if (m.kind === "Down") {
    if (contacts.size === 0) primaryId = m.id;
    const target = document.elementFromPoint(x, y);
    contacts.set(m.id, { target, startX: x, startY: y, x, y, moved: false });
    firePointer(target, "pointerdown", m.id, x, y);
    return;
  }
  const c = contacts.get(m.id);
  if (!c) return;
  c.x = x;
  c.y = y;
  if (!c.moved && Math.hypot(x - c.startX, y - c.startY) > slopPx) c.moved = true;
  if (m.kind === "Move") {
    firePointer(c.target, "pointermove", m.id, x, y);
    return;
  }
  firePointer(c.target, "pointerup", m.id, x, y);
  if (!c.moved) {
    c.target?.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, clientX: x, clientY: y, detail: 1 }));
  }
  contacts.delete(m.id);
  if (primaryId === m.id) primaryId = null;
}

/** Ends every live contact with pointercancel (device lost). */
function cancelAll() {
  for (const [id, c] of contacts) firePointer(c.target, "pointercancel", id, c.x, c.y);
  contacts.clear();
  predictors.clear();
  heldMoves.clear();
  primaryId = null;
}

function apply(m: DeviceMsg) {
  if (isStatus(m)) {
    if (m.status !== "Connected") cancelAll();
  } else if (m.type === "Pointer") {
    applyPointer(m);
  } else if (m.type === "Gesture") {
    const { x, y } = toCss(m.x, m.y);
    document.elementFromPoint(x, y)?.dispatchEvent(
      new CustomEvent<GestureDetail>("tp-gesture", { detail: { gesture: m.gesture, x, y }, bubbles: true }),
    );
  }
  listeners.forEach((fn) => fn(m));
}

function applyHeldMoves(frameWall?: number) {
  for (const { m, ipc, arrived } of heldMoves.values()) {
    if (frameWall !== undefined) options.onLatency?.(ipc, frameWall - arrived);
    apply(m);
  }
  heldMoves.clear();
}

function receive({ sent_ms, ...m }: UiMsg) {
  const msg = m as DeviceMsg;
  if (!isStatus(msg) && msg.type === "Pointer") track(msg, sent_ms);
  if (!isStatus(msg) && msg.type === "Pointer" && msg.kind === "Move") {
    const arrived = wallNow();
    heldMoves.set(msg.id, { m: msg, ipc: arrived - sent_ms, arrived });
    if (!frameRequested) {
      frameRequested = true;
      requestAnimationFrame((ts) => {
        frameRequested = false;
        applyHeldMoves(performance.timeOrigin + ts);
      });
    }
    return;
  }
  applyHeldMoves();
  apply(msg);
}

let started = false;

/** Subscribes to the Rust side once; safe to call from every component that needs raw messages. */
export function onDeviceMessage(l: Listener): () => void {
  listeners.add(l);
  if (!started) {
    started = true;
    const channel = new Channel<UiMsg>();
    channel.onmessage = receive;
    invoke(options.command, { channel }).catch((e) => options.onError("touch subscribe", e));
  }
  return () => listeners.delete(l);
}
