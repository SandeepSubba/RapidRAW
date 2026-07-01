// @ts-nocheck — standalone script, run via esbuild+node (not the app/tsc build).
// Self-check for the keystone solver. No test runner in this repo; run with:
//   ./node_modules/.bin/esbuild src/utils/keystone.test.ts --bundle --platform=node --format=esm --outfile=/tmp/kt.mjs && node /tmp/kt.mjs
import { solveKeystone, vanishingPoint } from './keystone';
import assert from 'node:assert';

const W = 4000,
  H = 3000;

// 1) Looking up at a building: 2 verticals converging to a point above center.
const VPx = 2000,
  VPy = -2000; // above the frame
const v = solveKeystone({
  verticalGuides: [
    { x1: 500, y1: 3000, x2: VPx, y2: VPy },
    { x1: 3500, y1: 3000, x2: VPx, y2: VPy },
  ],
  horizontalGuides: [],
  width: W,
  height: H,
});
console.log('vertical case:', v);
const vp = vanishingPoint([
  { x1: 500, y1: 3000, x2: VPx, y2: VPy },
  { x1: 3500, y1: 3000, x2: VPx, y2: VPy },
])!;
assert(Math.abs(vp.x - VPx) < 1 && Math.abs(vp.y - VPy) < 1, 'VP intersection wrong');
const expected = ((1 / (VPy - H / 2)) * 100000 * H) / 2000;
assert(Math.abs(v.transformVertical - expected) < 0.01, `vertical ${v.transformVertical} != ${expected}`);
assert(v.transformHorizontal === 0, 'horizontal should be 0');
assert(v.transformScale < 100 && v.transformScale >= 10, 'should zoom in to crop black');

// 2) Already-parallel verticals -> no correction, no crop.
const p = solveKeystone({
  verticalGuides: [
    { x1: 1000, y1: 0, x2: 1000, y2: 3000 },
    { x1: 3000, y1: 0, x2: 3000, y2: 3000 },
  ],
  horizontalGuides: [],
  width: W,
  height: H,
});
console.log('parallel case:', p);
assert(p.transformVertical === 0 && p.transformScale === 100, 'parallel should be identity');

// 3) Horizontal convergence -> nonzero horizontal.
const h = solveKeystone({
  verticalGuides: [],
  horizontalGuides: [
    { x1: 0, y1: 500, x2: 6000, y2: 1500 },
    { x1: 0, y1: 2500, x2: 6000, y2: 1500 },
  ],
  width: W,
  height: H,
});
console.log('horizontal case:', h);
assert(h.transformHorizontal !== 0 && h.transformVertical === 0, 'horizontal nonzero only');

// 4) Scale invariance: same geometry in normalized 0..1 coords -> identical params.
// This is what lets the canvas overlay work in normalized image coords.
const n = solveKeystone({
  verticalGuides: [
    { x1: 500 / W, y1: 3000 / H, x2: VPx / W, y2: VPy / H },
    { x1: 3500 / W, y1: 3000 / H, x2: VPx / W, y2: VPy / H },
  ],
  horizontalGuides: [],
  width: 1,
  height: 1,
});
console.log('normalized case:', n);
assert(Math.abs(n.transformVertical - v.transformVertical) < 1e-6, 'vertical should be scale invariant');
assert(Math.abs(n.transformScale - v.transformScale) < 1e-6, 'crop should be scale invariant');

console.log('ALL KEYSTONE SOLVER CHECKS PASSED');

// 5) Strong keystone can zoom past the old 50% floor.
const strong = solveKeystone({
  verticalGuides: [
    { x1: 200 / W, y1: 3000 / H, x2: 2000 / W, y2: -6000 / H },
    { x1: 3800 / W, y1: 3000 / H, x2: 2000 / W, y2: -6000 / H },
  ],
  horizontalGuides: [], width: 1, height: 1,
});
console.log('strong case:', strong);
assert(strong.transformScale < 100, 'strong keystone should still crop');
