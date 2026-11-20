/**
 * Which files an import copies, and how a RAW + JPEG pair shows in the grid. Driven by
 * the RAW / JPEG toggles in the culling toolbar ("JPEG" = every non-raw image: JPEG,
 * HEIF, TIFF, PNG…):
 *   all        RAW + JPEG, every file its own tile
 *   raw        RAW only
 *   nonraw     JPEG only
 *   pairs      RAW + JPEG, a pair shown as its RAW (keeping it imports the JPEG too)
 *   pairsJpeg  RAW + JPEG, a pair shown as its JPEG (keeping it imports the RAW too)
 * A pair is a RAW and a non-raw with the same folder and file name (case-insensitive).
 */
export type FileTypeFilter = 'all' | 'raw' | 'nonraw' | 'pairs' | 'pairsJpeg';

/** How a RAW + JPEG pair is shown while both types are imported. */
export type PairView = 'both' | 'raw' | 'jpeg';

/** Saved values from older versions: 'preferRaw' (RAW, plus JPEGs without a RAW) is now RAW only. */
export function normalizeFileTypeFilter(value: unknown): FileTypeFilter {
  if (value === 'preferRaw') return 'raw';
  return value === 'raw' || value === 'nonraw' || value === 'pairs' || value === 'pairsJpeg' ? value : 'all';
}

export const importsRaw = (f: FileTypeFilter) => f !== 'nonraw';
export const importsJpeg = (f: FileTypeFilter) => f !== 'raw';
export const pairViewOf = (f: FileTypeFilter): PairView => (f === 'pairs' ? 'raw' : f === 'pairsJpeg' ? 'jpeg' : 'both');

/** The filter for a RAW / JPEG toggle state; `view` only matters when both are on. */
export function fileTypeFilterFrom(raw: boolean, jpeg: boolean, view: PairView): FileTypeFilter {
  if (raw && !jpeg) return 'raw';
  if (jpeg && !raw) return 'nonraw';
  return view === 'raw' ? 'pairs' : view === 'jpeg' ? 'pairsJpeg' : 'all';
}

const extOf = (p: string) => p.split('?')[0].split(/[\\/]/).pop()?.split('.').pop()?.toLowerCase() || '';

/** Folder + file name without extension, lower-cased: the two files of a pair share it. */
export const frameKey = (p: string) => {
  const noVc = p.split('?')[0];
  const slash = Math.max(noVc.lastIndexOf('/'), noVc.lastIndexOf('\\'));
  const dir = noVc.slice(0, slash);
  const name = noVc.slice(slash + 1);
  const dot = name.lastIndexOf('.');
  return `${dir}/${dot === -1 ? name : name.slice(0, dot)}`.toLowerCase();
};

const rawTester = (rawExts: string[]) => {
  const rawSet = new Set(rawExts.map((e) => e.toLowerCase()));
  return (p: string) => rawSet.has(extOf(p));
};

/** How many RAWs, non-raws and RAW + JPEG pairs a scan holds (for the toolbar toggles). */
export function fileTypeCounts(paths: string[], rawExts: string[]) {
  const isRaw = rawTester(rawExts);
  const rawKeys = new Set<string>();
  let raw = 0;
  paths.forEach((p) => {
    if (isRaw(p)) {
      raw++;
      rawKeys.add(frameKey(p));
    }
  });
  const pairKeys = new Set(paths.filter((p) => !isRaw(p) && rawKeys.has(frameKey(p))).map(frameKey));
  return { raw, jpeg: paths.length - raw, pairs: pairKeys.size };
}

/**
 * Returns the set of paths visible under the given file-type filter, or `null` when
 * everything is visible. A scan holding only one of the two types ignores the filter
 * (a saved "RAW only" must not blank a JPEG-only card).
 */
export function computeVisibleSet(paths: string[], filter: FileTypeFilter, rawExts: string[]): Set<string> | null {
  if (filter === 'all') return null;
  const isRaw = rawTester(rawExts);
  const raws = paths.filter(isRaw);
  if (raws.length === 0 || raws.length === paths.length) return null;

  if (filter === 'raw') return new Set(raws);
  if (filter === 'nonraw') return new Set(paths.filter((p) => !isRaw(p)));

  // pairs / pairsJpeg: hide the other half of each pair.
  const showRaw = filter === 'pairs';
  const shownKeys = new Set(paths.filter((p) => isRaw(p) === showRaw).map(frameKey));
  return new Set(paths.filter((p) => isRaw(p) === showRaw || !shownKeys.has(frameKey(p))));
}

/**
 * For the pair modes, the hidden half of each pair keyed by the path whose tile stands
 * for it (RAW -> its JPEGs for 'pairs', JPEG -> its RAW for 'pairsJpeg'). Empty for the
 * other modes. Only tiles that actually have a companion appear.
 */
export function pairCompanions(paths: string[], rawExts: string[], filter: FileTypeFilter): Map<string, string[]> {
  const pairs = new Map<string, string[]>();
  if (filter !== 'pairs' && filter !== 'pairsJpeg') return pairs;
  const isRaw = rawTester(rawExts);
  const showRaw = filter === 'pairs';
  const shownByKey = new Map<string, string[]>();
  paths
    .filter((p) => isRaw(p) === showRaw)
    .forEach((p) => shownByKey.set(frameKey(p), [...(shownByKey.get(frameKey(p)) ?? []), p]));
  paths.forEach((p) => {
    if (isRaw(p) === showRaw) return;
    (shownByKey.get(frameKey(p)) ?? []).forEach((shown) => pairs.set(shown, [...(pairs.get(shown) ?? []), p]));
  });
  return pairs;
}

export const LABEL_COLORS = ['red', 'yellow', 'green', 'blue', 'purple', 'gray'] as const;

export const COLOR_HEX: Record<string, string> = {
  red: '#ef4444',
  yellow: '#eab308',
  green: '#22c55e',
  blue: '#3b82f6',
  purple: '#a855f7',
  gray: '#9ca3af',
};

export interface ImportFilterState {
  fileType: FileTypeFilter;
  rating: number; // 0 = all, -1 = unrated only, 1-5 = N stars and up
  colors: string[]; // color names + optional 'none'
}

/**
 * Combined visibility filter: file type + rating + color label. Returns the set of
 * visible paths, or `null` when no filter is active (everything visible).
 */
export function computeVisible(
  paths: string[],
  filter: ImportFilterState,
  ratings: Record<string, number>,
  colorOf: Record<string, string>,
  rawExts: string[],
): Set<string> | null {
  const fileVisible = computeVisibleSet(paths, filter.fileType, rawExts);
  const ratingActive = filter.rating !== 0;
  const colorActive = filter.colors.length > 0;
  if (!fileVisible && !ratingActive && !colorActive) return null;

  return new Set(
    paths.filter((p) => {
      if (fileVisible && !fileVisible.has(p)) return false;
      if (ratingActive) {
        const r = ratings[p] || 0;
        if (filter.rating === -1 ? r !== 0 : r < filter.rating) return false;
      }
      if (colorActive) {
        const c = colorOf[p];
        const matches = (c && filter.colors.includes(c)) || (!c && filter.colors.includes('none'));
        if (!matches) return false;
      }
      return true;
    }),
  );
}
