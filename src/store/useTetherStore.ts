import { create } from 'zustand';

export interface TetherCameraConfig {
  key: string;
  label: string;
  current: string;
  choices: Array<string>;
}

export interface TetherCamera {
  model: string;
  configs: Array<TetherCameraConfig>;
}

interface TetherState {
  isActive: boolean;
  folder: string | null;
  presetId: string | null;
  autoSelect: boolean;
  shotCount: number;
  lastShotName: string | null;
  camera: TetherCamera | null;
  liveView: boolean;

  setTether: (state: Partial<TetherState> | ((state: TetherState) => Partial<TetherState>)) => void;
}

export const useTetherStore = create<TetherState>((set) => ({
  isActive: false,
  folder: null,
  presetId: null,
  autoSelect: true,
  shotCount: 0,
  lastShotName: null,
  camera: null,
  liveView: false,

  setTether: (updater) => set((state) => (typeof updater === 'function' ? updater(state) : updater)),
}));
