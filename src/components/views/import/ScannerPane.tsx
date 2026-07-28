import { useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ChevronDown, Film, Loader2, Pipette, RefreshCw, RotateCw, X } from 'lucide-react';
import { toast } from 'react-toastify';
import { Invokes } from '../../ui/AppProperties';
import { useImportStore } from '../../../store/useImportStore';
import { useLibraryStore } from '../../../store/useLibraryStore';
import { useScannerStore, FilmType, ScannerCaps } from '../../../store/useScannerStore';
import Button from '../../ui/Button';
import Slider from '../../ui/Slider';
import Switch from '../../ui/Switch';

const FILM_TYPES: { id: FilmType; label: string }[] = [
  { id: 'c41', label: 'Color negative' },
  { id: 'bw', label: 'B&W negative' },
  { id: 'e6', label: 'Slide (E-6)' },
];
const FALLBACK_RESOLUTIONS = [900, 1800, 3600, 7200];

type PreviewResult = { data: string; crop: [number, number, number, number] | null };

export function detectScanner() {
  const { setScanner } = useScannerStore.getState();
  setScanner({ detect: 'detecting' });
  invoke<{ scanimageInstalled: boolean; device: { name: string; model: string } | null; caps: ScannerCaps | null }>(
    Invokes.ScanDetectScanner,
  )
    .then((res) => {
      if (!res.scanimageInstalled) setScanner({ detect: 'no-scanimage', device: null, caps: null });
      else if (!res.device) setScanner({ detect: 'no-scanner', device: null, caps: null });
      else {
        // Snap the chosen resolution to something this scanner actually offers.
        const st = useScannerStore.getState();
        const res_list = res.caps?.resolutions ?? [];
        const dpi = res_list.length && !res_list.includes(st.dpi) ? res.caps!.defaultResolution : st.dpi;
        setScanner({ detect: 'ready', device: res.device, caps: res.caps, dpi });
      }
    })
    .catch((e) => {
      toast.error(`Scanner detection failed: ${e}`);
      useScannerStore.getState().setScanner({ detect: 'no-scanner', device: null, caps: null });
    });
}

// A low-but-not-lowest resolution for framing previews; the true minimum often
// carries decimation noise the negative stretch amplifies.
function previewDpi(caps: ScannerCaps | null): number {
  const r = caps?.resolutions ?? [];
  if (!r.length) return 1800;
  return r.find((x) => x >= 1500) ?? r[r.length - 1];
}

