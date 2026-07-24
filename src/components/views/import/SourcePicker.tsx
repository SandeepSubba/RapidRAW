import { useEffect } from 'react';
import { HardDrive, FolderOpen, Film, RefreshCw, Loader2 } from 'lucide-react';
import { useImportStore } from '../../../store/useImportStore';
import { useScannerStore } from '../../../store/useScannerStore';
import { useSdImportActions } from '../../../hooks/useSdImportActions';
import { detectScanner } from './ScannerPane';
import Button from '../../ui/Button';

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let v = bytes;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 && i > 0 ? 1 : 0)} ${units[i]}`;
}

function ScannerSourceCard() {
  const detect = useScannerStore((s) => s.detect);
  const device = useScannerStore((s) => s.device);
  const setImport = useImportStore((s) => s.setImport);

  useEffect(() => {
    if (detect === 'unknown') detectScanner();
  }, [detect]);

  const ready = detect === 'ready' && device;
  return (
    <button
      onClick={() => (ready ? setImport({ stage: 'scanner' }) : detectScanner())}
      className="w-full flex items-center gap-3 p-3 rounded-lg bg-surface/60 hover:bg-surface transition-colors text-left"
    >
      {detect === 'detecting' ? (
        <Loader2 size={22} className="text-text-secondary animate-spin" />
      ) : (
        <Film size={22} className={ready ? 'text-accent' : 'text-text-secondary'} />
      )}
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-text-primary font-medium truncate">{device?.model || 'Film scanner'}</span>
          {ready && (
            <span className="text-[10px] uppercase tracking-wide bg-accent/20 text-accent rounded px-1.5 py-0.5">ready</span>
          )}
        </div>
        <span className="text-xs text-text-secondary truncate block">
          {detect === 'no-scanimage' && 'SANE not installed — brew install sane-backends, then click to retry'}
          {detect === 'no-scanner' && 'No scanner found — connect it, power on, click to retry'}
          {detect === 'detecting' && 'Looking for scanners…'}
          {(detect === 'ready' || detect === 'unknown') && (device?.name || 'Scan film directly into your library')}
        </span>
      </div>
    </button>
  );
}

export default function SourcePicker() {
  const drives = useImportStore((s) => s.drives);
  const { detectDrives, browseFolder, scanSource } = useSdImportActions();

  useEffect(() => {
    detectDrives();
  }, [detectDrives]);

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-2xl mx-auto">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-medium text-text-primary">Choose a source</h2>
          <button
            onClick={detectDrives}
            className="p-2 rounded-md text-text-secondary hover:bg-surface transition-colors"
            data-tooltip="Refresh drives"
          >
            <RefreshCw size={16} />
          </button>
        </div>

        <div className="space-y-2">
          {drives.map((drive) => (
            <button
              key={drive.path}
              onClick={() => scanSource(drive.path)}
              className="w-full flex items-center gap-3 p-3 rounded-lg bg-surface/60 hover:bg-surface transition-colors text-left"
            >
              <HardDrive size={22} className={drive.isRemovable ? 'text-accent' : 'text-text-secondary'} />
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="text-text-primary font-medium truncate">{drive.name}</span>
                  {drive.isRemovable && (
                    <span className="text-[10px] uppercase tracking-wide bg-accent/20 text-accent rounded px-1.5 py-0.5">
                      removable
                    </span>
                  )}
                </div>
                <span className="text-xs text-text-secondary truncate block">{drive.path}</span>
              </div>
              {drive.totalBytes > 0 && (
                <span className="text-xs text-text-secondary shrink-0">
                  {formatBytes(drive.availableBytes)} free / {formatBytes(drive.totalBytes)}
                </span>
              )}
            </button>
          ))}

          {drives.length === 0 && (
            <p className="text-sm text-text-secondary py-6 text-center">No drives detected. Browse to a folder instead.</p>
          )}

          <ScannerSourceCard />
        </div>

        <div className="mt-6 flex justify-center">
          <Button onClick={browseFolder}>
            <FolderOpen size={16} className="mr-2" />
            Browse for a folder…
          </Button>
        </div>
      </div>
    </div>
  );
}
