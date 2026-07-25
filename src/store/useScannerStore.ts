import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type ScannerDetect = 'unknown' | 'detecting' | 'no-scanimage' | 'no-scanner' | 'ready';
export type FilmType = 'c41' | 'bw' | 'e6';
export type ScanActivity = 'idle' | 'preview' | 'scan';

export interface ScannerDevice {
  name: string;
  model: string;
}

interface ScannerState {
  detect: ScannerDetect;
  device: ScannerDevice | null;
  filmType: FilmType;
  dpi: number;
  samples: number;
  irClean: boolean;
  bitDepth: number;
  autoCrop: boolean;
  cropRect: [number, number, number, number] | null;
  exposureOffset: number;
  contrast: number;
  rotationSteps: number;
  prefix: string;
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
      filmType: 'c41',
      dpi: 3600,
      samples: 1,
      irClean: false,
      bitDepth: 12,
      autoCrop: false,
      cropRect: null,
      exposureOffset: 0,
      contrast: 0,
      rotationSteps: 0,
      prefix: 'roll',
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
        bitDepth: state.bitDepth,
        autoCrop: state.autoCrop,
        exposureOffset: state.exposureOffset,
        contrast: state.contrast,
        rotationSteps: state.rotationSteps,
        prefix: state.prefix,
      }),
    },
  ),
);
