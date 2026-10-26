import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import debounce from 'lodash.debounce';
import { toast } from 'react-toastify';
import { useTranslation } from 'react-i18next';
import { useEditorStore } from '../store/useEditorStore';
import { useLibraryStore } from '../store/useLibraryStore';
import { useSettingsStore } from '../store/useSettingsStore';
import { useProcessStore } from '../store/useProcessStore';
import { useUIStore } from '../store/useUIStore';
import {
  Adjustments,
  INITIAL_ADJUSTMENTS,
  COPYABLE_ADJUSTMENT_KEYS,
  PasteMode,
  LensAdjustment,
  getActiveTools,
  mergeActiveTools,
  normalizeLoadedAdjustments,
  setToolActive,
} from '../utils/adjustments';
import { calculateAutoCropForRotation, calculateCenteredCrop, isFullFrameCrop } from '../utils/cropUtils';
import { Invokes, SelectedImage } from '../components/ui/AppProperties';
import { globalImageCache } from '../utils/ImageLRUCache';

export const debouncedSetHistory = debounce((newAdj: Adjustments) => {
  useEditorStore.getState().pushHistory(newAdj);
}, 500);

export const debouncedSave = debounce((path: string, adjustmentsToSave: Adjustments) => {
  invoke(Invokes.SaveMetadataAndUpdateThumbnail, { path, adjustments: adjustmentsToSave }).catch((err) => {
    console.error('Auto-save failed:', err);
    toast.error(`Failed to save changes: ${err}`);
  });
}, 300);

const waitForImageReady = (path: string) =>
  new Promise<boolean>((resolve) => {
    const unsubscribe = useEditorStore.subscribe((state) => {
      if (state.selectedImage?.path !== path) {
        unsubscribe();
        resolve(false);
      } else if (state.selectedImage.isReady) {
        unsubscribe();
        resolve(true);
      }
    });
  });

export const withRotation = (adjustments: Adjustments, image: SelectedImage | null, rotation: number): Adjustments => ({
  ...adjustments,
  rotation,
  crop:
    image?.width && image?.height
      ? calculateAutoCropForRotation(
          image.width,
          image.height,
          adjustments.orientationSteps || 0,
          adjustments.aspectRatio,
          rotation,
          adjustments.crop,
          rotation - (adjustments.rotation || 0),
        )
      : adjustments.crop,
});

export const reloadAdjustmentsForPaths = async (paths: string[]) => {
  const { selectedImage, setEditor, resetHistory } = useEditorStore.getState();
  const { libraryActivePath, setLibrary } = useLibraryStore.getState();
  if (selectedImage && paths.includes(selectedImage.path)) {
    const meta: any = await invoke(Invokes.LoadMetadata, { path: selectedImage.path });
    if (meta.adjustments && !meta.adjustments.is_null) {
      const normalized = normalizeLoadedAdjustments(meta.adjustments);
      setEditor({ adjustments: normalized });
      resetHistory(normalized);
    }
  }
  if (libraryActivePath && paths.includes(libraryActivePath)) {
    const meta: any = await invoke(Invokes.LoadMetadata, { path: libraryActivePath });
    if (meta.adjustments && !meta.adjustments.is_null) {
      setLibrary({ libraryActiveAdjustments: normalizeLoadedAdjustments(meta.adjustments) });
    }
  }
};

// Quarter-turn images straight in their sidecars, then pull the new orientation
// back into the editor/library state for any of them that is loaded there, so
// a later autosave can't write the old orientation back.
export const rotatePathsOnDisk = (paths: string[], direction: number) => {
  if (paths.length === 0) return;
  paths.forEach((p) => globalImageCache.delete(p));
  invoke(Invokes.ApplyOrientationToPaths, { paths, direction })
    .then(() => reloadAdjustmentsForPaths(paths))
    .catch((err) => {
      console.error(err);
      toast.error(`Failed to rotate selected images: ${err}`);
    });
};

