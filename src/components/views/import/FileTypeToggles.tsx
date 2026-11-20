import { useMemo } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { useImportStore } from '../../../store/useImportStore';
import { useSettingsStore } from '../../../store/useSettingsStore';
import { useSdImportActions } from '../../../hooks/useSdImportActions';
import {
  PairView,
  fileTypeCounts,
  fileTypeFilterFrom,
  importsJpeg,
  importsRaw,
  pairViewOf,
} from './importFilters';

const PAIR_VIEWS: { value: PairView; label: string; tooltip: string }[] = [
  { value: 'both', label: 'Both', tooltip: 'Show the RAW and the JPEG of each pair as separate tiles' },
  { value: 'raw', label: 'RAW', tooltip: 'Show each pair as its RAW; keeping it imports the JPEG too' },
  { value: 'jpeg', label: 'JPEG', tooltip: 'Show each pair as its JPEG; keeping it imports the RAW too' },
];

/**
 * Culling-toolbar controls for what an import copies — RAW, JPEG or both — and, with
 * both on, whether a RAW + JPEG pair shows as two tiles or as one (its RAW or its JPEG).
 * Hidden when the scan holds only one of the two types.
 */
export default function FileTypeToggles() {
  const { scannedPaths, fileTypeFilter, lastPairView } = useImportStore(
    useShallow((s) => ({ scannedPaths: s.scannedPaths, fileTypeFilter: s.fileTypeFilter, lastPairView: s.lastPairView })),
  );
  const rawExts = useSettingsStore((s) => s.supportedTypes?.raw);
  const { setFileTypeFilter } = useSdImportActions();
  const counts = useMemo(() => fileTypeCounts(scannedPaths, rawExts ?? []), [scannedPaths, rawExts]);

  if (counts.raw === 0 || counts.jpeg === 0) return null;

  const raw = importsRaw(fileTypeFilter);
  const jpeg = importsJpeg(fileTypeFilter);
  const both = raw && jpeg;
  // With one type off there is no pair view; turning it back on restores the last one.
  const view = both ? pairViewOf(fileTypeFilter) : lastPairView;
  const set = (nextRaw: boolean, nextJpeg: boolean, nextView: PairView = view) => {
    if (!nextRaw && !nextJpeg) return; // at least one type is always imported
    setFileTypeFilter(fileTypeFilterFrom(nextRaw, nextJpeg, nextView));
  };

  const chip = (on: boolean, onlyOne: boolean) =>
    `flex items-center gap-1.5 px-2.5 py-1 text-sm rounded-md border ${
      on ? 'border-accent text-accent' : 'border-surface text-text-secondary/60 line-through hover:bg-surface'
    } ${onlyOne ? 'cursor-default' : ''}`;

  return (
    <div className="flex items-center gap-1.5">
      <span className="text-xs text-text-secondary">Import</span>
      <button
        onClick={() => set(!raw, jpeg)}
        className={chip(raw, raw && !jpeg)}
        data-tooltip={raw ? (jpeg ? 'Skip the RAW files' : 'At least one file type is imported') : 'Import the RAW files'}
        aria-pressed={raw}
      >
        RAW <span className="text-xs tabular-nums opacity-70">{counts.raw}</span>
      </button>
      <button
        onClick={() => set(raw, !jpeg)}
        className={chip(jpeg, jpeg && !raw)}
        data-tooltip={
          jpeg
            ? raw
              ? 'Skip the JPEG files (JPEG, HEIF, TIFF and other non-RAW images)'
              : 'At least one file type is imported'
            : 'Import the JPEG files (JPEG, HEIF, TIFF and other non-RAW images)'
        }
        aria-pressed={jpeg}
      >
        JPEG <span className="text-xs tabular-nums opacity-70">{counts.jpeg}</span>
      </button>

      {/* Kept in place (disabled) with one type off, so the toolbar doesn't reflow. */}
      {counts.pairs > 0 && (
        <div
          className={`flex items-center gap-1 ml-1 ${both ? '' : 'opacity-40 pointer-events-none'}`}
          data-tooltip={
            both
              ? `${counts.pairs} photos were shot as RAW + JPEG`
              : 'Turn on both RAW and JPEG to choose how pairs show'
          }
          aria-disabled={!both}
        >
          <span className="text-xs text-text-secondary">Show pairs as</span>
          <div className="flex rounded-md border border-surface overflow-hidden">
            {PAIR_VIEWS.map((o) => (
              <button
                key={o.value}
                onClick={() => set(true, true, o.value)}
                disabled={!both}
                data-tooltip={o.tooltip}
                aria-pressed={both && view === o.value}
                className={`px-2 py-1 text-xs ${
                  both && view === o.value ? 'bg-surface text-text-primary' : 'text-text-secondary hover:bg-surface'
                }`}
              >
                {o.label}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
