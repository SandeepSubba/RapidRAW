import type { AutoAdjustOptions } from './ui';
import type { CopyPasteSettings } from '../utils/adjustments';
import type { WhiteBalanceMode } from '../utils/whiteBalance';
import type { ExportPreset } from './export';
import type { FilterCriteria, SortCriteria, SortDirection } from './library';
import type { CollapsibleSectionsState, Panel, PanelRegion, UiVisibility } from './ui';

export enum ExifOverlay {
  Off = 'off',
  Hover = 'hover',
  Always = 'always',
}

type FolderSortKey = 'name' | 'modified' | 'created' | 'imageCount';

export interface FolderTreeSort {
  key: FolderSortKey;
  order: SortDirection;
}

export enum Theme {
  Arctic = 'arctic',
  Blue = 'blue',
  Dark = 'dark',
  Grey = 'grey',
  Light = 'light',
  MutedGreen = 'muted-green',
  Sepia = 'sepia',
  Snow = 'snow',
}

export enum ThumbnailAspectRatio {
  Cover = 'cover',
  Contain = 'contain',
  Justified = 'justified',
}

export interface WorkspaceState {
  leftPanelWidth: number;
  rightPanelWidth: number;
  leftTopHeight: number;
  rightTopHeight: number;
  panelLayout: Record<PanelRegion, Panel[]>;
  activePanels: Record<PanelRegion, Panel | null>;
  panelSwitcherPlacement: Record<PanelRegion, 'left' | 'right' | 'top' | 'bottom'>;
  filmstripColumns: number;
  filmstripShowFileInfo: boolean;
}

export interface CustomAspectRatio {
  width: number;
  height: number;
}

export enum EditInMode {
  Original = 'original',
  Rendered = 'rendered',
}

export interface EditInApp {
  name: string;
  path: string;
  args: string;
  mode: EditInMode;
}

export type GroupPreference = 'jpeg' | 'raw';

export type GroupingMode = 'off' | GroupPreference;

export type CompareView = 'off' | 'vertical' | 'horizontal' | 'sideBySide';

export type CompareMode = 'toggle' | Exclude<CompareView, 'off'>;

export type SliderResetTarget = 'neutral' | 'defaults';

export interface Camera {
  make: string;
  model: string;
}

export interface CameraDefaults extends Camera {
  presetId?: string | null;
}

export type DefaultsSource = 'factory' | 'allCameras' | 'camera';

export interface DevelopDefaultsSettings {
  presetId?: string | null;
  autoLensCorrection?: boolean;
  applyToNonRaw?: boolean;
  cameras?: CameraDefaults[];
}

export interface CropPreset {
  name: string;
  width: number;
  height: number;
}

