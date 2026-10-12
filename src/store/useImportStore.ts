import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { CullingSettings, CullingSuggestions, SortDirection } from '../components/ui/AppProperties';
import { FileTypeFilter } from '../components/views/import/importFilters';

// What to order the culling grid by. 'date' uses EXIF capture time (loaded after a scan);
// 'quality' only becomes meaningful once scoring has run.
export type ImportSortKey = 'name' | 'date' | 'quality';

export interface DriveInfo {
  name: string;
  path: string;
  isRemovable: boolean;
  totalBytes: number;
  availableBytes: number;
}

export interface ImportFileSettings {
  filenameTemplate: string;
  organizeByDate: boolean;
  dateFolderFormat: string;
  deleteAfterImport: boolean;
  autoLensCorrection: boolean;
}

// The culling work done on one source: which photos to keep, plus ratings and colour
// labels. Saved so it survives the importer closing or the app restarting, and
// restored when that source is scanned again.
export interface SavedSelection {
  sourcePath: string;
  kept: string[];
  ratings: Record<string, number>;
  colors: Record<string, string>;
}

// What a fresh scan of `sourcePath` should start with: the saved work for that source,
// limited to photos still present, or nothing for any other source.
// ponytail: matched by source path + file path only, so a different shoot on a card that
// reuses the same file names would inherit the old picks; compare capture times if it bites.
export function restoreSelection(saved: SavedSelection | null, sourcePath: string, scannedPaths: string[]) {
  const scanned = new Set(scannedPaths);
  const mine = saved?.sourcePath === sourcePath ? saved : null;
  const present = <T>(m: Record<string, T>) => Object.fromEntries(Object.entries(m).filter(([p]) => scanned.has(p)));
  return {
    keptPaths: new Set((mine?.kept ?? []).filter((p) => scanned.has(p))),
    ratings: present(mine?.ratings ?? {}),
    colors: present(mine?.colors ?? {}),
  };
}

export type ImportStage = 'source' | 'scanner' | 'scanning' | 'culling' | 'scoring' | 'review' | 'importing';

export interface ImportProgress {
  current: number;
  total: number;
  stage: string;
}

interface ImportState {
  stage: ImportStage;
  drives: DriveInfo[];
  sourcePath: string | null;
  scannedPaths: string[];
  cullSettings: CullingSettings;
  cullProgress: ImportProgress | null;
  suggestions: CullingSuggestions | null;
  // Capture One–style "Group Overview": grouping is opt-in via a toggle + similarity %.
  enableGroups: boolean;
  similarity: number; // 0–100
  // Grouping mode: 'visual' = scenes (look + capture time); 'time' = burst-by-capture-time.
  groupMode: 'visual' | 'time';
  timeGapSeconds: number; // max gap between frames of a time burst
  sceneGapSeconds: number; // 'visual' mode: frames further apart than this never match
  analysisReady: boolean; // grouping analysis cached on the backend for the current scan
  scoresReady: boolean; // quality scoring has run (best-pick + score badges available)
  // Paths the user has chosen to import (keepers). Anything not in this set is skipped.
  keptPaths: Set<string>;
  savedSelection: SavedSelection | null;
  destinationFolder: string | null;
  importSettings: ImportFileSettings;
  // When true, photos already present in the destination are greyed out and excluded.
  excludeImported: boolean;
  alreadyImported: Set<string>;
  // Eject the source card/volume once the import finishes.
  ejectAfterImport: boolean;
  // Personalize the auto-selection by learning from your keep/skip picks over time.
  personalizeSelection: boolean;
  // Show/hide by file type (e.g. hide JPEGs when shooting RAW+JPEG).
  fileTypeFilter: FileTypeFilter;
  // Culling metadata (per session): star rating 0-5 and color label per photo.
  ratings: Record<string, number>;
  colors: Record<string, string>;
  // The photo the keyboard rating/label shortcuts act on (the active cursor cell).
  activePath: string | null;
  // Multi-selection in the grid (click / Ctrl-click toggle / Shift-click range). The active
  // cell is always included; `selectionAnchor` is the fixed end for Shift-click ranges.
  selectedPaths: string[];
  selectionAnchor: string | null;
  // View filters by assigned metadata. rating: 0=all, -1=unrated, 1-5=N+ stars.
  filterRating: number;
  filterColors: string[];
  // Display ordering for the culling grid (name/date/quality, ascending/descending).
  sortKey: ImportSortKey;
  sortOrder: SortDirection;
  // EXIF capture time (epoch seconds) per scanned path; loaded after a scan for date sorting.
  captureTimes: Record<string, number>;
  error: string | null;

  setImport: (updater: Partial<ImportState> | ((state: ImportState) => Partial<ImportState>)) => void;
  toggleKeep: (path: string) => void;
  reset: () => void;
}

