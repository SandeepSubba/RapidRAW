import { useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { ArrowDownUp, Check, ChevronDown, ChevronUp, Filter, Star, X } from 'lucide-react';
import { useImportStore, ImportSortKey } from '../../../store/useImportStore';
import { useSdImportActions } from '../../../hooks/useSdImportActions';
import { SortDirection } from '../../../components/ui/AppProperties';
import { FILE_TYPE_OPTIONS, LABEL_COLORS, COLOR_HEX, FileTypeFilter } from './importFilters';

const SORT_OPTIONS: { key: ImportSortKey; label: string; needsScores?: boolean }[] = [
  { key: 'name', label: 'File name' },
  { key: 'date', label: 'Capture date' },
  { key: 'quality', label: 'Quality score', needsScores: true },
];

export default function ImportFilterBar() {
  const [open, setOpen] = useState(false);
  const { fileTypeFilter, filterRating, filterColors, sortKey, sortOrder, scoresReady } = useImportStore(
    useShallow((s) => ({
      fileTypeFilter: s.fileTypeFilter,
      filterRating: s.filterRating,
      filterColors: s.filterColors,
      sortKey: s.sortKey,
      sortOrder: s.sortOrder,
      scoresReady: s.scoresReady,
    })),
  );
  const { setFileTypeFilter, setFilterRating, toggleFilterColor, setSortKey, setSortOrder } = useSdImportActions();

  const active = fileTypeFilter !== 'all' || filterRating !== 0 || filterColors.length > 0;
  const asc = sortOrder === SortDirection.Ascending;

  const clearAll = () => {
    setFileTypeFilter('all');
    setFilterRating(0);
    filterColors.forEach((c) => toggleFilterColor(c));
  };

  return (
    <div className="relative">
      <button
        onClick={() => setOpen((v) => !v)}
        className={`flex items-center gap-1 px-2.5 py-1 text-sm rounded-md border ${
          active ? 'border-accent text-accent' : 'border-surface text-text-secondary hover:bg-surface'
        }`}
        data-tooltip="Sort and filter what's shown"
      >
        <Filter size={14} /> Sort &amp; filter{active ? ' •' : ''}
      </button>

      {open && (
        <>
          <div className="fixed inset-0 z-40" onClick={() => setOpen(false)} />
          <div className="absolute left-0 top-full mt-1 z-50 w-64 rounded-lg border border-surface bg-bg-secondary shadow-xl p-3 space-y-4 text-sm">
            {/* SORT */}
            <div>
              <div className="flex items-center justify-between mb-1.5">
                <span className="text-[11px] uppercase tracking-wide text-text-secondary flex items-center gap-1">
                  <ArrowDownUp size={12} /> Sort by
                </span>
                <button
                  onClick={() => setSortOrder(asc ? SortDirection.Descending : SortDirection.Ascending)}
                  data-tooltip={asc ? 'Switch to descending' : 'Switch to ascending'}
                  className="flex items-center gap-1 text-text-secondary hover:text-text-primary"
                >
                  {asc ? <ChevronUp size={15} /> : <ChevronDown size={15} />}
                  <span className="text-xs">{asc ? 'Ascending' : 'Descending'}</span>
                </button>
              </div>
              {SORT_OPTIONS.map((o) => {
                const disabled = o.needsScores && !scoresReady;
                const selected = sortKey === o.key;
                return (
                  <button
                    key={o.key}
                    disabled={disabled}
                    onClick={() => setSortKey(o.key)}
                    title={disabled ? 'Available after scoring (Auto-select best)' : undefined}
                    className={`flex w-full items-center justify-between px-2 py-1 rounded ${
                      disabled
                        ? 'text-text-secondary/40 cursor-not-allowed'
                        : selected
                          ? 'bg-surface text-text-primary'
                          : 'text-text-secondary hover:bg-surface'
                    }`}
                  >
                    {o.label}
                    {selected && !disabled && <Check size={14} />}
                  </button>
                );
              })}
            </div>

            {/* RATING */}
            <div>
              <div className="text-[11px] uppercase tracking-wide text-text-secondary mb-1.5">Filter by rating</div>
              <button
                onClick={() => setFilterRating(0)}
                className={`block w-full text-left px-2 py-1 rounded ${filterRating === 0 ? 'bg-surface text-text-primary' : 'text-text-secondary hover:bg-surface'}`}
              >
                Show all
              </button>
              <button
                onClick={() => setFilterRating(-1)}
                className={`block w-full text-left px-2 py-1 rounded ${filterRating === -1 ? 'bg-surface text-text-primary' : 'text-text-secondary hover:bg-surface'}`}
              >
                Unrated only
              </button>
              <div className="flex items-center gap-1 px-2 py-1">
                {[1, 2, 3, 4, 5].map((n) => (
                  <button key={n} onClick={() => setFilterRating(filterRating === n ? 0 : n)} className="p-0.5">
                    <Star
                      size={16}
                      className={n <= filterRating ? 'fill-yellow-400 text-yellow-400' : 'text-text-secondary'}
                    />
                  </button>
                ))}
                <span className="text-xs text-text-secondary ml-1">&amp; up</span>
              </div>
            </div>

            {/* FILE TYPE */}
            <div>
              <div className="text-[11px] uppercase tracking-wide text-text-secondary mb-1.5">Filter by file type</div>
              {FILE_TYPE_OPTIONS.map((o) => (
                <button
                  key={o.value}
                  onClick={() => setFileTypeFilter(o.value as FileTypeFilter)}
                  className={`block w-full text-left px-2 py-1 rounded ${fileTypeFilter === o.value ? 'bg-surface text-text-primary' : 'text-text-secondary hover:bg-surface'}`}
                >
                  {o.label}
                </button>
              ))}
            </div>

            {/* COLOR LABEL */}
            <div>
              <div className="text-[11px] uppercase tracking-wide text-text-secondary mb-1.5">Filter by color label</div>
              <div className="flex items-center gap-2 px-2">
                {LABEL_COLORS.map((c) => {
                  const on = filterColors.includes(c);
                  return (
                    <button
                      key={c}
                      onClick={() => toggleFilterColor(c)}
                      className={`w-5 h-5 rounded-full border-2 ${on ? 'border-white' : 'border-transparent'}`}
                      style={{ background: COLOR_HEX[c] }}
                      title={c}
                    />
                  );
                })}
              </div>
            </div>

            {active && (
              <button onClick={clearAll} className="flex items-center gap-1 text-xs text-text-secondary hover:text-text-primary">
                <X size={12} /> Clear filters
              </button>
            )}
          </div>
        </>
      )}
    </div>
  );
}
