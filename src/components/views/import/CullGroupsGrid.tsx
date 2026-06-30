import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { Check, Layers, Image as ImageIcon, Droplet, Grid2x2, Eye, Sparkles } from 'lucide-react';
import { useImportStore } from '../../../store/useImportStore';
import { useSettingsStore } from '../../../store/useSettingsStore';
import { useSdImportActions } from '../../../hooks/useSdImportActions';
import { CullingSuggestions, SortDirection } from '../../../components/ui/AppProperties';
import { computeVisible } from './importFilters';
import ImportFilterBar from './ImportFilterBar';
import { RatingColor } from './RatingColor';
import LazyThumb from './LazyThumb';
import ImportViewer from './ImportViewer';

interface CellProps {
  path: string;
  kept: boolean;
  focused: boolean;
  disabled?: boolean;
  badge?: string;
  best?: boolean;
  rating?: number;
  color?: string;
  onToggleKeep(path: string): void;
  onFocus(path: string): void;
  onOpen(path: string): void;
}

function Cell({ path, kept, focused, disabled, badge, best, rating, color, onToggleKeep, onFocus, onOpen }: CellProps) {
  return (
    <div
      data-path={path}
      onClick={() => onFocus(path)}
      onDoubleClick={() => onOpen(path)}
      title={
        disabled
          ? `${path.split(/[\\/]/).pop()} — already imported`
          : `${path.split(/[\\/]/).pop()} — double-click to open the viewer`
      }
      className={`relative aspect-square rounded-md overflow-hidden cursor-pointer group ring-2 transition-all ${
        focused ? 'ring-accent' : kept ? 'ring-green-500' : 'ring-transparent hover:ring-surface'
      }`}
    >
      <LazyThumb
        path={path}
        className="w-full h-full"
        imgClassName={`w-full h-full object-cover ${disabled ? 'opacity-20 grayscale' : ''}`}
      />
      {!disabled && <RatingColor rating={rating} color={color} />}
      {disabled ? (
        <span className="absolute top-1.5 left-1.5 right-1.5 text-[9px] uppercase tracking-wide text-white/80 bg-black/70 rounded px-1 py-0.5 text-center pointer-events-none">
          already imported
        </span>
      ) : (
        /* keep toggle — the ONLY control that changes import selection */
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onToggleKeep(path);
          }}
          title={kept ? 'Selected for import — click to skip' : 'Skipped — click to keep'}
          className={`absolute top-1.5 right-1.5 w-5 h-5 rounded-full flex items-center justify-center border transition-colors ${
            kept ? 'bg-accent border-accent text-button-text' : 'bg-black/50 border-white/60 text-white/30 hover:text-white'
          }`}
        >
          <Check size={13} />
        </button>
      )}
      {badge && (
        <span className="absolute bottom-0 left-0 right-0 px-1 py-0.5 text-[10px] text-white bg-black/60 truncate text-center pointer-events-none">
          {badge}
        </span>
      )}
    </div>
  );
}

function SectionHeader({ icon, title, count }: { icon: React.ReactNode; title: string; count: number }) {
  return (
    <div className="flex items-center gap-2 mb-2 mt-1">
      {icon}
      <h3 className="text-sm font-medium text-text-primary">{title}</h3>
      <span className="bg-surface text-text-secondary rounded-full px-2 py-0.5 text-xs">{count}</span>
    </div>
  );
}

const CELL_GRID = 'grid grid-cols-4 sm:grid-cols-6 md:grid-cols-8 lg:grid-cols-10 gap-2';

