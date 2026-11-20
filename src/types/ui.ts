export const GLOBAL_KEYS = [
  ' ',
  'ArrowUp',
  'ArrowDown',
  'ArrowLeft',
  'ArrowRight',
  'f',
  'b',
  'a',
  's',
  'd',
  'r',
  'm',
  'k',
  'p',
  'i',
  'e',
  '0',
  '1',
  '2',
  '3',
  '4',
  '5',
  'Enter',
];

export const OPTION_SEPARATOR = 'separator';

export enum Panel {
  Adjustments = 'adjustments',
  Agent = 'agent',
  Ai = 'ai',
  Crop = 'crop',
  Export = 'export',
  Masks = 'masks',
  Metadata = 'metadata',
  Presets = 'presets',
  FolderTree = 'folderTree',
  Tethering = 'tethering',
  Filmstrip = 'filmstrip',
}

export type PanelRegion = 'leftTop' | 'leftBottom' | 'rightTop' | 'rightBottom';

/** Which sliders the Auto button writes (one switch each), and whether it corrects on top of the current look. */
export interface AutoAdjustOptions {
  exposure: boolean;
  brightness: boolean;
  contrast: boolean;
  highlights: boolean;
  shadows: boolean;
  whites: boolean;
  blacks: boolean;
  whiteBalance: boolean;
  vibrance: boolean;
  clarity: boolean;
  dehaze: boolean;
  vignette: boolean;
  keepLook: boolean;
  /** How hard Auto pushes: its corrections are scaled ×0.5 / ×1 / ×1.5 (upstream v1.6.7). */
  strength: 'subtle' | 'normal' | 'strong';
}

export const DEFAULT_AUTO_ADJUST: AutoAdjustOptions = {
  exposure: true,
  brightness: true,
  contrast: true,
  highlights: true,
  shadows: true,
  whites: true,
  blacks: true,
  whiteBalance: true,
  vibrance: false,
  clarity: false,
  dehaze: false,
  vignette: false,
  keepLook: true,
  strength: 'normal',
};

/**
 * Saved options merged over the defaults. The first version saved grouped switches
 * (tone, levels, effects); they still count until the per-slider ones are saved.
 */
export function resolveAutoAdjust(saved?: Partial<AutoAdjustOptions> & Record<string, any>): AutoAdjustOptions {
  const s = saved ?? {};
  const pick = (key: Exclude<keyof AutoAdjustOptions, 'strength'>, legacy?: string): boolean =>
    typeof s[key] === 'boolean'
      ? s[key]
      : legacy && typeof s[legacy] === 'boolean'
        ? s[legacy]
        : DEFAULT_AUTO_ADJUST[key];
  return {
    exposure: pick('exposure'),
    brightness: pick('brightness', 'exposure'),
    contrast: pick('contrast'),
    highlights: pick('highlights', 'tone'),
    shadows: pick('shadows', 'tone'),
    whites: pick('whites', 'levels'),
    blacks: pick('blacks', 'levels'),
    whiteBalance: pick('whiteBalance'),
    vibrance: pick('vibrance'),
    clarity: pick('clarity', 'effects'),
    dehaze: pick('dehaze', 'effects'),
    vignette: pick('vignette', 'effects'),
    keepLook: pick('keepLook'),
    strength:
      s.strength === 'subtle' || s.strength === 'normal' || s.strength === 'strong'
        ? s.strength
        : DEFAULT_AUTO_ADJUST.strength,
  };
}

export interface Option {
  color?: string;
  disabled?: boolean;
  icon?: any;
  isDestructive?: boolean;
  label?: string;
  onClick?(): void;
  onRightClick?(): void;
  submenu?: any;
  type?: string;
}

export enum Orientation {
  Horizontal = 'horizontal',
  Vertical = 'vertical',
}

export interface UiVisibility {
  filmstrip: boolean;
  leftPanel: boolean;
  rightPanel: boolean;
  quickFilter?: boolean;
  dockedQuickFilter?: boolean;
}

export interface CollapsibleSectionsState {
  basic: boolean;
  color: boolean;
  curves: boolean;
  details: boolean;
  effects: boolean;
  film: boolean;
}
