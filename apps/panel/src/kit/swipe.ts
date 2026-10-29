// The swipe commit rule (Touch Kit spec §2), shared by every control that swipes.
// Same rule as touch-core's recogniser: distance on the dominant axis, average speed over the stroke.

export type SwipeDir = "left" | "right" | "up" | "down";

export interface SwipeThresholds {
  swipe_min_px: number;
  flick_min_px: number;
  flick_min_speed: number;
}

/** Commits past `swipe_min_px`, or past `flick_min_px` when flicked faster than `flick_min_speed` px/ms. */
export function swipeDir(dx: number, dy: number, ms: number, t: SwipeThresholds): SwipeDir | null {
  const horizontal = Math.abs(dx) >= Math.abs(dy);
  const dist = horizontal ? Math.abs(dx) : Math.abs(dy);
  const speed = dist / Math.max(1, ms);
  if (!(dist > t.swipe_min_px || (dist > t.flick_min_px && speed > t.flick_min_speed))) return null;
  if (horizontal) return dx < 0 ? "left" : "right";
  return dy < 0 ? "up" : "down";
}
