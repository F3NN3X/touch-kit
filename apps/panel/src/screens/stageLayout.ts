// How the five demo panels fit a display of any size. Each panel has a design width (the 1920 × 550
// bar it was drawn for) and a design height of PANEL_H; a layout arranges them in rows, and the stage
// is zoomed so the layout's design size fills the screen. Panels never get smaller than designed:
// extra room stretches them, a small screen zooms the whole stage down.

/** Design widths, in panel order (Buttons, Gestures, Sliders, Swipe, Pinch): what each gets on the
 * 1920 × 550 bar (Pinch takes the rest). */
export const PANEL_W = [340, 330, 410, 400, 328];
export const PANEL_H = 502;
export const PAD = 24;
export const GAP = 16;
/** Height of the page dots row in a paged layout. */
export const DOTS_H = 44;
/** Below this zoom a 52 px touch target drops under the spec's 44 px floor. */
export const MIN_COMFORT_ZOOM = 44 / 52;
const MAX_ZOOM = 2;
const MIN_ZOOM = 0.4;

export interface StageLayout {
  /** Panel indices per row, per page; one page unless paged. */
  pages: number[][][];
  zoom: number;
}

/** Every-panel layouts, as rows of panel indices. */
const FULL: number[][][] = [
  [[0, 1, 2, 3, 4]],
  [
    [0, 1, 2],
    [3, 4],
  ],
  [[0, 1], [2, 3], [4]],
  [[0], [1], [2], [3], [4]],
];

const rowWidth = (row: number[]) => row.reduce((w, i) => w + PANEL_W[i], 0) + GAP * (row.length - 1);

/** Zoom at which `rows` (plus `extraH` of chrome) just fits a `vw` × `vh` CSS px screen. */
function fit(rows: number[][], vw: number, vh: number, extraH: number): number {
  const w = Math.max(...rows.map(rowWidth)) + 2 * PAD;
  const h = rows.length * PANEL_H + GAP * (rows.length - 1) + 2 * PAD + extraH;
  return Math.min(vw / w, vh / h);
}

const clampZoom = (z: number) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z));

/**
 * The layout for a `vw` × `vh` screen: the all-panels arrangement with the largest zoom, if that
 * keeps touch targets comfortable; otherwise pages with as many panels each as stay comfortable
 * (side by side or stacked, whichever fits better); if nothing does, whichever arrangement zooms
 * largest.
 */
export function stageLayout(vw: number, vh: number): StageLayout {
  const full = FULL.map((rows) => ({ pages: [rows], zoom: fit(rows, vw, vh, 0) })).reduce((a, b) =>
    b.zoom > a.zoom ? b : a,
  );
  if (full.zoom >= MIN_COMFORT_ZOOM) return { ...full, zoom: clampZoom(full.zoom) };

  // k panels per page, side by side, stacked, or (k = 4) two by two; the best of those per k.
  const paged = [4, 3, 2, 1].map((k) => {
    const chunks: number[][] = [];
    for (let i = 0; i < PANEL_W.length; i += k) chunks.push(PANEL_W.slice(i, i + k).map((_, j) => i + j));
    const shapes: ((c: number[]) => number[][])[] = [(c) => [c], (c) => c.map((i) => [i])];
    if (k === 4) shapes.push((c) => [c.slice(0, 2), c.slice(2)]);
    return shapes
      .map((shape) => {
        const pages = chunks.map(shape);
        return { pages, zoom: Math.min(...pages.map((rows) => fit(rows, vw, vh, DOTS_H))) };
      })
      .reduce((a, b) => (b.zoom > a.zoom ? b : a));
  });
  const comfortable = paged.find((p) => p.zoom >= MIN_COMFORT_ZOOM);
  const best = comfortable ?? [full, ...paged].reduce((a, b) => (b.zoom > a.zoom ? b : a));
  return { ...best, zoom: clampZoom(best.zoom) };
}
