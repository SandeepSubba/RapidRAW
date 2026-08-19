import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'react-toastify';
import { useLibraryStore } from '../store/useLibraryStore';
import { useEditorStore } from '../store/useEditorStore';
import { useUIStore } from '../store/useUIStore';
import { useProcessStore } from '../store/useProcessStore';
import { Invokes, ImageFile, AlbumItem, Album, AlbumGroup } from '../components/ui/AppProperties';
import { globalImageCache } from '../utils/ImageLRUCache';
import { useSettingsStore } from '../store/useSettingsStore';
import { computeSortedLibrary } from './useSortedLibrary';

export function useLibraryActions(handleImageSelect?: (path: string, openInEditor?: boolean) => void) {
  const handleRate = useCallback((newRating: number, paths?: string[]) => {
    const { multiSelectedPaths, imageRatings, setLibrary } = useLibraryStore.getState();
    const { selectedImage } = useEditorStore.getState();

    const pathsToRate =
      paths || (multiSelectedPaths.length > 0 ? multiSelectedPaths : selectedImage ? [selectedImage.path] : []);
    if (pathsToRate.length === 0) return;

    const currentRating = imageRatings[pathsToRate[0]] || 0;
    const finalRating = newRating === currentRating ? 0 : newRating;

    setLibrary((state) => {
      const newRatings = { ...state.imageRatings };
      pathsToRate.forEach((p) => {
        newRatings[p] = finalRating;
      });
      return { imageRatings: newRatings };
    });

    invoke(Invokes.SetRatingForPaths, { paths: pathsToRate, rating: finalRating }).catch((err) => {
      console.error(err);
      toast.error(`Failed to apply rating: ${err}`);
    });
  }, []);

  // Batch-rotate the selected library images a quarter turn (direction > 0 = clockwise).
  // Rotation is relative/per-image, so it goes through the dedicated backend command rather
  // than apply_adjustments_to_paths. Thumbnails refresh via the `thumbnail-generated` events.
  const handleRotateSelected = useCallback((direction: number, paths?: string[]) => {
    const { multiSelectedPaths, libraryActivePath } = useLibraryStore.getState();
    const { selectedImage } = useEditorStore.getState();

    const pathsToRotate =
      paths ||
      (multiSelectedPaths.length > 0
        ? multiSelectedPaths
        : libraryActivePath
          ? [libraryActivePath]
          : selectedImage
            ? [selectedImage.path]
            : []);
    if (pathsToRotate.length === 0) return;

    invoke(Invokes.ApplyOrientationToPaths, { paths: pathsToRotate, direction }).catch((err) => {
      console.error(err);
      toast.error(`Failed to rotate selected images: ${err}`);
    });
  }, []);

  const handleSetColorLabel = useCallback(async (color: string | null, paths?: string[]) => {
    const { multiSelectedPaths, libraryActivePath, imageList, setLibrary } = useLibraryStore.getState();
    const { selectedImage } = useEditorStore.getState();

    const pathsToUpdate =
      paths || (multiSelectedPaths.length > 0 ? multiSelectedPaths : selectedImage ? [selectedImage.path] : []);
    if (pathsToUpdate.length === 0) return;

    const primaryPath = selectedImage?.path || libraryActivePath;
    const primaryImage = imageList.find((img: ImageFile) => img.path === primaryPath);
    let currentColor = null;
    if (primaryImage && primaryImage.tags) {
      const colorTag = primaryImage.tags.find((tag: string) => tag.startsWith('color:'));
      if (colorTag) currentColor = colorTag.substring(6);
    }
    const finalColor = color !== null && color === currentColor ? null : color;

    try {
      await invoke(Invokes.SetColorLabelForPaths, { paths: pathsToUpdate, color: finalColor });
      setLibrary((state) => ({
        imageList: state.imageList.map((image: ImageFile) => {
          if (pathsToUpdate.includes(image.path)) {
            const otherTags = (image.tags || []).filter((tag: string) => !tag.startsWith('color:'));
            const newTags = finalColor ? [...otherTags, `color:${finalColor}`] : otherTags;
            return { ...image, tags: newTags };
          }
          return image;
        }),
      }));
    } catch (err) {
      toast.error(`Failed to set color label: ${err}`);
    }
  }, []);

  const handleTagsChanged = useCallback((changedPaths: string[], newTags: { tag: string; isUser: boolean }[]) => {
    useLibraryStore.getState().setLibrary((state) => ({
      imageList: state.imageList.map((image) => {
        if (changedPaths.includes(image.path)) {
          const colorTags = (image.tags || []).filter((t) => t.startsWith('color:'));
          const prefixedNewTags = newTags.map((t) => (t.isUser ? `user:${t.tag}` : t.tag));
          const finalTags = [...colorTags, ...prefixedNewTags].sort();
          return { ...image, tags: finalTags.length > 0 ? finalTags : null };
        }
        return image;
      }),
    }));
  }, []);

  const handleUpdateExif = useCallback(async (paths: Array<string> | undefined, updates: Record<string, string>) => {
    const { multiSelectedPaths, imageList, setLibrary } = useLibraryStore.getState();
    const { selectedImage, setEditor } = useEditorStore.getState();

    const pathsToUpdate =
      paths && paths.length > 0
        ? paths
        : multiSelectedPaths.length > 0
          ? multiSelectedPaths
          : selectedImage
            ? [selectedImage.path]
            : [];
    if (pathsToUpdate.length === 0) return;

    const physicalPathsSet = new Set(pathsToUpdate.map((p) => p.split('?vc=')[0]));
    const physicalPathsArray = Array.from(physicalPathsSet);

    try {
      await invoke(Invokes.UpdateExifFields, { paths: physicalPathsArray, updates });

      setEditor((state) => {
        if (!state.selectedImage || !physicalPathsSet.has(state.selectedImage.path.split('?vc=')[0])) return state;
        return { selectedImage: { ...state.selectedImage, exif: { ...(state.selectedImage.exif || {}), ...updates } } };
      });

      setLibrary((state) => ({
        imageList: state.imageList.map((img) => {
          if (physicalPathsSet.has(img.path.split('?vc=')[0])) {
            return { ...img, exif: { ...(img.exif || {}), ...updates } };
          }
          return img;
        }),
      }));

      pathsToUpdate.forEach((p) => {
        const cached = globalImageCache.get(p);
        if (cached && cached.selectedImage) {
          globalImageCache.set(p, {
            ...cached,
            selectedImage: { ...cached.selectedImage, exif: { ...(cached.selectedImage.exif || {}), ...updates } },
          });
        }
      });
    } catch (err) {
      toast.error(`Failed to update metadata: ${err}`);
    }
  }, []);

  // Rename a single image's file on disk to `newName` (no extension — it's kept),
  // then reflect the new path across the path-keyed stores in place so the grid,
  // filmstrip, selection, and open editor all follow without a full folder reload.
  const handleRenameToName = useCallback(async (path: string, newName: string): Promise<string | null> => {
    const cleanName = (newName || '')
      .trim()
      .replace(/\.[^.]*$/, '') // drop any extension the model included
      .replace(/[\\/:*?"<>|]/g, '') // strip characters illegal in filenames
      .trim();
    if (!cleanName) {
      toast.error('The assistant proposed an invalid file name.');
      return null;
    }
    const physicalPath = path.split('?vc=')[0];
    try {
      const newPaths: Array<string> = await invoke(Invokes.RenameFiles, {
        nameTemplate: cleanName,
        paths: [physicalPath],
        // The assistant can't see the directory, so let the backend settle
        // collisions with a -001 suffix rather than failing the rename.
        uniqueSuffix: true,
      });
      const newPath = newPaths?.[0];
      if (!newPath || newPath === physicalPath) return newPath || null;

      const matches = (p: string | null | undefined) => !!p && p.split('?vc=')[0] === physicalPath;
      const remap = (p: string) => (matches(p) ? newPath : p);

      const { setLibrary } = useLibraryStore.getState();
      setLibrary((state) => {
        const imageRatings = { ...state.imageRatings };
        if (imageRatings[physicalPath] !== undefined) {
          imageRatings[newPath] = imageRatings[physicalPath];
          delete imageRatings[physicalPath];
        }
        return {
          imageList: state.imageList.map((img) => (matches(img.path) ? { ...img, path: newPath } : img)),
          imageRatings,
          libraryActivePath: matches(state.libraryActivePath) ? newPath : state.libraryActivePath,
          multiSelectedPaths: state.multiSelectedPaths.map(remap),
          selectionAnchorPath: matches(state.selectionAnchorPath) ? newPath : state.selectionAnchorPath,
        };
      });

      const { selectedImage, setEditor } = useEditorStore.getState();
      if (matches(selectedImage?.path)) {
        setEditor({ selectedImage: { ...selectedImage!, path: newPath } });
      }

      // Carry over the cached thumbnail and edit-state so nothing flashes blank.
      const { thumbnails, setProcess } = useProcessStore.getState();
      if (thumbnails[physicalPath]) {
        setProcess({ thumbnails: { ...thumbnails, [newPath]: thumbnails[physicalPath] } });
      }
      const cached = globalImageCache.get(physicalPath);
      if (cached) globalImageCache.set(newPath, cached);

      return newPath;
    } catch (err) {
      toast.error(`Failed to rename file: ${err}`);
      return null;
    }
  }, []);

  const handleClearSelection = useCallback(() => {
    const activeView = useUIStore.getState().activeView;
    const { selectedImage } = useEditorStore.getState();

    if (activeView === 'editor' && selectedImage) {
      useLibraryStore.getState().setLibrary({
        multiSelectedPaths: [selectedImage.path],
        libraryActivePath: selectedImage.path,
        selectionAnchorPath: selectedImage.path,
      });
    } else {
      useLibraryStore.getState().setLibrary({
        multiSelectedPaths: [],
        libraryActivePath: null,
        selectionAnchorPath: null,
      });

      useEditorStore.getState().setEditor({
        selectedImage: null,
        finalPreviewUrl: null,
        uncroppedAdjustedPreviewUrl: null,
        histogram: null,
        waveform: null,
        activeMaskId: null,
        activeMaskContainerId: null,
        activeAiPatchContainerId: null,
        activeAiSubMaskId: null,
        isWbPickerActive: false,
        transformedOriginalUrl: null,
      });
    }
  }, []);

  const handleMultiSelectClick = useCallback(
    (
      path: string,
      event: any,
      options: {
        onSimpleClick(p: string, isAlreadySelected: boolean): void;
        updateLibraryActivePath: boolean;
        shiftAnchor: string | null;
      },
    ) => {
      const libraryState = useLibraryStore.getState();
      const { multiSelectedPaths, setLibrary } = libraryState;
      const { ctrlKey, metaKey, shiftKey } = event;
      const isCtrlPressed = ctrlKey || metaKey;
      const { shiftAnchor, onSimpleClick, updateLibraryActivePath } = options;

      const isAlreadySelected = multiSelectedPaths.includes(path);

      if (shiftKey && shiftAnchor) {
        const sortedImageList = computeSortedLibrary(libraryState, useSettingsStore.getState());
        const anchorIndex = sortedImageList.findIndex((f) => f.path === shiftAnchor);
        const currentIndex = sortedImageList.findIndex((f) => f.path === path);

        if (anchorIndex !== -1 && currentIndex !== -1) {
          const start = Math.min(anchorIndex, currentIndex);
          const end = Math.max(anchorIndex, currentIndex);
          const range = sortedImageList.slice(start, end + 1).map((f) => f.path);
          const baseSelection = isCtrlPressed ? multiSelectedPaths : [];
          const newSelection = Array.from(new Set([...baseSelection, ...range]));

          setLibrary({ multiSelectedPaths: newSelection, selectionAnchorPath: path });
          if (updateLibraryActivePath) setLibrary({ libraryActivePath: path });
        }
      } else if (isCtrlPressed) {
        const newSelection = new Set(multiSelectedPaths);
        if (newSelection.has(path)) newSelection.delete(path);
        else newSelection.add(path);

        const newSelectionArray = Array.from(newSelection);
        setLibrary({ multiSelectedPaths: newSelectionArray, selectionAnchorPath: path });

        if (updateLibraryActivePath) {
          if (newSelectionArray.includes(path)) setLibrary({ libraryActivePath: path });
          else if (newSelectionArray.length > 0)
            setLibrary({ libraryActivePath: newSelectionArray[newSelectionArray.length - 1] });
          else setLibrary({ libraryActivePath: null });
        }
      } else {
        onSimpleClick(path, isAlreadySelected);
      }
    },
    [],
  );

  const handleLibraryImageSingleClick = useCallback(
    (path: string, event: any) => {
      const { selectionAnchorPath, libraryActivePath, setLibrary } = useLibraryStore.getState();
      handleMultiSelectClick(path, event, {
        shiftAnchor: selectionAnchorPath ?? libraryActivePath,
        updateLibraryActivePath: true,
        onSimpleClick: (p: string, isAlreadySelected: boolean) => {
          if (isAlreadySelected) {
            setLibrary({ libraryActivePath: p, selectionAnchorPath: p });
          } else {
            setLibrary({ multiSelectedPaths: [p], libraryActivePath: p, selectionAnchorPath: p });
          }
          if (handleImageSelect) {
            handleImageSelect(p, false);
          }
        },
      });
    },
    [handleMultiSelectClick, handleImageSelect],
  );

  const handleImageClick = useCallback(
    (path: string, event: any) => {
      const { selectionAnchorPath, libraryActivePath, setLibrary } = useLibraryStore.getState();
      const { selectedImage } = useEditorStore.getState();
      const inEditor = !!selectedImage;

      handleMultiSelectClick(path, event, {
        shiftAnchor: selectionAnchorPath ?? (inEditor ? selectedImage.path : libraryActivePath),
        updateLibraryActivePath: !inEditor,
        onSimpleClick: (p: string, isAlreadySelected: boolean) => {
          if (!isAlreadySelected) {
            setLibrary({ multiSelectedPaths: [p] });
          }
          if (handleImageSelect) handleImageSelect(p);
          setLibrary({ selectionAnchorPath: p });
        },
      });
    },
    [handleMultiSelectClick, handleImageSelect],
  );

  const refreshAllFolderTrees = useCallback(async () => {
    const { rootPaths, expandedFolders, setLibrary } = useLibraryStore.getState();
    const { appSettings } = useSettingsStore.getState();

    const showImageCounts = appSettings?.enableFolderImageCounts ?? false;
    const pinnedFolders = appSettings?.pinnedFolders || [];
    const expandedArray = Array.from(expandedFolders);

    try {
      const updates: any = {};

      if (rootPaths && rootPaths.length > 0) {
        const treesData = await invoke(Invokes.GetPinnedFolderTrees, {
          paths: rootPaths,
          expandedFolders: expandedArray,
          showImageCounts,
        });
        updates.folderTrees = treesData;
      } else {
        updates.folderTrees = [];
      }

      if (pinnedFolders && pinnedFolders.length > 0) {
        const pinnedTreesData = await invoke(Invokes.GetPinnedFolderTrees, {
          paths: pinnedFolders,
          expandedFolders: expandedArray,
          showImageCounts,
        });
        updates.pinnedFolderTrees = pinnedTreesData;
      } else {
        updates.pinnedFolderTrees = [];
      }

      if (Object.keys(updates).length > 0) {
        setLibrary(updates);
      }
    } catch (err) {
      console.error('Failed to refresh folder trees:', err);
    }
  }, []);

  const handleTogglePinFolder = useCallback(async (path: string) => {
    const { appSettings, handleSettingsChange } = useSettingsStore.getState();
    const { expandedFolders, setLibrary } = useLibraryStore.getState();
    if (!appSettings) return;

    const currentPins = appSettings.pinnedFolders || [];
    const isPinned = currentPins.includes(path);
    const newPins = isPinned
      ? currentPins.filter((p: string) => p !== path)
      : [...currentPins, path].sort((a, b) => a.localeCompare(b));

    handleSettingsChange({ ...appSettings, pinnedFolders: newPins });

    try {
      const trees = await invoke(Invokes.GetPinnedFolderTrees, {
        paths: newPins,
        expandedFolders: Array.from(expandedFolders),
        showImageCounts: appSettings.enableFolderImageCounts ?? false,
      });
      setLibrary({ pinnedFolderTrees: trees });
    } catch (err) {
      toast.error(`Failed to refresh pinned folders: ${err}`);
    }
  }, []);

  const handleCreateAlbumItem = useCallback(async (name: string, type: 'album' | 'group') => {
    const { albumTree, setLibrary } = useLibraryStore.getState();
    const { albumActionTarget } = useUIStore.getState();

    const newTree = structuredClone(albumTree);
    const newItem: AlbumItem =
      type === 'album'
        ? ({ type: 'album', id: crypto.randomUUID(), name, images: [] } as Album)
        : ({ type: 'group', id: crypto.randomUUID(), name, children: [] } as AlbumGroup);

    let actualTarget = albumActionTarget;

    const findNode = (nodes: AlbumItem[], id: string): AlbumItem | undefined => {
      for (const n of nodes) {
        if (n.id === id) return n;
        if (n.type === 'group') {
          const found = findNode((n as AlbumGroup).children, id);
          if (found) return found;
        }
      }
      return undefined;
    };

    const findParentId = (nodes: AlbumItem[], childId: string, parentId: string | null): string | null | undefined => {
      for (const n of nodes) {
        if (n.id === childId) return parentId;
        if (n.type === 'group') {
          const found = findParentId((n as AlbumGroup).children, childId, n.id);
          if (found !== undefined) return found;
        }
      }
      return undefined;
    };

    if (actualTarget) {
      const targetNode = findNode(newTree, actualTarget);
      if (targetNode && targetNode.type === 'album') {
        const pId = findParentId(newTree, actualTarget, null);
        actualTarget = pId === undefined ? null : pId;
      }
    }

    const insert = (nodes: AlbumItem[], target: string | null): boolean => {
      if (!target) {
        nodes.push(newItem);
        return true;
      }
      for (const n of nodes) {
        if (n.id === target && n.type === 'group') {
          (n as AlbumGroup).children.push(newItem);
          return true;
        } else if (n.type === 'group') {
          if (insert((n as AlbumGroup).children, target)) return true;
        }
      }
      return false;
    };

    if (insert(newTree, actualTarget)) {
      try {
        await invoke(Invokes.SaveAlbums, { tree: newTree });
        const sortedTree = await invoke(Invokes.GetAlbums);
        setLibrary({ albumTree: sortedTree as AlbumItem[] });
      } catch (err) {
        toast.error(`Failed to create: ${err}`);
      }
    }
  }, []);

  const handleRenameAlbumItem = useCallback(async (newName: string) => {
    const { albumTree, setLibrary } = useLibraryStore.getState();
    const { albumActionTarget } = useUIStore.getState();
    if (!albumActionTarget) return;

    const newTree = structuredClone(albumTree);

    const rename = (nodes: AlbumItem[]) => {
      for (const n of nodes) {
        if (n.id === albumActionTarget) {
          n.name = newName;
          return true;
        }
        if (n.type === 'group' && rename((n as AlbumGroup).children)) return true;
      }
      return false;
    };

    if (rename(newTree)) {
      try {
        await invoke(Invokes.SaveAlbums, { tree: newTree });
        const sortedTree = await invoke(Invokes.GetAlbums);
        setLibrary({ albumTree: sortedTree as AlbumItem[] });
      } catch (err) {
        toast.error(`Failed to rename: ${err}`);
      }
    }
  }, []);

  return {
    handleRate,
    handleRotateSelected,
    handleSetColorLabel,
    handleTagsChanged,
    handleUpdateExif,
    handleRenameToName,
    handleClearSelection,
    handleLibraryImageSingleClick,
    handleImageClick,
    refreshAllFolderTrees,
    handleTogglePinFolder,
    handleCreateAlbumItem,
    handleRenameAlbumItem,
  };
}
