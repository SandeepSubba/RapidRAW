import { useEffect, useRef } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { ArrowLeft, Loader2, HardDriveDownload } from 'lucide-react';
import { toast } from 'react-toastify';
import { useImportStore } from '../../../store/useImportStore';
import { useProcessStore } from '../../../store/useProcessStore';
import { Status } from '../../ui/ExportImportProperties';
import { useSdImportActions } from '../../../hooks/useSdImportActions';
import { useImportKeyboard } from '../../../hooks/useImportKeyboard';
import SourcePicker from './SourcePicker';
import CullGroupsGrid from './CullGroupsGrid';
import ImportReviewBar from './ImportReviewBar';

function ProgressPane({ icon, title, current, total }: { icon: React.ReactNode; title: string; current?: number; total?: number }) {
  const pct = total && total > 0 ? Math.round(((current || 0) / total) * 100) : null;
  return (
    <div className="flex-1 flex flex-col items-center justify-center gap-4">
      {icon}
      <p className="text-text-primary">{title}</p>
      {pct !== null && (
        <div className="w-64 bg-surface rounded-full h-2">
          <div className="bg-accent h-2 rounded-full transition-all" style={{ width: `${pct}%` }} />
        </div>
      )}
      {pct !== null && (
        <p className="text-xs text-text-secondary">
          {current} / {total}
        </p>
      )}
    </div>
  );
}

export default function ImportView() {
  const { stage, sourcePath, scannedPaths, cullProgress, suggestions, error } = useImportStore(
    useShallow((s) => ({
      stage: s.stage,
      sourcePath: s.sourcePath,
      scannedPaths: s.scannedPaths,
      cullProgress: s.cullProgress,
      suggestions: s.suggestions,
      error: s.error,
    })),
  );
  const importStatus = useProcessStore((s) => s.importState.status);
  const importProgress = useProcessStore((s) => s.importState.progress);
  const { closeImporter, maybeEjectSource } = useSdImportActions();
  const handledCompletion = useRef(false);
  useImportKeyboard();

  // Thumbnails are requested lazily per visible cell in CullGroupsGrid (via an
  // IntersectionObserver), so we never enqueue hundreds of decodes at once.

  // React to the (global) import lifecycle while we're importing.
  useEffect(() => {
    if (stage !== 'importing') {
      handledCompletion.current = false;
      return;
    }
    if (handledCompletion.current) return;
    if (importStatus === Status.Success) {
      handledCompletion.current = true;
      toast.success('Import complete.');
      // Eject the card (if requested) before resetting the importer state.
      maybeEjectSource().finally(() => closeImporter());
    } else if (importStatus === Status.Error) {
      handledCompletion.current = true;
      useImportStore.getState().setImport({ stage: 'review' });
    }
  }, [stage, importStatus, closeImporter, maybeEjectSource]);

  const renderBody = () => {
    switch (stage) {
      case 'source':
        return <SourcePicker />;
      case 'scanning':
        return <ProgressPane icon={<Loader2 size={40} className="text-accent animate-spin" />} title="Scanning for images…" />;
      case 'culling':
      case 'scoring':
        return (
          <ProgressPane
            icon={<Loader2 size={40} className="text-accent animate-spin" />}
            title={cullProgress?.stage || (stage === 'scoring' ? 'Scoring images…' : 'Analyzing images…')}
            current={cullProgress?.current}
            total={cullProgress?.total}
          />
        );
      case 'review':
        if (error && !suggestions) {
          return <div className="flex-1 flex items-center justify-center text-red-500 text-sm">{error}</div>;
        }
        // suggestions may be null (not analyzed yet) — the grid shows a flat view + an Analyze button.
        return <CullGroupsGrid suggestions={suggestions} />;
      case 'importing':
        return (
          <ProgressPane
            icon={<HardDriveDownload size={40} className="text-accent animate-pulse" />}
            title="Importing selected images…"
            current={importProgress?.current}
            total={importProgress?.total}
          />
        );
      default:
        return null;
    }
  };

  return (
    <div className="flex flex-col h-full w-full bg-bg-primary">
      <header className="shrink-0 flex items-center gap-3 px-4 py-3 border-b border-surface">
        <button
          onClick={closeImporter}
          className="p-2 rounded-md text-text-secondary hover:bg-surface transition-colors"
          data-tooltip="Back to library"
        >
          <ArrowLeft size={18} />
        </button>
        <div className="min-w-0">
          <h1 className="text-text-primary font-medium leading-tight">Import from SD card</h1>
          {sourcePath && (
            <p className="text-xs text-text-secondary truncate">
              {sourcePath}
              {scannedPaths.length > 0 && ` · ${scannedPaths.length} images`}
            </p>
          )}
        </div>
      </header>

      {renderBody()}

      {stage === 'review' && <ImportReviewBar />}
    </div>
  );
}