export default function ScannerPane() {
  const s = useScannerStore();
  const destinationFolder = useImportStore((st) => st.destinationFolder);
  const currentFolderPath = useLibraryStore((st) => st.currentFolderPath);
  const rerenderTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const cropBoxRef = useRef<HTMLDivElement>(null);
  const draggingEdge = useRef(false);
  const [filmInfoOpen, setFilmInfoOpen] = useState(false);

  // Drag a crop edge (no handles): map the pointer to normalized coords in the
  // preview and move just that edge, keeping a 5% minimum size.
  const applyEdgeDrag = (edge: 'top' | 'right' | 'bottom' | 'left', e: PointerEvent | any) => {
    const box = cropBoxRef.current;
    const st = useScannerStore.getState();
    if (!box || !st.cropRect) return;
    const r = box.getBoundingClientRect();
    const nx = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
    const ny = Math.min(1, Math.max(0, (e.clientY - r.top) / r.height));
    let [x, y, w, h] = st.cropRect;
    const MIN = 0.05;
    if (edge === 'left') {
      const right = x + w;
      x = Math.min(nx, right - MIN);
      w = right - x;
    } else if (edge === 'right') {
      w = Math.max(MIN, nx - x);
    } else if (edge === 'top') {
      const bot = y + h;
      y = Math.min(ny, bot - MIN);
      h = bot - y;
    } else {
      h = Math.max(MIN, ny - y);
    }
    st.setScanner({ cropRect: [x, y, w, h], cropManual: true });
  };

  const edgeHandlers = (edge: 'top' | 'right' | 'bottom' | 'left') => ({
    onPointerDown: (e: any) => {
      e.preventDefault();
      e.currentTarget.setPointerCapture(e.pointerId);
      draggingEdge.current = true;
    },
    onPointerMove: (e: any) => {
      if (draggingEdge.current) applyEdgeDrag(edge, e);
    },
    onPointerUp: (e: any) => {
      draggingEdge.current = false;
      e.currentTarget.releasePointerCapture?.(e.pointerId);
    },
  });
  // Scans land in the open library folder (albums are virtual — not writable);
  // an earlier import-flow destination, if set, wins.
  const libraryFolder = currentFolderPath && !currentFolderPath.startsWith('Album: ') ? currentFolderPath : null;
  const effectiveDest = destinationFolder || libraryFolder;
  // Scan lifecycle listeners live in App.tsx so a scan keeps running and lands
  // in the library after this pane closes; the pane just reads store state.

  const busy = s.scanning !== 'idle';
  const resolutions = s.caps?.resolutions?.length ? s.caps.resolutions : FALLBACK_RESOLUTIONS;
  const hasIR = !s.caps || !!s.caps.sourceInfrared; // fallback (no caps) assumes the 7600i has IR
  const scanDepth = Math.min(16, s.caps?.maxDepth ?? 16);

  const handlePreview = async () => {
    if (!s.device || busy) return;
    s.setScanner({ scanning: 'preview', progress: 0, error: null });
    try {
      const res = await invoke<PreviewResult>(Invokes.ScanPreview, {
        filmType: s.filmType,
        exposureOffset: s.exposureOffset,
        contrast: s.contrast,
        rotationSteps: s.rotationSteps,
        autoCrop: s.autoCrop,
        sourceVisible: s.caps?.sourceVisible ?? '',
        previewDpi: previewDpi(s.caps),
        scanDepth,
        raw: false,
        basePoint: s.basePoint,
      });
      s.setScanner({ previewData: res.data, cropRect: res.crop, cropManual: false, scanning: 'idle', progress: 0 });
    } catch (e) {
      s.setScanner({ scanning: 'idle', progress: 0, error: String(e) });
    }
  };

  // Re-render the cached preview TIFF on exposure/rotation changes — no rescan.
  // `raw` shows the un-inverted negative so the eyedropper can aim at the rebate.
  const rerenderPreview = (delay: number, raw = false) => {
    if (rerenderTimer.current) clearTimeout(rerenderTimer.current);
    rerenderTimer.current = setTimeout(async () => {
      try {
        const st = useScannerStore.getState();
        const res = await invoke<PreviewResult>(Invokes.ScanRerenderPreview, {
          filmType: st.filmType,
          exposureOffset: st.exposureOffset,
          contrast: st.contrast,
          rotationSteps: st.rotationSteps,
          autoCrop: raw ? false : st.autoCrop,
          raw,
          basePoint: st.basePoint,
        });
        // A hand-dragged crop must survive re-renders (exposure/contrast tweaks).
        st.setScanner({ previewData: res.data, cropRect: raw ? null : st.cropManual ? st.cropRect : res.crop });
      } catch {
        // No cached preview (or a scan is running) — settings still apply to the next scan.
      }
    }, delay);
  };

  // Eyedropper: arm to show the raw negative, click the orange rebate to pin the
  // film base, or clear it back to the automatic estimate.
  const toggleEyedropper = () => {
    if (!s.previewData || busy) return;
    const next = !s.eyedropping;
    s.setScanner({ eyedropping: next });
    rerenderPreview(0, next);
  };

  const handlePreviewClick = (e: any) => {
    if (!s.eyedropping) return;
    const img = e.currentTarget;
    const nx = Math.min(1, Math.max(0, e.nativeEvent.offsetX / img.clientWidth));
    const ny = Math.min(1, Math.max(0, e.nativeEvent.offsetY / img.clientHeight));
    s.setScanner({ basePoint: [nx, ny], eyedropping: false });
    rerenderPreview(0, false);
  };

  const clearBase = () => {
    s.setScanner({ basePoint: null, eyedropping: false });
    if (s.previewData) rerenderPreview(0, false);
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
    // Rotating invalidates a hand-dragged crop's geometry — re-detect.
    s.setScanner((st) => ({ rotationSteps: (st.rotationSteps + 1) % 4, cropManual: false }));
    if (s.previewData) rerenderPreview(50);
  };

  const handleAutoCrop = (on: boolean) => {
    s.setScanner({ autoCrop: on, cropRect: on ? s.cropRect : null, cropManual: false });
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
        irClean: s.irClean && hasIR,
        irSensitivity: s.irSensitivity,
        autoCrop: s.autoCrop,
        cropOverride: s.autoCrop && s.cropRect ? s.cropRect : null,
        bitDepth: s.bitDepth,
        scanDepth,
        sourceVisible: s.caps?.sourceVisible ?? '',
        sourceInfrared: s.caps?.sourceInfrared ?? null,
        scannerModel: s.device?.model ?? '',
        destFolder: dest,
        fileName,
        filmMeta: {
          filmStock: s.filmStock.trim() || null,
          iso: s.iso.trim() ? Number(s.iso) || null : null,
          camera: s.camera.trim() || null,
          lens: s.lens.trim() || null,
          notes: s.notes.trim() || null,
        },
      });
    } catch (e) {
      s.setScanner({ scanning: 'idle', error: String(e) });
    }
  };

  const handleCancel = () => invoke(Invokes.ScanCancel).catch(() => {});

  const metaInput =
    'px-2 py-1.5 text-sm text-text-primary bg-surface/60 border border-surface rounded-md focus:border-accent focus:outline-none disabled:opacity-40';

  const selectCls =
    'px-2 py-1.5 text-sm text-text-primary bg-surface/60 border border-surface rounded-md focus:border-accent focus:outline-none cursor-pointer disabled:opacity-40 [color-scheme:dark]';
  // Label-beside-control row, matching the Switch layout used for Dust removal.
  const rowLabel = 'text-sm text-text-secondary';

  return (
    <div className="flex-1 flex min-h-0">
      <div className="flex-1 flex flex-col items-center justify-center p-6 min-w-0">
        {s.previewData ? (
          <div ref={cropBoxRef} className="relative max-w-full max-h-full overflow-hidden rounded-md shadow-lg">
            <img
              src={s.previewData}
              alt="Scan preview"
              onClick={handlePreviewClick}
              className={`block max-w-full max-h-full ${s.eyedropping ? 'cursor-crosshair' : ''}`}
            />
            {s.autoCrop && s.cropRect && (
              <>
                <div
                  className="absolute pointer-events-none border border-white/70"
                  style={{
                    left: `${s.cropRect[0] * 100}%`,
                    top: `${s.cropRect[1] * 100}%`,
                    width: `${s.cropRect[2] * 100}%`,
                    height: `${s.cropRect[3] * 100}%`,
                    boxShadow: '0 0 0 9999px rgba(0,0,0,0.55)',
                  }}
                />
                {(['top', 'bottom', 'left', 'right'] as const).map((edge) => {
                  const [x, y, w, h] = s.cropRect!;
                  const vertical = edge === 'left' || edge === 'right';
                  const style: any = vertical
                    ? {
                        left: `${(edge === 'left' ? x : x + w) * 100}%`,
                        top: `${y * 100}%`,
                        height: `${h * 100}%`,
                        width: 14,
                        transform: 'translateX(-7px)',
                        cursor: 'ew-resize',
                      }
                    : {
                        left: `${x * 100}%`,
                        top: `${(edge === 'top' ? y : y + h) * 100}%`,
                        width: `${w * 100}%`,
                        height: 14,
                        transform: 'translateY(-7px)',
                        cursor: 'ns-resize',
                      };
                  return <div key={edge} {...edgeHandlers(edge)} className="absolute touch-none" style={style} />;
                })}
              </>
            )}
          </div>
        ) : (
          <div className="text-text-secondary flex flex-col items-center gap-3">
            <Film size={48} className="opacity-40" />
            <p className="text-sm">Run a preview to check framing, or scan directly.</p>
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
          <button
            onClick={() => setFilmInfoOpen((o) => !o)}
            className="w-full flex items-center justify-between text-xs text-text-secondary mb-2 hover:text-text-primary transition-colors"
          >
            <span>Film Info</span>
            <ChevronDown size={14} className={`transition-transform ${filmInfoOpen ? 'rotate-180' : ''}`} />
          </button>
          {filmInfoOpen && (
          <>
          <div className="grid grid-cols-2 gap-2">
            <input
              type="text"
              placeholder="Film stock"
              value={s.filmStock}
              disabled={busy}
              onChange={(e) => s.setScanner({ filmStock: e.target.value })}
              className={metaInput}
            />
            <input
              type="text"
              inputMode="numeric"
              placeholder="ISO"
              value={s.iso}
              disabled={busy}
              onChange={(e) => s.setScanner({ iso: e.target.value.replace(/[^0-9]/g, '') })}
              className={metaInput}
            />
            <input
              type="text"
              placeholder="Camera"
              value={s.camera}
              disabled={busy}
              onChange={(e) => s.setScanner({ camera: e.target.value })}
              className={metaInput}
            />
            <input
              type="text"
              placeholder="Lens"
              value={s.lens}
              disabled={busy}
              onChange={(e) => s.setScanner({ lens: e.target.value })}
              className={metaInput}
            />
          </div>
          <input
            type="text"
            placeholder="Notes"
            value={s.notes}
            disabled={busy}
            onChange={(e) => s.setScanner({ notes: e.target.value })}
            className={`${metaInput} w-full mt-2`}
          />
          <p className="text-[10px] text-text-secondary mt-1">Written to the scan's metadata for library sorting.</p>
          </>
          )}
        </div>

        <div className="flex items-center justify-between gap-3">
          <span className={rowLabel}>Film Type</span>
          <select
            value={s.filmType}
            disabled={busy}
            onChange={(e) => s.setScanner({ filmType: e.target.value as FilmType })}
            className={selectCls}
          >
            {FILM_TYPES.map((f) => (
              <option key={f.id} value={f.id}>
                {f.label}
              </option>
            ))}
          </select>
        </div>

        <div className="flex items-center justify-between gap-3">
          <span className={rowLabel} data-tooltip="dpi · 7200 is slow; 3600 suits most rolls">
            Resolution
          </span>
          <select
            value={s.dpi}
            disabled={busy}
            onChange={(e) => s.setScanner({ dpi: Number(e.target.value) })}
            className={selectCls}
          >
            {resolutions.map((r) => (
              <option key={r} value={r}>
                {r} dpi
              </option>
            ))}
          </select>
        </div>

        <div className="flex items-center justify-between gap-3">
          <span className={rowLabel} data-tooltip="averages N scans · cuts shadow noise · N× scan time">
            Sampling
          </span>
          <select
            value={s.samples}
            disabled={busy}
            onChange={(e) => s.setScanner({ samples: Number(e.target.value) })}
            className={selectCls}
          >
            {[1, 2, 4].map((n) => (
              <option key={n} value={n}>
                {n}×
              </option>
            ))}
          </select>
        </div>

        <div className="flex items-center justify-between gap-3">
          <span
            className={rowLabel}
            data-tooltip="16: lossless, biggest file · 12: half size, discards only bits ~100× below the scanner's noise · 10: smallest, still below noise"
          >
            Bit Depth
          </span>
          <select
            value={s.bitDepth}
            disabled={busy}
            onChange={(e) => s.setScanner({ bitDepth: Number(e.target.value) })}
            className={selectCls}
          >
            {[10, 12, 16].map((b) => (
              <option key={b} value={b}>
                {b}-bit
              </option>
            ))}
          </select>
        </div>

        {s.filmType !== 'bw' && hasIR && (
          <div>
            <Switch
              label="Dust removal (IR)"
              checked={s.irClean}
              disabled={busy}
              onChange={(on) => s.setScanner({ irClean: on })}
              tooltip="extra infrared pass finds dust & scratches, fills them · not for silver B&W film"
            />
            {s.irClean && (
              <div className="mt-3" data-tooltip="higher removes more dust but can soften fine detail">
                <Slider
                  label="IR sensitivity"
                  min={0}
                  max={100}
                  step={1}
                  value={s.irSensitivity}
                  defaultValue={50}
                  onChange={(e: any) => s.setScanner({ irSensitivity: parseInt(e.target.value, 10) })}
                />
              </div>
            )}
          </div>
        )}

        {s.filmType !== 'e6' && (
          <div>
            <p className="text-xs text-text-secondary mb-2">Film base</p>
            <div className="flex gap-2">
              <button
                onClick={toggleEyedropper}
                disabled={busy || !s.previewData}
                className={`flex-1 flex items-center justify-center gap-2 px-3 py-1.5 rounded-md text-sm transition-colors disabled:opacity-40 ${
                  s.eyedropping ? 'bg-accent text-button-text' : 'bg-surface/60 hover:bg-surface text-text-primary'
                }`}
                data-tooltip="Click the clear orange film edge to neutralise the mask precisely"
              >
                <Pipette size={14} />
                {s.eyedropping ? 'Click the rebate…' : s.basePoint ? 'Base set' : 'Sample base'}
              </button>
              {s.basePoint && !s.eyedropping && (
                <button
                  onClick={clearBase}
                  disabled={busy}
                  className="px-3 py-1.5 rounded-md bg-surface/60 hover:bg-surface text-sm text-text-secondary"
                  data-tooltip="Back to automatic base"
                >
                  <X size={14} />
                </button>
              )}
            </div>
          </div>
        )}

        <div data-tooltip="Saved as editor adjustments — re-tune them any time in the edit view">
          <Slider
            label="Exposure"
            min={-3}
            max={3}
            step={0.1}
            value={s.exposureOffset}
            defaultValue={0}
            onChange={(e: any) => handleExposureChange(parseFloat(e.target.value))}
          />
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

        <Switch
          label="Auto crop"
          checked={s.autoCrop}
          disabled={busy}
          onChange={handleAutoCrop}
          tooltip="Non-destructive crop — drag any edge in the preview to adjust"
        />

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
            {s.sessionScans.length > 0 && (
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
