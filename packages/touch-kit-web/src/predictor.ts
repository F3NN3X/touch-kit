// Per-contact position predictor for visual feedback.
//
// The panel samples at 125 Hz but samples reach us 6/10 ms apart and the display refreshes at
// 60 Hz, so drawing "the latest sample" each frame moves unevenly; and the panel, compositor and
// display add latency we can't remove. So each frame draws the position *at* frame time + lead,
// extrapolated with a least-squares velocity over the last few samples. Hit-testing and
// tp-* events keep using raw positions; this is only for what is drawn.

export interface Sample {
  t: number; // wall ms
  x: number;
  y: number;
}

export interface PredictorOptions {
  /** How far ahead of the frame to draw, ms. 0 = no prediction, only even resampling. */
  leadMs: number;
  /** Velocity window. */
  windowMs: number;
  /** No new sample for this long = the finger stopped (touch-core only reports changes). */
  stillAfterMs: number;
  /** Never draw further than this from the last real sample, px. */
  maxAheadPx: number;
}

export const defaultOptions: PredictorOptions = { leadMs: 16, windowMs: 40, stillAfterMs: 14, maxAheadPx: 48 };

export class Predictor {
  private samples: Sample[] = [];

  constructor(private readonly opts: PredictorOptions = defaultOptions) {}

  add(s: Sample): void {
    this.samples.push(s);
    const cutoff = s.t - this.opts.windowMs;
    while (this.samples.length > 2 && this.samples[0].t < cutoff) this.samples.shift();
  }

  /** Least-squares velocity in px/ms over the window, or zero with fewer than three samples. */
  velocity(): { vx: number; vy: number } {
    const n = this.samples.length;
    if (n < 3) return { vx: 0, vy: 0 };
    const t0 = this.samples[0].t;
    let st = 0, sx = 0, sy = 0, stt = 0, stx = 0, sty = 0;
    for (const s of this.samples) {
      const t = s.t - t0;
      st += t;
      sx += s.x;
      sy += s.y;
      stt += t * t;
      stx += t * s.x;
      sty += t * s.y;
    }
    const den = n * stt - st * st;
    if (den <= 1e-9) return { vx: 0, vy: 0 };
    return { vx: (n * stx - st * sx) / den, vy: (n * sty - st * sy) / den };
  }

  /** Where to draw the contact in a frame that starts at `frameT` (wall ms). */
  at(frameT: number): { x: number; y: number } | null {
    const last = this.samples[this.samples.length - 1];
    if (!last) return null;
    if (frameT - last.t > this.opts.stillAfterMs) return { x: last.x, y: last.y };
    const { vx, vy } = this.velocity();
    const dt = frameT - last.t + this.opts.leadMs;
    let dx = vx * dt;
    let dy = vy * dt;
    const d = Math.hypot(dx, dy);
    if (d > this.opts.maxAheadPx) {
      dx *= this.opts.maxAheadPx / d;
      dy *= this.opts.maxAheadPx / d;
    }
    return { x: last.x + dx, y: last.y + dy };
  }
}
