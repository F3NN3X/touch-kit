import type { ComponentType } from "react";
import { ErrorBoundary } from "../kit/ErrorBoundary";
import { C } from "../kit/tokens";
import { ButtonsPanel } from "./ButtonsPanel";
import { GesturePanel } from "./GesturePanel";
import { PinchPanel } from "./PinchPanel";
import { SlidersPanel } from "./SlidersPanel";
import { SwipePanel } from "./SwipePanel";

const PANELS: [string, ComponentType][] = [
  ["Buttons", ButtonsPanel],
  ["Gestures", GesturePanel],
  ["Sliders", SlidersPanel],
  ["Swipe", SwipePanel],
  ["Pinch", PinchPanel],
];

/** The 1920 × 550 stage: five demo panels in one row. */
export function TouchKitScreen() {
  return (
    <div
      style={{
        position: "absolute",
        inset: 0,
        background: C.bg,
        padding: 24,
        boxSizing: "border-box",
        display: "grid",
        gridTemplateColumns: "340px 330px 410px 400px minmax(0,1fr)",
        gridTemplateRows: "minmax(0,1fr)",
        gap: 16,
      }}
    >
      {PANELS.map(([name, Panel]) => (
        <ErrorBoundary key={name} name={name}>
          <Panel />
        </ErrorBoundary>
      ))}
    </div>
  );
}
