export type FileTypeFilter = 'all' | 'raw' | 'nonraw' | 'preferRaw';

const extOf = (p: string) => p.split('?')[0].split(/[\\/]/).pop()?.split('.').pop()?.toLowerCase() || '';

const stemKey = (p: string) => {
  const noVc = p.split('?')[0];
  const slash = Math.max(noVc.lastIndexOf('/'), noVc.lastIndexOf('\\'));
  const dir = noVc.slice(0, slash);
  const name = noVc.slice(slash + 1);
  const dot = name.lastIndexOf('.');
  return `${dir}/${dot === -1 ? name : name.slice(0, dot)}`.toLowerCase();
};

/**
 * Returns the set of paths visible under the given file-type filter, or `null` when
 * everything is visible (filter = 'all'). "preferRaw" hides a non-raw when a raw of the
 * same name exists (RAW + JPEG pairs collapse to the RAW).
 */
export function computeVisibleSet(paths: string[], filter: FileTypeFilter, rawExts: string[]): Set<string> | null {
  if (filter === 'all') return null;
  const rawSet = new Set(rawExts.map((e) => e.toLowerCase()));
  const isRaw = (p: string) => rawSet.has(extOf(p));

  if (filter === 'raw') return new Set(paths.filter(isRaw));
  if (filter === 'nonraw') return new Set(paths.filter((p) => !isRaw(p)));

  // preferRaw
  const rawStems = new Set(paths.filter(isRaw).map(stemKey));
  return new Set(paths.filter((p) => isRaw(p) || !rawStems.has(stemKey(p))));
}

export const FILE_TYPE_OPTIONS: { value: FileTypeFilter; label: string }[] = [
  { value: 'all', label: 'All types' },
  { value: 'raw', label: 'RAW only' },
  { value: 'nonraw', label: 'JPEG only' },
  { value: 'preferRaw', label: 'Prefer RAW' },
];

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