export function useEditorActions() {
  const { t } = useTranslation();
  const setEditor = useEditorStore((s) => s.setEditor);

  const setAdjustments = useCallback(
    (value: Partial<Adjustments> | ((prev: Adjustments) => Adjustments)) => {
      setEditor((state) => {
        const prev = state.adjustments;
        const newAdjustments = typeof value === 'function' ? value(prev) : { ...prev, ...value };
        if (newAdjustments === prev) return state;
        debouncedSetHistory(newAdjustments);
        return {
          adjustments: newAdjustments,
          ...(state.showOriginal ? { showOriginal: false, previewOverride: null } : {}),
        };
      });
    },
    [setEditor],
  );

  const handleRotate = useCallback(
    (degrees: number) => {
      const { selectedImage, adjustments } = useEditorStore.getState();
      const increment = degrees > 0 ? 1 : 3;
      const newAspectRatio = adjustments.aspectRatio ? 1 / adjustments.aspectRatio : adjustments.aspectRatio;
      const newOrientationSteps = ((adjustments.orientationSteps || 0) + increment) % 4;
      // An uncropped image stays uncropped: its aspectRatio may only be the
      // remembered crop-tool ratio, which must not turn into a real crop here.
      const wasUncropped =
        !adjustments.rotation &&
        isFullFrameCrop(
          adjustments.crop,
          selectedImage?.width || 0,
          selectedImage?.height || 0,
          adjustments.orientationSteps || 0,
        );
      const newCrop =
        !wasUncropped && selectedImage?.width && selectedImage?.height
          ? calculateCenteredCrop(selectedImage.width, selectedImage.height, newOrientationSteps, newAspectRatio)
          : null;

      setAdjustments((prev) => ({
        ...prev,
        aspectRatio: newAspectRatio,
        orientationSteps: newOrientationSteps,
        rotation: 0,
        crop: newCrop,
      }));
    },
    [setAdjustments],
  );

  // Rotate the open image (undoable, through the editor's adjustment state)
  // and, when the filmstrip carries a multi-selection, batch-rotate the other
  // selected images through the sidecar command too — a selection behaves the
  // same whether you rotate from the crop panel, the shortcuts, or the
  // library grid.
  //
  // The editor only autosaves an image that has finished loading. So the loaded
  // image is rotated through the editor state (writing it on disk too would race
  // its pending autosave), and everything else — including a library pick that
  // is still loading — is rotated on disk.
  const handleRotateWithSelection = useCallback(
    (degrees: number) => {
      const { selectedImage } = useEditorStore.getState();
      const { multiSelectedPaths, libraryActivePath } = useLibraryStore.getState();
      const inEditor = useUIStore.getState().activeView === 'editor';

      const selection =
        multiSelectedPaths.length > 0 ? multiSelectedPaths : libraryActivePath ? [libraryActivePath] : [];
      const targets = Array.from(
        new Set(inEditor && selectedImage ? [selectedImage.path, ...selection] : selection),
      );
      if (targets.length === 0 && selectedImage) targets.push(selectedImage.path);

      const loadedPath = selectedImage?.isReady ? selectedImage.path : null;
      if (loadedPath && targets.includes(loadedPath)) {
        handleRotate(degrees);
        rotatePathsOnDisk(
          targets.filter((p) => p !== loadedPath),
          degrees,
        );
      } else {
        rotatePathsOnDisk(targets, degrees);
      }
    },
    [handleRotate],
  );

  // Auto-correct with the tools chosen in the Auto options. In keep-look mode the
  // backend analyses the photo as rendered with the current adjustments and returns
  // corrections tagged `_relative`, which are added to the sliders so a preset's look
  // survives; otherwise it returns absolute values measured on the untouched photo.
  const handleAutoAdjustments = useCallback(async () => {
    const { selectedImage, adjustments } = useEditorStore.getState();
    if (!selectedImage?.isReady) return;
    try {
      // Always send the current adjustments: the backend needs them for the raw's
      // baseline exposure (v1.6.6) and, with "keep current look", renders them.
      const result: Record<string, any> = await invoke(Invokes.CalculateAutoAdjustments, {
        adjustments,
      });
      const { _relative: relative, activeTools, ...values } = result;
      const limit = (key: string) => (key === 'exposure' || key === 'brightness' ? 5 : 100);
      setAdjustments((prev: Adjustments) => {
        const next: Record<string, any> = { ...prev };
        Object.entries(values).forEach(([key, value]) => {
          if (typeof value !== 'number') return;
          const current = typeof next[key] === 'number' ? next[key] : 0;
          const raw = relative ? current + value : value;
          next[key] = Math.round(Math.max(-limit(key), Math.min(limit(key), raw)) * 100) / 100;
        });
        next.activeTools = mergeActiveTools(prev, { activeTools: activeTools ?? {} });
        return next as Adjustments;
      });
    } catch (err) {
      toast.error(`Failed to apply auto adjustments: ${err}`);
    }
  }, [setAdjustments]);

  const handleAutoStraighten = useCallback(async () => {
    const { selectedImage, isAutoStraightening } = useEditorStore.getState();
    if (!selectedImage || isAutoStraightening) return;
    setEditor({ isAutoStraightening: true });
    try {
      if (!selectedImage.isReady && !(await waitForImageReady(selectedImage.path))) return;
      const rotation: number | null = await invoke(Invokes.CalculateAutoStraighten, {
        jsAdjustments: useEditorStore.getState().adjustments,
      });
      const loadedImage = useEditorStore.getState().selectedImage;
      if (loadedImage?.path !== selectedImage.path) return;
      if (rotation === null) {
        toast.info(t('editor.crop.autoStraightenNoLines'));
        return;
      }
      setEditor({ liveRotation: null, isStraightenActive: false });
      setAdjustments((prev: Adjustments) => withRotation(prev, loadedImage, rotation));
    } catch (err) {
      toast.error(t('editor.crop.autoStraightenFailed', { error: String(err) }));
    } finally {
      setEditor({ isAutoStraightening: false });
    }
  }, [setAdjustments, setEditor, t]);

  const toggleShowOriginal = useCallback(() => {
    setEditor((state) => {
      const isShowing = !state.showOriginal;

      if (isShowing) {
        const override = { ...INITIAL_ADJUSTMENTS };
        const preservedKeys: Array<keyof Adjustments> = [
          'baselineExposure',
          'crop',
          'rotation',
          'flipHorizontal',
          'flipVertical',
          'orientationSteps',
          'aspectRatio',
          'transformDistortion',
          'transformVertical',
          'transformHorizontal',
          'transformRotate',
          'transformAspect',
          'transformScale',
          'transformXOffset',
          'transformYOffset',
          'lensDistortionAmount',
          'lensVignetteAmount',
          'lensTcaAmount',
          'lensDistortionParams',
          'lensMaker',
          'lensModel',
          'lensDistortionEnabled',
          'lensTcaEnabled',
          'lensVignetteEnabled',
        ];

        preservedKeys.forEach((key) => {
          (override as any)[key] = state.adjustments[key];
        });

        return { showOriginal: true, previewOverride: override };
      } else {
        return { showOriginal: false, previewOverride: null };
      }
    });
  }, [setEditor]);

  const handleLutSelect = useCallback(
    async (path: string, isBuiltIn: boolean = false) => {
      const isAndroid = useSettingsStore.getState().osPlatform === 'android';
      try {
        const result: { size: number } = await invoke('load_and_parse_lut', { path });
        const name =
          isAndroid && path.startsWith('content://')
            ? await invoke<string>('resolve_android_content_uri_name', { uriStr: path })
            : path.split(/[\\/]/).pop() || 'LUT';
        setAdjustments((prev: Adjustments) => ({
          ...prev,
          lutPath: path,
          lutName: name,
          lutSize: result.size,
          lutIntensity: 100,
          lutIsSceneReferred: isBuiltIn,
          activeTools: setToolActive(getActiveTools(prev), 'lut', true),
        }));
      } catch (err) {
        toast.error(`Failed to load LUT: ${err}`);
      }
    },
    [setAdjustments],
  );

  const setLutPreviewOverride = useCallback(
    (path: string | null, isBuiltIn: boolean = false) => {
      setEditor((state) => {
        if (!path) return { previewOverride: null };
        const name = path.split(/[\\/]/).pop() || 'LUT';
        return {
          previewOverride: {
            ...state.adjustments,
            lutPath: path,
            lutName: name,
            lutIntensity: state.adjustments.lutIntensity,
            lutIsSceneReferred: isBuiltIn,
          },
        };
      });
    },
    [setEditor],
  );

  const handleResetAdjustments = useCallback(
    (paths?: string[]) => {
      const { multiSelectedPaths, libraryActivePath, setLibrary } = useLibraryStore.getState();
      const { selectedImage, resetHistory } = useEditorStore.getState();
      const pathsToReset = paths || multiSelectedPaths;
      if (pathsToReset.length === 0) return;

      pathsToReset.forEach((p) => globalImageCache.delete(p));
      debouncedSetHistory.cancel();

      invoke(Invokes.ResetAdjustmentsForPaths, { paths: pathsToReset })
        .then(() => {
          if (libraryActivePath && pathsToReset.includes(libraryActivePath))
            setLibrary({ libraryActiveAdjustments: { ...INITIAL_ADJUSTMENTS } });
          if (selectedImage && pathsToReset.includes(selectedImage.path)) {
            const resetData = { ...INITIAL_ADJUSTMENTS, aiPatches: [] };
            resetHistory(resetData);
            setEditor({ adjustments: resetData });
          }
        })
        .catch((err) => toast.error(`Failed to reset adjustments: ${err}`));
    },
    [setEditor],
  );

  const handleAutoLensCorrection = useCallback((paths?: string[]) => {
    const { multiSelectedPaths } = useLibraryStore.getState();
    const { selectedImage } = useEditorStore.getState();

    const pathsToUpdate =
      paths && paths.length > 0
        ? paths
        : multiSelectedPaths.length > 0
          ? multiSelectedPaths
          : selectedImage
            ? [selectedImage.path]
            : [];

    if (pathsToUpdate.length === 0) return;

    pathsToUpdate.forEach((p) => globalImageCache.delete(p));

    invoke('apply_auto_lens_correction_to_paths', { paths: pathsToUpdate })
      .then(() => reloadAdjustmentsForPaths(pathsToUpdate))
      .catch((err) => toast.error(`Failed to apply auto lens correction: ${err}`));
  }, []);

  const handleAutoStraightenPaths = useCallback(
    (paths: string[]) => {
      const { selectedImage } = useEditorStore.getState();
      const editingPath = useUIStore.getState().activeView === 'editor' ? selectedImage?.path : undefined;
      const batchPaths = paths.filter((path) => path !== editingPath);
      if (batchPaths.length < paths.length) {
        handleAutoStraighten();
      }
      if (batchPaths.length === 0) return;
      batchPaths.forEach((p) => globalImageCache.delete(p));
      invoke(Invokes.ApplyAutoStraightenToPaths, { paths: batchPaths })
        .then(() => reloadAdjustmentsForPaths(batchPaths))
        .catch((err) => toast.error(t('editor.crop.autoStraightenFailed', { error: String(err) })));
    },
    [handleAutoStraighten, t],
  );

  const handleCopyAdjustments = useCallback(async (pathOrEvent?: string | any) => {
    const pathOverride = typeof pathOrEvent === 'string' ? pathOrEvent : undefined;
    const { selectedImage, adjustments } = useEditorStore.getState();
    const { libraryActivePath, multiSelectedPaths } = useLibraryStore.getState();
    let sourceAdjustments: any = null;

    const pathToCopyFrom =
      pathOverride || (selectedImage ? selectedImage.path : libraryActivePath || multiSelectedPaths[0]);

    if (selectedImage && pathToCopyFrom === selectedImage.path) {
      sourceAdjustments = adjustments;
    } else if (pathToCopyFrom) {
      try {
        const meta: any = await invoke(Invokes.LoadMetadata, { path: pathToCopyFrom });
        if (meta?.adjustments && !meta.adjustments.is_null) {
          sourceAdjustments = normalizeLoadedAdjustments(meta.adjustments);
        } else {
          sourceAdjustments = INITIAL_ADJUSTMENTS;
        }
      } catch (err) {
        toast.error(`Failed to load metadata for copying: ${err}`);
        return;
      }
    }

    if (!sourceAdjustments) return;

    const adjustmentsToCopy: any = {};

    for (const key of COPYABLE_ADJUSTMENT_KEYS) {
      if (Object.prototype.hasOwnProperty.call(sourceAdjustments, key)) {
        adjustmentsToCopy[key] = structuredClone(sourceAdjustments[key]);
      }
    }
    useEditorStore.getState().setEditor({ copiedAdjustments: adjustmentsToCopy });
    useProcessStore.getState().setProcess({ isCopied: true });
  }, []);

  const handlePasteAdjustments = useCallback(
    (paths?: string[]) => {
      const { copiedAdjustments, selectedImage, adjustments } = useEditorStore.getState();
      const { multiSelectedPaths } = useLibraryStore.getState();
      const { appSettings } = useSettingsStore.getState();
      const { setProcess } = useProcessStore.getState();

      if (!copiedAdjustments || !appSettings) return;

      const { mode, includedAdjustments } = appSettings.copyPasteSettings;
      const adjustmentsToApply: Partial<Adjustments> = {};

      for (const key of includedAdjustments) {
        if (Object.prototype.hasOwnProperty.call(copiedAdjustments, key)) {
          const value = copiedAdjustments[key as keyof Adjustments];
          if (mode === PasteMode.Merge) {
            const defaultValue = INITIAL_ADJUSTMENTS[key as keyof Adjustments];
            if (JSON.stringify(value) !== JSON.stringify(defaultValue))
              adjustmentsToApply[key as keyof Adjustments] = value;
          } else {
            adjustmentsToApply[key as keyof Adjustments] = value;
          }
        }
      }

      if (includedAdjustments.includes(LensAdjustment.LensMaker)) {
        if (!adjustmentsToApply.lensMaker) {
          adjustmentsToApply.lensDistortionParams = null;
        }
      }

      if (Object.keys(adjustmentsToApply).length === 0) {
        setProcess({ isPasted: true });
        return;
      }

      const pathsToUpdate =
        paths || (multiSelectedPaths.length > 0 ? multiSelectedPaths : selectedImage ? [selectedImage.path] : []);
      if (pathsToUpdate.length === 0) return;

      pathsToUpdate.forEach((p) => globalImageCache.delete(p));

      if (selectedImage && pathsToUpdate.includes(selectedImage.path)) {
        setAdjustments({ ...adjustments, ...adjustmentsToApply });
      }

      invoke(Invokes.ApplyAdjustmentsToPaths, { paths: pathsToUpdate, adjustments: adjustmentsToApply })
        .then(() => {
          if (selectedImage && pathsToUpdate.includes(selectedImage.path)) {
            invoke('load_metadata', { path: selectedImage.path }).then((meta: any) => {
              if (meta.adjustments) {
                setAdjustments((prev: any) => ({
                  ...prev,
                  lensMaker: meta.adjustments.lensMaker,
                  lensModel: meta.adjustments.lensModel,
                  lensDistortionParams: meta.adjustments.lensDistortionParams,
                }));
              }
            });
          }
        })
        .catch((err) => toast.error(`Failed to paste adjustments: ${err}`));

      setProcess({ isPasted: true });
    },
    [setAdjustments],
  );

  const handleZoomChange = useCallback((zoomValue: number, fitToWindow: boolean = false) => {
    const { originalSize, baseRenderSize, adjustments } = useEditorStore.getState();
    const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
    let targetZoomPercent: number;

    const orientationSteps = adjustments.orientationSteps || 0;
    const isSwapped = orientationSteps === 1 || orientationSteps === 3;
    const effectiveOriginalWidth = isSwapped ? originalSize.height : originalSize.width;
    const effectiveOriginalHeight = isSwapped ? originalSize.width : originalSize.height;

    if (fitToWindow) {
      if (
        effectiveOriginalWidth > 0 &&
        effectiveOriginalHeight > 0 &&
        baseRenderSize.width > 0 &&
        baseRenderSize.height > 0
      ) {
        const originalAspect = effectiveOriginalWidth / effectiveOriginalHeight;
        const baseAspect = baseRenderSize.width / baseRenderSize.height;
        targetZoomPercent =
          originalAspect > baseAspect
            ? baseRenderSize.width / effectiveOriginalWidth
            : baseRenderSize.height / effectiveOriginalHeight;
      } else {
        targetZoomPercent = 1.0;
      }
    } else {
      targetZoomPercent = zoomValue / dpr;
    }

    targetZoomPercent = Math.max(0.1 / dpr, Math.min(2.0, targetZoomPercent));

    let transformZoom = 1.0;
    if (
      effectiveOriginalWidth > 0 &&
      effectiveOriginalHeight > 0 &&
      baseRenderSize.width > 0 &&
      baseRenderSize.height > 0
    ) {
      const originalAspect = effectiveOriginalWidth / effectiveOriginalHeight;
      const baseAspect = baseRenderSize.width / baseRenderSize.height;
      if (originalAspect > baseAspect) {
        transformZoom = (targetZoomPercent * effectiveOriginalWidth) / baseRenderSize.width;
      } else {
        transformZoom = (targetZoomPercent * effectiveOriginalHeight) / baseRenderSize.height;
      }
    }
    useEditorStore.getState().setEditor({ zoom: transformZoom });
  }, []);

  return {
    setAdjustments,
    handleRotate,
    handleRotateWithSelection,
    handleAutoAdjustments,
    handleAutoStraighten,
    handleAutoStraightenPaths,
    handleLutSelect,
    setLutPreviewOverride,
    handleResetAdjustments,
    handleAutoLensCorrection,
    handleCopyAdjustments,
    handlePasteAdjustments,
    handleZoomChange,
    toggleShowOriginal,
  };
}
