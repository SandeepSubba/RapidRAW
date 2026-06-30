import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { toast } from 'react-toastify';
import { CullingSuggestions, Invokes, SortDirection } from '../components/ui/AppProperties';
import { DriveInfo, ImportSortKey, useImportStore } from '../store/useImportStore';
import { useUIStore } from '../store/useUIStore';
import { useSettingsStore } from '../store/useSettingsStore';
import { FileTypeFilter, ImportFilterState, computeVisible, computeVisibleSet } from '../components/views/import/importFilters';

const rawExts = () => useSettingsStore.getState().supportedTypes?.raw ?? [];

const filterState = (): ImportFilterState => {
  const s = useImportStore.getState();
  return { fileType: s.fileTypeFilter, rating: s.filterRating, colors: s.filterColors };
};
const visibleNow = () => {
  const s = useImportStore.getState();
  return computeVisible(s.scannedPaths, filterState(), s.ratings, s.colors, rawExts());
};

// Write ratings/labels onto the source sidecars so they travel with import_files.
async function writeCullingMetadata(paths: string[], ratings: Record<string, number>, colors: Record<string, string>) {
  const byRating: Record<number, string[]> = {};
  paths.forEach((p) => {
    const r = ratings[p] || 0;
    if (r > 0) (byRating[r] ||= []).push(p);
  });
  const byColor: Record<string, string[]> = {};
  paths.forEach((p) => {
    const c = colors[p];
    if (c) (byColor[c] ||= []).push(p);
  });
  await Promise.all([
    ...Object.entries(byRating).map(([r, ps]) => invoke(Invokes.SetRatingForPaths, { paths: ps, rating: Number(r) })),
    ...Object.entries(byColor).map(([c, ps]) => invoke(Invokes.SetColorLabelForPaths, { paths: ps, color: c })),
  ]);
}

// Map a Capture One–style "Similarity %" (higher = stricter) to the backend's
// perceptual-hash Hamming-distance threshold (lower = stricter).
function groupSettings(similarity: number) {
  const threshold = Math.max(1, Math.round(((100 - similarity) / 100) * 64));
  return { groupSimilar: true, filterBlurry: false, similarityThreshold: threshold, blurThreshold: 100 };
}

// Grouping invoke args from current store state — visual similarity or time-burst mode.
function groupArgs() {
  const s = useImportStore.getState();
  return { settings: groupSettings(s.similarity), mode: s.groupMode, timeGapSeconds: s.timeGapSeconds };
}

// Minimum 0–5 grade an auto-selected photo must reach to be kept.
const KEEP_MIN_GRADE = 3;

/**
 * "Auto-select best": pick exactly ONE photo per similar group — the highest-scoring
 * (the representative) — plus every ungrouped single. Duplicates and blurry shots are
 * dropped, and a group whose best shot grades below {@link KEEP_MIN_GRADE} (on the same
 * 0–5 scale shown on the badges) is dropped entirely. The user can still manually keep
 * extras afterward.
 */
export function computeDefaultKeepers(scannedPaths: string[], suggestions: CullingSuggestions): Set<string> {
  const grouped = new Set<string>();
  const kept = new Set<string>();

  // 0–5 grade, min–max normalized across grouped scores (matches the cell badges).
  const allScores: number[] = [];
  suggestions.similarGroups.forEach((g) => {
    allScores.push(g.representative.qualityScore, ...g.duplicates.map((d) => d.qualityScore));
  });
  const sMin = allScores.length ? Math.min(...allScores) : 0;
  const sMax = allScores.length ? Math.max(...allScores) : 1;
  const grade = (s: number) => Math.round(sMax > sMin ? ((s - sMin) / (sMax - sMin)) * 5 : 5);

  suggestions.similarGroups.forEach((group) => {
    grouped.add(group.representative.path);
    group.duplicates.forEach((dup) => grouped.add(dup.path)); // dropped (not the best)
    // Keep the group's best shot only if it's good enough.
    if (grade(group.representative.qualityScore) >= KEEP_MIN_GRADE) {
      kept.add(group.representative.path);
    }
  });

  const blurry = new Set(suggestions.blurryImages.map((img) => img.path));
  scannedPaths.forEach((p) => {
    if (!grouped.has(p) && !blurry.has(p)) kept.add(p); // ungrouped singles (unique shots)
  });

  return kept;
}

