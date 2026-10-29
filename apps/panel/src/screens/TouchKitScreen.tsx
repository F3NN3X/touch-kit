import { useEffect, useLayoutEffect, useState, type ComponentType } from "react";
import { setStageZoom } from "../kit/drag";
import { ErrorBoundary } from "../kit/ErrorBoundary";
import { usePress } from "../kit/usePress";
import { C } from "../kit/tokens";
import { ButtonsPanel } from "./ButtonsPanel";
import { GesturePanel } from "./GesturePanel";
import { PinchPanel } from "./PinchPanel";
import { SlidersPanel } from "./SlidersPanel";
import { SwipePanel } from "./SwipePanel";
import { DOTS_H, GAP, PAD, PANEL_W, stageLayout } from "./stageLayout";

const PANELS: [string, ComponentType][] = [
  ["Buttons", ButtonsPanel],
  ["Gestures", GesturePanel],
  ["Sliders", SlidersPanel],
  ["Swipe", SwipePanel],
  ["Pinch", PinchPanel],
];

function useViewport(): [number, number] {
  const [size, setSize] = useState<[number, number]>([window.innerWidth, window.innerHeight]);
  useEffect(() => {
    const onResize = () => setSize([window.innerWidth, window.innerHeight]);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);
  return size;
}

/**
 * The five demo panels, arranged for this screen (stageLayout): one row on a bar like 1920 × 550,
 * a grid on 16:9 or portrait screens, pages with dots on small ones. The stage is zoomed so panels
 * keep their designed size; drag maths reads the zoom through `getScale`.
 */
export function TouchKitScreen() {
  const [vw, vh] = useViewport();
  const { pages, zoom } = stageLayout(vw, vh);
  const [page, setPage] = useState(0);
  const current = Math.min(page, pages.length - 1);
  useLayoutEffect(() => setStageZoom(zoom), [zoom]);

  return (
    <div style={{ position: "absolute", inset: 0, background: C.bg, overflow: "hidden" }}>
      <div
        style={{
          zoom,
          width: vw / zoom,
          height: vh / zoom,
          padding: PAD,
          boxSizing: "border-box",
          display: "flex",
          flexDirection: "column",
          gap: GAP,
        }}
      >
        {pages[current].map((row) => (
          <div key={row.join()} style={{ flex: 1, minHeight: 0, display: "flex", gap: GAP }}>
            {row.map((i) => {
              const [name, Panel] = PANELS[i];
              return (
                <div key={name} style={{ flex: `${PANEL_W[i]} 1 0`, minWidth: 0, display: "grid" }}>
                  <ErrorBoundary name={name}>
                    <Panel />
                  </ErrorBoundary>
                </div>
              );
            })}
          </div>
        ))}
        {pages.length > 1 && (
          <div style={{ height: DOTS_H, flexShrink: 0, display: "flex", justifyContent: "center" }}>
            {pages.map((_, k) => (
              <PageDot key={k} active={k === current} onTap={() => setPage(k)} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

/** A 44 × 44 tap target around a page dot: a pill for the current page, a dot for the others. */
function PageDot({ active, onTap }: { active: boolean; onTap: () => void }) {
  const press = usePress();
  return (
    <div
      onPointerDown={press.onPointerDown}
      onClick={onTap}
      style={{ width: 44, height: DOTS_H, display: "flex", alignItems: "center", justifyContent: "center" }}
    >
      <div
        style={{
          width: 22,
          height: 6,
          borderRadius: 3,
          background: active ? C.accent : C.pressedRing,
          clipPath: active ? "inset(0 round 3px)" : "inset(0 8px 0 8px round 3px)",
          transition: "clip-path .2s, background .2s",
        }}
      />
    </div>
  );
}
