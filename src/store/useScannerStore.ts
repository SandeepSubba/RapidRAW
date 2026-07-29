import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type ScannerDetect = 'unknown' | 'detecting' | 'no-scanimage' | 'no-scanner' | 'ready';
export type FilmType = 'c41' | 'bw' | 'e6';
export type ScanActivity = 'idle' | 'preview' | 'scan';

export interface ScannerDevice {
  name: string;
  model: string;
}

export interface ScannerCaps {
  sourceVisible: string;
  sourceInfrared: string | null;
  resolutions: number[];
  defaultResolution: number;
  maxDepth: number;
  hasTransparency: boolean;
}

interface ScannerState {
  detect: ScannerDetect;
  device: ScannerDevice | null;
  caps: ScannerCaps | null;
  filmType: FilmType;
  dpi: number;
  samples: number;
  irClean: boolean;
  irSensitivity: number; // 0..100, 50 = default IR dust-removal aggressiveness
  bitDepth: number;
  autoCrop: boolean;
  cropRect: [number, number, number, number] | null;
  cropManual: boolean; // true once the user drags an edge — stops re-renders re-detecting
  // Film-base eyedropper: pinned rebate point (normalized, in the displayed
  // preview) and whether the next preview click samples it.
  basePoint: [number, number] | null;
  eyedropping: boolean;
  exposureOffset: number;
  contrast: number;
  rotationSteps: number;
  // Optional shooting metadata written to the scan's sidecar + EXIF (roll-level).
  filmStock: string;
  iso: string;
  camera: string;
  lens: string;
  notes: string;
  // Filename pattern, same token style as image export: {sequence} plus
  // {YYYY} {MM} {DD} {hh} {mm}. Literal text (the roll name) is typed inline.
  namePattern: string;
  frameCount: number;
  scanning: ScanActivity;
  progress: number;
  previewData: string | null;
  sessionScans: string[];
  error: string | null;
  setScanner: (partial: Partial<ScannerState> | ((state: ScannerState) => Partial<ScannerState>)) => void;
}

export const useScannerStore = create<ScannerState>()(
  persist(
    (set) => ({
      detect: 'unknown',
      device: null,
      caps: null,
      filmType: 'c41',
      dpi: 3600,
      samples: 1,
      irClean: false,
      irSensitivity: 50,
      bitDepth: 12,
      autoCrop: false,
      cropRect: null,
      cropManual: false,
      basePoint: null,
      eyedropping: false,
      exposureOffset: 0,
      contrast: 0,
      rotationSteps: 0,
      filmStock: '',
      iso: '',
      camera: '',
      lens: '',
      notes: '',
      namePattern: 'roll-{sequence}',
      frameCount: 0,
      scanning: 'idle',
      progress: 0,
      previewData: null,
      sessionScans: [],
      error: null,
      setScanner: (partial) => set((state) => (typeof partial === 'function' ? partial(state) : partial)),
    }),
    {
      name: 'rapidraw-scanner-prefs',
      partialize: (state) => ({
        filmType: state.filmType,
        dpi: state.dpi,
        samples: state.samples,
        irClean: state.irClean,
        irSensitivity: state.irSensitivity,
        bitDepth: state.bitDepth,
        autoCrop: state.autoCrop,
        exposureOffset: state.exposureOffset,
        contrast: state.contrast,
        rotationSteps: state.rotationSteps,
        filmStock: state.filmStock,
        iso: state.iso,
        camera: state.camera,
        lens: state.lens,
        namePattern: state.namePattern,
      }),
    },
  ),
);