const INITIAL: Omit<ImportState, 'setImport' | 'toggleKeep' | 'reset'> = {
  stage: 'source',
  drives: [],
  sourcePath: null,
  scannedPaths: [],
  cullSettings: {
    groupSimilar: true,
    similarityThreshold: 28,
    filterBlurry: true,
    blurThreshold: 100.0,
  },
  cullProgress: null,
  suggestions: null,
  enableGroups: false,
  similarity: 80,
  groupMode: 'visual',
  timeGapSeconds: 5,
  sceneGapSeconds: 20,
  analysisReady: false,
  scoresReady: false,
  keptPaths: new Set<string>(),
  savedSelection: null,
  destinationFolder: null,
  importSettings: {
    filenameTemplate: '{original_filename}',
    organizeByDate: false,
    dateFolderFormat: 'YYYY/MM-DD',
    deleteAfterImport: false,
    autoLensCorrection: true,
  },
  excludeImported: false,
  alreadyImported: new Set<string>(),
  ejectAfterImport: false,
  personalizeSelection: true,
  fileTypeFilter: 'all',
  ratings: {},
  colors: {},
  activePath: null,
  selectedPaths: [],
  selectionAnchor: null,
  filterRating: 0,
  filterColors: [],
  sortKey: 'name',
  sortOrder: SortDirection.Ascending,
  captureTimes: {},
  error: null,
};

export const useImportStore = create<ImportState>()(
  persist(
    (set) => ({
      ...INITIAL,

      setImport: (updater) => set((state) => (typeof updater === 'function' ? updater(state) : updater)),

      toggleKeep: (path) =>
        set((state) => {
          if (state.alreadyImported.has(path)) return {}; // already-imported photos can't be kept
          const next = new Set(state.keptPaths);
          if (next.has(path)) {
            next.delete(path);
          } else {
            next.add(path);
          }
          return { keptPaths: next };
        }),

      // Reset the per-session state but keep the user's saved preferences (import
      // settings, exclude/eject toggles, similarity).
      reset: () =>
        set({
          stage: 'source',
          drives: [],
          sourcePath: null,
          scannedPaths: [],
          captureTimes: {},
          cullProgress: null,
          suggestions: null,
          enableGroups: false,
          analysisReady: false,
          scoresReady: false,
          keptPaths: new Set<string>(),
          destinationFolder: null,
          alreadyImported: new Set<string>(),
          ratings: {},
          colors: {},
          activePath: null,
          selectedPaths: [],
          selectionAnchor: null,
          filterRating: 0,
          filterColors: [],
          error: null,
        }),
    }),
    {
      name: 'rapidraw-import-prefs',
      // Persist the user preferences and the saved culling work across sessions — never
      // the transient scan state (which includes non-serializable Sets).
      partialize: (state) => ({
        savedSelection: state.savedSelection,
        importSettings: state.importSettings,
        excludeImported: state.excludeImported,
        ejectAfterImport: state.ejectAfterImport,
        personalizeSelection: state.personalizeSelection,
        similarity: state.similarity,
        groupMode: state.groupMode,
        timeGapSeconds: state.timeGapSeconds,
        sceneGapSeconds: state.sceneGapSeconds,
        fileTypeFilter: state.fileTypeFilter,
        sortKey: state.sortKey,
        sortOrder: state.sortOrder,
      }),
      // Deep-merge nested prefs so defaults for newly-added fields survive when an
      // older persisted `importSettings` (saved before the field existed) lacks them.
      // Without this, zustand's shallow merge replaces `importSettings` wholesale and
      // e.g. `autoLensCorrection` comes back `undefined` instead of its default.
      merge: (persisted, current) => {
        const p = (persisted ?? {}) as Partial<ImportState>;
        return {
          ...current,
          ...p,
          importSettings: { ...current.importSettings, ...(p.importSettings ?? {}) },
        };
      },
    },
  ),
);

// Mirror the culling work into `savedSelection` while a scan is open. reset() and the
// start of a scan empty the working sets with no scan open, so they never reach here.
useImportStore.subscribe((s, prev) => {
  if (!s.sourcePath || s.scannedPaths.length === 0) return;
  if (s.keptPaths === prev.keptPaths && s.ratings === prev.ratings && s.colors === prev.colors) return;
  // Opening another source with nothing picked yet must not discard the saved work.
  const empty = s.keptPaths.size === 0 && Object.keys(s.ratings).length === 0 && Object.keys(s.colors).length === 0;
  if (empty && s.savedSelection?.sourcePath !== s.sourcePath) return;
  useImportStore.setState({
    savedSelection: { sourcePath: s.sourcePath, kept: [...s.keptPaths], ratings: s.ratings, colors: s.colors },
  });
});
