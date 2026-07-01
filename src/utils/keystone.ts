// Guided keystone: turn user-drawn reference lines into the app's existing
// perspective params (transformVertical / transformHorizontal / transformScale).
//
// The backend warp (build_transform_matrices in image_processing.rs) applies a
// centered keystone homography whose bottom row is [p_horiz, p_vert, 1]:
//   p_vert  = (vertical   / 100000) * (REF_DIM / height)
//   p_horiz = (-horizontal / 100000) * (REF_DIM / width)
// That maps output verticals (x=const) to source lines converging at the
// centered point (0, 1/p_vert), and output horizontals to (1/p_horiz, 0).
// So a set of lines the user marks as "should be vertical" gives a source
// vanishing point whose centered Y yields p_vert = 1 / Vv_y, and likewise for
// horizontal. We then zoom (transformScale) until the warped frame has no black.
//
// Limitation: this is the constrained keystone model (pure vertical/horizontal
// convergence, symmetric about center). Residual roll/tilt is left to the
// existing rotation slider — same division of labour as Lightroom's Vertical
// vs. Level upright.

export const REF_DIM = 2000;

export interface Line {
  // endpoints in ORIGINAL image pixel coordinates
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

export type GuideOrientation = 'vertical' | 'horizontal';

export interface KeystoneParams {
  transformVertical: number;
  transformHorizontal: number;
  transformScale: number;
}

// Ax + By + C = 0 through the two endpoints.
function lineCoeffs(l: Line): [number, number, number] {
  const A = l.y2 - l.y1;
  const B = l.x1 - l.x2;
  const C = -(A * l.x1 + B * l.y1);
  return [A, B, C];
}

// Homogeneous intersection (vanishing point) of the average of several lines.
// Averaging pairwise intersections is noisy; instead we intersect the two
// lines that are most separated in angle. For the common 2-line case that is
// just their intersection. Returns null when (near) parallel — no convergence.
export function vanishingPoint(lines: Line[]): { x: number; y: number } | null {
  if (lines.length < 2) return null;
  let best: { x: number; y: number } | null = null;
  let bestW = 1e-6;
  for (let i = 0; i < lines.length; i++) {
    for (let j = i + 1; j < lines.length; j++) {
      const [a1, b1, c1] = lineCoeffs(lines[i]);
      const [a2, b2, c2] = lineCoeffs(lines[j]);
      // cross product of the two lines
      const x = b1 * c2 - c1 * b2;
      const y = c1 * a2 - a1 * c2;
      const w = a1 * b2 - b1 * a2;
      if (Math.abs(w) > Math.abs(bestW)) {
        bestW = w;
        best = { x: x / w, y: y / w };
      }
    }
  }
  return best;
}

// output(x,y) -> source(x,y) using the centered keystone+scale homography.
// Mirrors build_transform_matrices with rotate=0, aspect=0, offsets=0.
function forwardSource(
  x: number,
  y: number,
  pHoriz: number,
  pVert: number,
  scaleFactor: number,
  cx: number,
  cy: number,
): { x: number; y: number; w: number } {
  const sx = (x - cx) * scaleFactor;
  const sy = (y - cy) * scaleFactor;
  const w = pHoriz * sx + pVert * sy + 1;
  return { x: cx + sx / w, y: cy + sy / w, w };
}

function frameFitsInside(
  pHoriz: number,
  pVert: number,
  scaleFactor: number,
  width: number,
  height: number,
): boolean {
  const cx = width / 2;
  const cy = height / 2;
  const xs = [0, cx, width];
  const ys = [0, cy, height];
  for (const x of xs) {
    for (const y of ys) {
      const s = forwardSource(x, y, pHoriz, pVert, scaleFactor, cx, cy);
      if (s.w <= 0) return false;
      if (s.x < 0 || s.x > width || s.y < 0 || s.y > height) return false;
    }
  }
  return true;
}

// Largest scaleFactor in [0.1, 1] that leaves no black border. As the factor
// shrinks the sampled region moves toward the center, so "fits" is monotonic.
function autoFitScale(pHoriz: number, pVert: number, width: number, height: number): number {
  if (frameFitsInside(pHoriz, pVert, 1, width, height)) return 1;
  let lo = 0.1;
  let hi = 1;
  if (!frameFitsInside(pHoriz, pVert, lo, width, height)) return lo;
  for (let i = 0; i < 28; i++) {
    const mid = (lo + hi) / 2;
    if (frameFitsInside(pHoriz, pVert, mid, width, height)) lo = mid;
    else hi = mid;
  }
  return lo;
}

// Zoom (transformScale) that crops the black borders for a given set of
// perspective params. Scale-invariant, so it runs in normalized coords.
const PARAM_TO_P = 100000 / REF_DIM;
export function fitScaleForParams(vertical: number, horizontal: number): number {
  const pVert = vertical / PARAM_TO_P;
  const pHoriz = -horizontal / PARAM_TO_P;
  return clamp(autoFitScale(pHoriz, pVert, 1, 1) * 100, 10, 150);
}

export interface SolveInput {
  verticalGuides: Line[];
  horizontalGuides: Line[];
  width: number;
  height: number;
}

export function solveKeystone({ verticalGuides, horizontalGuides, width, height }: SolveInput): KeystoneParams {
  const cx = width / 2;
  const cy = height / 2;

  let pVert = 0;
  let pHoriz = 0;

  const vv = vanishingPoint(verticalGuides);
  if (vv) {
    const vyC = vv.y - cy;
    if (Math.abs(vyC) > 1e-3) pVert = 1 / vyC;
  }
  const vh = vanishingPoint(horizontalGuides);
  if (vh) {
    const vxC = vh.x - cx;
    if (Math.abs(vxC) > 1e-3) pHoriz = 1 / vxC;
  }

  const scaleFactor = autoFitScale(pHoriz, pVert, width, height);

  const vertical = (pVert * 100000 * height) / REF_DIM;
  const horizontal = (-pHoriz * 100000 * width) / REF_DIM;

  return {
    transformVertical: clamp(vertical, -100, 100),
    transformHorizontal: clamp(horizontal, -100, 100),
    transformScale: clamp(scaleFactor * 100, 10, 150),
  };
}

function clamp(v: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, v));
}
