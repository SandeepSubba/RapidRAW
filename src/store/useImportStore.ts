import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { CullingSettings, CullingSuggestions } from '../components/ui/AppProperties';
import { FileTypeFilter } from '../components/views/import/importFilters';

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
}

export type ImportStage = 'source' | 'scanning' | 'culling' | 'scoring' | 'review' | 'importing';

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
  analysisReady: boolean; // grouping analysis cached on the backend for the current scan
  scoresReady: boolean; // quality scoring has run (best-pick + score badges available)
  // Paths the user has chosen to import (keepers). Anything not in this set is skipped.
  keptPaths: Set<string>;
  destinationFolder: string | null;
  importSettings: ImportFileSettings;
  // When true, photos already present in the destination are greyed out and excluded.
  excludeImported: boolean;
  alreadyImported: Set<string>;
  // Eject the source card/volume once the import finishes.
  ejectAfterImport: boolean;
  // Show/hide by file type (e.g. hide JPEGs when shooting RAW+JPEG).
  fileTypeFilter: FileTypeFilter;
  // Culling metadata (per session): star rating 0-5 and color label per photo.
  ratings: Record<string, number>;
  colors: Record<string, string>;
  // The photo the keyboard rating/label shortcuts act on.
  activePath: string | null;
  // View filters by assigned metadata. rating: 0=all, -1=unrated, 1-5=N+ stars.
  filterRating: number;
  filterColors: string[];
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
  analysisReady: false,
  scoresReady: false,
  keptPaths: new Set<string>(),
  destinationFolder: null,
  importSettings: {
    filenameTemplate: '{original_filename}',
    organizeByDate: false,
    dateFolderFormat: 'YYYY/MM-DD',
    deleteAfterImport: false,
  },
  excludeImported: false,
  alreadyImported: new Set<string>(),
  ejectAfterImport: false,
  fileTypeFilter: 'all',
  ratings: {},
  colors: {},
  activePath: null,
  filterRating: 0,
  filterColors: [],
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
          filterRating: 0,
          filterColors: [],
          error: null,
        }),
    }),
    {
      name: 'rapidraw-import-prefs',
      // Persist only the user preferences across sessions — never the transient scan/
      // selection state (which includes non-serializable Sets).
      partialize: (state) => ({
        importSettings: state.importSettings,
        excludeImported: state.excludeImported,
        ejectAfterImport: state.ejectAfterImport,
        similarity: state.similarity,
        fileTypeFilter: state.fileTypeFilter,
      }),
    },
  ),
);
