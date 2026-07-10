import { create } from 'zustand';

interface TetherState {
  isActive: boolean;
  folder: string | null;
  presetId: string | null;
  autoSelect: boolean;
  shotCount: number;
  lastShotName: string | null;

  setTether: (state: Partial<TetherState> | ((state: TetherState) => Partial<TetherState>)) => void;
}

export const useTetherStore = create<TetherState>((set) => ({
  isActive: false,
  folder: null,
  presetId: null,
  autoSelect: true,
  shotCount: 0,
  lastShotName: null,

  setTether: (updater) => set((state) => (typeof updater === 'function' ? updater(state) : updater)),
}));