export interface AppSettings {
  filmPanelAdvanced?: boolean;
  displayNegativeIcon?: boolean;
  negativeProfiles?: Array<{
    name: string;
    filmStock?: string;
    params: { redWeight: number; greenWeight: number; blueWeight: number; exposure: number; contrast: number };
    clipBlack?: number;
    clipWhite?: number;
  }>;
  cropPresets?: Array<CropPreset>;
  lastCropRatio?: number | null;
  lastCropPreferPortrait?: boolean;
  assistantProvider?: string;
  assistantEndpoint?: string;
  assistantApiKey?: string;
  assistantModel?: string;
  assistantThinking?: string;
  assistantDevRepoPath?: string;
  externalEditorPath?: string;
  externalEditorFormat?: string;
  aiCacheLimitGb?: number;
  aiMapsCacheLimitGb?: number;
  aiUpscaleCacheLimitGb?: number;
  thumbnailCacheLimitGb?: number;
  aiConnectorAddress?: string;
  aiGpuAcceleration?: boolean;
  aiProvider?: string;
  decorations?: any;
  editorPreviewResolution?: number;
  smallThumbnailResolution?: number;
  mediumThumbnailResolution?: number;
  enableZoomHifi?: boolean;
  useFullDpiRendering?: boolean;
  uiScale?: number;
  highResZoomMultiplier?: number;
  enableLivePreviews?: boolean;
  livePreviewQuality?: string;
  exportWorkerThreads?: number;
  exportMemoryLimit?: number;
  enableAiTagging?: boolean;
  aiTagCount?: number;
  customAiTags?: string[];
  filterCriteria?: FilterCriteria;
  lastFolderState?: any;
  pinnedFolders?: any;
  lastRootPath: string | null;
  rootFolders?: string[];
  libraryViewMode?: LibraryViewMode;
  sortCriteria?: SortCriteria;
  theme: Theme;
  thumbnailSize?: ThumbnailSize;
  thumbnailAspectRatio?: ThumbnailAspectRatio;
  uiVisibility?: UiVisibility;
  rawHighlightCompression?: number;
  processingBackend?: string;
  rawDecoding?: string;
  linuxGpuOptimization?: boolean;
  exportPresets?: ExportPreset[];
  myLenses?: any;
  customAspectRatios?: CustomAspectRatio[];
  editInApps?: EditInApp[];
  adjustmentLayout?: AdjustmentLayout;
  enableFolderImageCounts?: boolean;
  displayEditIcon?: boolean;
  linearRawMode?: string;
  enableXmpSync?: boolean;
  createXmpIfMissing?: boolean;
  isWaveformVisible?: boolean;
  waveformHeight?: number;
  activeWaveformChannel?: string;
  useWgpuRenderer?: boolean;
  editorNeutralGreyBg?: boolean;
  // Editor canvas preset (see EDITOR_CANVAS_COLORS); unset falls back to editorNeutralGreyBg.
  editorCanvasColor?: string;
  canvasInputMode?: 'mouse' | 'trackpad';
  compareMode?: CompareMode;
  whiteBalanceMode?: WhiteBalanceMode;
  zoomSpeedMultiplier?: number;
  zoomPhotoToPixelClick?: boolean;
  keybinds?: { [action: string]: string[] };
  adjustmentSteps?: { [adjustmentKey: string]: number };
  autoAdjust?: AutoAdjustOptions;
  speedEditAdjustments?: string[];
  enableBaselineExposure?: boolean;
  baselineExposure?: number;
  compensateHighlightPreservation?: boolean;
  hdrDisplay?: boolean;
  hdrHeadroom?: number;
  displayColorMode?: 'auto' | 'off' | 'custom';
  displayIccProfile?: string | null;
  wideGamutPreview?: boolean;
  tonemapperOverrideEnabled?: boolean;
  defaultRawTonemapper?: string;
  defaultNonRawTonemapper?: string;
  copyPasteSettings?: CopyPasteSettings;
  developDefaults?: DevelopDefaultsSettings;
  sliderResetTarget?: SliderResetTarget;
  enableFocusMode?: boolean;
  wrapImageNavigation?: boolean;
  enableToolFocusMode?: boolean;
  useAppleRaw9?: boolean;
  hideCollapsedPanels?: boolean;
  openTreeSections?: string[];
  folderIcons?: Record<string, string>;
  exifOverlay?: ExifOverlay;
  language?: string;
  fontFamily?: string;
  folderTreeSort?: FolderTreeSort;
  taggingShortcuts?: string[];
  libraryDisplayMode?: LibraryDisplayMode;
  grouping?: GroupingMode;
  requireMatchingExif?: boolean;
  groupEditedFiles?: boolean;
  groupPreferredType?: GroupPreference; // legacy
  alwaysDecodeRawThumbnails?: boolean;
  workspace?: WorkspaceState;
  immich?: ImmichSettings;
}

export enum LibraryViewMode {
  Flat = 'flat',
  Recursive = 'recursive',
}

export enum LibraryDisplayMode {
  Grid = 'grid',
  Cull = 'cull',
  List = 'list',
}

export enum ThumbnailSize {
  ExtraLarge = 'extraLarge',
  Large = 'large',
  Medium = 'medium',
  SmallMedium = 'smallMedium',
  Small = 'small',
  ExtraSmall = 'extraSmall',
}

export const THUMBNAIL_SIZE_STEPS: Array<{ id: ThumbnailSize; size: number }> = [
  { id: ThumbnailSize.ExtraSmall, size: 120 },
  { id: ThumbnailSize.Small, size: 160 },
  { id: ThumbnailSize.SmallMedium, size: 200 },
  { id: ThumbnailSize.Medium, size: 240 },
  { id: ThumbnailSize.Large, size: 320 },
  { id: ThumbnailSize.ExtraLarge, size: 400 },
];

export interface AdjustmentLayout {
  collapsedTools?: string[];
  hiddenSections?: string[];
  hiddenTools?: string[];
  openSections?: Partial<CollapsibleSectionsState>;
  sectionOrder?: string[];
  toolOrder?: Record<string, string[]>;
}

export interface ImmichSettings {
  serverUrl: string;
  cacheLimitGb: number;
  exportAlbum: string;
  exportWithOriginal: boolean;
}
