import { useState } from 'react';
import { FolderInput, Settings2, ArrowRight } from 'lucide-react';
import { useShallow } from 'zustand/react/shallow';
import { useImportStore } from '../../../store/useImportStore';
import { useSdImportActions } from '../../../hooks/useSdImportActions';
import Button from '../../ui/Button';
import Switch from '../../ui/Switch';

export default function ImportReviewBar() {
  const { keptCount, destinationFolder, importSettings, excludeImported, excludedCount, ejectAfterImport, setImport } =
    useImportStore(
      useShallow((s) => ({
        keptCount: s.keptPaths.size,
        destinationFolder: s.destinationFolder,
        importSettings: s.importSettings,
        excludeImported: s.excludeImported,
        excludedCount: s.alreadyImported.size,
        ejectAfterImport: s.ejectAfterImport,
        setImport: s.setImport,
      })),
    );
  const { pickDestination, startImport, setExcludeImported } = useSdImportActions();
  const [showSettings, setShowSettings] = useState(false);

  const destLabel = destinationFolder ? destinationFolder.split(/[\\/]/).pop() : 'Choose destination…';

  return (
    <div className="shrink-0 border-t border-surface bg-bg-secondary">
      {showSettings && (
        <div className="px-4 py-3 border-b border-surface grid grid-cols-1 md:grid-cols-2 gap-3 text-sm">
          <label className="flex flex-col gap-1">
            <span className="text-text-secondary text-xs">Filename template</span>
            <input
              className="bg-surface rounded-md px-2 py-1.5 text-text-primary outline-none focus:ring-1 focus:ring-accent"
              value={importSettings.filenameTemplate}
              onChange={(e) => setImport({ importSettings: { ...importSettings, filenameTemplate: e.target.value } })}
            />
          </label>
          <div className="flex flex-col gap-2 justify-center">
            <Switch
              label="Organize into date folders"
              checked={importSettings.organizeByDate}
              onChange={(v: boolean) => setImport({ importSettings: { ...importSettings, organizeByDate: v } })}
            />
            {importSettings.organizeByDate && (
              <input
                className="bg-surface rounded-md px-2 py-1.5 text-text-primary outline-none focus:ring-1 focus:ring-accent"
                value={importSettings.dateFolderFormat}
                onChange={(e) => setImport({ importSettings: { ...importSettings, dateFolderFormat: e.target.value } })}
              />
            )}
            <Switch
              label="Delete from source after import"
              checked={importSettings.deleteAfterImport}
              onChange={(v: boolean) => setImport({ importSettings: { ...importSettings, deleteAfterImport: v } })}
            />
            <Switch
              label="Exclude already-imported (skip duplicates)"
              checked={excludeImported}
              onChange={(v: boolean) => setExcludeImported(v)}
            />
            {excludeImported && (
              <span className="text-xs text-text-secondary">
                {destinationFolder
                  ? `${excludedCount} already in destination`
                  : 'Choose a destination to detect duplicates'}
              </span>
            )}
            <Switch
              label="Eject card after import"
              checked={ejectAfterImport}
              onChange={(v: boolean) => setImport({ ejectAfterImport: v })}
            />
          </div>
        </div>
      )}

      <div className="flex items-center gap-3 px-4 py-3">
        <div className="text-sm text-text-secondary">
          <span className="text-text-primary font-medium">{keptCount}</span> selected to import
        </div>

        <div className="flex-1" />

        <button
          onClick={() => setShowSettings((v) => !v)}
          className="p-2 rounded-md text-text-secondary hover:bg-surface transition-colors"
          data-tooltip="Import settings"
        >
          <Settings2 size={18} />
        </button>

        <button
          onClick={pickDestination}
          className="flex items-center gap-2 px-3 py-2 rounded-md bg-surface hover:bg-card-active transition-colors text-sm text-text-primary max-w-[40%]"
          data-tooltip={destinationFolder || 'Choose destination'}
        >
          <FolderInput size={16} className="shrink-0" />
          <span className="truncate">{destLabel}</span>
        </button>

        {/* Enabled as soon as photos are selected. If no destination is chosen yet, clicking
            opens the folder picker first, then proceeds with the import (no dead button). */}
        <Button
          onClick={async () => {
            if (!useImportStore.getState().destinationFolder) await pickDestination();
            if (useImportStore.getState().destinationFolder) startImport();
          }}
          disabled={keptCount === 0}
        >
          Import {keptCount}
          <ArrowRight size={16} className="ml-2" />
        </Button>
      </div>
    </div>
  );
}
