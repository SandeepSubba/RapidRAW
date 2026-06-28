import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Check, Maximize2, Loader2, Image as ImageIcon, LayoutGrid, Star } from 'lucide-react';
import { Invokes } from '../../ui/AppProperties';
import { useImportStore } from '../../../store/useImportStore';
import LazyThumb from './LazyThumb';
import { RatingColor } from './RatingColor';
import { COLOR_HEX } from './importFilters';

interface ImportViewerProps {
  initialPath: string;
  collapsed: { lead: string; count: number }[];
  memberOf: Record<string, string[]>;
  leadOf: Record<string, string>;
  badgeOf: Record<string, string>;
  bestOf: Set<string>;
}

export default function ImportViewer({ initialPath, collapsed, memberOf, leadOf, badgeOf, bestOf }: ImportViewerProps) {
  const keptPaths = useImportStore((s) => s.keptPaths);
  const alreadyImported = useImportStore((s) => s.alreadyImported);
  const ratings = useImportStore((s) => s.ratings);
  const colors = useImportStore((s) => s.colors);
  const toggleKeep = useImportStore((s) => s.toggleKeep);

  // The active photo lives in the store so the global rating/label shortcuts act on it.
  const activePath = useImportStore((s) => s.activePath) ?? initialPath;
  const setActivePath = useCallback((p: string) => useImportStore.getState().setImport({ activePath: p }), []);
  useEffect(() => {
    if (!useImportStore.getState().activePath) useImportStore.getState().setImport({ activePath: initialPath });
  }, [initialPath]);

  const [mode, setMode] = useState<'compare' | 'single'>('single');

  const currentLead = leadOf[activePath] ?? activePath;
  const members = memberOf[currentLead] ?? [activePath];
  const groupIndex = collapsed.findIndex((c) => c.lead === currentLead);

  const goGroup = useCallback(
    (delta: number) => {
      const ni = groupIndex + delta;
      if (groupIndex !== -1 && ni >= 0 && ni < collapsed.length) setActivePath(collapsed[ni].lead);
    },
    [groupIndex, collapsed],
  );
  const goMember = useCallback(
    (delta: number) => {
      const mi = members.indexOf(activePath);
      const ni = mi + delta;
      if (ni >= 0 && ni < members.length) setActivePath(members[ni]);
    },
    [members, activePath],
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setMode('compare');
      else if (e.key === 'ArrowRight') goGroup(1);
      else if (e.key === 'ArrowLeft') goGroup(-1);
      else if (e.key === 'ArrowDown') {
        e.preventDefault();
        goMember(1);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        goMember(-1);
      } else if (e.key === ' ') {
        e.preventDefault();
        toggleKeep(activePath);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [goGroup, goMember, toggleKeep, activePath]);

  const cols = members.length <= 1 ? 1 : members.length <= 4 ? 2 : members.length <= 9 ? 3 : 4;
  const activeKept = keptPaths.has(activePath);

  // Compare-mode zoom/pan, shared across all panes so you zoom into the same spot on every
  // frame at once. Mouse wheel zooms; drag pans when zoomed in.
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const compareRef = useRef<HTMLDivElement>(null);

  // Reset zoom when the group or view mode changes.
  useEffect(() => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  }, [currentLead, mode]);
  // Snap pan back to centre once fully zoomed out.
  useEffect(() => {
    if (zoom === 1) setPan({ x: 0, y: 0 });
  }, [zoom]);
  // Wheel-to-zoom (registered non-passive so we can preventDefault the page scroll).
  useEffect(() => {
    const el = compareRef.current;
    if (!el || mode !== 'compare') return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      setZoom((z) => Math.min(6, Math.max(1, z * Math.exp(-e.deltaY * 0.0015))));
    };
    el.addEventListener('wheel', onWheel, { passive: false });
    return () => el.removeEventListener('wheel', onWheel);
  }, [mode]);

  return (
    <div className="flex-1 min-h-0 flex">
      <FilmStrip
        items={collapsed.map((c) => c.lead)}
        activePath={currentLead}
        onPick={setActivePath}
        countByPath={Object.fromEntries(collapsed.map((c) => [c.lead, c.count]))}
      />

      {/* this-group strip, shown beside the groups strip while in single view */}
      {mode === 'single' && members.length > 1 && (
        <GroupStrip members={members} activePath={activePath} onPick={setActivePath} />
      )}

      <div className="flex-1 min-w-0 flex flex-col bg-bg-primary">
        {/* view header: Single / Compare toggle + per-image controls */}
        <div className="shrink-0 flex items-center gap-3 px-3 py-1.5 border-b border-surface text-sm">
          <div className="flex rounded-md overflow-hidden border border-surface">
            <button
              onClick={() => setMode('single')}
              className={`flex items-center gap-1 px-2.5 py-1 ${mode === 'single' ? 'bg-accent text-button-text' : 'text-text-secondary hover:bg-surface'}`}
            >
              <ImageIcon size={14} /> Single
            </button>
            <button
              onClick={() => setMode('compare')}
              className={`flex items-center gap-1 px-2.5 py-1 ${mode === 'compare' ? 'bg-accent text-button-text' : 'text-text-secondary hover:bg-surface'}`}
            >
              <LayoutGrid size={14} /> Compare
            </button>
          </div>
          <span className="truncate text-text-primary min-w-0">{activePath.split(/[\\/]/).pop()}</span>
          {colors[activePath] && (
            <span className="w-3 h-3 rounded-full border border-white/60 shrink-0" style={{ background: COLOR_HEX[colors[activePath]] }} />
          )}
          {(ratings[activePath] || 0) > 0 && (
            <span className="flex items-center gap-px shrink-0">
              {Array.from({ length: ratings[activePath] }).map((_, i) => (
                <Star key={i} size={12} className="fill-yellow-400 text-yellow-400" />
              ))}
            </span>
          )}
          <button
            onClick={() => toggleKeep(activePath)}
            className={`ml-auto flex items-center gap-1 px-2 py-1 rounded ${activeKept ? 'bg-accent text-button-text' : 'hover:bg-surface text-text-secondary'}`}
          >
            <Check size={14} /> {activeKept ? 'Keeping' : 'Skipped'}
          </button>
        </div>

        <div className="flex-1 min-h-0 relative">
          {mode === 'single' ? (
            <SingleView path={activePath} />
          ) : (
            <div
              ref={compareRef}
              className="absolute inset-0 overflow-y-auto p-3 grid gap-3 auto-rows-fr"
              style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}
            >
              {members.map((p) => {
                const active = p === activePath;
                const kept = keptPaths.has(p);
                const disabled = alreadyImported.has(p);
                return (
                  <div
                    key={p}
                    onClick={() => setActivePath(p)}
                    onDoubleClick={() => {
                      setActivePath(p);
                      setMode('single');
                    }}
                    className={`relative flex flex-col rounded-md overflow-hidden border-2 cursor-pointer ${
                      active ? 'border-accent' : bestOf.has(p) ? 'border-green-500/70' : 'border-transparent hover:border-surface'
                    }`}
                    title={`${p.split(/[\\/]/).pop()} — double-click for single view`}
                  >
                    <div className="relative flex-1 min-h-0">
                      <ComparePane path={p} disabled={disabled} zoom={zoom} pan={pan} onPanChange={setPan} />
                      {!disabled && <RatingColor rating={ratings[p] || 0} color={colors[p]} />}
                    </div>
                    <div className="shrink-0 flex items-center gap-2 px-2 py-1 bg-surface/70 text-xs">
                      {!disabled && (
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            toggleKeep(p);
                          }}
                          className={`w-4 h-4 rounded-full flex items-center justify-center border shrink-0 ${
                            kept ? 'bg-accent border-accent text-button-text' : 'border-text-secondary text-transparent'
                          }`}
                        >
                          <Check size={11} />
                        </button>
                      )}
                      <span className="truncate text-text-secondary">{p.split(/[\\/]/).pop()}</span>
                      {badgeOf[p] && <span className="ml-auto text-text-secondary shrink-0">{badgeOf[p]}</span>}
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          {mode === 'compare' && (
            <div className="absolute bottom-2 left-1/2 -translate-x-1/2 flex items-center gap-2 text-[11px] text-text-secondary bg-bg-secondary/80 rounded px-2 py-1">
              <span className="pointer-events-none">← → groups · ↑ ↓ within group · scroll to zoom · double-click for single · space keeps</span>
              {zoom > 1 && (
                <button onClick={() => setZoom(1)} className="text-accent hover:underline">
                  {zoom.toFixed(1)}× · reset
                </button>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function GroupStrip({ members, activePath, onPick }: { members: string[]; activePath: string; onPick(p: string): void }) {
  const keptPaths = useImportStore((s) => s.keptPaths);
  const activeRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    activeRef.current?.scrollIntoView({ block: 'nearest' });
  }, [activePath]);

  return (
    <div className="w-24 shrink-0 overflow-y-auto bg-bg-secondary border-r border-surface p-1.5 space-y-1.5">
      {members.map((p) => {
        const active = p === activePath;
        const kept = keptPaths.has(p);
        return (
          <button
            key={p}
            ref={active ? activeRef : undefined}
            onClick={() => onPick(p)}
            className={`relative block w-full aspect-square rounded overflow-hidden border-2 ${
              active ? 'border-accent' : 'border-transparent hover:border-surface'
            }`}
          >
            <LazyThumb path={p} className="w-full h-full" imgClassName="w-full h-full object-cover" />
            {kept && <span className="absolute top-0.5 right-0.5 w-3 h-3 rounded-full bg-accent border border-white/60" />}
          </button>
        );
      })}
    </div>
  );
}

// A single compared frame: loads the full-resolution preview (not the low-res thumbnail) so
// the comparison is sharp, and applies the shared zoom/pan so all panes track together. Drag
// to pan when zoomed in.
function ComparePane({
  path,
  disabled,
  zoom,
  pan,
  onPanChange,
}: {
  path: string;
  disabled: boolean;
  zoom: number;
  pan: { x: number; y: number };
  onPanChange(p: { x: number; y: number }): void;
}) {
  const [url, setUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const drag = useRef<{ x: number; y: number; px: number; py: number } | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setUrl(null);
    invoke<string>(Invokes.GetImportPreview, { path })
      .then((u) => !cancelled && (setUrl(u), setLoading(false)))
      .catch(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [path]);

  const onPointerDown = (e: React.PointerEvent) => {
    if (zoom <= 1) return;
    drag.current = { x: e.clientX, y: e.clientY, px: pan.x, py: pan.y };
  };
  const onPointerMove = (e: React.PointerEvent) => {
    if (!drag.current) return;
    onPanChange({ x: drag.current.px + (e.clientX - drag.current.x), y: drag.current.py + (e.clientY - drag.current.y) });
  };
  const end = () => (drag.current = null);

  return (
    <div
      className="absolute inset-0 overflow-hidden bg-black/40 flex items-center justify-center"
      style={{ cursor: zoom > 1 ? 'grab' : 'default' }}
      onClick={(e) => zoom > 1 && e.stopPropagation()}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={end}
      onPointerLeave={end}
    >
      {loading && <Loader2 className="w-8 h-8 text-white/60 animate-spin" />}
      {url && (
        <img
          src={url}
          alt=""
          draggable={false}
          className={`max-w-full max-h-full object-contain select-none ${disabled ? 'opacity-20 grayscale' : ''}`}
          style={{
            transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
            transition: drag.current ? 'none' : 'transform 60ms',
          }}
        />
      )}
    </div>
  );
}

function SingleView({ path }: { path: string }) {
  const [url, setUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [actualSize, setActualSize] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const drag = useRef<{ x: number; y: number; left: number; top: number } | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setUrl(null);
    invoke<string>(Invokes.GetImportPreview, { path })
      .then((u) => !cancelled && (setUrl(u), setLoading(false)))
      .catch(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [path]);

  const onPointerDown = (e: React.PointerEvent) => {
    if (!actualSize || !scrollRef.current) return;
    drag.current = { x: e.clientX, y: e.clientY, left: scrollRef.current.scrollLeft, top: scrollRef.current.scrollTop };
  };
  const onPointerMove = (e: React.PointerEvent) => {
    if (!drag.current || !scrollRef.current) return;
    scrollRef.current.scrollLeft = drag.current.left - (e.clientX - drag.current.x);
    scrollRef.current.scrollTop = drag.current.top - (e.clientY - drag.current.y);
  };

  return (
    <div className="absolute inset-0 flex flex-col bg-black/40">
      <button
        onClick={() => setActualSize((v) => !v)}
        className="absolute top-2 right-2 z-10 flex items-center gap-1 px-2 py-1 rounded bg-black/50 text-white text-sm hover:bg-black/70"
      >
        <Maximize2 size={14} /> {actualSize ? '100%' : 'Fit'}
      </button>
      <div
        ref={scrollRef}
        className={`flex-1 min-h-0 ${actualSize ? 'overflow-auto cursor-grab active:cursor-grabbing' : 'overflow-hidden flex items-center justify-center'}`}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={() => (drag.current = null)}
        onPointerLeave={() => (drag.current = null)}
      >
        {loading && <Loader2 className="w-10 h-10 text-white/70 animate-spin m-auto" />}
        {url && (
          <img
            src={url}
            alt=""
            draggable={false}
            className={actualSize ? 'max-w-none select-none' : 'max-w-full max-h-full object-contain select-none'}
          />
        )}
      </div>
    </div>
  );
}

function FilmStrip({
  items,
  activePath,
  onPick,
  countByPath,
}: {
  items: string[];
  activePath: string;
  onPick(p: string): void;
  countByPath: Record<string, number>;
}) {
  const keptPaths = useImportStore((s) => s.keptPaths);
  const activeRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    activeRef.current?.scrollIntoView({ block: 'nearest' });
  }, [activePath]);

  return (
    <div className="w-24 shrink-0 overflow-y-auto bg-bg-secondary border-r border-surface p-1.5 space-y-1.5">
      {items.map((p) => {
        const active = p === activePath;
        const kept = keptPaths.has(p);
        const count = countByPath[p] ?? 1;
        return (
          <button
            key={p}
            ref={active ? activeRef : undefined}
            onClick={() => onPick(p)}
            className={`relative block w-full aspect-square rounded overflow-hidden border-2 ${
              active ? 'border-accent' : 'border-transparent hover:border-surface'
            }`}
          >
            <LazyThumb path={p} className="w-full h-full" imgClassName="w-full h-full object-cover" />
            {count > 1 && (
              <span className="absolute bottom-0.5 left-0.5 text-[9px] bg-black/70 text-white rounded px-1">{count}</span>
            )}
            {kept && <span className="absolute top-0.5 right-0.5 w-3 h-3 rounded-full bg-accent border border-white/60" />}
          </button>
        );
      })}
    </div>
  );
}
