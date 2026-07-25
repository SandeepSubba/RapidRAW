import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Film, Loader2, RefreshCw, RotateCw, X } from 'lucide-react';
import { toast } from 'react-toastify';
import { Invokes } from '../../ui/AppProperties';
import { useImportStore } from '../../../store/useImportStore';
import { useLibraryStore } from '../../../store/useLibraryStore';
import { useScannerStore, FilmType } from '../../../store/useScannerStore';
import Button from '../../ui/Button';
import Slider from '../../ui/Slider';

const FILM_TYPES: { id: FilmType; label: string }[] = [
  { id: 'c41', label: 'Color negative' },
  { id: 'bw', label: 'B&W negative' },
  { id: 'e6', label: 'Slide (E-6)' },
];
const RESOLUTIONS = [900, 1800, 3600, 7200];

export function detectScanner() {
  const { setScanner } = useScannerStore.getState();
  setScanner({ detect: 'detecting' });
  invoke<{ scanimageInstalled: boolean; device: { name: string; model: string } | null }>(Invokes.ScanDetectScanner)
    .then((res) => {
      if (!res.scanimageInstalled) setScanner({ detect: 'no-scanimage', device: null });
      else if (!res.device) setScanner({ detect: 'no-scanner', device: null });
      else setScanner({ detect: 'ready', device: res.device });
    })
    .catch((e) => {
      toast.error(`Scanner detection failed: ${e}`);
      useScannerStore.getState().setScanner({ detect: 'no-scanner', device: null });
    });
}

