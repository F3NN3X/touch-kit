import { describe, expect, it } from "vitest";
import { swipeDir } from "./swipe";

// Touch Kit spec §2 defaults; the same values touch-core's tests use.
const t = { swipe_min_px: 60, flick_min_px: 10, flick_min_speed: 0.5 };

describe("swipeDir", () => {
  it("commits past 60 px at any speed, not at 60", () => {
    expect(swipeDir(61, 0, 5000, t)).toBe("right");
    expect(swipeDir(60, 0, 5000, t)).toBeNull();
  });

  it("commits a short flick only when faster than 0.5 px/ms and longer than 10 px", () => {
    expect(swipeDir(-50, 0, 80, t)).toBe("left");
    expect(swipeDir(50, 0, 100, t)).toBeNull(); // exactly 0.5 px/ms
    expect(swipeDir(10, 0, 1, t)).toBeNull(); // fast but not past 10 px
    expect(swipeDir(11, 0, 10, t)).toBe("right");
  });

  it("takes the direction from the dominant axis and measures distance on it", () => {
    expect(swipeDir(150, 140, 300, t)).toBe("right");
    expect(swipeDir(140, -150, 300, t)).toBe("up");
    expect(swipeDir(0, 200, 300, t)).toBe("down");
    expect(swipeDir(40, 40, 1000, t)).toBeNull(); // 57 px diagonal, but 40 px on either axis
  });

  it("follows the live thresholds", () => {
    expect(swipeDir(80, 0, 5000, { ...t, swipe_min_px: 100 })).toBeNull();
  });
});
