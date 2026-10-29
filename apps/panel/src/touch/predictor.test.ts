import { describe, expect, it } from "vitest";
import { defaultOptions, Predictor, type PredictorOptions } from "./predictor";

/** Samples moving right at `v` px/ms, 8 ms apart (the panel's scan), optionally with arrival jitter. */
function feed(p: Predictor, v: number, n: number, jitter: (i: number) => number = () => 0) {
  for (let i = 0; i < n; i++) p.add({ t: 1000 + i * 8 + jitter(i), x: 100 + v * i * 8, y: 50 });
}

const opts = (o: Partial<PredictorOptions>): PredictorOptions => ({ ...defaultOptions, ...o });

describe("Predictor", () => {
  it("draws ahead of the last sample by velocity × (time since it + lead)", () => {
    const p = new Predictor(opts({ leadMs: 16 }));
    feed(p, 1, 6); // last sample t=1040, x=140
    const at = p.at(1044)!;
    expect(at.x).toBeCloseTo(140 + 1 * (4 + 16), 5);
    expect(at.y).toBeCloseTo(50, 5);
  });

  it("does not extrapolate with fewer than three samples", () => {
    const p = new Predictor();
    feed(p, 1, 2);
    expect(p.at(1010)).toEqual({ x: 108, y: 50 });
  });

  it("snaps to the last sample once the finger has stopped", () => {
    const p = new Predictor(opts({ stillAfterMs: 14 }));
    feed(p, 1, 6);
    expect(p.at(1040 + 15)).toEqual({ x: 140, y: 50 });
  });

  it("never draws further ahead than maxAheadPx", () => {
    const p = new Predictor(opts({ leadMs: 16, maxAheadPx: 48 }));
    feed(p, 10, 6); // 10 px/ms would put it 160 px ahead
    const at = p.at(1040)!;
    expect(Math.hypot(at.x - 500, at.y - 50)).toBeCloseTo(48, 5);
  });

  it("keeps the velocity within 15% under the 6/10 ms arrival jitter seen on the device", () => {
    const p = new Predictor();
    feed(p, 0.5, 5, (i) => (i % 2 ? -2 : 2));
    expect(p.velocity().vx).toBeGreaterThan(0.5 * 0.85);
    expect(p.velocity().vx).toBeLessThan(0.5 * 1.15);
  });

  it("with no lead, draws the last sample at its own time", () => {
    const p = new Predictor(opts({ leadMs: 0 }));
    feed(p, 1, 6);
    expect(p.at(1040)!.x).toBeCloseTo(140, 5);
  });
});