export default function CullGroupsGrid({ suggestions }: { suggestions: CullingSuggestions | null }) {
  const {
    scannedPaths,
    keptPaths,
    alreadyImported,
    enableGroups,
    similarity,
    groupMode,
    timeGapSeconds,
    scoresReady,
    fileTypeFilter,
    filterRating,
    filterColors,
    ratings,
    colors,
    activePath,
    sortKey,
    sortOrder,
    captureTimes,
    toggleKeep,
  } = useImportStore(
    useShallow((s) => ({
      scannedPaths: s.scannedPaths,
      keptPaths: s.keptPaths,
      alreadyImported: s.alreadyImported,
      enableGroups: s.enableGroups,
      similarity: s.similarity,
      groupMode: s.groupMode,
      timeGapSeconds: s.timeGapSeconds,
      scoresReady: s.scoresReady,
      fileTypeFilter: s.fileTypeFilter,
      filterRating: s.filterRating,
      filterColors: s.filterColors,
      ratings: s.ratings,
      colors: s.colors,
      activePath: s.activePath,
      sortKey: s.sortKey,
      sortOrder: s.sortOrder,
      captureTimes: s.captureTimes,
      toggleKeep: s.toggleKeep,
    })),
  );
  const actions = useSdImportActions();
  const { setEnableGroups, setSimilarity, setGroupMode, setTimeGap, setActivePath, selectAll, selectNone, autoSelectBest } = actions;
  const rawExts = useSettingsStore((s) => s.supportedTypes?.raw);
  const selectedCount = keptPaths.size;

  // Combined visibility filter (file type + rating + color); null = show everything.
  const visibleSet = useMemo(
    () => computeVisible(scannedPaths, { fileType: fileTypeFilter, rating: filterRating, colors: filterColors }, ratings, colors, rawExts ?? []),
    [scannedPaths, fileTypeFilter, filterRating, filterColors, ratings, colors, rawExts],
  );
  // Hide already-imported photos entirely (not just dimmed). `alreadyImported` is only
  // populated when "Exclude already-imported" is on AND a destination is set, so this hides
  // them exactly when exclusion is active and shows everything otherwise.
  const vis = (p: string) => (!visibleSet || visibleSet.has(p)) && !alreadyImported.has(p);

  const [viewMode, setViewMode] = useState<'grid' | 'viewer'>('grid');
  const [viewerInitialPath, setViewerInitialPath] = useState<string | null>(null);
  const gridScrollRef = useRef<HTMLDivElement>(null);

  // Reading order + grouping lookups + render-ready arrays, all honouring the file-type
  // filter (hidden paths are dropped; a group that loses members collapses to a single)
  // and the user's chosen sort order (by file name or — once scored — quality).
  const { flat, groups, singles, blurry, ordered, badgeOf, bestOf, memberOf, collapsed, leadOf } = useMemo(() => {
    const fileName = (p: string) => p.split(/[\\/]/).pop() || p;
    const badge: Record<string, string> = {};
    const best = new Set<string>();
    const members: Record<string, string[]> = {};
    const lead: Record<string, string> = {};
    const visScanned = scannedPaths.filter(vis);

    // Quality scores keyed by path (only populated once an analysis exists).
    const scoreOf: Record<string, number> = {};
    if (suggestions) {
      suggestions.similarGroups.forEach((g) => {
        scoreOf[g.representative.path] = g.representative.qualityScore;
        g.duplicates.forEach((d) => (scoreOf[d.path] = d.qualityScore));
      });
    }

    // User-selected comparator, with file name as the tiebreaker. Quality/date degrade
    // gracefully to the name order when their data isn't loaded yet (before scoring, or
    // before EXIF capture times have finished loading after a scan).
    const dir = sortOrder === SortDirection.Ascending ? 1 : -1;
    const cmp = (a: string, b: string) => {
      let primary = 0;
      if (sortKey === 'quality') primary = (scoreOf[a] ?? -1) - (scoreOf[b] ?? -1);
      else if (sortKey === 'date') primary = (captureTimes[a] ?? 0) - (captureTimes[b] ?? 0);
      if (primary !== 0) return primary * dir;
      return fileName(a).localeCompare(fileName(b), undefined, { numeric: true }) * dir;
    };

    if (!suggestions) {
      const sorted = [...visScanned].sort(cmp);
      sorted.forEach((p) => {
        members[p] = [p];
        lead[p] = p;
      });
      const collapsedList = sorted.map((p) => ({ lead: p, count: 1 }));
      return { flat: sorted, groups: [] as string[][], singles: sorted, blurry: [] as string[], ordered: sorted, badgeOf: badge, bestOf: best, memberOf: members, collapsed: collapsedList, leadOf: lead };
    }

    const grouped = new Set<string>();
    suggestions.similarGroups.forEach((g) => {
      [g.representative.path, ...g.duplicates.map((d) => d.path)].forEach((p) => grouped.add(p));
    });

    // The raw quality score is an abstract 0–1 number; present it as a whole-number 0–5
    // grade (5 = best on this card) by min–max normalizing across all scored photos.
    const scoreVals = Object.values(scoreOf);
    const sMin = scoreVals.length ? Math.min(...scoreVals) : 0;
    const sMax = scoreVals.length ? Math.max(...scoreVals) : 1;
    const to5 = (s: number) => String(Math.round(sMax > sMin ? ((s - sMin) / (sMax - sMin)) * 5 : 5));

    const groupList: string[][] = [];
    const reducedSingles: string[] = [];
    suggestions.similarGroups.forEach((g) => {
      const rep = g.representative.path; // backend's best pick — anchors "best of group" + stack lead
      const visMembers = [rep, ...g.duplicates.map((d) => d.path)].filter(vis);
      if (visMembers.length >= 2) {
        if (scoresReady) best.add(rep); // "best of group" only meaningful once scored
        const sortedMembers = [...visMembers].sort(cmp);
        sortedMembers.forEach((p) => {
          // After scoring, show the 0–5 quality grade ("Q" distinguishes it from the manual
          // 1–5 star rating); before scoring, just the filename since the score is unranked.
          badge[p] = scoresReady ? `Q ${to5(scoreOf[p] ?? 0)}` : fileName(p);
          members[p] = sortedMembers;
          lead[p] = rep;
        });
        groupList.push(sortedMembers);
      } else if (visMembers.length === 1) {
        reducedSingles.push(visMembers[0]);
      }
    });
    // Order the groups themselves by their representative.
    groupList.sort((g1, g2) => cmp(lead[g1[0]], lead[g2[0]]));

    const blurrySet = new Set(suggestions.blurryImages.map((b) => b.path));
    suggestions.blurryImages.forEach((b) => (badge[b.path] = `sharp ${b.sharpnessMetric.toFixed(0)}`));
    const visBlurry = suggestions.blurryImages.filter((b) => vis(b.path)).map((b) => b.path).sort(cmp);

    const ungrouped = visScanned.filter((p) => !grouped.has(p) && !blurrySet.has(p));
    const singleList = [...ungrouped, ...reducedSingles].sort(cmp);
    [...singleList, ...visBlurry].forEach((p) => {
      members[p] = [p];
      lead[p] = p;
    });

    // Collapsed-stack order (viewer) and reading order (keyboard nav) follow the final
    // on-screen order: groups, then singles, then blurry.
    const collapsedList = [
      ...groupList.map((g) => ({ lead: lead[g[0]], count: g.length })),
      ...singleList.map((p) => ({ lead: p, count: 1 })),
      ...visBlurry.map((p) => ({ lead: p, count: 1 })),
    ];
    const orderedList = [...groupList.flat(), ...singleList, ...visBlurry];

    return { flat: visScanned, groups: groupList, singles: singleList, blurry: visBlurry, ordered: orderedList, badgeOf: badge, bestOf: best, memberOf: members, collapsed: collapsedList, leadOf: lead };
  }, [suggestions, scannedPaths, visibleSet, scoresReady, alreadyImported, sortKey, sortOrder, captureTimes]);

  const onOpen = (p: string) => {
    setViewerInitialPath(p);
    setViewMode('viewer');
  };
  const openViewer = () => {
    const start = activePath || ordered[0];
    if (start) {
      setViewerInitialPath(start);
      setViewMode('viewer');
    }
  };

  // Arrow-key navigation across the (sectioned) grid. Left/Right step through the
  // reading order; Up/Down pick the geometrically nearest cell in the row above/below
  // (robust to the responsive column count and the group/single/blurry section breaks).
  const navigate = useCallback(
    (dir: 'left' | 'right' | 'up' | 'down') => {
      const list = ordered;
      if (!list.length) return;
      const cur = activePath && list.includes(activePath) ? activePath : null;
      if (!cur) {
        setActivePath(list[0]); // first keypress just focuses the start
        return;
      }
      if (dir === 'left' || dir === 'right') {
        const i = list.indexOf(cur);
        const ni = dir === 'right' ? i + 1 : i - 1;
        if (ni >= 0 && ni < list.length) setActivePath(list[ni]);
        return;
      }
      const container = gridScrollRef.current;
      const curEl = container?.querySelector<HTMLElement>(`[data-path="${CSS.escape(cur)}"]`);
      if (!container || !curEl) return;
      const r = curEl.getBoundingClientRect();
      const cx = r.left + r.width / 2;
      let best: string | null = null;
      let bestScore = Infinity;
      for (const p of list) {
        if (p === cur) continue;
        const el = container.querySelector<HTMLElement>(`[data-path="${CSS.escape(p)}"]`);
        if (!el) continue;
        const rr = el.getBoundingClientRect();
        const isDown = rr.top > r.top + r.height * 0.5;
        const isUp = rr.bottom < r.bottom - r.height * 0.5;
        if (dir === 'down' && !isDown) continue;
        if (dir === 'up' && !isUp) continue;
        const dy = dir === 'down' ? rr.top - r.top : r.top - rr.top;
        const dx = Math.abs(rr.left + rr.width / 2 - cx);
        const score = Math.abs(dy) + dx * 3; // weight horizontal distance to stay in-column
        if (score < bestScore) {
          bestScore = score;
          best = p;
        }
      }
      if (best) setActivePath(best);
    },
    [ordered, activePath, setActivePath],
  );

  // Keep the focused cell scrolled into view as the user navigates.
  useEffect(() => {
    if (viewMode !== 'grid' || !activePath) return;
    gridScrollRef.current
      ?.querySelector<HTMLElement>(`[data-path="${CSS.escape(activePath)}"]`)
      ?.scrollIntoView({ block: 'nearest' });
  }, [activePath, viewMode]);

  useEffect(() => {
    if (viewMode !== 'grid') return;
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT' || t.isContentEditable)) return;
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      switch (e.key) {
        case 'ArrowRight':
          e.preventDefault();
          navigate('right');
          break;
        case 'ArrowLeft':
          e.preventDefault();
          navigate('left');
          break;
        case 'ArrowDown':
          e.preventDefault();
          navigate('down');
          break;
        case 'ArrowUp':
          e.preventDefault();
          navigate('up');
          break;
        case ' ':
          if (activePath) {
            e.preventDefault();
            toggleKeep(activePath);
          }
          break;
        case 'Enter':
          if (activePath) {
            e.preventDefault();
            onOpen(activePath);
          }
          break;
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [viewMode, navigate, activePath, toggleKeep]);

  const keptInGroupCount = (paths: string[]) => paths.filter((p) => keptPaths.has(p)).length;

  const cellProps = (p: string) => ({
    path: p,
    kept: keptPaths.has(p),
    focused: activePath === p,
    disabled: alreadyImported.has(p),
    rating: ratings[p] || 0,
    color: colors[p],
    onToggleKeep: toggleKeep,
    onFocus: setActivePath,
    onOpen,
  });

  return (
    <div className="flex-1 min-h-0 flex flex-col">
      {/* toolbar */}
      <div className="shrink-0 flex items-center flex-wrap gap-x-3 gap-y-2 px-4 py-2 border-b border-surface">
        <div className="flex rounded-md overflow-hidden border border-surface">
          <button
            onClick={() => setViewMode('grid')}
            className={`flex items-center gap-1 px-2.5 py-1 text-sm ${
              viewMode === 'grid' ? 'bg-accent text-button-text' : 'text-text-secondary hover:bg-surface'
            }`}
          >
            <Grid2x2 size={14} /> Grid
          </button>
          <button
            onClick={openViewer}
            className={`flex items-center gap-1 px-2.5 py-1 text-sm ${
              viewMode === 'viewer' ? 'bg-accent text-button-text' : 'text-text-secondary hover:bg-surface'
            }`}
          >
            <Eye size={14} /> Viewer
          </button>
        </div>

        {/* Group Overview: opt-in grouping, by visual similarity or by capture time */}
        <label className="flex items-center gap-1.5 text-sm text-text-primary cursor-pointer select-none">
          <input type="checkbox" checked={enableGroups} onChange={(e) => setEnableGroups(e.target.checked)} />
          <Layers size={14} className="text-accent" /> Group
        </label>
        <div className={`flex items-center gap-2 ${enableGroups ? '' : 'opacity-40'}`}>
          {/* mode: similar look vs burst-by-time */}
          <div className="flex rounded-md overflow-hidden border border-surface text-xs">
            <button
              onClick={() => setGroupMode('visual')}
              disabled={!enableGroups}
              className={`px-2 py-1 ${groupMode === 'visual' ? 'bg-accent text-button-text' : 'text-text-secondary hover:bg-surface'}`}
            >
              Similar look
            </button>
            <button
              onClick={() => setGroupMode('time')}
              disabled={!enableGroups}
              className={`px-2 py-1 ${groupMode === 'time' ? 'bg-accent text-button-text' : 'text-text-secondary hover:bg-surface'}`}
            >
              By time
            </button>
          </div>
          {groupMode === 'visual' ? (
            <>
              <span className="text-xs text-text-secondary">Similarity</span>
              <input
                type="range"
                min={0}
                max={100}
                step={1}
                value={similarity}
                disabled={!enableGroups}
                onChange={(e) => setSimilarity(Number(e.target.value))}
                className="w-24 accent-accent"
              />
              <span className="text-xs tabular-nums text-text-secondary w-9">{similarity}%</span>
            </>
          ) : (
            <>
              <span className="text-xs text-text-secondary" data-tooltip="Photos taken within this gap form a burst">
                Max gap
              </span>
              <input
                type="range"
                min={1}
                max={60}
                step={1}
                value={timeGapSeconds}
                disabled={!enableGroups}
                onChange={(e) => setTimeGap(Number(e.target.value))}
                className="w-24 accent-accent"
              />
              <span className="text-xs tabular-nums text-text-secondary w-9">{timeGapSeconds}s</span>
            </>
          )}
        </div>

        {/* filters: sort order + rating / file type / color label */}
        <ImportFilterBar />

        <div className="h-5 w-px bg-surface" />

        {/* Auto-select best = AI-score (if needed) + keep the single best of each group. */}
        <button
          onClick={autoSelectBest}
          className="flex items-center gap-1 px-2.5 py-1 text-sm rounded-md bg-accent text-button-text hover:opacity-90"
          data-tooltip="Score the photos and select the best of each group (plus all ungrouped photos)"
        >
          <Sparkles size={14} /> Auto-select best
        </button>
        <button onClick={selectAll} className="px-2.5 py-1 text-sm rounded-md hover:bg-surface text-text-secondary">
          Select all
        </button>
        <button onClick={selectNone} className="px-2.5 py-1 text-sm rounded-md hover:bg-surface text-text-secondary">
          Select none
        </button>

        <span className="text-xs text-text-secondary">
          {selectedCount} / {flat.length} shown{suggestions ? ` · ${groups.length} groups` : ''}
        </span>
        <span className="ml-auto text-xs text-text-secondary">click ✓ to keep · double-click for viewer</span>
      </div>

      {viewMode === 'viewer' && (viewerInitialPath || ordered[0]) && (
        <ImportViewer
          initialPath={(viewerInitialPath || ordered[0]) as string}
          collapsed={collapsed}
          memberOf={memberOf}
          leadOf={leadOf}
          badgeOf={badgeOf}
          bestOf={bestOf}
        />
      )}

      <div ref={gridScrollRef} className={`flex-1 overflow-y-auto px-4 pb-4 space-y-6 ${viewMode === 'viewer' ? 'hidden' : ''}`}>
        {/* Before analysis: flat grid of everything scanned (file-type filtered). */}
        {!suggestions && (
          <section>
            <div className={CELL_GRID}>
              {flat.map((p) => (
                <Cell key={p} {...cellProps(p)} badge={p.split(/[\\/]/).pop()} />
              ))}
            </div>
          </section>
        )}

        {/* Similar groups as Capture One–style stacks */}
        {suggestions && groups.length > 0 && (
          <section>
            <SectionHeader icon={<Layers size={16} className="text-accent" />} title="Similar groups" count={groups.length} />
            <div className="space-y-3">
              {groups.map((paths, idx) => (
                <div key={idx} className="rounded-lg border border-border-color bg-surface/40 p-3">
                  <div className="flex items-center gap-2 mb-2 text-xs text-text-secondary">
                    <span className="bg-accent/20 text-accent rounded px-1.5 py-0.5 font-medium">{paths.length} similar</span>
                    <span>
                      {keptInGroupCount(paths)} of {paths.length} kept
                    </span>
                  </div>
                  <div className={CELL_GRID}>
                    {paths.map((p) => (
                      <Cell key={p} {...cellProps(p)} best={bestOf.has(p)} badge={badgeOf[p]} />
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </section>
        )}

        {/* Ungrouped singles */}
        {suggestions && singles.length > 0 && (
          <section>
            <SectionHeader
              icon={<ImageIcon size={16} className="text-text-secondary" />}
              title="Other images"
              count={singles.length}
            />
            <div className={CELL_GRID}>
              {singles.map((p) => (
                <Cell key={p} {...cellProps(p)} badge={p.split(/[\\/]/).pop()} />
              ))}
            </div>
          </section>
        )}

        {/* Blurry */}
        {suggestions && blurry.length > 0 && (
          <section>
            <SectionHeader
              icon={<Droplet size={16} className="text-amber-500" />}
              title="Blurry (excluded by default)"
              count={blurry.length}
            />
            <div className={CELL_GRID}>
              {blurry.map((p) => (
                <Cell key={p} {...cellProps(p)} badge={badgeOf[p]} />
              ))}
            </div>
          </section>
        )}
      </div>
    </div>
  );
}
