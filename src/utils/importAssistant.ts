import { useImportStore } from '../store/useImportStore';
import { useSettingsStore } from '../store/useSettingsStore';
import { computeVisible } from '../components/views/import/importFilters';
import { CullDecision, CullTarget, ShotGroups, ShotInfo } from './assistantCull';

// The Assistant inside the import window: it sees the import grid (groups,
// the app's Q grades, ratings, labels, what is marked to import) and culls
// into the import store. Nothing touches disk here; ratings and labels are
// written to the photos when they are imported, like manual ones.

const fileNameOf = (p: string) => p.split(/[\\/]/).pop() || p;

/** True while the import window shows a reviewable card / folder scan. */
export function isImportReviewActive(isImportViewActive: boolean): boolean {
  if (!isImportViewActive) return false;
  const { stage, scannedPaths } = useImportStore.getState();
  return stage === 'review' && scannedPaths.length > 0;
}

/**
 * True while the import window is open on something the assistant can't act on
 * yet — the source picker, analysis/scoring, a film scan or the import itself.
 * The library and editor sit hidden behind it, so the assistant must not fall
 * through to them ("rate these" would rate library photos the user can't see).
 * The scanner pane has its own mode and is excluded.
 */
export function isImportWindowBusy(isImportViewActive: boolean): boolean {
  if (!isImportViewActive) return false;
  return useImportStore.getState().stage !== 'scanner' && !isImportReviewActive(isImportViewActive);
}

/** Photos the grid currently shows: file type / rating / label filters, minus already-imported. */
export function importVisiblePaths(): string[] {
  const s = useImportStore.getState();
  const rawExts = useSettingsStore.getState().supportedTypes?.raw ?? [];
  const visible = computeVisible(
    s.scannedPaths,
    { fileType: s.fileTypeFilter, rating: s.filterRating, colors: s.filterColors },
    s.ratings,
    s.colors,
    rawExts,
  );
  return s.scannedPaths.filter((p) => (!visible || visible.has(p)) && !s.alreadyImported.has(p));
}

/** What a cull covers: the selection when several photos are selected, else everything shown. */
export function importCullScope(): string[] {
  const visible = importVisiblePaths();
  const { selectedPaths } = useImportStore.getState();
  if (selectedPaths.length > 1) {
    const shown = new Set(visible);
    return selectedPaths.filter((p) => shown.has(p));
  }
  return visible;
}

/** The grid's "Q" grade: quality min-max normalized to 0-5 across scored photos. */
export function importQGrades(): Record<string, number> | undefined {
  const { suggestions, scoresReady } = useImportStore.getState();
  if (!suggestions || !scoresReady) return undefined;
  const score: Record<string, number> = {};
  suggestions.similarGroups.forEach((g) => {
    score[g.representative.path] = g.representative.qualityScore;
    g.duplicates.forEach((d) => (score[d.path] = d.qualityScore));
  });
  const vals = Object.values(score);
  if (vals.length === 0) return undefined;
  const min = Math.min(...vals);
  const max = Math.max(...vals);
  const out: Record<string, number> = {};
  Object.entries(score).forEach(([p, s]) => (out[p] = Math.round(max > min ? ((s - min) / (max - min)) * 5 : 5)));
  return out;
}

/**
 * The import's own similar-shot groups, restricted to `scope`, or null when it
 * hasn't grouped this scan (the caller then asks the backend to group).
 */
export function importShotGroups(scope: string[]): ShotGroups | null {
  const { suggestions } = useImportStore.getState();
  if (!suggestions || suggestions.similarGroups.length === 0) return null;
  const inScope = new Set(scope);
  const grouped = new Set<string>();
  const groups: ShotInfo[][] = [];
  suggestions.similarGroups.forEach((g) => {
    const members = [g.representative, ...g.duplicates]
      .filter((r) => inScope.has(r.path))
      .map((r) => ({ path: r.path, quality: r.qualityScore, sharpness: r.sharpnessMetric }));
    members.forEach((m) => grouped.add(m.path));
    if (members.length > 1) groups.push(members);
    else grouped.delete(members[0]?.path ?? '');
  });
  const singles = scope.filter((p) => !grouped.has(p)).map((path) => ({ path, quality: 0, sharpness: 0 }));
  return { groups, singles, failed: [] };
}

const IMPORT_CONTEXT_LIMIT = 600;

/** Compact description of the import grid for chat questions ("how many are marked?"). */
export function buildImportContext(): Record<string, any> {
  const s = useImportStore.getState();
  const visible = importVisiblePaths();
  const list = visible.slice(0, IMPORT_CONTEXT_LIMIT);
  const q = importQGrades();
  const groupOf: Record<string, number> = {};
  s.suggestions?.similarGroups.forEach((g, i) => {
    [g.representative, ...g.duplicates].forEach((r) => (groupOf[r.path] = i + 1));
  });
  return {
    source: s.sourcePath,
    total: visible.length,
    marked: visible.filter((p) => s.keptPaths.has(p)).length,
    selected: s.selectedPaths.length,
    truncated: visible.length > list.length,
    files: list.map((p) => ({
      file: fileNameOf(p),
      ...(groupOf[p] ? { group: groupOf[p] } : {}),
      ...(q?.[p] !== undefined ? { q: q[p] } : {}),
      ...(s.ratings[p] ? { rating: s.ratings[p] } : {}),
      ...(s.colors[p] ? { label: s.colors[p] } : {}),
      ...(s.keptPaths.has(p) ? { marked: true } : {}),
    })),
  };
}

/** Select import photos by filename (the "select" action in import mode). */
export function selectImportFiles(files: string[]): number {
  const wanted = new Set(files.map((f) => f.trim().toLowerCase()));
  const picked = importVisiblePaths().filter((p) => wanted.has(fileNameOf(p).toLowerCase()));
  if (picked.length > 0) {
    useImportStore.getState().setImport({ selectedPaths: picked, selectionAnchor: picked[0], activePath: picked[0] });
  }
  return picked.length;
}

/**
 * Culling into the import: a pick marks the photo to import, a reject unmarks
 * it; ratings and labels land in the store and are written on import.
 */
export const importCullTarget: CullTarget = {
  mode: 'import',
  stateOf(path) {
    const s = useImportStore.getState();
    return { rating: s.ratings[path] || 0, label: s.colors[path] ?? null, kept: s.keptPaths.has(path) };
  },
  apply(path: string, d: CullDecision) {
    const s = useImportStore.getState();
    const changes: string[] = [];
    const patch: Record<string, any> = {};
    if (d.rating && d.flag !== 'reject' && (s.ratings[path] || 0) !== d.rating) {
      patch.ratings = { ...s.ratings, [path]: d.rating };
      changes.push('rating');
    }
    if (d.colorLabel && s.colors[path] !== d.colorLabel) {
      patch.colors = { ...s.colors, [path]: d.colorLabel };
      changes.push('label');
    }
    if (d.flag) {
      const keep = d.flag === 'pick';
      if (s.keptPaths.has(path) !== keep) {
        const next = new Set(s.keptPaths);
        if (keep) next.add(path);
        else next.delete(path);
        patch.keptPaths = next;
        changes.push(keep ? 'marked' : 'unmarked');
      }
    }
    if (Object.keys(patch).length > 0) s.setImport(patch);
    return changes;
  },
};
