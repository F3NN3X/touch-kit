import { useEffect, useRef, useState, type ReactNode } from "react";
import { getSettings } from "../touch/settings";
import { getScale, track } from "./drag";
import { swipeDir } from "./swipe";
import { C, EASE, clamp } from "./tokens";
import { usePress } from "./usePress";

const CARD_W = 364;
const GAP = 12;
const RUBBER_BAND = 0.35;
const TAP_PX = 6;

interface CarouselProps {
  cards: ReactNode[];
  index: number;
  onIndexChange: (index: number) => void;
  /** Tap (< 6 px) on the current card. */
  onSelect: (index: number) => void;
}

interface DotProps {
  active: boolean;
  onClick: () => void;
}

/** Spec §1: 44 px is the absolute floor for a hit target; each dot gets its own, no overlap. */
const DOT_HIT = 44;

function Dot({ active, onClick }: DotProps) {
  const press = usePress();
  return (
    <div
      onPointerDown={press.onPointerDown}
      onClick={onClick}
      style={{ width: DOT_HIT, height: DOT_HIT, display: "flex", alignItems: "center", justifyContent: "center" }}
    >
      {/* A 22 px pill clipped to a 6 px dot when inactive: the clip animates, the layout never moves. */}
      <div
        style={{
          width: 22,
          height: 6,
          background: active || press.pressed ? C.text : C.pressedRing,
          clipPath: active ? "inset(0 0 0 0 round 3px)" : "inset(0 8px 0 8px round 3px)",
          transition: "clip-path .2s, background .2s",
        }}
      />
    </div>
  );
}

/** Card pager: 1:1 finger follow, rubber band at the ends, snap on release, tappable dots. */
export function Carousel({ cards, index, onIndexChange, onSelect }: CarouselProps) {
  const [dx, setDx] = useState(0);
  const [dragging, setDragging] = useState(false);
  const last = cards.length - 1;
  const at = useRef(index);
  at.current = index;
  const change = useRef(onIndexChange);
  change.current = onIndexChange;

  useEffect(() => {
    const onPage = (e: Event) => {
      const to = (e as CustomEvent<{ to: "next" | "prev" }>).detail.to;
      change.current(clamp(at.current + (to === "next" ? 1 : -1), 0, last));
    };
    window.addEventListener("kit:page", onPage);
    return () => window.removeEventListener("kit:page", onPage);
  }, [last]);

  return (
    <>
      <div
        onPointerDown={(e) => {
          const s = getSettings().gestures;
          const scale = getScale();
          const startX = e.clientX;
          const t0 = performance.now();
          setDragging(true);
          track(
            e,
            (ev) => {
              let d = (ev.clientX - startX) / scale;
              if ((at.current === 0 && d > 0) || (at.current === last && d < 0)) d *= RUBBER_BAND;
              setDx(d);
            },
            (ev) => {
              const d = (ev.clientX - startX) / scale;
              let i = at.current;
              setDx(0);
              setDragging(false);
              if (Math.abs(d) < TAP_PX) {
                onSelect(i);
                return;
              }
              const dir = swipeDir(d, 0, performance.now() - t0, s);
              if (dir === "left") i = Math.min(i + 1, last);
              else if (dir === "right") i = Math.max(i - 1, 0);
              change.current(i);
            },
          );
        }}
        style={{ height: 136, borderRadius: 14, overflow: "hidden", touchAction: "pan-y" }}
      >
        <div
          style={{
            display: "flex",
            gap: GAP,
            height: "100%",
            transform: `translateX(${-index * (CARD_W + GAP) + dx}px)`,
            transition: dragging ? "none" : `transform .32s ${EASE}`,
          }}
        >
          {cards.map((card, i) => (
            <div key={i} style={{ flex: "none", width: CARD_W, height: "100%" }}>
              {card}
            </div>
          ))}
        </div>
      </div>
      <div style={{ display: "flex", justifyContent: "center", alignItems: "center", height: DOT_HIT }}>
        {cards.map((_, i) => (
          <Dot key={i} active={i === index} onClick={() => onIndexChange(i)} />
        ))}
      </div>
    </>
  );
}