export function useSdImportActions() {
  const detectDrives = useCallback(async () => {
    try {
      const drives = await invoke<DriveInfo[]>(Invokes.ListSourceDrives);
      useImportStore.getState().setImport({ drives });
    } catch (err) {
      console.error('Failed to list drives:', err);
    }
  }, []);

  // Cheap re-group of the cached analysis at the current similarity. Runs live as the
  // slider moves. Does NOT change the selection.
  const regroup = useCallback(async () => {
    try {
      const suggestions = await invoke<CullingSuggestions>(Invokes.GroupForImport, groupArgs());
      useImportStore.getState().setImport({ suggestions });
    } catch (err) {
      toast.error(`Grouping failed: ${err}`);
    }
  }, []);

  // First-time grouping: analyze for grouping (hash only — fast) then group. Scoring is
  // a separate step, so this resets scoresReady.
  const analyzeThenGroup = useCallback(async () => {
    const { scannedPaths, setImport } = useImportStore.getState();
    if (scannedPaths.length === 0) return;
    setImport({ stage: 'culling', cullProgress: { current: 0, total: scannedPaths.length, stage: 'Starting…' } });
    const unlisten = await listen('sd-import-cull-progress', (event: any) => {
      useImportStore.getState().setImport({ cullProgress: event.payload });
    });
    try {
      await invoke<number>(Invokes.AnalyzeForImport, { paths: scannedPaths });
      const suggestions = await invoke<CullingSuggestions>(Invokes.GroupForImport, groupArgs());
      setImport({ suggestions, analysisReady: true, scoresReady: false, cullProgress: null, stage: 'review' });
    } catch (err) {
      setImport({ error: String(err), cullProgress: null, stage: 'review' });
      toast.error(`Grouping failed: ${err}`);
    } finally {
      unlisten();
    }
  }, []);

  // The "AI score" step: score the photos, then re-group so each group is ranked with a
  // best-of-group pick and the score badges appear. Separate from grouping for speed.
  const scoreImages = useCallback(async () => {
    const { scannedPaths, analysisReady, setImport } = useImportStore.getState();
    if (scannedPaths.length === 0) return;
    // Scoring needs the grouping analysis cached; run it first if needed.
    if (!analysisReady) await analyzeThenGroup();
    setImport({ stage: 'scoring', cullProgress: { current: 0, total: scannedPaths.length, stage: 'Scoring…' } });
    const unlisten = await listen('sd-import-score-progress', (event: any) => {
      useImportStore.getState().setImport({ cullProgress: event.payload });
    });
    try {
      const s = useImportStore.getState();
      await invoke<number>(Invokes.ScoreForImport, {
        groupSettings: groupSettings(s.similarity),
        mode: s.groupMode,
        timeGapSeconds: s.timeGapSeconds,
        personalize: s.personalizeSelection,
      });
      const suggestions = await invoke<CullingSuggestions>(Invokes.GroupForImport, groupArgs());
      setImport({ suggestions, scoresReady: true, cullProgress: null, stage: 'review' });
    } catch (err) {
      setImport({ error: String(err), cullProgress: null, stage: 'review' });
      toast.error(`Scoring failed: ${err}`);
    } finally {
      unlisten();
    }
  }, [analyzeThenGroup]);

  const setEnableGroups = useCallback(
    (on: boolean) => {
      const { analysisReady, setImport } = useImportStore.getState();
      setImport({ enableGroups: on });
      if (on) {
        if (analysisReady) regroup();
        else analyzeThenGroup();
      } else {
        setImport({ suggestions: null });
      }
    },
    [regroup, analyzeThenGroup],
  );

  const setSimilarity = useCallback(
    (pct: number) => {
      const { enableGroups, analysisReady, setImport } = useImportStore.getState();
      setImport({ similarity: pct });
      if (enableGroups && analysisReady) regroup();
    },
    [regroup],
  );

  const setGroupMode = useCallback(
    (mode: 'visual' | 'time') => {
      const { enableGroups, analysisReady, setImport } = useImportStore.getState();
      setImport({ groupMode: mode, scoresReady: false });
      if (enableGroups && analysisReady) regroup();
    },
    [regroup],
  );

  const setTimeGap = useCallback(
    (seconds: number) => {
      const { enableGroups, analysisReady, groupMode, setImport } = useImportStore.getState();
      setImport({ timeGapSeconds: seconds });
      if (enableGroups && analysisReady && groupMode === 'time') regroup();
    },
    [regroup],
  );

  // --- selection helpers ------------------------------------------------------------
  const selectAll = useCallback(() => {
    const { scannedPaths, alreadyImported, setImport } = useImportStore.getState();
    const visible = visibleNow();
    setImport({
      keptPaths: new Set(scannedPaths.filter((p) => !alreadyImported.has(p) && (!visible || visible.has(p)))),
    });
  }, []);

  // The file-type filter governs what gets imported, so dropping a type also unselects
  // those photos. Rating/colour filters are view-only and don't change the selection.
  const setFileTypeFilter = useCallback((filter: FileTypeFilter) => {
    const { scannedPaths, keptPaths, setImport } = useImportStore.getState();
    const visible = computeVisibleSet(scannedPaths, filter, rawExts());
    const keptVisible = visible ? new Set([...keptPaths].filter((p) => visible.has(p))) : keptPaths;
    setImport({ fileTypeFilter: filter, keptPaths: keptVisible });
  }, []);

  const setActivePath = useCallback((p: string | null) => useImportStore.getState().setImport({ activePath: p }), []);
  const setFilterRating = useCallback((r: number) => useImportStore.getState().setImport({ filterRating: r }), []);
  const setSortKey = useCallback((key: ImportSortKey) => useImportStore.getState().setImport({ sortKey: key }), []);
  const setSortOrder = useCallback((order: SortDirection) => useImportStore.getState().setImport({ sortOrder: order }), []);
  const toggleFilterColor = useCallback((c: string) => {
    const { filterColors, setImport } = useImportStore.getState();
    setImport({ filterColors: filterColors.includes(c) ? filterColors.filter((x) => x !== c) : [...filterColors, c] });
  }, []);

  // --- culling metadata (act on the active photo) -----------------------------------
  const rateActive = useCallback((rating: number) => {
    const { activePath, ratings, setImport } = useImportStore.getState();
    if (!activePath) return;
    const final = rating === (ratings[activePath] || 0) ? 0 : rating; // press same star to clear
    setImport({ ratings: { ...ratings, [activePath]: final } });
  }, []);

  const colorActive = useCallback((color: string | null) => {
    const { activePath, colors, setImport } = useImportStore.getState();
    if (!activePath) return;
    const next = { ...colors };
    if (!color || colors[activePath] === color) delete next[activePath];
    else next[activePath] = color;
    setImport({ colors: next });
  }, []);

  const keepActive = useCallback(() => {
    const { activePath, keptPaths, alreadyImported, setImport } = useImportStore.getState();
    if (!activePath || alreadyImported.has(activePath)) return;
    setImport({ keptPaths: new Set(keptPaths).add(activePath) });
  }, []);

  const skipActive = useCallback(() => {
    const { activePath, keptPaths, setImport } = useImportStore.getState();
    if (!activePath) return;
    const next = new Set(keptPaths);
    next.delete(activePath);
    setImport({ keptPaths: next });
  }, []);

  const selectNone = useCallback(() => {
    useImportStore.getState().setImport({ keptPaths: new Set() });
  }, []);

  // Auto-select best = run the full analysis (group + score) if it hasn't been done, then
  // keep the single best of each group + all ungrouped singles. This is the one-click
  // entry point — it no longer requires the user to enable grouping first. (Previously it
  // only scored when groups already existed, so a first click with no analysis fell
  // through to "select everything".)
  const autoSelectBest = useCallback(async () => {
    if (!useImportStore.getState().scoresReady) {
      useImportStore.getState().setImport({ enableGroups: true }); // reflect that grouping is now active
      await scoreImages(); // analyzes + groups + scores as needed, sets scoresReady + suggestions
    }
    const { scannedPaths, suggestions, alreadyImported, setImport } = useImportStore.getState();
    if (!suggestions) return; // analysis failed; leave the current selection untouched
    const kept = computeDefaultKeepers(scannedPaths, suggestions);
    alreadyImported.forEach((p) => kept.delete(p));
    setImport({ keptPaths: kept });
  }, [scoreImages]);

  const scanSource = useCallback(async (path: string) => {
    const { setImport } = useImportStore.getState();
    setImport({ sourcePath: path, stage: 'scanning', scannedPaths: [], captureTimes: {}, suggestions: null, error: null });
    try {
      const scannedPaths = await invoke<string[]>(Invokes.ScanSourceImages, { path });
      if (scannedPaths.length === 0) {
        toast.info('No supported images found in that location.');
        setImport({ stage: 'source' });
        return;
      }
      // Show all photos with NOTHING selected by default. Grouping is opt-in (the "Group
      // similar" toggle); selection is explicit (checkboxes / Select all / Auto-select).
      setImport({
        scannedPaths,
        keptPaths: new Set(),
        suggestions: null,
        enableGroups: false,
        analysisReady: false,
        scoresReady: false,
        stage: 'review',
      });
      // Best-effort, in the background: load EXIF capture dates so "sort by date" works.
      // The grid renders immediately; date sorting falls back to filename until these land.
      invoke<Record<string, number>>(Invokes.GetCaptureTimes, { paths: scannedPaths })
        .then((times) => {
          // Ignore if a newer scan has started in the meantime.
          if (useImportStore.getState().scannedPaths === scannedPaths) setImport({ captureTimes: times });
        })
        .catch(() => {});
    } catch (err) {
      setImport({ error: String(err), stage: 'source' });
      toast.error(`Scan failed: ${err}`);
    }
  }, []);

  const browseFolder = useCallback(async () => {
    try {
      const selected = await open({ directory: true, multiple: false, title: 'Select SD card or folder' });
      if (typeof selected === 'string') {
        await scanSource(selected);
      }
    } catch (err) {
      toast.error(`Failed to open folder: ${err}`);
    }
  }, [scanSource]);

  // Recompute which scanned photos already exist in the destination, removing them from
  // the keep set. No-op (and clears the set) when the option is off or no destination yet.
  const refreshAlreadyImported = useCallback(async () => {
    const { excludeImported, destinationFolder, scannedPaths, setImport } = useImportStore.getState();
    if (!excludeImported || !destinationFolder || scannedPaths.length === 0) {
      setImport({ alreadyImported: new Set() });
      return;
    }
    try {
      const existing = await invoke<string[]>(Invokes.FindExistingInDestination, {
        sourcePaths: scannedPaths,
        destinationFolder,
      });
      const set = new Set(existing);
      setImport((state) => ({
        alreadyImported: set,
        keptPaths: new Set([...state.keptPaths].filter((p) => !set.has(p))),
      }));
      if (set.size > 0) toast.info(`${set.size} photo(s) already in the destination will be skipped.`);
    } catch (err) {
      toast.error(`Failed to check for duplicates: ${err}`);
    }
  }, []);

  const setExcludeImported = useCallback(
    (on: boolean) => {
      useImportStore.getState().setImport({ excludeImported: on });
      refreshAlreadyImported();
    },
    [refreshAlreadyImported],
  );

  const pickDestination = useCallback(async () => {
    try {
      const selected = await open({ directory: true, multiple: false, title: 'Select destination folder' });
      if (typeof selected === 'string') {
        useImportStore.getState().setImport({ destinationFolder: selected });
        refreshAlreadyImported();
      }
    } catch (err) {
      toast.error(`Failed to choose destination: ${err}`);
    }
  }, [refreshAlreadyImported]);

  const startImport = useCallback(async () => {
    const { keptPaths, destinationFolder, importSettings, setImport } = useImportStore.getState();
    const sourcePaths = Array.from(keptPaths);
    if (sourcePaths.length === 0) {
      toast.warn('No images selected to import.');
      return;
    }
    if (!destinationFolder) {
      toast.warn('Choose a destination folder first.');
      return;
    }
    setImport({ stage: 'importing' });
    try {
      // Persist culling ratings/labels onto the source sidecars so they travel with the
      // imported copies (import_files copies the .rrdata sidecar). This is best-effort —
      // a read-only card / sidecar write failure must NOT block the actual import.
      const { ratings, colors } = useImportStore.getState();
      try {
        await writeCullingMetadata(sourcePaths, ratings, colors);
      } catch (metaErr) {
        console.warn('Could not write culling metadata to source sidecars (continuing import):', metaErr);
      }
      // Learn from this culling: tell the model which frame you kept vs the group-mates you
      // skipped, within each similar group. Best-effort — never blocks the import.
      const { suggestions, keptPaths, personalizeSelection } = useImportStore.getState();
      if (personalizeSelection && suggestions) {
        const kept: string[] = [];
        const skipped: string[] = [];
        suggestions.similarGroups.forEach((g) => {
          const members = [g.representative.path, ...g.duplicates.map((d) => d.path)];
          if (members.length < 2) return; // only learn from real bursts
          members.forEach((p) => (keptPaths.has(p) ? kept : skipped).push(p));
        });
        if (kept.length && skipped.length) {
          try {
            await invoke(Invokes.RecordCullPicks, { kept, skipped });
          } catch (e) {
            console.warn('Culling-preference learning skipped:', e);
          }
        }
      }
      await invoke(Invokes.ImportFiles, { destinationFolder, settings: importSettings, sourcePaths });
      // import-complete / import-error are handled by the global listeners (useProcessStore.importState).
    } catch (err) {
      setImport({ stage: 'review', error: String(err) });
      toast.error(`Failed to start import: ${err}`);
    }
  }, []);

  // Eject the source card if requested. Resolves the source's mount point from the
  // detected drives (longest matching prefix). No-op if the source isn't a known drive.
  const maybeEjectSource = useCallback(async () => {
    const { ejectAfterImport, drives, sourcePath } = useImportStore.getState();
    if (!ejectAfterImport || !sourcePath) return;
    const drive = drives
      .filter((d) => sourcePath.startsWith(d.path))
      .sort((a, b) => b.path.length - a.path.length)[0];
    if (!drive) return;
    try {
      await invoke(Invokes.EjectDrive, { mountPoint: drive.path });
      toast.success(`Ejected ${drive.name}.`);
    } catch (err) {
      toast.error(`Couldn't eject ${drive.name}: ${err}`);
    }
  }, []);

  const setPersonalize = useCallback((on: boolean) => useImportStore.getState().setImport({ personalizeSelection: on }), []);

  // Forget everything the auto-selection has learned from past culling.
  const resetLearning = useCallback(async () => {
    try {
      await invoke(Invokes.ResetCullModel);
      toast.success('Selection learning reset to defaults.');
    } catch (e) {
      toast.error(`Failed to reset learning: ${e}`);
    }
  }, []);

  const closeImporter = useCallback(() => {
    useImportStore.getState().reset();
    useUIStore.getState().setUI({ isImportViewActive: false });
  }, []);

  return {
    detectDrives,
    browseFolder,
    scanSource,
    setEnableGroups,
    setSimilarity,
    setGroupMode,
    setTimeGap,
    scoreImages,
    setFileTypeFilter,
    setActivePath,
    setFilterRating,
    toggleFilterColor,
    setSortKey,
    setSortOrder,
    rateActive,
    colorActive,
    keepActive,
    skipActive,
    selectAll,
    selectNone,
    autoSelectBest,
    pickDestination,
    refreshAlreadyImported,
    setExcludeImported,
    startImport,
    maybeEjectSource,
    setPersonalize,
    resetLearning,
    closeImporter,
  };
}