export default function ScannerPane() {
  const s = useScannerStore();
  const destinationFolder = useImportStore((st) => st.destinationFolder);
  const currentFolderPath = useLibraryStore((st) => st.currentFolderPath);
  const lastScanRef = useRef<string | null>(null);
  const rerenderTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Scans land in the open library folder (albums are virtual — not writable);
  // an earlier import-flow destination, if set, wins.
  const libraryFolder = currentFolderPath && !currentFolderPath.startsWith('Album: ') ? currentFolderPath : null;
  const effectiveDest = destinationFolder || libraryFolder;

  useEffect(() => {
    const unlistens = [
      listen<{ percent: number }>('scan-progress', (e) => {
        useScannerStore.getState().setScanner({ progress: e.payload.percent });
      }),
      listen<{ path: string; fileName: string }>('scan-complete', (e) => {
        lastScanRef.current = e.payload.fileName;
        useScannerStore.getState().setScanner((st) => ({
          scanning: 'idle',
          progress: 0,
          frameCount: st.frameCount + 1,
          sessionScans: [...st.sessionScans, e.payload.fileName],
        }));
        toast.success(`Scanned ${e.payload.fileName}`);
      }),
      listen<{ message: string }>('scan-error', (e) => {
        useScannerStore.getState().setScanner({ scanning: 'idle', progress: 0, error: e.payload.message });
      }),
      listen('scan-cancelled', () => {
        useScannerStore.getState().setScanner({ scanning: 'idle', progress: 0 });
      }),
    ];
    return () => {
      unlistens.forEach((u) => u.then((f) => f()));
      if (useScannerStore.getState().scanning !== 'idle') {
        invoke(Invokes.ScanCancel).catch(() => {});
      }
    };
  }, []);

  const busy = s.scanning !== 'idle';

  const handlePreview = async () => {
    if (!s.device || busy) return;
    s.setScanner({ scanning: 'preview', progress: 0, error: null });
    try {
      const data = await invoke<string>(Invokes.ScanPreview, {
        filmType: s.filmType,
        exposureOffset: s.exposureOffset,
        contrast: s.contrast,
        rotationSteps: s.rotationSteps,
      });
      s.setScanner({ previewData: data, scanning: 'idle', progress: 0 });
    } catch (e) {
      s.setScanner({ scanning: 'idle', progress: 0, error: String(e) });
    }
  };

  // Re-render the cached preview TIFF on exposure/rotation changes — no rescan.
  const rerenderPreview = (delay: number) => {
    if (rerenderTimer.current) clearTimeout(rerenderTimer.current);
    rerenderTimer.current = setTimeout(async () => {
      try {
        const st = useScannerStore.getState();
        const data = await invoke<string>(Invokes.ScanRerenderPreview, {
          filmType: st.filmType,
          exposureOffset: st.exposureOffset,
          contrast: st.contrast,
          rotationSteps: st.rotationSteps,
        });
        st.setScanner({ previewData: data });
      } catch {
        // No cached preview (or a scan is running) — settings still apply to the next scan.
      }
    }, delay);
  };

  const handleExposureChange = (value: number) => {
    s.setScanner({ exposureOffset: value });
    if (s.previewData) rerenderPreview(250);
  };

  const handleContrastChange = (value: number) => {
    s.setScanner({ contrast: value });
    if (s.previewData) rerenderPreview(250);
  };

  const handleRotate = () => {
    s.setScanner((st) => ({ rotationSteps: (st.rotationSteps + 1) % 4 }));
    if (s.previewData) rerenderPreview(50);
  };

  const handleScan = async () => {
    if (!s.device || busy) return;
    const dest = effectiveDest;
    if (!dest) {
      s.setScanner({ error: 'Open a library folder first — scans land in the current folder.' });
      return;
    }
    const fileName = `${s.prefix || 'roll'}-${String(s.frameCount + 1).padStart(3, '0')}.tif`;
    s.setScanner({ scanning: 'scan', progress: 0, error: null });
    try {
      await invoke(Invokes.ScanStart, {
        dpi: s.dpi,
        filmType: s.filmType,
        exposureOffset: s.exposureOffset,
        contrast: s.contrast,
        rotationSteps: s.rotationSteps,
        samples: s.samples,
        irClean: s.irClean,
        destFolder: dest,
        fileName,
      });
    } catch (e) {
      s.setScanner({ scanning: 'idle', error: String(e) });
    }
  };

  const handleCancel = () => invoke(Invokes.ScanCancel).catch(() => {});

  const segBtn = (active: boolean) =>
    `px-3 py-1.5 rounded-md text-sm transition-colors ${
      active ? 'bg-accent text-button-text' : 'bg-surface/60 text-text-secondary hover:bg-surface'
    }`;

  return (
    <div className="flex-1 flex min-h-0">
      <div className="flex-1 flex flex-col items-center justify-center p-6 min-w-0">
        {s.previewData ? (
          <img src={s.previewData} alt="Scan preview" className="max-w-full max-h-full object-contain rounded-md shadow-lg" />
        ) : (
          <div className="text-text-secondary flex flex-col items-center gap-3">
            <Film size={48} className="opacity-40" />
            <p className="text-sm">Run a preview to check framing, or scan directly.</p>
          </div>
        )}
        {s.sessionScans.length > 0 && (
          <div className="shrink-0 w-full mt-4 flex flex-wrap gap-2 justify-center">
            {s.sessionScans.map((name) => (
              <span key={name} className="text-xs px-2 py-1 rounded bg-surface/60 text-text-secondary">
                {name}
              </span>
            ))}
          </div>
        )}
      </div>

      <div className="w-80 shrink-0 border-l border-surface p-5 flex flex-col gap-5 overflow-y-auto">
        <div>
          <div className="flex items-center justify-between mb-1">
            <h3 className="text-text-primary font-medium">{s.device?.model || 'Scanner'}</h3>
            <button
              onClick={detectScanner}
              className="p-1.5 rounded-md text-text-secondary hover:bg-surface transition-colors"
              data-tooltip="Re-detect scanner"
            >
              <RefreshCw size={14} />
            </button>
          </div>
          <p className="text-xs text-text-secondary">
            {s.frameCount > 0 ? `${s.frameCount} frame${s.frameCount === 1 ? '' : 's'} scanned this roll` : 'Ready'}
          </p>
        </div>

        <div>
          <p className="text-xs text-text-secondary mb-2">Roll Name</p>
          <input
            type="text"
            value={s.prefix}
            disabled={busy}
            onChange={(e) => s.setScanner({ prefix: e.target.value.replace(/[/\\:]/g, '') })}
            className="w-full px-2 py-1.5 text-sm text-text-primary bg-surface/60 border border-surface rounded-md focus:border-accent focus:outline-none"
          />
          <p className="text-[10px] text-text-secondary mt-1">
            Next frame: {`${s.prefix || 'roll'}-${String(s.frameCount + 1).padStart(3, '0')}.tif`}
          </p>
        </div>

        <div>
          <p className="text-xs text-text-secondary mb-2">Film Type</p>
          <div className="flex flex-col gap-1.5">
            {FILM_TYPES.map((f) => (
              <button key={f.id} disabled={busy} onClick={() => s.setScanner({ filmType: f.id })} className={segBtn(s.filmType === f.id)}>
                {f.label}
              </button>
            ))}
          </div>
        </div>

        <div>
          <p className="text-xs text-text-secondary mb-2">Resolution</p>
          <div className="flex gap-1.5">
            {RESOLUTIONS.map((r) => (
              <button key={r} disabled={busy} onClick={() => s.setScanner({ dpi: r })} className={`flex-1 ${segBtn(s.dpi === r)}`}>
                {r}
              </button>
            ))}
          </div>
          <p className="text-[10px] text-text-secondary mt-1">
            {s.dpi === 7200
              ? '7200: slow, and the driver’s color calibration is off at this setting — expect color casts on color film. 3600 is the scanner’s optical sweet spot.'
              : 'dpi · 7200 is slow; 3600 suits most rolls'}
          </p>
        </div>

        <div>
          <p className="text-xs text-text-secondary mb-2">Sampling</p>
          <div className="flex gap-1.5">
            {[1, 2, 4].map((n) => (
              <button key={n} disabled={busy} onClick={() => s.setScanner({ samples: n })} className={`flex-1 ${segBtn(s.samples === n)}`}>
                {n}×
              </button>
            ))}
          </div>
          <p className="text-[10px] text-text-secondary mt-1">averages N scans · cuts shadow noise · N× scan time</p>
        </div>

        {s.filmType !== 'bw' && (
          <div>
            <p className="text-xs text-text-secondary mb-2">Dust removal (IR)</p>
            <div className="flex gap-1.5">
              {[false, true].map((on) => (
                <button
                  key={String(on)}
                  disabled={busy}
                  onClick={() => s.setScanner({ irClean: on })}
                  className={`flex-1 ${segBtn(s.irClean === on)}`}
                >
                  {on ? 'On' : 'Off'}
                </button>
              ))}
            </div>
            <p className="text-[10px] text-text-secondary mt-1">
              extra infrared pass finds dust &amp; scratches, fills them · not for silver B&amp;W film
            </p>
          </div>
        )}

        <div>
          <Slider
            label="Exposure"
            min={-3}
            max={3}
            step={0.1}
            value={s.exposureOffset}
            defaultValue={0}
            onChange={(e: any) => handleExposureChange(parseFloat(e.target.value))}
          />
          <p className="text-[10px] text-text-secondary mt-1">
            Saved as editor adjustments — re-tune them any time in the edit view
          </p>
        </div>

        <Slider
          label="Contrast"
          min={-100}
          max={100}
          step={1}
          value={s.contrast}
          defaultValue={0}
          onChange={(e: any) => handleContrastChange(parseFloat(e.target.value))}
        />

        <div>
          <p className="text-xs text-text-secondary mb-2">Orientation</p>
          <button
            onClick={handleRotate}
            disabled={busy}
            className="w-full flex items-center justify-center gap-2 px-3 py-1.5 rounded-md bg-surface/60 hover:bg-surface transition-colors text-sm text-text-primary"
          >
            <RotateCw size={14} />
            Rotate 90° {s.rotationSteps > 0 && `(${s.rotationSteps * 90}°)`}
          </button>
        </div>

        {s.error && <p className="text-xs text-red-500">{s.error}</p>}

        {busy ? (
          <div className="flex flex-col gap-2">
            <div className="flex items-center gap-2 text-sm text-text-primary">
              <Loader2 size={16} className="animate-spin text-accent" />
              {s.scanning === 'preview' ? 'Previewing…' : 'Scanning…'} {s.progress}%
            </div>
            <div className="w-full bg-surface rounded-full h-2">
              <div className="bg-accent h-2 rounded-full transition-all" style={{ width: `${s.progress}%` }} />
            </div>
            <Button onClick={handleCancel} className="w-full">
              <X size={14} className="mr-1" /> Cancel
            </Button>
          </div>
        ) : (
          <div className="flex flex-col gap-2">
            {lastScanRef.current && (
              <p className="text-xs text-accent">Advance the holder to the next frame, then Scan.</p>
            )}
            <Button onClick={handlePreview} disabled={!s.device} className="w-full">
              Preview
            </Button>
            <Button onClick={handleScan} disabled={!s.device} className="w-full bg-accent text-button-text hover:bg-accent/90">
              Scan frame
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
