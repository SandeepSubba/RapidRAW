import { useCallback, useEffect, useRef } from 'react';
import { toast } from 'react-toastify';
import { ImageFile, Panel, ExifOverlay } from '../components/ui/AppProperties';
import { KEYBIND_DEFINITIONS, ADJUSTMENT_NUDGES, normalizeCombo, resolveNudgeStep } from '../utils/keyboardUtils';
import { useEditorStore } from '../store/useEditorStore';
import { useLibraryStore } from '../store/useLibraryStore';
import { useSettingsStore } from '../store/useSettingsStore';
import { useUIStore } from '../store/useUIStore';
import { useProcessStore } from '../store/useProcessStore';
import { useEditorActions } from './useEditorActions';
import { useLibraryActions } from './useLibraryActions';

interface KeyboardShortcutsProps {
  sortedImageList: Array<ImageFile>;
  handleBackToLibrary(): void;
  handleDeleteSelected(): void;
  handleImageSelect(path: string, openInEditor?: boolean): void;
  handlePasteFiles(str: string): void;
  handleToggleFullScreen(): void;
  handleZoomChange(zoomValue: number, fitToWindow?: boolean): void;
}

export const useKeyboardShortcuts = ({
  sortedImageList,
  handleBackToLibrary,
  handleDeleteSelected,
  handleImageSelect,
  handlePasteFiles,
  handleToggleFullScreen,
  handleZoomChange,
}: KeyboardShortcutsProps) => {
  const { setAdjustments, handleRotate, handleCopyAdjustments, handlePasteAdjustments } = useEditorActions();
  const { handleRate, handleSetColorLabel, handleRotateSelected } = useLibraryActions();

  const sortedListRef = useRef(sortedImageList);
  useEffect(() => {
    sortedListRef.current = sortedImageList;
  }, [sortedImageList]);

  // Held leader keys must survive this hook's frequent effect re-runs (a nudge
  // re-renders and rebuilds the listener); a closure-local map would reset
  // mid-hold and let + leak through to zoom.
  const heldLeadersRef = useRef(new Map<string, { used: boolean; event: KeyboardEvent }>());
  const handleCopyImagePaths = useCallback(async (paths: Array<string>) => {
    const physicalPaths = [...new Set(paths.map((path) => path.split('?vc=')[0]))];
    if (physicalPaths.length === 0) {
      return;
    }
    try {
      await navigator.clipboard.writeText(physicalPaths.join('\n'));
    } catch (err) {
      console.error('Failed to copy image path to clipboard', err);
      toast.error(`Failed to copy path: ${err}`);
    }
  }, []);

  useEffect(() => {
    const getStoreState = () => ({
      editor: useEditorStore.getState(),
      library: useLibraryStore.getState(),
      ui: useUIStore.getState(),
      settings: useSettingsStore.getState(),
      process: useProcessStore.getState(),
    });

    const comboMap = new Map<string, string>();
    // Held-chord bindings: a keybind whose first element is a non-modifier key,
    // e.g. ['KeyE','Equal'] = hold E, then press +. Keyed leaderCode -> (restKey
    // -> action). Empty unless the user records a hold-then-key shortcut.
    const leaderChords = new Map<string, Map<string, string>>();
    const MODS = ['ctrl', 'shift', 'alt'];
    const keybinds = useSettingsStore.getState().appSettings?.keybinds;

    for (const def of KEYBIND_DEFINITIONS) {
      const userCombo = keybinds?.[def.action];
      const effective = userCombo && userCombo.length > 0 ? userCombo : def.defaultCombo;
      if (!effective || effective.length === 0) continue;
      if (effective.length >= 2 && !MODS.includes(effective[0])) {
        const leader = effective[0];
        if (!leaderChords.has(leader)) leaderChords.set(leader, new Map());
        leaderChords.get(leader)!.set(effective.slice(1).join('+'), def.action);
      } else {
        comboMap.set(effective.join('+'), def.action);
      }
    }

    // Leader keys currently held down. `used` marks that a chord already fired,
    // so the leader's own tap action isn't triggered when it's released.
    const heldLeaders = heldLeadersRef.current;
    const getImagePathsForCopy = (s: any): Array<string> => {
      if (s.editor.selectedImage) {
        return [s.editor.selectedImage.path];
      }
      const { libraryActivePath, multiSelectedPaths } = s.library;
      if (multiSelectedPaths.length > 0) {
        const listOrder = new Map(sortedListRef.current.map((image: ImageFile, index: number) => [image.path, index]));
        return [...multiSelectedPaths].sort(
          (a: string, b: string) =>
            (listOrder.get(a) ?? Number.MAX_SAFE_INTEGER) - (listOrder.get(b) ?? Number.MAX_SAFE_INTEGER),
        );
      }
      return libraryActivePath ? [libraryActivePath] : [];
    };

    const actions: Record<string, any> = {
      open_image: {
        shouldFire: (s: any) => s.ui.activeView === 'library' && s.library.libraryActivePath !== null,
        execute: (e: any, s: any) => {
          e.preventDefault();
          handleImageSelect(s.library.libraryActivePath!, true);
        },
      },
      copy_adjustments: {
        shouldFire: () => true,
        execute: (e: any) => {
          e.preventDefault();
          handleCopyAdjustments();
        },
      },
      paste_adjustments: {
        shouldFire: () => true,
        execute: (e: any) => {
          e.preventDefault();
          handlePasteAdjustments();
        },
      },
      copy_image_path: {
        shouldFire: (s: any) => getImagePathsForCopy(s).length > 0,
        execute: (e: any, s: any) => {
          e.preventDefault();
          handleCopyImagePaths(getImagePathsForCopy(s));
        },
      },
      copy_files: {
        shouldFire: (s: any) => s.library.multiSelectedPaths.length > 0,
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.process.setProcess({ copiedFilePaths: s.library.multiSelectedPaths });
        },
      },
      paste_files: {
        shouldFire: () => true,
        execute: (e: any) => {
          e.preventDefault();
          handlePasteFiles('copy');
        },
      },
      select_all: {
        shouldFire: () => sortedListRef.current.length > 0,
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.library.setLibrary({ multiSelectedPaths: sortedListRef.current.map((f: ImageFile) => f.path) });
          if (s.ui.activeView === 'library') {
            const lastPath = sortedListRef.current[sortedListRef.current.length - 1].path;
            s.library.setLibrary({ libraryActivePath: lastPath });
            handleImageSelect(lastPath, false);
          }
        },
      },
      delete_selected: {
        shouldFire: (s: any) => !s.editor.activeMaskContainerId && !s.editor.activeAiPatchContainerId,
        execute: (e: any) => {
          e.preventDefault();
          handleDeleteSelected();
        },
      },
      preview_prev: {
        shouldFire: (s: any) => s.ui.activeView === 'editor' && !!s.editor.selectedImage,
        execute: (e: any, s: any) => {
          e.preventDefault();
          const currentIndex = sortedListRef.current.findIndex((img) => img.path === s.editor.selectedImage!.path);
          if (currentIndex === -1) return;
          const wrap = s.settings.appSettings?.wrapImageNavigation ?? true;
          if (currentIndex - 1 < 0 && !wrap) return;
          let nextIndex = currentIndex - 1 < 0 ? sortedListRef.current.length - 1 : currentIndex - 1;
          handleImageSelect(sortedListRef.current[nextIndex].path, true);
        },
      },
      preview_next: {
        shouldFire: (s: any) => s.ui.activeView === 'editor' && !!s.editor.selectedImage,
        execute: (e: any, s: any) => {
          e.preventDefault();
          const currentIndex = sortedListRef.current.findIndex((img) => img.path === s.editor.selectedImage!.path);
          if (currentIndex === -1) return;
          const wrap = s.settings.appSettings?.wrapImageNavigation ?? true;
          if (currentIndex + 1 >= sortedListRef.current.length && !wrap) return;
          let nextIndex = currentIndex + 1 >= sortedListRef.current.length ? 0 : currentIndex + 1;
          handleImageSelect(sortedListRef.current[nextIndex].path, true);
        },
      },
      zoom_in_step: {
        shouldFire: (s: any) => s.ui.activeView === 'editor' && !!s.editor.selectedImage,
        execute: (e: any, s: any) => {
          e.preventDefault();
          const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
          const currentPercent =
            s.editor.originalSize?.width > 0 && s.editor.displaySize?.width > 0
              ? (s.editor.displaySize.width * dpr) / s.editor.originalSize.width
              : 1.0;
          handleZoomChange(Math.min(currentPercent + 0.1, 2.0));
        },
      },
      zoom_out_step: {
        shouldFire: (s: any) => s.ui.activeView === 'editor' && !!s.editor.selectedImage,
        execute: (e: any, s: any) => {
          e.preventDefault();
          const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
          const currentPercent =
            s.editor.originalSize?.width > 0 && s.editor.displaySize?.width > 0
              ? (s.editor.displaySize.width * dpr) / s.editor.originalSize.width
              : 1.0;
          handleZoomChange(Math.max(currentPercent - 0.1, 0.1));
        },
      },
      cycle_zoom: {
        shouldFire: (s: any) => s.ui.activeView === 'editor' && !!s.editor.selectedImage,
        execute: (e: any, s: any) => {
          e.preventDefault();
          const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
          const { originalSize, displaySize, baseRenderSize } = s.editor;
          const currentPercent =
            originalSize?.width > 0 && displaySize?.width > 0
              ? Math.round(((displaySize.width * dpr) / originalSize.width) * 100)
              : 100;
          let fitPercent = 100;

          if (originalSize?.width > 0 && baseRenderSize?.width > 0) {
            const originalAspect = originalSize.width / originalSize.height;
            const baseAspect = baseRenderSize.width / baseRenderSize.height;
            fitPercent =
              originalAspect > baseAspect
                ? Math.round(((baseRenderSize.width * dpr) / originalSize.width) * 100)
                : Math.round(((baseRenderSize.height * dpr) / originalSize.height) * 100);
          }

          const doubleFitPercent = fitPercent * 2;
          if (Math.abs(currentPercent - fitPercent) < 5) {
            handleZoomChange(doubleFitPercent < 100 ? doubleFitPercent / 100 : 1.0);
          } else if (Math.abs(currentPercent - doubleFitPercent) < 5 && doubleFitPercent < 100) {
            handleZoomChange(1.0);
          } else {
            handleZoomChange(0, true);
          }
        },
      },
      zoom_in: {
        shouldFire: (s: any) => s.ui.activeView === 'editor' && !!s.editor.selectedImage,
        execute: (e: any, s: any) => {
          e.preventDefault();
          const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
          const currentPercent =
            s.editor.originalSize?.width > 0 && s.editor.displaySize?.width > 0
              ? (s.editor.displaySize.width * dpr) / s.editor.originalSize.width
              : 1.0;
          handleZoomChange(Math.min(currentPercent * 1.2, 2.0));
        },
      },
      zoom_out: {
        shouldFire: (s: any) => s.ui.activeView === 'editor' && !!s.editor.selectedImage,
        execute: (e: any, s: any) => {
          e.preventDefault();
          const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
          const currentPercent =
            s.editor.originalSize?.width > 0 && s.editor.displaySize?.width > 0
              ? (s.editor.displaySize.width * dpr) / s.editor.originalSize.width
              : 1.0;
          handleZoomChange(Math.max(currentPercent / 1.2, 0.1));
        },
      },
      zoom_fit: {
        shouldFire: (s: any) => s.ui.activeView === 'editor' && !!s.editor.selectedImage,
        execute: (e: any) => {
          e.preventDefault();
          handleZoomChange(0, true);
        },
      },
      zoom_100: {
        shouldFire: (s: any) => s.ui.activeView === 'editor' && !!s.editor.selectedImage,
        execute: (e: any) => {
          e.preventDefault();
          handleZoomChange(1.0);
        },
      },
      rotate_left: {
        // In the editor: rotate the open image. In the library: batch-rotate the
        // whole selection. In library view `selectedImage` is also set (it feeds
        // the metadata/preview panels), so gate on the active view — otherwise a
        // multi-selection would collapse to rotating just the one active image.
        shouldFire: (s: any) => !!s.editor.selectedImage || s.library.multiSelectedPaths.length > 0,
        execute: (e: any, s: any) => {
          e.preventDefault();
          if (s.ui.activeView === 'editor' && s.editor.selectedImage) handleRotate(-90);
          else handleRotateSelected(-90);
        },
      },
      rotate_right: {
        shouldFire: (s: any) => !!s.editor.selectedImage || s.library.multiSelectedPaths.length > 0,
        execute: (e: any, s: any) => {
          e.preventDefault();
          if (s.ui.activeView === 'editor' && s.editor.selectedImage) handleRotate(90);
          else handleRotateSelected(90);
        },
      },
      undo: {
        shouldFire: (s: any) => s.ui.activeView === 'editor' && !!s.editor.selectedImage && s.editor.historyIndex > 0,
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.editor.undo();
        },
      },
      redo: {
        shouldFire: (s: any) =>
          s.ui.activeView === 'editor' &&
          !!s.editor.selectedImage &&
          s.editor.historyIndex < s.editor.history.length - 1,
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.editor.redo();
        },
      },
      toggle_fullscreen: {
        shouldFire: (s: any) => !!s.editor.selectedImage,
        execute: (e: any) => {
          e.preventDefault();
          handleToggleFullScreen();
        },
      },
      show_original: {
        shouldFire: (s: any) => s.ui.activeView === 'editor' && !!s.editor.selectedImage,
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.editor.setEditor({ showOriginal: !s.editor.showOriginal });
        },
      },
      toggle_adjustments: {
        shouldFire: () => true,
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.ui.setPanel(Panel.Adjustments);
        },
      },
      toggle_crop_panel: {
        shouldFire: () => true,
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.ui.setPanel(Panel.Crop);
        },
      },
      toggle_masks: {
        shouldFire: () => true,
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.ui.setPanel(Panel.Masks);
        },
      },
      toggle_ai: {
        shouldFire: () => true,
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.ui.setPanel(Panel.Ai);
        },
      },
      toggle_presets: {
        shouldFire: () => true,
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.ui.setPanel(Panel.Presets);
        },
      },
      toggle_metadata: {
        shouldFire: () => true,
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.ui.setPanel(Panel.Metadata);
        },
      },
      toggle_folder_tree: {
        shouldFire: () => true,
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.ui.setPanel(Panel.FolderTree);
        },
      },
      toggle_analytics: {
        shouldFire: (s: any) => !!s.editor.selectedImage,
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.editor.setEditor({ isWaveformVisible: !s.editor.isWaveformVisible });
        },
      },
      toggle_export: {
        shouldFire: () => true,
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.ui.setPanel(Panel.Export);
        },
      },
      toggle_left_panel: {
        shouldFire: () => true,
        execute: (e: any, s: any) => {
          e.preventDefault();
          const isOpening = !s.ui.uiVisibility.leftPanel;
          s.ui.setUI((state: any) => ({
            uiVisibility: { ...state.uiVisibility, leftPanel: isOpening },
            leftPanelWidth: isOpening && state.leftPanelWidth < 250 ? 350 : state.leftPanelWidth,
          }));
        },
      },
      toggle_right_panel: {
        shouldFire: () => true,
        execute: (e: any, s: any) => {
          e.preventDefault();
          const isOpening = !s.ui.uiVisibility.rightPanel;
          s.ui.setUI((state: any) => ({
            uiVisibility: { ...state.uiVisibility, rightPanel: isOpening },
            rightPanelWidth: isOpening && state.rightPanelWidth < 250 ? 350 : state.rightPanelWidth,
          }));
        },
      },
      toggle_bottom_panel: {
        shouldFire: (s: any) => s.ui.activeView !== 'library',
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.ui.setUI((state: any) => ({
            uiVisibility: { ...state.uiVisibility, filmstrip: !state.uiVisibility.filmstrip },
          }));
        },
      },
      toggle_library_exif: {
        shouldFire: (s: any) => s.ui.activeView === 'library',
        execute: (e: any, s: any) => {
          e.preventDefault();
          const current = s.settings.appSettings?.exifOverlay || ExifOverlay.Off;
          const nextState = {
            [ExifOverlay.Off]: ExifOverlay.Hover,
            [ExifOverlay.Hover]: ExifOverlay.Always,
            [ExifOverlay.Always]: ExifOverlay.Off,
          }[current as ExifOverlay];
          s.settings.handleSettingsChange({ ...s.settings.appSettings, exifOverlay: nextState });
        },
      },
      open_settings: {
        shouldFire: () => true,
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.ui.setUI({ isSettingsOpen: true });
        },
      },
      activate_crop: {
        shouldFire: (s: any) => !!s.editor.selectedImage,
        execute: (e: any, s: any) => {
          e.preventDefault();
          // In the crop panel: toggle the crop overlay on/off. Otherwise open the
          // panel AND switch the crop tool on (straighten off — that's the S tool)
          // so the crop handles appear right away instead of needing a click.
          if (s.ui.activePanel === Panel.Crop) {
            s.editor.setEditor({ cropToolActive: !s.editor.cropToolActive });
          } else {
            s.editor.setEditor({ isStraightenActive: false, cropToolActive: true });
            s.ui.setPanel(Panel.Crop);
          }
        },
      },
      focus_search: {
        shouldFire: (s: any) => s.ui.activeView === 'library',
        execute: (e: any, s: any) => {
          e.preventDefault();
          s.ui.requestSearchFocus();
        },
      },
      toggle_crop: {
        shouldFire: (s: any) => s.ui.activeView === 'editor' && !!s.editor.selectedImage,
        execute: (e: any, s: any) => {
          e.preventDefault();
          if (s.ui.activePanel === Panel.Crop) {
            s.editor.setEditor({ isStraightenActive: !s.editor.isStraightenActive });
          } else {
            s.ui.setPanel(Panel.Crop);
            s.editor.setEditor({ isStraightenActive: true });
          }
        },
      },
      rate_0: {
        shouldFire: () => true,
        execute: (e: any) => {
          e.preventDefault();
          handleRate(0);
        },
      },
      rate_1: {
        shouldFire: () => true,
        execute: (e: any) => {
          e.preventDefault();
          handleRate(1);
        },
      },
      rate_2: {
        shouldFire: () => true,
        execute: (e: any) => {
          e.preventDefault();
          handleRate(2);
        },
      },
      rate_3: {
        shouldFire: () => true,
        execute: (e: any) => {
          e.preventDefault();
          handleRate(3);
        },
      },
      rate_4: {
        shouldFire: () => true,
        execute: (e: any) => {
          e.preventDefault();
          handleRate(4);
        },
      },
      rate_5: {
        shouldFire: () => true,
        execute: (e: any) => {
          e.preventDefault();
          handleRate(5);
        },
      },
      color_label_none: {
        shouldFire: () => true,
        execute: (e: any) => {
          e.preventDefault();
          handleSetColorLabel(null);
        },
      },
      color_label_red: {
        shouldFire: () => true,
        execute: (e: any) => {
          e.preventDefault();
          handleSetColorLabel('red');
        },
      },
      color_label_yellow: {
        shouldFire: () => true,
        execute: (e: any) => {
          e.preventDefault();
          handleSetColorLabel('yellow');
        },
      },
      color_label_green: {
        shouldFire: () => true,
        execute: (e: any) => {
          e.preventDefault();
          handleSetColorLabel('green');
        },
      },
      color_label_blue: {
        shouldFire: () => true,
        execute: (e: any) => {
          e.preventDefault();
          handleSetColorLabel('blue');
        },
      },
      color_label_purple: {
        shouldFire: () => true,
        execute: (e: any) => {
          e.preventDefault();
          handleSetColorLabel('purple');
        },
      },
      brush_size_up: {
        shouldFire: (s: any) =>
          s.ui.activeView === 'editor' &&
          !!s.editor.selectedImage &&
          !!s.editor.brushSettings &&
          s.ui.activePanel === Panel.Masks,
        execute: (e: any, s: any) => {
          e.preventDefault();
          const newSize = Math.min((s.editor.brushSettings.size || 50) + 10, 200);
          s.editor.setEditor({ brushSettings: { ...s.editor.brushSettings, size: newSize } });
        },
      },
      brush_size_down: {
        shouldFire: (s: any) =>
          s.ui.activeView === 'editor' &&
          !!s.editor.selectedImage &&
          !!s.editor.brushSettings &&
          s.ui.activePanel === Panel.Masks,
        execute: (e: any, s: any) => {
          e.preventDefault();
          const newSize = Math.max((s.editor.brushSettings.size || 50) - 10, 1);
          s.editor.setEditor({ brushSettings: { ...s.editor.brushSettings, size: newSize } });
        },
      },
    };

    // Capture One–style increase/decrease shortcuts for the tonal & color
    // sliders, generated from the shared ADJUSTMENT_NUDGES config so the
    // keybinds and their behaviour stay in lockstep. Each nudges the global
    // adjustment value and rides the same debounced-history path as the sliders.
    for (const nudge of ADJUSTMENT_NUDGES) {
      actions[nudge.action] = {
        shouldFire: (s: any) => !!s.editor.selectedImage,
        execute: (e: any) => {
          e.preventDefault();
          // Read the step live so user changes in Settings apply without a rebuild.
          const step = resolveNudgeStep(nudge, useSettingsStore.getState().appSettings?.adjustmentSteps);
          const delta = nudge.delta < 0 ? -step : step;
          setAdjustments((prev: any) => {
            const current = typeof prev[nudge.adjustmentKey] === 'number' ? prev[nudge.adjustmentKey] : 0;
            // Round to 2 decimals so repeated fractional steps don't accumulate float drift.
            const next = Math.round(Math.min(nudge.max, Math.max(nudge.min, current + delta)) * 100) / 100;
            return { ...prev, [nudge.adjustmentKey]: next };
          });
        },
      };
    }

    const builtinShortcuts = [
      {
        match: (e: KeyboardEvent) => e.code === 'Escape',
        execute: (e: KeyboardEvent, s: any) => {
          e.preventDefault();
          if (s.editor.isStraightenActive) s.editor.setEditor({ isStraightenActive: false });
          else if (s.ui.customEscapeHandler) s.ui.customEscapeHandler();
          else if (s.editor.activeAiSubMaskId) s.editor.setEditor({ activeAiSubMaskId: null });
          else if (s.editor.activeAiPatchContainerId) s.editor.setEditor({ activeAiPatchContainerId: null });
          else if (s.editor.activeMaskId) s.editor.setEditor({ activeMaskId: null });
          else if (s.editor.activeMaskContainerId) s.editor.setEditor({ activeMaskContainerId: null });
          else if (s.ui.activePanel === Panel.Crop) s.ui.setPanel(Panel.Adjustments);
          else if (s.ui.isFullScreen) handleToggleFullScreen();
          else if (s.ui.activeView === 'editor') handleBackToLibrary();
        },
      },
      {
        match: (e: KeyboardEvent, s: any) => {
          const isDeleteKey = s.settings.osPlatform === 'macos' ? e.code === 'Backspace' : e.code === 'Delete';
          return isDeleteKey && (!!s.editor.activeMaskContainerId || !!s.editor.activeAiPatchContainerId);
        },
        execute: (e: KeyboardEvent, s: any) => {
          e.preventDefault();
          if (s.editor.activeMaskContainerId) {
            s.editor.setEditor((state: any) => ({
              adjustments: {
                ...state.adjustments,
                masks: state.adjustments.masks.filter((c: any) => c.id !== s.editor.activeMaskContainerId),
              },
              activeMaskContainerId: null,
              activeMaskId: null,
            }));
          } else if (s.editor.activeAiPatchContainerId) {
            s.editor.setEditor((state: any) => ({
              adjustments: {
                ...state.adjustments,
                aiPatches: state.adjustments.aiPatches.filter((c: any) => c.id !== s.editor.activeAiPatchContainerId),
              },
              activeAiPatchContainerId: null,
              activeAiSubMaskId: null,
            }));
          }
        },
      },
      {
        // Editor view: build a filmstrip selection with the keyboard, so copied
        // settings can be pasted onto several frames without reaching for the
        // mouse. Only modified arrows are claimed here — plain Left/Right still
        // fall through to preview_prev/preview_next and change the open image.
        // The filmstrip is one-dimensional, so Up/Down are left alone.
        match: (e: KeyboardEvent, s: any) =>
          s.ui.activeView === 'editor' &&
          !!s.editor.selectedImage &&
          ['ArrowLeft', 'ArrowRight'].includes(e.code) &&
          (e.shiftKey || e.ctrlKey || e.metaKey),
        execute: (e: KeyboardEvent, s: any) => {
          e.preventDefault();
          const list = sortedListRef.current;
          if (list.length === 0) return;

          // The moving cursor lives in libraryActivePath, same as library view.
          // Mouse clicks in the editor deliberately leave it alone, so seed it
          // from the open image on the first keyboard gesture.
          const cursorPath = s.library.libraryActivePath ?? s.editor.selectedImage!.path;
          const currentIndex = list.findIndex((img) => img.path === cursorPath);
          if (currentIndex === -1) return;

          // Never wrap: a selection that jumped from the last frame to the first
          // would silently include the whole roll.
          const nextIndex = e.code === 'ArrowRight' ? currentIndex + 1 : currentIndex - 1;
          if (nextIndex < 0 || nextIndex >= list.length) return;
          const nextImage = list[nextIndex];
          if (!nextImage) return;

          if (e.shiftKey) {
            const anchorPath = s.library.selectionAnchorPath ?? s.editor.selectedImage!.path;
            const anchorIndex = list.findIndex((img) => img.path === anchorPath);
            if (anchorIndex === -1) {
              s.library.setLibrary({
                libraryActivePath: nextImage.path,
                multiSelectedPaths: [nextImage.path],
                selectionAnchorPath: nextImage.path,
              });
              return;
            }
            const start = Math.min(anchorIndex, nextIndex);
            const end = Math.max(anchorIndex, nextIndex);
            s.library.setLibrary({
              libraryActivePath: nextImage.path,
              multiSelectedPaths: list.slice(start, end + 1).map((f: ImageFile) => f.path),
              selectionAnchorPath: anchorPath,
            });
          } else {
            // Ctrl/Cmd: step the cursor and add that frame, leaving the rest of
            // the selection (and the anchor) intact so ranges can be combined.
            const grown = new Set(s.library.multiSelectedPaths);
            grown.add(s.editor.selectedImage!.path);
            grown.add(nextImage.path);
            s.library.setLibrary({
              libraryActivePath: nextImage.path,
              multiSelectedPaths: Array.from(grown),
              selectionAnchorPath: nextImage.path,
            });
          }
          // The open image deliberately stays put: these are selection-building
          // gestures, and loading a new frame would reset the anchor.
        },
      },
      {
        match: (e: KeyboardEvent, s: any) =>
          s.ui.activeView === 'library' && ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(e.code),
        execute: (e: KeyboardEvent, s: any) => {
          e.preventDefault();
          const list = sortedListRef.current;
          const isNext = e.code === 'ArrowRight' || e.code === 'ArrowDown';
          const activePath = s.library.libraryActivePath;
          if (list.length === 0) return;
          // Nothing focused yet (e.g. just entered the folder, no click): the first
          // arrow press seeds the cursor on the first image so keyboard nav works
          // without needing an initial mouse click.
          if (!activePath) {
            const first = list[0];
            s.library.setLibrary({
              libraryActivePath: first.path,
              multiSelectedPaths: [first.path],
              selectionAnchorPath: first.path,
            });
            handleImageSelect(first.path, false);
            return;
          }
          const currentIndex = list.findIndex((img) => img.path === activePath);
          if (currentIndex === -1) return;

          // Shift = range-extend from the anchor; Ctrl/Cmd = grow selection by
          // the newly focused image. Either way we never wrap around the ends,
          // so a selection can't jump from the last image to the first.
          const extend = e.shiftKey || e.ctrlKey || e.metaKey;
          const wrap = s.settings.appSettings?.wrapImageNavigation ?? true;

          // Up/Down move a whole grid row; Left/Right move one item. Vertical
          // moves never wrap and clamp at the edges (a no-op past top/bottom).
          const isVertical = e.code === 'ArrowUp' || e.code === 'ArrowDown';
          const columnCount = Math.max(1, s.library.libraryColumnCount ?? 1);
          const step = isVertical ? columnCount : 1;

          let nextIndex = isNext ? currentIndex + step : currentIndex - step;
          if (isVertical) {
            if (nextIndex < 0 || nextIndex >= list.length) return;
          } else {
            if (nextIndex >= list.length) {
              if (!wrap || extend) return;
              nextIndex = 0;
            }
            if (nextIndex < 0) {
              if (!wrap || extend) return;
              nextIndex = list.length - 1;
            }
          }
          const nextImage = list[nextIndex];
          if (!nextImage) return;

          if (e.shiftKey) {
            const anchorPath = s.library.selectionAnchorPath ?? activePath;
            const anchorIndex = list.findIndex((img) => img.path === anchorPath);
            if (anchorIndex === -1) {
              s.library.setLibrary({
                libraryActivePath: nextImage.path,
                multiSelectedPaths: [nextImage.path],
                selectionAnchorPath: nextImage.path,
              });
              return;
            }
            const start = Math.min(anchorIndex, nextIndex);
            const end = Math.max(anchorIndex, nextIndex);
            const range = list.slice(start, end + 1).map((f) => f.path);
            s.library.setLibrary({
              libraryActivePath: nextImage.path,
              multiSelectedPaths: range,
              selectionAnchorPath: anchorPath,
            });
          } else if (e.ctrlKey || e.metaKey) {
            const grown = new Set(s.library.multiSelectedPaths);
            grown.add(nextImage.path);
            s.library.setLibrary({
              libraryActivePath: nextImage.path,
              multiSelectedPaths: Array.from(grown),
              selectionAnchorPath: nextImage.path,
            });
          } else {
            s.library.setLibrary({
              libraryActivePath: nextImage.path,
              multiSelectedPaths: [nextImage.path],
              selectionAnchorPath: nextImage.path,
            });
            // Actually load the frame: panels (Metadata, histogram) read the editor
            // store's selectedImage, so moving the library focus alone leaves them
            // showing the previous image. Not done for shift/ctrl — those are
            // selection-building gestures, and handleImageSelect resets the
            // range anchor.
            handleImageSelect(nextImage.path, false);
          }
        },
      },
    ];

    const handleKeyDown = (event: KeyboardEvent) => {
      const state = getStoreState();

      const isModalOpen =
        state.ui.isImportViewActive || // the SD-card importer owns its own keyboard map
        state.ui.isCreateFolderModalOpen ||
        state.ui.isRenameFolderModalOpen ||
        state.ui.isRenameFileModalOpen ||
        state.ui.isImportModalOpen ||
        state.ui.isCopyPasteSettingsModalOpen ||
        state.ui.confirmModalState.isOpen ||
        state.ui.panoramaModalState.isOpen ||
        state.ui.cullingModalState.isOpen ||
        state.ui.collageModalState.isOpen ||
        state.ui.denoiseModalState.isOpen;

      if (isModalOpen) return;

      if (state.ui.isSettingsOpen) {
        if (event.code === 'Escape') {
          event.preventDefault();
          state.ui.setUI({ isSettingsOpen: false });
        }
        return;
      }

      const isInputFocused =
        document.activeElement?.tagName === 'INPUT' || document.activeElement?.tagName === 'TEXTAREA';
      if (isInputFocused) return;

      for (const builtin of builtinShortcuts) {
        if (builtin.match(event, state)) {
          builtin.execute(event, state);
          return;
        }
      }

      const nkey = normalizeCombo(event, state.settings.osPlatform).join('+');

      // Held-chord: a leader key is down and this press completes a chord (hold
      // E, press +). Fires the chord and blocks the plain key (so + won't zoom).
      // Held +/- auto-repeat keeps nudging.
      for (const [leader, info] of heldLeaders) {
        if (leader === event.code) continue;
        const chordAction = leaderChords.get(leader)?.get(nkey);
        const chordHandler = chordAction ? actions[chordAction] : null;
        if (chordHandler && (!chordHandler.shouldFire || chordHandler.shouldFire(state))) {
          event.preventDefault();
          chordHandler.execute(event, state);
          info.used = true;
          return;
        }
      }

      // This key starts a chord: suppress its own single action while held (its
      // tap action fires on release instead). Only arm when a chord here could
      // actually fire, so non-editor single keys stay instant. A modifier means
      // it's a combo, not a leader hold — otherwise binding a chord on KeyC
      // would swallow ⌘C/Ctrl+C before the combo lookup ever runs.
      const table = event.ctrlKey || event.metaKey || event.altKey ? null : leaderChords.get(event.code);
      if (table && [...table.values()].some((a) => !actions[a]?.shouldFire || actions[a].shouldFire(state))) {
        if (!heldLeaders.has(event.code)) heldLeaders.set(event.code, { used: false, event });
        return;
      }

      const action = comboMap.get(nkey);
      if (action) {
        const handler = actions[action];
        if (handler && (!handler.shouldFire || handler.shouldFire(state))) {
          handler.execute(event, state);
          return;
        }
      }
    };

    // Release: if a leader was held but no chord fired, run its tap action.
    const handleKeyUp = (event: KeyboardEvent) => {
      const info = heldLeaders.get(event.code);
      if (!info) return;
      heldLeaders.delete(event.code);
      if (info.used) return;
      const active = document.activeElement?.tagName;
      const st = getStoreState();
      if (st.ui.isSettingsOpen || active === 'INPUT' || active === 'TEXTAREA') return;
      const action = comboMap.get(event.code);
      const handler = action ? actions[action] : null;
      if (handler && (!handler.shouldFire || handler.shouldFire(st))) handler.execute(info.event, st);
    };
    const handleBlur = () => heldLeaders.clear();

    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    window.addEventListener('blur', handleBlur);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
      window.removeEventListener('blur', handleBlur);
    };
  }, [
    handleBackToLibrary,
    handleDeleteSelected,
    handleImageSelect,
    handlePasteFiles,
    handleToggleFullScreen,
    handleZoomChange,
    setAdjustments,
    handleRotate,
    handleCopyAdjustments,
    handleCopyImagePaths,
    handlePasteAdjustments,
    handleRate,
    handleSetColorLabel,
  ]);
};
