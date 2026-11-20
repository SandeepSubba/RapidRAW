// @ts-nocheck — standalone script, run via esbuild+node (not the app/tsc build).
// Self-check for the importer's RAW / JPEG modes. No test runner in this repo; run with:
//   ./node_modules/.bin/esbuild src/components/views/import/importFilters.test.ts --bundle --platform=node --format=esm --outfile=/tmp/if.mjs && node /tmp/if.mjs
import assert from 'node:assert';
import {
  computeVisibleSet,
  fileTypeCounts,
  fileTypeFilterFrom,
  normalizeFileTypeFilter,
  pairCompanions,
} from './importFilters';

const RAW = ['cr3', 'raf', 'nef'];
const card = [
  '/card/DCIM/IMG_0001.CR3',
  '/card/DCIM/IMG_0001.JPG', // pair 1
  '/card/DCIM/IMG_0002.CR3', // RAW only
  '/card/DCIM/IMG_0003.cr3',
  '/card/DCIM/img_0003.jpg', // pair 2 (case-insensitive)
  '/card/DCIM/IMG_0004.JPG', // JPEG only
  '/card/OTHER/IMG_0001.JPG', // same name, other folder: not a pair
];
const sorted = (s) => [...s].sort();

// Counts drive the toolbar chips.
assert.deepStrictEqual(fileTypeCounts(card, RAW), { raw: 3, jpeg: 4, pairs: 2 });

// Toggle state -> mode.
assert.strictEqual(fileTypeFilterFrom(true, true, 'both'), 'all');
assert.strictEqual(fileTypeFilterFrom(true, true, 'raw'), 'pairs');
assert.strictEqual(fileTypeFilterFrom(true, true, 'jpeg'), 'pairsJpeg');
assert.strictEqual(fileTypeFilterFrom(true, false, 'jpeg'), 'raw');
assert.strictEqual(fileTypeFilterFrom(false, true, 'raw'), 'nonraw');

// Old saved values migrate.
assert.strictEqual(normalizeFileTypeFilter('preferRaw'), 'raw');
assert.strictEqual(normalizeFileTypeFilter(undefined), 'all');
assert.strictEqual(normalizeFileTypeFilter('pairsJpeg'), 'pairsJpeg');

// Visibility per mode.
assert.strictEqual(computeVisibleSet(card, 'all', RAW), null);
assert.deepStrictEqual(sorted(computeVisibleSet(card, 'raw', RAW)), sorted(card.filter((p) => /cr3$/i.test(p))));
assert.deepStrictEqual(sorted(computeVisibleSet(card, 'nonraw', RAW)), sorted(card.filter((p) => /jpg$/i.test(p))));
assert.deepStrictEqual(
  sorted(computeVisibleSet(card, 'pairs', RAW)),
  sorted([
    '/card/DCIM/IMG_0001.CR3',
    '/card/DCIM/IMG_0002.CR3',
    '/card/DCIM/IMG_0003.cr3',
    '/card/DCIM/IMG_0004.JPG',
    '/card/OTHER/IMG_0001.JPG',
  ]),
);
assert.deepStrictEqual(
  sorted(computeVisibleSet(card, 'pairsJpeg', RAW)),
  sorted([
    '/card/DCIM/IMG_0001.JPG',
    '/card/DCIM/IMG_0002.CR3',
    '/card/DCIM/img_0003.jpg',
    '/card/DCIM/IMG_0004.JPG',
    '/card/OTHER/IMG_0001.JPG',
  ]),
);

// A single-type card ignores the mode (a saved "RAW only" must not blank a JPEG card).
const jpegCard = ['/c/a.jpg', '/c/b.jpg'];
assert.strictEqual(computeVisibleSet(jpegCard, 'raw', RAW), null);
assert.strictEqual(computeVisibleSet(['/c/a.cr3'], 'nonraw', RAW), null);

// Companions: the hidden half of each pair, keyed by the tile on show.
assert.deepStrictEqual(
  [...pairCompanions(card, RAW, 'pairs')].sort(),
  [
    ['/card/DCIM/IMG_0001.CR3', ['/card/DCIM/IMG_0001.JPG']],
    ['/card/DCIM/IMG_0003.cr3', ['/card/DCIM/img_0003.jpg']],
  ],
);
assert.deepStrictEqual(
  [...pairCompanions(card, RAW, 'pairsJpeg')].sort(),
  [
    ['/card/DCIM/IMG_0001.JPG', ['/card/DCIM/IMG_0001.CR3']],
    ['/card/DCIM/img_0003.jpg', ['/card/DCIM/IMG_0003.cr3']],
  ],
);
assert.strictEqual(pairCompanions(card, RAW, 'all').size, 0);
assert.strictEqual(pairCompanions(card, RAW, 'raw').size, 0);

console.log('importFilters: all checks passed');
