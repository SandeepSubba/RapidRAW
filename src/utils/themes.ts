import { Theme } from '../components/ui/AppProperties';

export interface ThemeProps {
  cssVariables: any;
  id: Theme;
  name: string;
  splashImage: string;
}

export const THEMES: Array<ThemeProps> = [
  {
    id: Theme.Dark,
    name: 'settings.themes.dark',
    splashImage: '/splash-dark.jpg',
    cssVariables: {
      '--app-bg-primary': 'rgb(24, 24, 24)',
      '--app-bg-secondary': 'rgb(35, 35, 35)',
      '--app-surface': 'rgb(28, 28, 28)',
      '--app-card-active': 'rgb(43, 43, 43)',
      '--app-button-text': 'rgb(0, 0, 0)',
      '--app-text-primary': 'rgb(232, 234, 237)',
      '--app-text-secondary': 'rgb(158, 158, 158)',
      '--app-accent': 'rgb(255, 255, 255)',
      '--app-border-color': 'rgb(45, 45, 45)',
      '--app-hover-color': 'rgb(255, 255, 255)',
    },
  },
  {
    id: Theme.Light,
    name: 'settings.themes.light',
    splashImage: '/splash-light.jpg',
    cssVariables: {
      '--app-bg-primary': 'rgb(245, 245, 245)',
      '--app-bg-secondary': 'rgb(255, 255, 255)',
      '--app-surface': 'rgb(241, 241, 241)',
      '--app-card-active': 'rgb(250, 250, 250)',
      '--app-button-text': 'rgb(255, 255, 255)',
      '--app-text-primary': 'rgb(20, 20, 20)',
      '--app-text-secondary': 'rgb(108, 108, 108)',
      '--app-accent': 'rgb(198, 142, 110)',
      '--app-border-color': 'rgb(224, 224, 224)',
      '--app-hover-color': 'rgb(198, 142, 110)',
    },
  },
  {
    id: Theme.Grey,
    name: 'settings.themes.grey',
    splashImage: '/splash-grey.jpg',
    cssVariables: {
      '--app-bg-primary': 'rgb(112, 112, 112)',
      '--app-bg-secondary': 'rgb(118, 118, 118)',
      '--app-surface': 'rgb(108, 108, 108)',
      '--app-card-active': 'rgb(133, 133, 133)',
      '--app-button-text': 'rgb(45, 45, 45)',
      '--app-text-primary': 'rgb(240, 240, 240)',
      '--app-text-secondary': 'rgb(180, 180, 180)',
      '--app-accent': 'rgb(220, 220, 220)',
      '--app-border-color': 'rgb(138, 138, 138)',
      '--app-hover-color': 'rgb(220, 220, 220)',
    },
  },
];

export const DEFAULT_THEME_ID = Theme.Dark;

// Editor canvas presets: the area around the image only, independent of the UI
// theme. Neutral greys so the surround doesn't bias colour/tone judgement.
// 'theme' (or unset) falls back to editorNeutralGreyBg, then to the theme.
// midGrey is upstream's neutral grey (#808080), so the two settings agree.
export const EDITOR_CANVAS_COLORS: Array<{ id: string; name: string; rgb: [number, number, number] | null }> = [
  { id: 'theme', name: 'settings.general.editorCanvasTheme', rgb: null },
  { id: 'white', name: 'settings.general.editorCanvasWhite', rgb: [255, 255, 255] },
  { id: 'lightGrey', name: 'settings.general.editorCanvasLightGrey', rgb: [204, 204, 204] },
  { id: 'midGrey', name: 'settings.general.editorCanvasMidGrey', rgb: [128, 128, 128] },
  { id: 'darkGrey', name: 'settings.general.editorCanvasDarkGrey', rgb: [64, 64, 64] },
  { id: 'black', name: 'settings.general.editorCanvasBlack', rgb: [0, 0, 0] },
];

export const editorCanvasRgb = (id?: string | null): [number, number, number] | null =>
  EDITOR_CANVAS_COLORS.find((c) => c.id === id)?.rgb ?? null;
