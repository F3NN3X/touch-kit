import { describe, expect, it } from "vitest";
import { MIN_COMFORT_ZOOM, stageLayout } from "./stageLayout";

const shape = (vw: number, vh: number) => {
  const l = stageLayout(vw, vh);
  return { pages: l.pages.length, rows: l.pages[0].map((r) => r.length), zoom: Math.round(l.zoom * 100) / 100 };
};

describe("stageLayout", () => {
  it("keeps the 1920 × 550 bar it was designed for", () => {
    expect(shape(1920, 550)).toEqual({ pages: 1, rows: [5], zoom: 1 });
  });

  it("fits common display shapes without shrinking targets below the floor", () => {
    expect(shape(2560, 720)).toMatchObject({ pages: 1, rows: [5] }); // Xeneon Edge class bar
    expect(shape(1920, 1080)).toMatchObject({ pages: 1, rows: [3, 2] }); // 16:9
    expect(shape(1080, 1920)).toMatchObject({ pages: 1, rows: [2, 2, 1] }); // portrait
    for (const [w, h] of [
      [1920, 550],
      [2560, 720],
      [1920, 1080],
      [1280, 800],
      [1024, 600],
      [1080, 1920],
      [600, 1024],
    ]) {
      expect(stageLayout(w, h).zoom, `${w}x${h}`).toBeGreaterThanOrEqual(MIN_COMFORT_ZOOM);
    }
  });

  it("pages small screens, keeping every panel on some page", () => {
    const l = stageLayout(800, 480);
    expect(l.pages.length).toBeGreaterThan(1);
    expect(l.pages.flat(2).sort()).toEqual([0, 1, 2, 3, 4]);
    // 480 px is too short for a 502 px panel at full size: the closest it gets (targets ~42 px).
    expect(l.zoom).toBeGreaterThan(0.8);
  });

  it("stacks panels on a page of a tall narrow screen", () => {
    expect(shape(720, 1280)).toMatchObject({ pages: 3, rows: [1, 1] });
  });

  it("prefers the larger zoom when nothing is comfortable", () => {
    // A very flat bar: the single row beats paging.
    expect(shape(1920, 440)).toMatchObject({ pages: 1, rows: [5] });
  });
});
