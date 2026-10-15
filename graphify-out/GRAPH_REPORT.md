# 📊 Graph Analysis Report

**Root:** `.`

## Summary

| Metric | Value |
|--------|-------|
| Nodes | 5588 |
| Edges | 7086 |
| Communities | 290 |
| Hyperedges | 0 |

### Confidence Breakdown

| Level | Count | Percentage |
|-------|-------|------------|
| EXTRACTED | 5359 | 75.6% |
| INFERRED | 1727 | 24.4% |
| AMBIGUOUS | 0 | 0.0% |

## 🌟 God Nodes (Most Connected)

| Node | Degree | Community |
|------|--------|-----------|
| file_management | 162 | 0 |
| AssistantPanel | 115 | 2 |
| MasksPanel | 100 | 1 |
| exif_processing | 98 | 16 |
| SettingsPanel | 91 | 3 |
| image_processing | 90 | 10 |
| App | 90 | 4 |
| export_processing | 88 | 6 |
| CropPanel | 85 | 5 |
| AIPanel | 80 | 7 |

## 🔮 Surprising Connections

- **src_components_panel_library_tethermenu_tsx_handleconfig** → **src_components_panel_library_tethermenu_tsx_run** (calls)
- **src_components_panel_right_assistantpanel_tsx_applypointcolors** → **src_components_panel_right_assistantpanel_tsx_sampleviewcolor** (calls)
- **src_utils_croputils_ts_calculateareapreservingcrop** → **src_utils_croputils_ts_getorienteddimensions** (calls)
- **src_utils_croputils_ts_calculateautocropforrotation** → **src_utils_croputils_ts_iscropwithinbounds** (calls)
- **src_tauri_src_ai_processing_rs_download_and_verify_model** → **src_tauri_src_ai_processing_rs_download_model** (calls)

## 🏘️ Communities

### Community 0 — view_pixels_match_a_library_rotate_then_crop() (132 nodes, cohesion: 0.02)

- file_management
- apply_auto_lens_correction()
- assistant_prepare_image()
- assistant_prepare_label()
- assistant_prepare_region()
- cached_edited_thumbnail()
- clear_all_sidecars()
- clear_thumbnail_cache()
- collect_top_level_preset_names()
- compute_thumbnail_cache_hash()
- copy_files()
- create_folder()
- delete_files_from_disk()
- delete_folder()
- emit_image_metadata_loaded()
- encode_assistant_jpeg()
- encode_thumbnail()
- ExportPresetFile
- extension_is_preserved_and_stem_not_mangled()
- extract_xmp_label()
- _…and 112 more_

### Community 1 — setCombinedRef() (101 nodes, cohesion: 0.02)

- MasksPanel
- handleMouseEnter()
- handleMouseLeave()
- handleRenameSubmit()
- if()
- ../../adjustments/Basic/BasicAdjustments
- ../../adjustments/Color/ColorPanel
- ../../adjustments/Curves/CurveGraph
- ../../adjustments/Details/DetailsPanel
- ../../adjustments/Effects/EffectsPanel
- ./AdjustmentSectionsSubMenu/AdjustmentSectionsSubMenu
- clsx/clsx
- ../../../context/ContextMenuContext/useContextMenu
- @dnd-kit/core/DndContext
- @dnd-kit/core/DragEndEvent
- @dnd-kit/core/DragOverlay
- @dnd-kit/core/DragStartEvent
- @dnd-kit/core/PointerSensor
- @dnd-kit/core/pointerWithin
- @dnd-kit/core/useDraggable
- _…and 81 more_

### Community 2 — wantsLabelInspect() (97 nodes, cohesion: 0.02)

- AssistantPanel
- applyScannerPatch()
- blobUrlToImage()
- buildLibraryContext()
- changedKeys()
- commitRename()
- dataUrlToImage()
- downscaleBlob()
- fileNameOf()
- fileToAttachment()
- formatMetadata()
- handleKeyDown()
- @clerk/react/useAuth
- clsx/clsx
- ../../../hooks/useAiMasking/getTransformAdjustments
- ../../../hooks/useEditorActions/useEditorActions
- ../../../hooks/useLibraryActions/useLibraryActions
- ../../../hooks/useSortedLibrary/computeSortedLibrary
- lucide-react/AlertTriangle
- lucide-react/Bot
- _…and 77 more_

### Community 3 — refreshAssistantModels() (92 nodes, cohesion: 0.02)

- SettingsPanel
- async()
- closeConfirmModal()
- clsx()
- executeClearAiTags()
- executeClearCache()
- executeClearSidecars()
- executeClearTags()
- executeResetLayout()
- handleAddAiTag()
- handleAddLens()
- handleAddShortcut()
- handleAdjustmentStepSave()
- handleAiTagInputKeyDown()
- handleAssistantTest()
- handleClearAiTags()
- handleClearCache()
- handleClearSidecars()
- handleClearTags()
- handleInputKeyDown()
- _…and 72 more_

### Community 4 — insertChildrenIntoTree() (91 nodes, cohesion: 0.02)

- App
- createResizeHandler()
- createResizeResetHandler()
- getDynamicCompactPanelHeight()
- imageDragModifier()
- @clerk/react/ClerkProvider
- clsx/clsx
- ./components/managers/ImageLoaderManager/ImageLoaderManager
- ./components/managers/ImageProcessingManager/ImageProcessingManager
- ./components/modals/AppModals/AppModals
- ./components/panel/library/LiveViewOverlay/LiveViewOverlay
- ./components/panel/PanelSwitcher/PANEL_ICONS
- ./components/panel/right/AIPanel/AIPanel
- ./components/panel/right/AssistantPanel/AssistantPanel
- ./components/panel/right/ControlsPanel/Controls
- ./components/panel/right/CropPanel/CropPanel
- ./components/panel/right/ExportPanel/ExportPanel
- ./components/panel/right/FolderTree/FolderTree
- ./components/panel/right/MasksPanel/MasksPanel
- ./components/panel/right/MetadataPanel/MetadataPanel
- _…and 71 more_

### Community 5 — toggleSection() (86 nodes, cohesion: 0.02)

- CropPanel
- formatRatioLabel()
- getOrientationTooltip()
- getOverlayTooltip()
- greatestCommonDivisor()
- handleApplyCustomRatio()
- handleCustomInputChange()
- handleCustomInputFocus()
- handleCustomInputKeyDown()
- handleFineRotationChange()
- handleLensContextMenu()
- handleOverlayCycle()
- handlePresetClick()
- handleRemoveSavedRatio()
- handleReset()
- handleSaveCustomRatio()
- handleTransformContextMenu()
- clsx/clsx
- ../../../context/ContextMenuContext/useContextMenu
- framer-motion/AnimatePresence
- _…and 66 more_

### Community 6 — WatermarkSettings (81 nodes, cohesion: 0.03)

- export_processing
- apply_export_geometry()
- apply_export_geometry_and_watermark()
- apply_watermark()
- BorderBasis
- bordered_size()
- BorderOptions
- build_single_mask_adjustments()
- calculate_border_thickness()
- calculate_pad_target()
- calculate_resize_target()
- cancel_export()
- component_matches()
- compose_image_on_canvas()
- compute_export_geometry()
- compute_export_output_size()
- compute_fused_geometry()
- encode_grayscale_to_png()
- ensure_export_not_cancelled()
- exceeds_max_dimension()
- _…and 61 more_

### Community 7 — setCombinedRef() (7) (81 nodes, cohesion: 0.03)

- AIPanel
- handleMouseEnter()
- handleMouseLeave()
- handleRenameSubmit()
- if()
- ../../../context/ContextMenuContext/useContextMenu
- @dnd-kit/core/DndContext
- @dnd-kit/core/DragEndEvent
- @dnd-kit/core/DragOverlay
- @dnd-kit/core/DragStartEvent
- @dnd-kit/core/PointerSensor
- @dnd-kit/core/pointerWithin
- @dnd-kit/core/useDraggable
- @dnd-kit/core/useDroppable
- @dnd-kit/core/useSensor
- @dnd-kit/core/useSensors
- framer-motion/AnimatePresence
- framer-motion/motion
- ../../../hooks/useAiMasking/useAiMasking
- ../../../hooks/useCloudUsage/useCloudUsage
- _…and 61 more_

### Community 8 — useAppContextMenus() (77 nodes, cohesion: 0.03)

- useAppContextMenus
- ../components/ui/AppProperties/Album
- ../components/ui/AppProperties/AlbumGroup
- ../components/ui/AppProperties/AlbumItem
- ../components/ui/AppProperties/ImageFlag
- ../components/ui/AppProperties/Invokes
- ../components/ui/AppProperties/Option
- ../components/ui/AppProperties/OPTION_SEPARATOR
- ../components/ui/AppProperties/Panel
- ../context/ContextMenuContext/useContextMenu
- ../context/TaggingSubMenu/TaggingSubMenu
- lucide-react/Album
- lucide-react/Aperture
- lucide-react/Ban
- lucide-react/Briefcase
- lucide-react/Camera
- lucide-react/Car
- lucide-react/Check
- lucide-react/ClipboardPaste
- lucide-react/Copy
- _…and 57 more_

### Community 9 — truncate() (74 nodes, cohesion: 0.05)

- assistant
- anthropic_content()
- apply_edits_tool()
- assistant_chat()
- assistant_cull()
- assistant_dev_cancel()
- assistant_dev_chat()
- assistant_list_models()
- assistant_test_connection()
- AssistantResponse
- build_cli_prompt()
- build_url()
- call_anthropic()
- call_claude_code()
- call_openai_compatible()
- ChatMessage
- claude_cli_oauth_token()
- corr()
- corrections_and_habits_reach_the_prompt()
- credentials_json_token_extraction()
- _…and 54 more_

### Community 10 — yc_to_rgb() (72 nodes, cohesion: 0.04)

- image_processing
- AdjustmentScales
- AllAdjustments
- apply_cpu_agx_tonemap()
- apply_cpu_default_raw_processing()
- apply_gentle_detail_enhance()
- apply_linear_to_srgb()
- apply_orientation()
- apply_srgb_to_linear()
- AutoAdjustmentResults
- build_transform_matrices()
- calculate_agx_matrices_glam()
- ColorCalibrationSettings
- ColorGradeSettings
- compute_lens_auto_crop_scale()
- compute_white_balance_sample()
- Crop
- deserialize_image_flag()
- downscale_f32_image()
- GeometryParams
- _…and 52 more_

### Community 11 — linearToSrgb8() (72 nodes, cohesion: 0.03)

- ImageCanvas
- commitDrag()
- for()
- getCropDimensions()
- handleStraightenMouseDown()
- handleStraightenMouseLeave()
- handleStraightenMouseMove()
- handleStraightenMouseUp()
- ../../../hooks/useImageRenderSize/RenderSize
- ../../../hooks/useOsPlatform/useOsPlatform
- lucide-react/Bandage
- lucide-react/BrushCleaning
- lucide-react/Spline
- lucide-react/Stamp
- ./overlays/CompositionOverlays/CompositionOverlays
- react-i18next/useTranslation
- react-image-crop/Crop
- react-image-crop/dist/ReactCrop.css
- react-image-crop/PercentCrop
- react-image-crop/ReactCrop
- _…and 52 more_

### Community 12 — WgpuTransformPayload (70 nodes, cohesion: 0.03)

- lib
- apply_adjustments()
- cancel_thumbnail_generation()
- CommunityPreset
- compute_full_transformed_res()
- compute_patched_and_warped()
- fetch_community_presets()
- force_exit()
- frontend_log()
- frontend_ready()
- generate_all_community_previews()
- generate_preset_preview()
- generate_preview_for_path()
- generate_transformed_preview()
- generate_uncropped_preview()
- get_cached_full_warped_image()
- get_cached_full_warped_image_for_path()
- get_image_dimensions()
- get_log_file_path()
- get_original_image()
- _…and 50 more_

### Community 13 — setActiveItem() (67 nodes, cohesion: 0.03)

- PresetsPanel
- DroppableFolderItem()
- FolderItemDisplay()
- ../../../context/ContextMenuContext/useContextMenu
- @dnd-kit/core/DndContext
- @dnd-kit/core/DragOverlay
- @dnd-kit/core/PointerSensor
- @dnd-kit/core/useDraggable
- @dnd-kit/core/useDroppable
- @dnd-kit/core/useSensor
- @dnd-kit/core/useSensors
- framer-motion/AnimatePresence
- framer-motion/motion
- ../../../hooks/useEditorActions/useEditorActions
- ../../../hooks/usePresets/PresetImportFailure
- ../../../hooks/usePresets/PresetListType
- ../../../hooks/usePresets/usePresets
- ../../../hooks/usePresets/UserPreset
- lucide-react/CopyPlus
- lucide-react/Edit
- _…and 47 more_

### Community 14 — try_fast_embedded_preview() (67 nodes, cohesion: 0.05)

- culling
- analyze_image()
- analyze_paths()
- assistant_group_images()
- best_region_sharpness()
- build_cr2_like()
- calculate_exposure_metric()
- calculate_laplacian_variance()
- candidates_are_largest_first_and_all_start_with_soi()
- canon_layout_full_preview_wins_over_thumb_and_raw_track()
- color_hist_distance()
- color_histogram()
- cull_images()
- CullGroup
- CullingProgress
- CullingSettings
- CullingSuggestions
- decode_jpeg_scaled()
- entry()
- face_exposure()
- _…and 47 more_

### Community 15 — toggleSection() (15) (66 nodes, cohesion: 0.03)

- FolderTree
- clsx()
- filterAlbumTree()
- filterTree()
- getAutoExpandedAlbumGroups()
- getAutoExpandedPaths()
- handleEmptyAreaContextMenu()
- handleFolderIconClick()
- handleNameClick()
- handleNameDoubleClick()
- clsx/clsx
- @dnd-kit/core/useDroppable
- framer-motion/AnimatePresence
- framer-motion/LayoutGroup
- framer-motion/motion
- ../../../hooks/useLibraryActions/useLibraryActions
- lucide-react/Album
- lucide-react/ArrowLeft
- lucide-react/ArrowRight
- lucide-react/Briefcase
- _…and 46 more_

### Community 16 — try_get_exif_creation_date() (62 nodes, cohesion: 0.05)

- exif_processing
- CachedExifEntry
- clean_creation_datetime_str()
- creation_datetime_to_utc()
- declared_segment_length_matches_the_bytes_written()
- decode_user_comment()
- embeddable_formats_are_reported_correctly()
- ExifCacheState
- .get_cache_file_path()
- flush_all_dirty_caches()
- fmt_date_str()
- format_min_max()
- get_creation_date_from_bytes()
- get_creation_date_from_path()
- get_exif_cache()
- get_exif_from_rrcache()
- get_file_stamp()
- has_embedded_xmp()
- has_embedded_xmp_is_false_for_a_plain_encode()
- heal_cached_user_comment()
- _…and 42 more_

### Community 17 — updateSetting() (61 nodes, cohesion: 0.03)

- TetheringPanel
- captureImage()
- clsx()
- commitValue()
- connectCamera()
- generatePresetSubmenu()
- handleOpenPresetMenu()
- IconColorTemp()
- clsx/clsx
- ../../../context/ContextMenuContext/useContextMenu
- ../editor/ExifIcons/IconAperture
- ../editor/ExifIcons/IconIso
- ../editor/ExifIcons/IconShutter
- ../editor/overlays/CompositionOverlays/CompositionOverlays
- ../../../hooks/usePresets/usePresets
- lucide-react/Battery
- lucide-react/BatteryCharging
- lucide-react/BatteryFull
- lucide-react/BatteryLow
- lucide-react/BatteryMedium
- _…and 41 more_

### Community 18 — ValidatedNumberInput() (59 nodes, cohesion: 0.03)

- ExportPanel
- ColorField()
- getPositionStyles()
- framer-motion/AnimatePresence
- framer-motion/motion
- ../../../hooks/useExportSettings/useExportSettings
- ../../../hooks/useOsPlatform/useOsPlatform
- lodash.debounce/debounce
- lucide-react/Ban
- lucide-react/CheckCircle
- lucide-react/ChevronDown
- lucide-react/ChevronRight
- lucide-react/FileInput
- lucide-react/Loader
- lucide-react/Settings
- lucide-react/X
- lucide-react/XCircle
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- _…and 39 more_

### Community 19 — setLibraryDisplayMode() (58 nodes, cohesion: 0.03)

- MainLibrary
- framer-motion/AnimatePresence
- framer-motion/motion
- ./library/CullingView/CullingView
- ./library/LibraryGrid/LibraryGrid
- ./library/LibraryHeader/SearchInput
- ./library/LibraryHeader/ViewOptionsDropdown
- ./library/TetherMenu/TetherMenu
- lucide-react/AlertTriangle
- lucide-react/Check
- lucide-react/Columns
- lucide-react/Folder
- lucide-react/FolderInput
- lucide-react/HardDriveDownload
- lucide-react/Home
- lucide-react/LayoutGrid
- lucide-react/Loader2
- lucide-react/RefreshCw
- lucide-react/Rows3
- lucide-react/Search
- _…and 38 more_

### Community 20 — update() (57 nodes, cohesion: 0.04)

- ControlsPanel
- describeAutoLearning()
- if()
- ../../adjustments/Basic/BasicAdjustments
- ../../adjustments/Color/ColorPanel
- ../../adjustments/Curves/CurveGraph
- ../../adjustments/Details/DetailsPanel
- ../../adjustments/Effects/EffectsPanel
- ../../adjustments/FilmPanel/FilmPanel
- ./AdjustmentSectionsSubMenu/AdjustmentSectionsSubMenu
- clsx/clsx
- ../../../context/ContextMenuContext/useContextMenu
- ../editor/Waveform/Waveform
- framer-motion/AnimatePresence
- framer-motion/motion
- ../../../hooks/useEditorActions/useEditorActions
- ../../../hooks/useWaveformControls/useWaveformControls
- ../library/TetherMenu/CameraSection
- lucide-react/Aperture
- lucide-react/ChartArea
- _…and 37 more_

### Community 21 — toggleMode() (53 nodes, cohesion: 0.04)

- LibraryHeader
- clearSearch()
- getActiveIndex()
- handleInputChange()
- handleKeyDown()
- clsx/clsx
- framer-motion/AnimatePresence
- framer-motion/motion
- ../../../hooks/useSortedLibrary/ADVANCED_QUERY_REGEX
- lucide-react/Check
- lucide-react/ChevronDown
- lucide-react/ChevronUp
- lucide-react/HelpCircle
- lucide-react/Loader2
- lucide-react/Search
- lucide-react/SlidersHorizontal
- lucide-react/Star
- lucide-react/X
- react-i18next/useTranslation
- react/React
- _…and 33 more_

### Community 22 — write_scan_sidecar() (52 nodes, cohesion: 0.08)

- scanning
- auto_tone_for()
- average_scans()
- average_scans_midpoints_two_passes()
- base_sample_reads_density_and_follows_orientation()
- detect_frame_crop()
- detect_frame_rect()
- fill_masked()
- FilmMeta
- .field()
- frame_crop_trims_holder_bars_and_lamp_area()
- crate::tethering::unique_path
- serde::Serialize
- std::io::Read
- std::path::{Path, PathBuf}
- std::process::{Child, Command, Stdio}
- std::sync::{Arc, Mutex}
- std::sync::atomic::{AtomicBool, AtomicU64, Ordering}
- super::{average_scans, detect_frame_crop, fill_masked, ir_defect_mask}
- tauri::{AppHandle, Emitter}
- _…and 32 more_

### Community 23 — CullingPreview() (51 nodes, cohesion: 0.04)

- CullingView
- clsx()
- CullingPreview()
- clsx/clsx
- ../editor/ExifIcons/IconAperture
- ../editor/ExifIcons/IconFocalLength
- ../editor/ExifIcons/IconIso
- ../editor/ExifIcons/IconShutter
- framer-motion/AnimatePresence
- framer-motion/motion
- ../../../hooks/useLibraryActions/useLibraryActions
- ./LibraryItems/Thumbnail
- lucide-react/Check
- lucide-react/Info
- lucide-react/Link
- lucide-react/Loader2
- lucide-react/Maximize
- lucide-react/Plus
- lucide-react/SlidersHorizontal
- lucide-react/SquarePen
- _…and 31 more_

### Community 24 — startEditing() (51 nodes, cohesion: 0.04)

- MetadataPanel
- catch()
- clsx()
- formatExifTag()
- handleKeyDown()
- handleSave()
- if()
- clsx/clsx
- ../editor/ExifIcons/IconAperture
- ../editor/ExifIcons/IconFocalLength
- ../editor/ExifIcons/IconIso
- ../editor/ExifIcons/IconLens
- ../editor/ExifIcons/IconShutter
- framer-motion/AnimatePresence
- framer-motion/motion
- ../../../hooks/useLibraryActions/useLibraryActions
- lucide-react/Camera
- lucide-react/Check
- lucide-react/ChevronDown
- lucide-react/ChevronRight
- _…and 31 more_

### Community 25 — transpose() (50 nodes, cohesion: 0.05)

- ai_processing
- accumulator_to_rgb32f()
- AiDepthMaskParameters
- AiForegroundMaskParameters
- AiModels
- AiSkyMaskParameters
- AiState
- AiSubjectMaskParameters
- apply_seamless()
- box_filter_horiz()
- box_filter_opt()
- CachedDepthMap
- ClipModels
- download_model()
- edt_1d()
- edt_2d()
- extract_tile_mirror()
- FaceBox
- fast_guided_filter()
- generate_image_embeddings()
- _…and 30 more_

### Community 26 — TransformParams (49 nodes, cohesion: 0.05)

- mask_generation
- AiPatchDefinition
- BrushLine
- BrushMaskParameters
- build_full_warped_image()
- build_warped_image_for_masks()
- default_brush_feather()
- default_line_flow()
- default_opacity()
- default_range()
- default_tolerance()
- FlowLine
- FlowMaskParameters
- generate_brush_bitmap()
- generate_flow_bitmap()
- generate_mask_bitmap()
- generate_mask_overlay()
- get_cached_or_generate_mask()
- GrowFeatherParameters
- base64::{Engine as _, engine::general_purpose}
- _…and 29 more_

### Community 27 — try_decode_color_managed_cmyk() (48 nodes, cohesion: 0.06)

- image_loader
- classify_raw_develop_error()
- cmyk_inks_to_rgb32f()
- composite_patches_on_image()
- decode_cmyk_jpeg()
- decode_cmyk_tiff()
- embedded_preview_fallback()
- flatten_alpha_over_white()
- anyhow::{Context, Result, anyhow}
- base64::{Engine as _, engine::general_purpose}
- crate::app_settings::{AppSettings, load_settings}
- crate::app_state::{AppState, LoadedImage}
- crate::Cursor
- crate::exif_processing
- crate::file_management::{parse_virtual_path, read_file_mapped}
- crate::formats::is_raw_file
- crate::image_processing::{
    apply_orientation, apply_srgb_to_linear, remove_raw_artifacts_and_enhance,
}
- crate::image_processing::ImageMetadata
- crate::mask_generation::{MaskDefinition, SubMask, generate_mask_bitmap}
- crate::white_balance::WhiteBalance
- _…and 28 more_

### Community 28 — handleMouseUp() (46 nodes, cohesion: 0.04)

- Color
- getTransform()
- handleClick()
- handleMouseDown()
- handleMouseEnter()
- handleMouseLeave()
- handleMouseUp()
- ./AdjustmentSubSection/AdjustmentSubSection
- framer-motion/AnimatePresence
- framer-motion/motion
- lucide-react/Pipette
- lucide-react/Sliders
- lucide-react/Trash2
- react-i18next/useTranslation
- react/useEffect
- react/useMemo
- react/useState
- ../../store/useSettingsStore/useSettingsStore
- ../../types/typography/TextColors
- ../../types/typography/TextWeights
- _…and 26 more_

### Community 29 — updateParametricValue() (45 nodes, cohesion: 0.05)

- Curves
- buildParametricPoints()
- convertParametricToPoints()
- getCurvePath()
- getHistogramPath()
- getParametricMarkers()
- getSplitterGradient()
- getZeroHistogramPath()
- handleContainerStart()
- handleContextMenu()
- handleDoubleClick()
- handlePointContextMenu()
- handlePointStart()
- handleToggleMode()
- hasFineAdjustmentModifier()
- ../../context/ContextMenuContext/useContextMenu
- framer-motion/AnimatePresence
- framer-motion/motion
- lucide-react/ClipboardPaste
- lucide-react/Copy
- _…and 25 more_

### Community 30 — parseRgb() (45 nodes, cohesion: 0.04)

- Editor
- checkCropValid()
- clsx/clsx
- ./editor/EditorToolbar/EditorToolbar
- ./editor/ImageCanvas/ImageCanvas
- ../../hooks/useAiMasking/useAiMasking
- ../../hooks/useEditorActions/useEditorActions
- ../../hooks/useImageRenderSize/ImageDimensions
- ../../hooks/useImageRenderSize/RenderSize
- ../../hooks/useImageRenderSize/useImageRenderSize
- lodash.debounce/debounce
- lucide-react/Loader2
- react-image-crop/Crop
- react-image-crop/PercentCrop
- react/useCallback
- react/useEffect
- react/useImperativeHandle
- react/useLayoutEffect
- react/useMemo
- react/useRef
- _…and 25 more_

### Community 31 — weighted_median_labels() (45 nodes, cohesion: 0.10)

- box_filter()
- build_align_pyramids()
- collapse_pyramid()
- convolve_separable()
- decide_labels()
- decode_frame()
- depth_preview_plane()
- .len()
- downsample()
- fit_dims()
- focus_domain()
- focus_scores()
- gaussian_blur()
- gaussian_kernel()
- gaussian_pyramid()
- guided_filter()
- laplacian_pyramid()
- .fold_radial_into_poly()
- .rescaled()
- level_count()
- _…and 25 more_

### Community 32 — ThumbnailComponent() (44 nodes, cohesion: 0.05)

- LibraryItems
- clsx()
- getW()
- clsx/clsx
- @dnd-kit/core/useDraggable
- @dnd-kit/core/useDroppable
- ../editor/ExifIcons/IconAperture
- ../editor/ExifIcons/IconFocalLength
- ../editor/ExifIcons/IconIso
- ../editor/ExifIcons/IconShutter
- lucide-react/Ban
- lucide-react/CloudOff
- lucide-react/Film
- lucide-react/Flag
- lucide-react/Folder
- lucide-react/FolderOpen
- lucide-react/Image
- lucide-react/Layers
- lucide-react/SlidersHorizontal
- lucide-react/Star
- _…and 24 more_

### Community 33 — handleWheel() (33) (43 nodes, cohesion: 0.05)

- CollageModal
- clsx()
- handleAspectRatioChange()
- handleExportDimChange()
- handleOrientationToggle()
- handleOriginalAspectRatio()
- handlePanMouseDown()
- handleSave()
- handleShuffleImages()
- handleThumbnailMouseDown()
- handleWheel()
- clsx/clsx
- framer-motion/AnimatePresence
- framer-motion/motion
- lucide-react/CheckCircle
- lucide-react/Crop
- lucide-react/LayoutTemplate
- lucide-react/Loader2
- lucide-react/Palette
- lucide-react/Proportions
- _…and 23 more_

### Community 34 — sync_xmp_for_rrdata() (39 nodes, cohesion: 0.07)

- tagging
- add_tag_for_paths()
- clear_ai_tags()
- clear_all_tags()
- clip_aesthetic()
- clip_prompt_probs()
- extract_color_tags()
- FaceCues
- generate_tags_with_clip()
- anyhow::Result
- crate::{AppState, candidates::TAG_CANDIDATES}
- crate::file_management::{self, parse_virtual_path}
- crate::formats::is_supported_image_file
- crate::hierarchy::TAG_HIERARCHY
- crate::image_processing::ImageMetadata
- futures::stream::{self, StreamExt}
- image::{DynamicImage, imageops::FilterType}
- ndarray::{Array, Axis}
- ort::session::Session
- ort::value::Tensor
- _…and 19 more_

### Community 35 — UsbCameraState (39 nodes, cohesion: 0.07)

- usb
- aperture_from_lens_names()
- camera_thread()
- CameraCommand
- CameraConfig
- CameraInfo
- ConnectedCamera
- download_file()
- f_number_choices()
- fuji_shoot()
- fuji_shoot_once()
- grab_preview_frame()
- serde::Serialize
- std::path::PathBuf
- std::sync::mpsc::{channel, RecvTimeoutError, Sender}
- std::sync::Mutex
- std::time::Duration
- super::{max_aperture_from_name, parse_f_number}
- super::unique_path
- tauri::Emitter
- _…and 19 more_

### Community 36 — unique_lut_destination() (38 nodes, cohesion: 0.08)

- lut_processing
- convert_image_to_cube_lut()
- film_luts_dir()
- generate_identity_lut_image()
- generate_lut_previews()
- get_lut_cache_dir()
- get_luts_dir()
- get_or_load_lut()
- import_android_lut()
- anyhow::anyhow
- base64::{Engine as _, engine::general_purpose}
- crate::android_integration::{
    get_android_cached_lut_path, read_android_content_uri, resolve_android_content_uri_name,
}
- crate::android_integration::is_android_content_uri
- crate::AppState
- crate::cache_utils::calculate_transform_hash
- crate::image_processing::{
    RenderRequest, get_all_adjustments_from_json, process_and_get_dynamic_image,
    resolve_tonemapper_override_from_handle,
}
- image::{DynamicImage, GenericImageView, Rgb, Rgb32FImage}
- import_luts()
- import_luts_to_dir()
- mozjpeg_rs::{Encoder, Preset}
- _…and 18 more_

### Community 37 — TetherChip() (37 nodes, cohesion: 0.05)

- BottomBar
- clsx()
- clsx/clsx
- ./Filmstrip/Filmstrip
- framer-motion/AnimatePresence
- framer-motion/motion
- ../../hooks/useLibraryActions/useLibraryActions
- lucide-react/Check
- lucide-react/ClipboardPaste
- lucide-react/Copy
- lucide-react/Filter
- lucide-react/PanelBottom
- lucide-react/PanelLeft
- lucide-react/PanelRight
- lucide-react/Settings
- lucide-react/Star
- react-i18next/useTranslation
- react/useEffect
- react/useRef
- react/useState
- _…and 17 more_

### Community 38 — handleWheel() (37 nodes, cohesion: 0.05)

- DenoiseModal
- defaultIntensityFor()
- handleBackdropClick()
- handleBackdropMouseDown()
- handleMouseDown()
- handleOpen()
- handleRunDenoise()
- handleSave()
- handleSliderMouseDown()
- handleWheel()
- framer-motion/motion
- lucide-react/CheckCircle
- lucide-react/Grip
- lucide-react/Loader2
- lucide-react/Move
- lucide-react/RefreshCw
- lucide-react/Save
- lucide-react/XCircle
- lucide-react/ZoomIn
- lucide-react/ZoomOut
- _…and 17 more_

### Community 39 — walsh_hadamard_1d() (37 nodes, cohesion: 0.07)

- denoising
- block_matching_joint()
- Bm3dParams
- .from_intensity()
- build_3d_group()
- compute_ssd_flat()
- dct_1d_8()
- dct_2d_8x8()
- DctTables
- .new()
- extract_patch()
- idct_1d_8()
- idct_2d_8x8()
- base64::{Engine as _, engine::general_purpose}
- crate::app_settings::load_settings
- crate::app_state::AppState
- crate::file_management::parse_virtual_path
- crate::formats::is_raw_file
- crate::image_loader::load_base_image_from_bytes
- crate::image_processing::apply_cpu_default_raw_processing
- _…and 17 more_

### Community 40 — update_negative_conversion() (36 nodes, cohesion: 0.09)

- negative_conversion
- analyze_bounds()
- analyze_bounds_for()
- analyze_bounds_for_clipped()
- bounds_json()
- camel_case_params_deserialize_from_frontend_shape()
- ChannelBounds
- clear_decode_caches()
- get_negative_raw_preview()
- crate::AppState
- crate::file_management::{parse_virtual_path, read_file_mapped}
- crate::image_loader::load_base_image_raw
- crate::image_processing::downscale_f32_image
- crate::load_settings
- image::{DynamicImage, Rgb32FImage}
- rayon::prelude::*
- serde::{Deserialize, Serialize}
- std::cmp::Ordering
- std::fs
- std::path::Path
- _…and 16 more_

### Community 41 — WindowState (35 nodes, cohesion: 0.06)

- app_state
- AiTaskGuard
- AnalyticsConfig
- AnalyticsJob
- AppState
- CachedPreview
- GpuImageCache
- GpuProcessorState
- crate::ai_processing::AiState
- crate::cache_utils::DecodedImageCache
- crate::gpu_processing::GpuProcessor
- crate::image_processing::GpuContext
- crate::launch_request::ExternalEditSession
- crate::lens_correction::LensDatabase
- crate::lut_processing::Lut
- crate::white_balance::WhiteBalance
- image::{DynamicImage, GrayImage}
- serde::{Deserialize, Serialize}
- std::collections::{HashMap, HashSet, VecDeque}
- std::path::PathBuf
- _…and 15 more_

### Community 42 — x_bounds_at() (35 nodes, cohesion: 0.12)

- guided_perspective
- calculate_guided_perspective()
- clip_poly_edge()
- compute_guided_homography()
- compute_max_inscribed_crop()
- count_valid_lines()
- cross()
- det3()
- filter_valid_lines()
- finalize_h()
- flatten_row_major()
- GuidedResult
- GuidedResultJson
- GuideLine
- GuideOrientation
- GuidePoint
- hom_mul()
- serde::{Deserialize, Serialize}
- is_line_valid()
- line_centered()
- _…and 15 more_

### Community 43 — setConfigCurrent() (35 nodes, cohesion: 0.06)

- TetherMenu
- ConfigSlider()
- handleConfig()
- handleStart()
- handleStop()
- ../../../hooks/usePresets/usePresets
- ../../../hooks/usePresets/UserPreset
- ./LibraryHeader/DropdownMenu
- lucide-react/Aperture
- lucide-react/Camera
- lucide-react/Play
- lucide-react/Square
- lucide-react/Unplug
- react-i18next/useTranslation
- react/useEffect
- react/useMemo
- react/useRef
- react/useState
- ../../../store/useEditorStore/useEditorStore
- ../../../store/useLibraryStore/useLibraryStore
- _…and 15 more_

### Community 44 — ScannerPane() (35 nodes, cohesion: 0.06)

- ScannerPane
- detectScanner()
- lucide-react/ChevronDown
- lucide-react/Film
- lucide-react/FlipHorizontal2
- lucide-react/FlipVertical2
- lucide-react/Loader2
- lucide-react/Pipette
- lucide-react/Plus
- lucide-react/RefreshCw
- lucide-react/RotateCcw
- lucide-react/RotateCw
- lucide-react/Trash2
- lucide-react/X
- react-toastify/toast
- react/useRef
- react/useState
- ../../../store/useImportStore/useImportStore
- ../../../store/useLibraryStore/useLibraryStore
- ../../../store/useScannerStore/FilmType
- _…and 15 more_

### Community 45 — updateSnapshots() (34 nodes, cohesion: 0.08)

- SnapshotsSection
- commitRename()
- dateLabel()
- handleApply()
- handleDelete()
- handleOverwrite()
- handleRowContextMenu()
- handleSave()
- ../../../context/ContextMenuContext/useContextMenu
- ../../../hooks/useEditorActions/useEditorActions
- lucide-react/Check
- lucide-react/FileEdit
- lucide-react/History
- lucide-react/Plus
- lucide-react/RefreshCw
- lucide-react/Trash2
- ./PresetItemDisplay/PresetItemDisplay
- react-i18next/useTranslation
- react/useState
- ../../../store/useEditorStore/useEditorStore
- _…and 14 more_

### Community 46 — writeCullingMetadata() (34 nodes, cohesion: 0.08)

- useSdImportActions
- computeDefaultKeepers()
- filterState()
- gapSeconds()
- groupArgs()
- groupSettings()
- ../components/ui/AppProperties/CullingSuggestions
- ../components/ui/AppProperties/Invokes
- ../components/ui/AppProperties/SortDirection
- ../components/views/import/importFilters/computeVisible
- ../components/views/import/importFilters/computeVisibleSet
- ../components/views/import/importFilters/FileTypeFilter
- ../components/views/import/importFilters/ImportFilterState
- react-toastify/toast
- react/useCallback
- ../store/useImportStore/DriveInfo
- ../store/useImportStore/ImportSortKey
- ../store/useImportStore/restoreSelection
- ../store/useImportStore/useImportStore
- ../store/useSettingsStore/useSettingsStore
- _…and 14 more_

### Community 47 — shuffleArray() (34 nodes, cohesion: 0.06)

- CommunityPage
- handleDownloadPreset()
- framer-motion/AnimatePresence
- framer-motion/motion
- lucide-react/ArrowLeft
- lucide-react/CheckCircle2
- lucide-react/ChevronDown
- lucide-react/Crop
- lucide-react/Layers
- lucide-react/Loader2
- lucide-react/Search
- lucide-react/Users
- react-i18next/useTranslation
- react/React
- react/useCallback
- react/useEffect
- react/useMemo
- react/useRef
- react/useState
- simple-icons/siGithub
- _…and 14 more_

### Community 48 — handleButtonKeyDown() (34 nodes, cohesion: 0.06)

- EditorToolbar
- handleButtonKeyDown()
- clsx/clsx
- ./ExifIcons/IconAperture
- ./ExifIcons/IconCalendar
- ./ExifIcons/IconClock
- ./ExifIcons/IconFocalLength
- ./ExifIcons/IconIso
- ./ExifIcons/IconShutter
- framer-motion/AnimatePresence
- framer-motion/motion
- lucide-react/ArrowLeft
- lucide-react/Eye
- lucide-react/EyeOff
- lucide-react/Loader2
- lucide-react/Maximize
- lucide-react/Redo
- lucide-react/Undo
- react-i18next/useTranslation
- react/memo
- _…and 14 more_

### Community 49 — StraightSource (33 nodes, cohesion: 0.08)

- panorama_stitching
- crop_to_content()
- Feature
- ImageInfo
- base64::{Engine as _, engine::general_purpose}
- crate::app_settings::load_settings
- crate::app_state::AppState
- crate::file_management::parse_virtual_path
- crate::formats::is_raw_file
- crate::image_processing::apply_cpu_default_raw_processing
- crate::panorama_utils::{processing, stitching}
- image::{DynamicImage, GenericImageView, GrayImage, Rgb32FImage}
- image::ImageFormat
- nalgebra::Matrix3
- rayon::prelude::*
- std::collections::{HashMap, HashSet, VecDeque}
- std::fs
- std::io::Cursor
- std::path::Path
- std::time::Instant
- _…and 13 more_

### Community 50 — VisibilityToggle() (32 nodes, cohesion: 0.06)

- AdjustmentSectionsSubMenu
- DragHandle()
- clsx/clsx
- framer-motion/DragControls
- framer-motion/Reorder
- framer-motion/useDragControls
- i18next/ParseKeys
- lucide-react/ChevronDown
- lucide-react/Eye
- lucide-react/EyeOff
- lucide-react/GripVertical
- lucide-react/RotateCcw
- react-i18next/useTranslation
- react/memo
- react/useEffect
- react/useMemo
- react/useRef
- react/useState
- ../../../store/useSettingsStore/useSettingsStore
- ../../ui/AppProperties/AdjustmentLayout
- _…and 12 more_

### Community 51 — Vignetting (32 nodes, cohesion: 0.08)

- lens_correction
- any_name_matches()
- Aperture
- autodetect_lens()
- Calibration
- CalibrationElement
- Camera
- .get_maker()
- .get_model()
- Distortion
- find_best_lens_match()
- find_lens_by_camera_mount()
- Focal
- get_lensfun_lenses_for_maker()
- crate::AppState
- fuzzy_matcher::FuzzyMatcher
- include_dir::{Dir, include_dir}
- serde::{Deserialize, Serialize}
- std::cmp::Ordering
- std::fs
- _…and 12 more_

### Community 52 — upload_source() (31 nodes, cohesion: 0.06)

- main
- EndpointFilter
- .filter()
- health()
- aiofiles
- asyncio
- base64
- contextlib.asynccontextmanager
- engine.build_workflow
- engine.cache
- engine.ComfyClient
- engine.config
- engine.ImageProcessor
- engine.save_inputs_for_debug
- fastapi.FastAPI
- fastapi.File
- fastapi.Form
- fastapi.HTTPException
- fastapi.middleware.cors.CORSMiddleware
- fastapi.UploadFile
- _…and 11 more_

### Community 53 — handleReset() (31 nodes, cohesion: 0.06)

- Effects
- handleReset()
- ./AdjustmentSubSection/AdjustmentSubSection
- clsx/clsx
- framer-motion/motion
- lucide-react/Aperture
- lucide-react/Circle
- lucide-react/Hexagon
- lucide-react/Loader2
- lucide-react/Octagon
- react-i18next/useTranslation
- react-toastify/toast
- react/useEffect
- react/useMemo
- react/useRef
- react/useState
- ../../store/useProcessStore/useProcessStore
- ../../store/useSettingsStore/useSettingsStore
- @tauri-apps/api/core/invoke
- ../../types/typography/TextVariants
- _…and 11 more_

### Community 54 — onDragMove() (31 nodes, cohesion: 0.06)

- PanelSwitcher
- clsx()
- handleClick()
- clsx/clsx
- @dnd-kit/core/useDndMonitor
- @dnd-kit/core/useDraggable
- @dnd-kit/core/useDroppable
- framer-motion/AnimatePresence
- framer-motion/LayoutGroup
- framer-motion/motion
- lucide-react/Bot
- lucide-react/Camera
- lucide-react/Crop
- lucide-react/FileInput
- lucide-react/Folder
- lucide-react/Info
- lucide-react/Layers
- lucide-react/LucideIcon
- lucide-react/Paintbrush
- lucide-react/SlidersHorizontal
- _…and 11 more_

### Community 55 — useAppInitialization() (31 nodes, cohesion: 0.07)

- useAppInitialization
- getDefaultLanguage()
- ../components/ui/AppProperties/EditedStatus
- ../components/ui/AppProperties/FilterCriteria
- ../components/ui/AppProperties/FlagStatus
- ../components/ui/AppProperties/Invokes
- ../components/ui/AppProperties/LibraryViewMode
- ../components/ui/AppProperties/NegativeStatus
- ../components/ui/AppProperties/Panel
- ../components/ui/AppProperties/PanelRegion
- ../components/ui/AppProperties/RawStatus
- ../components/ui/AppProperties/Theme
- ../components/ui/AppProperties/ThumbnailAspectRatio
- ../components/ui/AppProperties/ThumbnailSize
- react-i18next/useTranslation
- react/useEffect
- react/useRef
- ../store/useEditorStore/useEditorStore
- ../store/useLibraryStore/useLibraryStore
- ../store/useProcessStore/useProcessStore
- _…and 11 more_

### Community 56 — ../../utils/adjustments/COLOR_LABELS (31 nodes, cohesion: 0.06)

- Filmstrip
- clsx/clsx
- @dnd-kit/core/useDraggable
- lucide-react/Image
- lucide-react/SlidersHorizontal
- lucide-react/Star
- react-i18next/useTranslation
- react/memo
- react/React
- react/useCallback
- react/useEffect
- react/useMemo
- react/useRef
- react/useState
- react-window/Grid
- react-window/useGridCallbackRef
- ../../store/useLibraryStore/useLibraryStore
- ../../store/useProcessStore/useProcessStore
- ../../store/useSettingsStore/useSettingsStore
- ../../types/typography/TextColors
- _…and 11 more_

### Community 57 — handleResize() (30 nodes, cohesion: 0.07)

- LibraryGrid
- handleResize()
- ./LibraryItems/Row
- lodash.debounce/debounce
- lucide-react/ChevronDown
- lucide-react/ChevronUp
- react-i18next/useTranslation
- react/React
- react/useCallback
- react/useEffect
- react/useMemo
- react/useRef
- react/useState
- react-window/List
- react-window/useListCallbackRef
- ../../../store/useLibraryStore/useLibraryStore
- ../../../store/useProcessStore/useProcessStore
- ../../../store/useSettingsStore/useSettingsStore
- ../../../types/typography/TEXT_COLOR_KEYS
- ../../../types/typography/TextColors
- _…and 10 more_

### Community 58 — warp_or_copy() (30 nodes, cohesion: 0.07)

- focus_stacking
- AlignConfig
- .default()
- AlignPyramids
- DecisionMaps
- erode_mask()
- FramePose
- FusionConfig
- .default()
- base64::{Engine as _, engine::general_purpose}
- crate::app_settings::load_settings
- crate::app_state::AppState
- crate::file_management::parse_virtual_path
- crate::formats::is_raw_file
- crate::image_processing::apply_cpu_default_raw_processing
- image::{DynamicImage, ImageFormat, Rgb32FImage}
- nalgebra::{DMatrix, DVector}
- rayon::prelude::*
- std::fs
- std::io::{Cursor, Read, Write}
- _…and 10 more_

### Community 59 — transform_to_srgb() (29 nodes, cohesion: 0.12)

- icc
- adobe_rgb_patches_convert_to_the_lcms_reference_values()
- cmyk_lut_maps_paper_white_and_full_ink_through_lab()
- cmyk_parser_rejects_rgb_profiles_and_vice_versa()
- CmykTransform
- .to_srgb()
- every_tag_is_aligned_sorted_and_inside_the_profile()
- find_tag()
- header_declares_an_rgb_display_profile_sized_to_the_bytes()
- std::sync::OnceLock
- super::*
- InputTransform
- invert_3x3()
- lab16()
- lab_to_xyz_d50()
- non_rgb_and_lut_profiles_are_skipped()
- our_own_srgb_profile_parses_as_a_no_op()
- parse_cmyk_profile()
- parse_mft_lut()
- read_u32()
- _…and 9 more_

### Community 60 — score_for_import() (29 nodes, cohesion: 0.08)

- sd_import
- analyze_for_import()
- cull_images_for_import()
- DriveInfo
- eject_drive()
- file_content_hash()
- find_existing_in_destination()
- get_capture_times()
- get_cull_learning()
- get_import_preview()
- group_cached()
- group_for_import()
- base64::{Engine as _, engine::general_purpose}
- crate::culling::{
    CullingSettings, CullingSuggestions, ImageAnalysisData, analyze_paths, group_by_scene,
    group_by_time,
}
- crate::formats::{is_raw_file, is_supported_image_file}
- rawler::decoders::RawDecodeParams
- serde::Serialize
- std::collections::{HashMap, HashSet}
- std::path::{Path, PathBuf}
- std::sync::Mutex
- _…and 9 more_

### Community 61 — startEyedropper() (29 nodes, cohesion: 0.07)

- FilmPanel
- applyProfile()
- deleteProfile()
- lucide-react/Pipette
- lucide-react/Plus
- lucide-react/RotateCcw
- lucide-react/Trash2
- lucide-react/X
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useRef
- react/useState
- ../../store/useEditorStore/useEditorStore
- ../../store/useSettingsStore/useSettingsStore
- @tauri-apps/api/core/invoke
- ../ui/AppProperties/Invokes
- ../ui/Dropdown/Dropdown
- ../ui/Slider/Slider
- ../ui/Switch/Switch
- _…and 9 more_

### Community 62 — closeConfirmModal() (29 nodes, cohesion: 0.07)

- AppModals
- closeConfirmModal()
- ./CollageModal/CollageModal
- ./ConfirmModal/ConfirmModal
- ./CopyPasteSettingsModal/CopyPasteSettingsModal
- ./CreateFolderModal/CreateFolderModal
- ./CullingModal/CullingModal
- ./DenoiseModal/DenoiseMethod
- ./DenoiseModal/DenoiseModal
- ./FocusStackModal/FocusStackModal
- ./HdrModal/HdrModal
- ./ImportSettingsModal/ImportSettingsModal
- ./PanoramaModal/PanoramaModal
- react-i18next/useTranslation
- ./RenameFileModal/RenameFileModal
- ./RenameFolderModal/RenameFolderModal
- ../../store/useEditorStore/useEditorStore
- ../../store/useLibraryStore/useLibraryStore
- ../../store/useProcessStore/useProcessStore
- ../../store/useSettingsStore/useSettingsStore
- _…and 9 more_

### Community 63 — ImageThumbnail() (28 nodes, cohesion: 0.07)

- CullingModal
- ImageThumbnail()
- framer-motion/AnimatePresence
- framer-motion/motion
- lucide-react/CheckCircle
- lucide-react/Loader2
- lucide-react/Star
- lucide-react/Tag
- lucide-react/Trash2
- lucide-react/Users
- lucide-react/XCircle
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useMemo
- react/useState
- @tauri-apps/api/core/invoke
- ../../types/typography/TextColors
- ../../types/typography/TextVariants
- ../ui/AppProperties/CullingSettings
- _…and 8 more_

### Community 64 — WorkspaceState (28 nodes, cohesion: 0.07)

- app_settings
- AdjustmentLayout
- AutoAdjustOptionsWire
- CustomAspectRatio
- ExportPreset
- FilterCriteria
- .default()
- FolderTreeSort
- .default()
- get_settings_path()
- grouped_switches_from_the_first_version_still_apply()
- crate::app_state::AppState
- serde::{Deserialize, Serialize}
- serde_json::Value
- std::collections::{BTreeMap, HashMap, HashSet}
- std::fs
- std::path::PathBuf
- super::*
- tauri::{AppHandle, Manager}
- is_tethering_supported()
- _…and 8 more_

### Community 65 — NewMaskDropZone() (28 nodes, cohesion: 0.08)

- Masks
- formatMaskTypeName()
- getMaskTypeName()
- getSubMaskName()
- framer-motion/motion
- i18next/i18n
- lucide-react/Bandage
- lucide-react/BringToFront
- lucide-react/Brush
- lucide-react/Circle
- lucide-react/Cloud
- lucide-react/Droplet
- lucide-react/Droplets
- lucide-react/Eraser
- lucide-react/Eye
- lucide-react/MopSparkles
- lucide-react/RectangleHorizontal
- lucide-react/Smile
- lucide-react/Spline
- lucide-react/SquareMousePointer
- _…and 8 more_

### Community 66 — write_preset_sidecar() (27 nodes, cohesion: 0.08)

- mod
- ActiveSession
- CameraInfo
- crate::formats::is_supported_image_file
- crate::image_processing::ImageMetadata
- notify::{RecommendedWatcher, RecursiveMode, Watcher}
- serde::Serialize
- std::collections::{HashMap, HashSet}
- std::path::PathBuf
- std::sync::mpsc::{channel, RecvTimeoutError}
- std::sync::Mutex
- std::time::Duration
- tauri::Emitter
- ingest_loop()
- start_tether_session()
- stop_tether_session()
- tether_connect_camera()
- tether_disconnect_camera()
- tether_list_cameras()
- tether_set_config()
- _…and 7 more_

### Community 67 — clsx() (27 nodes, cohesion: 0.07)

- widgets
- clsx()
- @clerk/react/useAuth
- @clerk/react/useClerk
- @clerk/react/useUser
- clsx/clsx
- framer-motion/motion
- lucide-react/Cpu
- lucide-react/ExternalLink
- lucide-react/Image
- lucide-react/Mouse
- lucide-react/Scaling
- lucide-react/Server
- lucide-react/Touchpad
- react-i18next/useTranslation
- react/useEffect
- react/useMemo
- react/useState
- @tauri-apps/plugin-shell/open
- ../../../types/typography/TextColors
- _…and 7 more_

### Community 68 — save_inputs_for_debug() (26 nodes, cohesion: 0.08)

- engine
- build_workflow()
- ImageProcessor
- .crop_and_pack()
- ._pack()
- .process_mask_for_comfyui()
- aiofiles
- aiohttp
- base64
- collections.OrderedDict
- io
- json
- logging
- numpy
- os
- pathlib.Path
- PIL.Image
- pydantic_settings.BaseSettings
- shutil
- time
- _…and 6 more_

### Community 69 — render_depth_of_field() (26 nodes, cohesion: 0.15)

- lens_blur
- apply_lens_blur()
- blur_layer_bokeh()
- BokehTap
- build_bokeh_kernel()
- build_coc_field()
- build_guided_model()
- depth_to_signed_coc()
- dof_aperture_intensity()
- dof_box_filter()
- dof_composite_stack()
- dof_depth_to_f32()
- dof_despeckle()
- dof_dilate_spans()
- dof_downsample_fused()
- dof_med3()
- dof_polygon_scale()
- dof_smoothstep()
- dof_soft_pow()
- dof_tent()
- _…and 6 more_

### Community 70 — handleSave() (70) (25 nodes, cohesion: 0.08)

- PanoramaModal
- handleBackdropClick()
- handleBackdropMouseDown()
- handleOpen()
- handleSave()
- framer-motion/motion
- lucide-react/CheckCircle
- lucide-react/Layers
- lucide-react/Loader2
- lucide-react/RefreshCw
- lucide-react/Save
- lucide-react/XCircle
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useRef
- react/useState
- @tauri-apps/api/core/invoke
- ../../types/typography/TextColors
- ../../types/typography/TextVariants
- _…and 5 more_

### Community 71 — max_corner_displacement() (24 nodes, cohesion: 0.10)

- hdr_deghosting 2
- align_frame_to_reference()
- align_hdr_frames()
- AlignmentOutcome
- assert_uniform_dimensions()
- centroid()
- detect_frame_features()
- estimate_rigid_transform()
- FrameDetection
- crate::app_settings::AppSettings
- crate::exif_processing::{read_exposure_time_secs, read_iso}
- crate::formats::is_raw_file
- crate::image_loader::load_base_image_from_bytes
- crate::image_processing::{
    apply_cpu_default_raw_processing, apply_linear_to_srgb, apply_srgb_to_linear,
}
- crate::panorama_stitching::{Feature, KeyPoint, Match}
- crate::panorama_utils::{processing, stitching}
- image::{DynamicImage, GenericImageView, Rgb32FImage}
- nalgebra::{Matrix2, Matrix3, Point2}
- std::fs
- std::path::Path
- _…and 4 more_

### Community 72 — Cell() (24 nodes, cohesion: 0.08)

- CullGroupsGrid
- Cell()
- ../../../components/ui/AppProperties/CullingSuggestions
- ../../../components/ui/AppProperties/SortDirection
- ../../../hooks/useSdImportActions/useSdImportActions
- ./ImportFilterBar/ImportFilterBar
- ./importFilters/computeVisible
- ./ImportViewer/ImportViewer
- ./LazyThumb/LazyThumb
- lucide-react/Check
- lucide-react/Droplet
- lucide-react/Eye
- lucide-react/Grid2x2
- lucide-react/Layers
- lucide-react/Sparkles
- ./RatingColor/RatingColor
- react/useCallback
- react/useEffect
- react/useMemo
- react/useRef
- _…and 4 more_

### Community 73 — withFlag() (24 nodes, cohesion: 0.10)

- useLibraryActions
- applyWithAutoAdvance()
- clearLibrarySelection()
- ../components/ui/AppProperties/Album
- ../components/ui/AppProperties/AlbumGroup
- ../components/ui/AppProperties/AlbumItem
- ../components/ui/AppProperties/ImageFile
- ../components/ui/AppProperties/ImageFlag
- ../components/ui/AppProperties/Invokes
- react-toastify/toast
- react/useCallback
- ../store/useEditorStore/useEditorStore
- ../store/useLibraryStore/useLibraryStore
- ../store/useProcessStore/useProcessStore
- ../store/useSettingsStore/useSettingsStore
- ../store/useUIStore/useUIStore
- @tauri-apps/api/core/invoke
- ./useSortedLibrary/computeSortedLibrary
- ../utils/imageFlags/getImageFlag
- ../utils/imageGrouping/expandGroupedPaths
- _…and 4 more_

### Community 74 — Roi (24 nodes, cohesion: 0.09)

- gpu_processing
- BlurParams
- DisplayTransform
- FlareParams
- GfParams
- HighPrecisionPipeline
- HighPrecisionTile
- crate::{AppState, GpuImageCache}
- crate::image_processing::{AllAdjustments, GpuContext, MAX_MASKS, MaskAdjustments}
- crate::lut_processing::Lut
- half::f16
- image::{DynamicImage, GenericImageView, ImageBuffer, Luma, Rgba}
- std::num::NonZero
- std::sync::Arc
- std::time::Instant
- super::processor_side
- tauri::Manager
- wgpu::util::{DeviceExt, TextureDataOrder}
- processor_reused_across_orientation()
- processor_side()
- _…and 4 more_

### Community 75 — handleSave() (75) (24 nodes, cohesion: 0.08)

- FocusStackModal
- clsx()
- handleBackdropClick()
- handleBackdropMouseDown()
- handleOpen()
- handleSave()
- clsx/clsx
- framer-motion/AnimatePresence
- framer-motion/motion
- lucide-react/CheckCircle
- lucide-react/Layers
- lucide-react/Loader2
- lucide-react/Map
- lucide-react/Save
- lucide-react/XCircle
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useRef
- react/useState
- _…and 4 more_

### Community 76 — prepare_source_image() (24 nodes, cohesion: 0.13)

- inpainting
- bilinear_sample()
- calculate_mask_bounds()
- draw_stroke_to_mask()
- encode_patch_result()
- generate_liquify_patch()
- generate_manual_cleanup_patch()
- generate_retouch_patch()
- base64::{Engine as _, engine::general_purpose}
- crate::ai_connector
- crate::ai_processing
- crate::app_settings::load_settings
- crate::app_state::AppState
- crate::image_loader::composite_patches_on_image
- crate::image_processing::apply_linear_to_srgb
- crate::mask_generation::{AiPatchDefinition, MaskDefinition, generate_mask_bitmap}
- crate::resolve_warped_image_for_masks
- image::{DynamicImage, GenericImageView, Rgb, RgbImage, RgbaImage}
- rayon::prelude::*
- serde_json::Value
- _…and 4 more_

### Community 77 — handleSwatchClick() (24 nodes, cohesion: 0.08)

- LUTControl
- handleContextMenu()
- handleImport()
- handleSwatchClick()
- ../../context/ContextMenuContext/useContextMenu
- framer-motion/AnimatePresence
- framer-motion/motion
- lucide-react/ChevronDown
- lucide-react/ImageOff
- lucide-react/Trash2
- lucide-react/Upload
- lucide-react/X
- react-i18next/useTranslation
- react/React
- react-toastify/toast
- react/useCallback
- react/useEffect
- react/useRef
- react/useState
- ./Slider/Slider
- _…and 4 more_

### Community 78 — WaveformData (23 nodes, cohesion: 0.13)

- analysis
- apply_gaussian_smoothing()
- auto_white_balance_shift()
- calculate_auto_adjustments()
- calculate_histogram_from_image()
- calculate_waveform_from_image()
- detect_face_median_luma()
- estimate_white_balance()
- get_auto_learning()
- HistogramData
- crate::app_settings::AutoAdjustOptions
- crate::app_settings::AutoProfile
- super::*
- linear_pixels()
- normalize_histogram_range()
- old_profiles_load_with_stock_style()
- OutputStyle
- perform_auto_analysis()
- render_for_analysis()
- reset_auto_profile()
- _…and 3 more_

### Community 79 — hasFineAdjustmentModifier() (23 nodes, cohesion: 0.10)

- Slider
- getFraction()
- getPositionFraction()
- handleChange()
- handleInputChange()
- handleInputCommit()
- handleInputKeyDown()
- handleMouseDown()
- handleRangeKeyDown()
- handleReset()
- handleTouchEnd()
- handleTouchMove()
- handleTouchStart()
- handleValueClick()
- hasFineAdjustmentModifier()
- ./AppProperties/GLOBAL_KEYS
- react-i18next/useTranslation
- react/React
- react/useCallback
- react/useEffect
- _…and 3 more_

### Community 80 — upload_source_image() (23 nodes, cohesion: 0.11)

- ai_connector
- check_status()
- CloudInpaintRequest
- CloudInpaintResponse
- composite_full_res()
- generate_source_id()
- image_to_base64()
- image_to_base64_jpeg()
- image_to_jpeg_bytes()
- anyhow::{Result, anyhow}
- base64::{Engine as _, engine::general_purpose}
- image::{
    DynamicImage, GenericImageView, ImageFormat, RgbaImage, codecs::jpeg::JpegEncoder, imageops,
}
- reqwest::{Client, multipart}
- serde::{Deserialize, Serialize}
- std::fs
- std::io::Cursor
- std::path::Path
- std::time::SystemTime
- InpaintRequest
- MiddlewareResponse
- _…and 3 more_

### Community 81 — useAppNavigation() (23 nodes, cohesion: 0.09)

- useAppNavigation
- ../components/ui/AppProperties/ImageFile
- ../components/ui/AppProperties/Invokes
- ../components/ui/AppProperties/LibraryViewMode
- react-toastify/toast
- react/useCallback
- ../store/useEditorStore/useEditorStore
- ../store/useLibraryStore/useLibraryStore
- ../store/useProcessStore/useProcessStore
- ../store/useSettingsStore/useSettingsStore
- ../store/useUIStore/useUIStore
- @tauri-apps/api/core/invoke
- @tauri-apps/api/path/homeDir
- @tauri-apps/plugin-dialog/open
- ./useEditorActions/debouncedSave
- ./useEditorActions/debouncedSetHistory
- ./useLibraryActions/clearLibrarySelection
- ./useSortedLibrary/computeSortedLibrary
- ../utils/adjustments/INITIAL_ADJUSTMENTS
- ../utils/adjustments/normalizeLoadedAdjustments
- _…and 3 more_

### Community 82 — sync_metadata_to_xmp() (22 nodes, cohesion: 0.29)

- add_to_thumbnail_queue()
- apply_adjustments_to_paths()
- apply_auto_adjustments_to_paths()
- apply_auto_lens_correction_to_paths()
- apply_orientation_to_paths()
- create_virtual_copy()
- delete_files_with_associated()
- duplicate_file()
- emit_thumbnail_cache_setup_error()
- emit_thumbnail_generated()
- generate_single_thumbnail_and_cache()
- increment_thumbnail_progress()
- parse_virtual_path()
- prefetch_source_file()
- regenerate_thumbnails_for_paths()
- reset_adjustments_for_paths()
- resolve_lens_params_in_adjustments()
- resolve_thumbnail_cache_dir()
- save_metadata_and_update_thumbnail()
- show_in_finder()
- _…and 2 more_

### Community 83 — handleSelectNone() (22 nodes, cohesion: 0.09)

- CopyPasteSettingsModal
- capitalize()
- clsx()
- handleGroupToggle()
- handleSelectAll()
- handleSelectNone()
- clsx/clsx
- framer-motion/motion
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useMemo
- react/useRef
- react/useState
- ../../types/typography/TextVariants
- ../ui/Button/Button
- ../ui/Switch/Switch
- ../ui/Text/Text
- ../../utils/adjustments/ADJUSTMENT_GROUPS
- ../../utils/adjustments/COPYABLE_ADJUSTMENT_KEYS
- _…and 2 more_

### Community 84 — useKeyboardShortcuts() (22 nodes, cohesion: 0.09)

- useKeyboardShortcuts
- ../components/ui/AppProperties/ExifOverlay
- ../components/ui/AppProperties/ImageFile
- ../components/ui/AppProperties/ImageFlag
- ../components/ui/AppProperties/Panel
- react-toastify/toast
- react/useCallback
- react/useEffect
- react/useRef
- ../store/useEditorStore/useEditorStore
- ../store/useLibraryStore/useLibraryStore
- ../store/useProcessStore/useProcessStore
- ../store/useSettingsStore/useSettingsStore
- ../store/useUIStore/useUIStore
- ./useEditorActions/useEditorActions
- ./useLibraryActions/useLibraryActions
- ../utils/keyboardUtils/ADJUSTMENT_NUDGES
- ../utils/keyboardUtils/getDefaultCombo
- ../utils/keyboardUtils/KEYBIND_DEFINITIONS
- ../utils/keyboardUtils/normalizeCombo
- _…and 2 more_

### Community 85 — withAdjustmentLayout() (21 nodes, cohesion: 0.13)

- adjustments
- deepCloneCurves()
- deepCloneParametric()
- getAdjustmentSectionOrder()
- getAdjustmentSectionToolIds()
- getAdjustmentToolOrder()
- getDefaultCurves()
- getDefaultParametricCurve()
- getHiddenAdjustmentTools()
- getVisibleAdjustmentSections()
- guideLabel()
- ../components/panel/right/Masks/SubMask
- ../components/panel/right/Masks/SubMaskMode
- ../components/ui/AppProperties/AdjustmentLayout
- ../components/ui/AppProperties/AppSettings
- react-image-crop/Crop
- uuid/v4
- ./whiteBalance/WhiteBalance
- normalizeLoadedAdjustments()
- reconcileOrder()
- _…and 1 more_

### Community 86 — handleSave() (21 nodes, cohesion: 0.10)

- HdrModal
- handleBackdropClick()
- handleBackdropMouseDown()
- handleOpen()
- handleSave()
- framer-motion/motion
- lucide-react/CheckCircle
- lucide-react/Images
- lucide-react/Loader2
- lucide-react/RefreshCw
- lucide-react/Save
- lucide-react/XCircle
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useRef
- react/useState
- ../../types/typography/TextColors
- ../../types/typography/TextVariants
- ../ui/Button/Button
- _…and 1 more_

### Community 87 — useImageProcessing() (20 nodes, cohesion: 0.10)

- useImageProcessing
- ../components/ui/AppProperties/Invokes
- ../components/ui/AppProperties/Panel
- lodash.debounce/debounce
- lodash.throttle/throttle
- react/React
- react/useCallback
- react/useEffect
- react/useMemo
- react/useRef
- ../store/useEditorStore/useEditorStore
- ../store/useLibraryStore/useLibraryStore
- ../store/useSettingsStore/useSettingsStore
- ../store/useUIStore/useUIStore
- @tauri-apps/api/core/invoke
- ./useEditorActions/debouncedSave
- ../utils/adjustments/Adjustments
- ../utils/adjustments/COPYABLE_ADJUSTMENT_KEYS
- ../utils/ImageLRUCache/globalImageCache
- useImageProcessing()

### Community 88 — zustand/react/shallow/useShallow (20 nodes, cohesion: 0.10)

- EditorView
- clsx/clsx
- framer-motion/AnimatePresence
- framer-motion/motion
- ../panel/BottomBar/BottomBar
- ../panel/Editor/Editor
- ../panel/PanelSwitcher/MobilePanelSwitcher
- react/PointerEvent
- react/RefObject
- ../../store/useEditorStore/useEditorStore
- ../../store/useLibraryStore/useLibraryStore
- ../../store/useProcessStore/useProcessStore
- ../../store/useUIStore/useUIStore
- ../ui/AppProperties/ImageFile
- ../ui/AppProperties/Orientation
- ../ui/AppProperties/Panel
- ../ui/AppProperties/ThumbnailAspectRatio
- ../ui/Resizer/Resizer
- ../../utils/imageFlags/getImageFlag
- zustand/react/shallow/useShallow

### Community 89 — renderBody() (20 nodes, cohesion: 0.10)

- ImportView
- ./CullGroupsGrid/CullGroupsGrid
- ../../../hooks/useImportKeyboard/useImportKeyboard
- ../../../hooks/useSdImportActions/refreshAlreadyImportedNow
- ../../../hooks/useSdImportActions/useSdImportActions
- ./ImportReviewBar/ImportReviewBar
- lucide-react/ArrowLeft
- lucide-react/HardDriveDownload
- lucide-react/Loader2
- react-toastify/toast
- react/useEffect
- react/useRef
- ./ScannerPane/ScannerPane
- ./SourcePicker/SourcePicker
- ../../../store/useImportStore/useImportStore
- ../../../store/useLibraryStore/useLibraryStore
- ../../../store/useProcessStore/useProcessStore
- ../../ui/ExportImportProperties/Status
- zustand/react/shallow/useShallow
- renderBody()

### Community 90 — normalize_grayscale() (20 nodes, cohesion: 0.13)

- processing
- are_points_collinear()
- build_integral_images()
- calculate_downscale_dimensions()
- calculate_downscale_dimensions_capped()
- compute_homography()
- find_homography_ransac()
- generate_brief_pairs()
- generate_low_detail_mask()
- hamming_distance()
- crate::panorama_stitching::{BRIEF_DESCRIPTOR_SIZE, Descriptor, Feature, KeyPoint, Match}
- image::{GrayImage, ImageBuffer, Luma}
- imageproc::corners::{Corner, corners_fast9}
- imageproc::filter::gaussian_blur_f32
- nalgebra::{Matrix3, Point2, SVD}
- rand::prelude::*
- rand::rng
- rayon::prelude::*
- match_features()
- normalize_grayscale()

### Community 91 — useEditorActions() (20 nodes, cohesion: 0.10)

- useEditorActions
- ../components/ui/AppProperties/Invokes
- ../components/ui/AppProperties/resolveAutoAdjust
- lodash.debounce/debounce
- react-toastify/toast
- react/useCallback
- ../store/useEditorStore/useEditorStore
- ../store/useLibraryStore/useLibraryStore
- ../store/useProcessStore/useProcessStore
- ../store/useSettingsStore/useSettingsStore
- @tauri-apps/api/core/invoke
- ../utils/adjustments/Adjustments
- ../utils/adjustments/COPYABLE_ADJUSTMENT_KEYS
- ../utils/adjustments/INITIAL_ADJUSTMENTS
- ../utils/adjustments/LensAdjustment
- ../utils/adjustments/normalizeLoadedAdjustments
- ../utils/adjustments/PasteMode
- ../utils/cropUtils/calculateCenteredCrop
- ../utils/ImageLRUCache/globalImageCache
- useEditorActions()

### Community 92 — onDragMove() (92) (20 nodes, cohesion: 0.10)

- SidePanelArea
- handleContentInteraction()
- clsx/clsx
- @dnd-kit/core/useDndMonitor
- @dnd-kit/core/useDroppable
- framer-motion/AnimatePresence
- framer-motion/motion
- ./PanelSwitcher/MobilePanelSwitcher
- ./PanelSwitcher/PanelSwitcher
- react-i18next/useTranslation
- react/useCallback
- react/useRef
- react/useState
- ../../store/useUIStore/DEFAULT_PANEL_SECTION_HEIGHT
- ../../store/useUIStore/SwitcherPlacement
- ../../store/useUIStore/useUIStore
- ../ui/AppProperties/Panel
- ../ui/AppProperties/PanelRegion
- onDragEnd()
- onDragMove()

### Community 93 — handleRemoveTag() (19 nodes, cohesion: 0.12)

- TaggingSubMenu
- getPathsToUpdate()
- handleAddTag()
- handleInputKeyDown()
- handleRemoveTag()
- ../components/ui/AppProperties/Invokes
- ../components/ui/Text/Text
- framer-motion/AnimatePresence
- framer-motion/motion
- lucide-react/Plus
- lucide-react/X
- react-i18next/useTranslation
- react/useEffect
- react/useRef
- react/useState
- ../store/useLibraryStore/useLibraryStore
- @tauri-apps/api/core/invoke
- ../types/typography/TextVariants
- ../utils/imageGrouping/expandGroupedPaths

### Community 94 — viewPoint() (19 nodes, cohesion: 0.25)

- applyLensBlur()
- applyLut()
- applyMaskUpdates()
- applyPointColors()
- applyVisualExtras()
- brushLines()
- buildMaskContainer()
- buildRemovalPatch()
- clampTo()
- fillPolygon()
- formatPatch()
- mergeNested()
- numberOr()
- pointColorFields()
- pointFrom()
- sanitizeAdjustments()
- sanitizeMaskAdjustments()
- toNumber()
- viewPoint()

### Community 95 — handleSelect() (19 nodes, cohesion: 0.11)

- ExportPresetsList
- handleDeletePreset()
- handleOverwritePreset()
- handleSavePreset()
- handleSelect()
- ./AppProperties/AppSettings
- ./Dropdown/Dropdown
- ./ExportImportProperties/ExportPreset
- lucide-react/Check
- lucide-react/Plus
- lucide-react/Save
- lucide-react/Trash2
- lucide-react/X
- react-i18next/useTranslation
- react/React
- react/useState
- ./Text/Text
- ../../types/typography/TextVariants
- uuid/v4

### Community 96 — stateOf() (19 nodes, cohesion: 0.15)

- importAssistant
- apply()
- buildImportContext()
- fileNameOf()
- ./assistantCull/CullDecision
- ./assistantCull/CullTarget
- ./assistantCull/ShotGroups
- ./assistantCull/ShotInfo
- ../components/views/import/importFilters/computeVisible
- ../store/useImportStore/useImportStore
- ../store/useSettingsStore/useSettingsStore
- importCullScope()
- importQGrades()
- importShotGroups()
- importVisiblePaths()
- isImportReviewActive()
- isImportWindowBusy()
- selectImportFiles()
- stateOf()

### Community 97 — clearAll() (19 nodes, cohesion: 0.11)

- ImportFilterBar
- clearAll()
- ../../../components/ui/AppProperties/SortDirection
- ../../../hooks/useSdImportActions/useSdImportActions
- ./importFilters/COLOR_HEX
- ./importFilters/FILE_TYPE_OPTIONS
- ./importFilters/FileTypeFilter
- ./importFilters/LABEL_COLORS
- lucide-react/ArrowDownUp
- lucide-react/Check
- lucide-react/ChevronDown
- lucide-react/ChevronUp
- lucide-react/Filter
- lucide-react/Star
- lucide-react/X
- react/useState
- ../../../store/useImportStore/ImportSortKey
- ../../../store/useImportStore/useImportStore
- zustand/react/shallow/useShallow

### Community 98 — Raw9Options (19 nodes, cohesion: 0.11)

- apple_raw
- denoise_raw9()
- anyhow::Result
- anyhow::{Result, anyhow}
- image::DynamicImage
- image::{DynamicImage, ImageBuffer, Rgba}
- objc2_core_foundation::CGRect
- objc2_core_graphics::{CGColorSpace, kCGColorSpaceExtendedLinearSRGB}
- objc2_core_image::{
        CIContext, CIContextOption, CIRAWFilter, kCIContextCacheIntermediates, kCIFormatRGBAf,
    }
- objc2_foundation::{NSData, NSDictionary, NSNumber, NSString, NSURL}
- objc2::rc::{Retained, autoreleasepool}
- objc2::runtime::{AnyClass, AnyObject}
- pub use imp::{develop_raw9, is_raw9_supported_for_path}
- std::ptr::NonNull
- std::time::Instant
- super::Raw9Options
- Raw9Options
- .for_denoise()
- .for_loading()

### Community 99 — rgb_to_lms() (19 nodes, cohesion: 0.13)

- white_balance
- adaptation_log_gains()
- as_shot_cache()
- as_shot_white_balance()
- crate::file_management::{parse_virtual_path, read_file_mapped}
- crate::formats::is_raw_file
- crate::image_processing::{PRIMARIES_SRGB, WP_D65, primaries_to_xyz_matrix}
- glam::{DMat3, DVec3, Mat3}
- serde::{Deserialize, Serialize}
- serde_json::Value
- std::collections::HashMap
- std::path::Path
- std::sync::{Mutex, OnceLock}
- super::*
- pick_white_balance()
- relative_shift()
- relative_shift_inverts_shifted()
- rgb_to_lms()
- .lms()

### Community 100 — useContextMenu() (19 nodes, cohesion: 0.11)

- ContextMenuContext
- closeSubmenu()
- if()
- clsx/clsx
- ../components/ui/AppProperties/Option
- ../components/ui/AppProperties/OPTION_SEPARATOR
- framer-motion/AnimatePresence
- framer-motion/motion
- lucide-react/ChevronRight
- react/createContext
- react-dom/createPortal
- react/FC
- react/useCallback
- react/useContext
- react/useEffect
- react/useLayoutEffect
- react/useRef
- react/useState
- useContextMenu()

### Community 101 — save_image_with_metadata() (18 nodes, cohesion: 0.19)

- encode_image_to_bytes()
- estimate_export_sizes()
- export_images()
- export_images_impl()
- exported_jpegs_are_tagged_full_chroma_and_baseline()
- ExportTaskGuard
- .drop()
- .new()
- .with_app_handle()
- finish_export_task()
- mime_type_for_extension()
- process_image_for_export()
- register_export_task()
- relative_dir_is_safe()
- relative_export_dir_for_preserved_folders()
- render_output_precision()
- run_headless_export()
- save_image_with_metadata()

### Community 102 — ../../../utils/adjustments/ADJUSTMENT_GROUPS (18 nodes, cohesion: 0.11)

- PresetItemDisplay
- framer-motion/AnimatePresence
- framer-motion/motion
- lucide-react/Crop
- lucide-react/Layers
- lucide-react/Loader2
- lucide-react/Palette
- lucide-react/Wrench
- react-i18next/useTranslation
- react/ReactNode
- react/useMemo
- ../../../types/typography/TextColors
- ../../../types/typography/TextVariants
- ../../../types/typography/TextWeights
- ../../ui/AppProperties/Preset
- ../../ui/Slider/Slider
- ../../ui/Text/Text
- ../../../utils/adjustments/ADJUSTMENT_GROUPS

### Community 103 — srgb_to_linear() (18 nodes, cohesion: 0.16)

- raw_processing
- develop_internal()
- develop_raw_image()
- extract_embedded_preview()
- get_fast_demosaic_scale_factor()
- anyhow::{Result, anyhow}
- crate::image_processing::apply_orientation
- crate::white_balance::WhiteBalance
- image::{DynamicImage, ImageBuffer, Rgba}
- rawler::{
    decoders::{Decoder, Orientation, RawDecodeParams},
    imgop::{
        develop::{DemosaicAlgorithm, Intermediate, ProcessingStep, RawDevelop},
        xyz::Illuminant,
    },
    rawimage::{RawImage, RawPhotometricInterpretation},
    rawsource::RawSource,
}
- std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
}
- is_linear_raw_format()
- metadata_orientation()
- read_as_shot_white_balance()
- recover_clipped_pixel()
- smootherstep()
- smoothstep()
- srgb_to_linear()

### Community 104 — react-i18next/initReactI18next (18 nodes, cohesion: 0.11)

- index
- i18next/i18n
- ./locales/ca.json/ca
- ./locales/cs.json/cs
- ./locales/de.json/de
- ./locales/en.json/en
- ./locales/es.json/es
- ./locales/fr.json/fr
- ./locales/it.json/it
- ./locales/ja.json/ja
- ./locales/ko.json/ko
- ./locales/nl.json/nl
- ./locales/pl.json/pl
- ./locales/pt.json/pt
- ./locales/ru.json/ru
- ./locales/zh-CN.json/zhCN
- ./locales/zh-TW.json/zhTW
- react-i18next/initReactI18next

### Community 105 — useAiMasking() (18 nodes, cohesion: 0.12)

- useAiMasking
- getTransformAdjustments()
- ../components/panel/right/Masks/SubMask
- ../components/ui/AppProperties/Invokes
- react-toastify/toast
- react/useCallback
- react/useEffect
- ../store/useCloudStore/useCloudStore
- ../store/useEditorStore/useEditorStore
- ../store/useProcessStore/useProcessStore
- @tauri-apps/api/core/invoke
- ./useEditorActions/debouncedSetHistory
- ./useEditorActions/useEditorActions
- ../utils/adjustments/Adjustments
- ../utils/adjustments/AiPatch
- ../utils/adjustments/Coord
- ../utils/adjustments/MaskContainer
- useAiMasking()

### Community 106 — ../../utils/adjustments/HueSatLum (18 nodes, cohesion: 0.11)

- ColorWheel
- framer-motion/AnimatePresence
- framer-motion/motion
- lucide-react/Sun
- react-i18next/useTranslation
- react/useEffect
- react/useId
- react/useRef
- react/useState
- ./Slider/Slider
- ./Text/Text
- ../../types/typography/TextColors
- ../../types/typography/TextVariants
- @uiw/color-convert/ColorResult
- @uiw/color-convert/HsvaColor
- @uiw/color-convert/hsvaToHex
- @uiw/react-color-wheel/Wheel
- ../../utils/adjustments/HueSatLum

### Community 107 — ../../ui/AppProperties/Invokes (17 nodes, cohesion: 0.12)

- ImportViewer
- ./importFilters/COLOR_HEX
- ./LazyThumb/LazyThumb
- lucide-react/Check
- lucide-react/Image
- lucide-react/LayoutGrid
- lucide-react/Loader2
- lucide-react/Maximize2
- lucide-react/Star
- ./RatingColor/RatingColor
- react/useCallback
- react/useEffect
- react/useRef
- react/useState
- ../../../store/useImportStore/useImportStore
- @tauri-apps/api/core/invoke
- ../../ui/AppProperties/Invokes

### Community 108 — verticals_stay_vertical_at_any_strength() (17 nodes, cohesion: 0.18)

- projection
- cylindrical_warp()
- estimate_focal()
- focals_from_homography()
- full_strength_is_rectilinear()
- full_strength_straightens_a_wall_line()
- image::{GrayImage, Rgb32FImage}
- nalgebra::Matrix3
- rayon::prelude::*
- super::*
- super::stitching::sample_bilinear
- pannini_forward()
- pannini_theta()
- pannini_theta_inverts_forward()
- remap_keeps_content_and_straightens_the_bow()
- straighten_cylindrical()
- verticals_stay_vertical_at_any_strength()

### Community 109 — clsx() (109) (17 nodes, cohesion: 0.12)

- Dropdown
- clsx()
- clsx/clsx
- framer-motion/AnimatePresence
- framer-motion/motion
- ./Input/Input
- lucide-react/Check
- lucide-react/ChevronDown
- react/useEffect
- react/useMemo
- react/useRef
- react/useState
- ./Text/Text
- ../../types/typography/TEXT_COLOR_KEYS
- ../../types/typography/TextColors
- ../../types/typography/TextVariants
- ../../types/typography/TextWeights

### Community 110 — zustand/react/shallow/useShallow (110) (17 nodes, cohesion: 0.12)

- LibraryView
- ../panel/BottomBar/BottomBar
- ../panel/CommunityPage/CommunityPage
- ../panel/MainLibrary/MainLibrary
- ../../store/useEditorStore/useEditorStore
- ../../store/useLibraryStore/useLibraryStore
- ../../store/useProcessStore/useProcessStore
- ../../store/useSettingsStore/useSettingsStore
- ../../store/useUIStore/useUIStore
- ../ui/AppProperties/ImageFile
- ../ui/AppProperties/LibraryViewMode
- ../ui/AppProperties/ThumbnailAspectRatio
- ../ui/AppProperties/ThumbnailSize
- ../../utils/imageFlags/getImageFlag
- ../../utils/imageGrouping/GroupBadgeInfo
- ../../utils/imageGrouping/GroupId
- zustand/react/shallow/useShallow

### Community 111 — test_ai_connector_connection() (17 nodes, cohesion: 0.12)

- ai_commands
- cancel_ai_task()
- check_ai_connector_status()
- generate_full_image_depth_map()
- base64::{Engine as _, engine::general_purpose}
- crate::ai_connector
- crate::ai_processing::{
    AiDepthMaskParameters, AiForegroundMaskParameters, AiSkyMaskParameters,
    AiSubjectMaskParameters, CachedDepthMap, generate_face_region_mask, generate_image_embeddings,
    get_or_init_ai_models, get_or_init_face_model, run_depth_anything_model, run_sam_decoder,
    run_sky_seg_model, run_u2netp_model,
}
- crate::app_settings::load_settings
- crate::app_state::{AiTaskGuard, AppState}
- crate::cache_utils::GEOMETRY_KEYS
- crate::get_cached_full_warped_image
- image::{GrayImage, ImageFormat}
- std::collections::hash_map::DefaultHasher
- std::hash::{Hash, Hasher}
- std::io::Cursor
- precompute_ai_subject_mask()
- test_ai_connector_connection()

### Community 112 — score() (16 nodes, cohesion: 0.25)

- label_detect
- Component
- detect_label_region()
- fill_rect()
- finds_a_bright_card_on_dark_fabric()
- frame()
- ignores_a_small_stray_thread()
- ignores_the_steel_ruler_and_picks_the_card()
- image::{DynamicImage, GenericImageView}
- image::{Rgb, RgbImage}
- super::*
- never_reports_a_box_outside_the_image()
- Rect
- returns_none_when_nothing_is_card_like()
- scale_and_pad()
- score()

### Community 113 — PresetTypeSwitch() (16 nodes, cohesion: 0.13)

- ConfigurePresetModal
- ConfigurePresetModal()
- clsx/clsx
- framer-motion/motion
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useMemo
- react/useRef
- react/useState
- ../../types/typography/TextVariants
- ../ui/AppProperties/Preset
- ../ui/Switch/Switch
- ../ui/Text/Text
- ../../utils/adjustments/ADJUSTMENT_GROUPS
- PresetTypeSwitch()

### Community 114 — async() (15 nodes, cohesion: 0.13)

- ImportReviewBar
- async()
- ../../../hooks/useSdImportActions/useSdImportActions
- lucide-react/ArrowRight
- lucide-react/FolderInput
- lucide-react/Settings2
- react/useEffect
- react/useState
- ../../../store/useImportStore/useImportStore
- ../../ui/Button/Button
- ../../ui/Switch/Switch
- ../../../utils/importLearning/CullLearning
- ../../../utils/importLearning/describeLearning
- ../../../utils/importLearning/loadCullLearning
- zustand/react/shallow/useShallow

### Community 115 — handleClose() (15 nodes, cohesion: 0.13)

- LiveViewOverlay
- handleClose()
- lucide-react/Loader2
- lucide-react/X
- react-i18next/useTranslation
- react/useEffect
- react/useState
- ../../../store/useTetherStore/useTetherStore
- @tauri-apps/api/core/invoke
- @tauri-apps/api/event/listen
- ./TetherMenu/CameraSection
- ../../../types/typography/TextColors
- ../../../types/typography/TextVariants
- ../../ui/AppProperties/Invokes
- ../../ui/Text/Text

### Community 116 — handleVisibilityClick() (15 nodes, cohesion: 0.13)

- CollapsibleSection
- handleMouseEnter()
- handleMouseLeave()
- handleVisibilityClick()
- clsx/clsx
- ../../hooks/useCollapsibleHeight/useCollapsibleHeight
- lucide-react/ChevronDown
- lucide-react/Eye
- lucide-react/EyeOff
- react-i18next/useTranslation
- react/useRef
- react/useState
- ./Text/Text
- ../../types/typography/TextVariants
- ../../types/typography/TextWeights

### Community 117 — usePresets() (15 nodes, cohesion: 0.14)

- usePresets
- arrayMove()
- ../components/ui/AppProperties/Folder
- ../components/ui/AppProperties/Invokes
- ../components/ui/AppProperties/Preset
- lodash.debounce/debounce
- react/useCallback
- react/useEffect
- react/useState
- @tauri-apps/api/core/invoke
- ../utils/adjustments/ADJUSTMENT_GROUPS
- ../utils/adjustments/Adjustments
- ../utils/adjustments/COPYABLE_ADJUSTMENT_KEYS
- ../utils/adjustments/INITIAL_ADJUSTMENTS
- usePresets()

### Community 118 — GlobalTooltip() (15 nodes, cohesion: 0.14)

- GlobalTooltip
- clamp()
- GlobalTooltip()
- clsx/clsx
- framer-motion/AnimatePresence
- framer-motion/motion
- react-dom/createPortal
- react/useEffect
- react/useLayoutEffect
- react/useRef
- react/useState
- ./Text/Text
- ../../types/typography/TextColors
- ../../types/typography/TextVariants
- ../../types/typography/TextWeights

### Community 119 — stringifyArg() (15 nodes, cohesion: 0.27)

- frontendLogBridge
- extractViteDetails()
- formatLogMessage()
- formatViteErrorDetails()
- getRecordField()
- ../components/ui/AppProperties/Invokes
- @tauri-apps/api/core/invoke
- installFrontendLogBridge()
- isPlainRecord()
- isViteLikeError()
- sendToBackend()
- serializeValue()
- shouldDropDuplicate()
- shouldIgnoreMessage()
- stringifyArg()

### Community 120 — useFileOperations() (15 nodes, cohesion: 0.13)

- useFileOperations
- ../components/ui/AppProperties/ImageFlag
- ../components/ui/AppProperties/Invokes
- ../components/ui/ExportImportProperties/Status
- react-i18next/useTranslation
- react-toastify/toast
- react/useCallback
- ../store/useEditorStore/useEditorStore
- ../store/useLibraryStore/useLibraryStore
- ../store/useProcessStore/useProcessStore
- ../store/useSettingsStore/useSettingsStore
- ../store/useUIStore/useUIStore
- @tauri-apps/api/core/invoke
- @tauri-apps/plugin-dialog/open
- useFileOperations()

### Community 121 — ../utils/imageGrouping/GroupId (15 nodes, cohesion: 0.13)

- useSortedLibrary
- ../components/ui/AppProperties/EditedStatus
- ../components/ui/AppProperties/FlagStatus
- ../components/ui/AppProperties/GroupingMode
- ../components/ui/AppProperties/ImageFile
- ../components/ui/AppProperties/ImageFlag
- ../components/ui/AppProperties/NegativeStatus
- ../components/ui/AppProperties/RawStatus
- ../components/ui/AppProperties/SortDirection
- react/useMemo
- ../store/useLibraryStore/useLibraryStore
- ../store/useSettingsStore/useSettingsStore
- ../utils/imageGrouping/buildImageGroups
- ../utils/imageGrouping/GroupBadgeInfo
- ../utils/imageGrouping/GroupId

### Community 122 — rankedGroups() (14 nodes, cohesion: 0.18)

- importLearning
- assistantFeedback()
- describeLearning()
- fileNameOf()
- ../components/ui/AppProperties/CullingSuggestions
- ../components/ui/AppProperties/Invokes
- ../components/views/import/importFilters/computeVisibleSet
- ../store/useImportStore/useImportStore
- ../store/useSettingsStore/useSettingsStore
- @tauri-apps/api/core/invoke
- keepersOf()
- learnFromImport()
- loadCullLearning()
- rankedGroups()

### Community 123 — useExternalEditSession() (14 nodes, cohesion: 0.14)

- useExternalEditSession
- ../components/ui/AppProperties/Invokes
- ../components/ui/ExportImportProperties/ExportSettings
- ../components/ui/ExportImportProperties/Status
- react/useCallback
- react/useEffect
- react/useRef
- react/useState
- ../store/useEditorStore/useEditorStore
- ../store/useProcessStore/useProcessStore
- @tauri-apps/api/core/invoke
- @tauri-apps/plugin-process/exit
- ./useEditorActions/debouncedSave
- useExternalEditSession()

### Community 124 — withdrawAutoCrop() (14 nodes, cohesion: 0.23)

- guidedPerspective
- applyGuidedAutoCrop()
- commitGuideLines()
- cropIsOurs()
- ./adjustments/Adjustments
- ./adjustments/GuideLine
- ./cropUtils/getOrientedDimensions
- ./cropUtils/guidedRectToCrop
- react-image-crop/Crop
- @tauri-apps/api/core/invoke
- isUncropped()
- sameCrop()
- setGuidedAutoCrop()
- withdrawAutoCrop()

### Community 125 — DecodedImageCache (13 nodes, cohesion: 0.18)

- cache_utils
- clear_image_caches()
- clear_session_caches()
- DecodedImageCache
- .clear()
- .insert()
- .set_capacity()
- crate::AppState
- image::DynamicImage
- std::collections::hash_map::DefaultHasher
- std::collections::HashMap
- std::hash::{Hash, Hasher}
- std::sync::Arc

### Community 126 — onWebViewCreate (13 nodes, cohesion: 0.15)

- MainActivity
- handleOnBackPressed
- android.graphics.Color
- android.os.Bundle
- android.view.View
- android.webkit.WebView
- androidx.activity.enableEdgeToEdge
- androidx.activity.OnBackPressedCallback
- androidx.core.view.ViewCompat
- androidx.core.view.WindowInsetsCompat
- MainActivity
- onCreate
- onWebViewCreate

### Community 127 — getLine() (13 nodes, cohesion: 0.15)

- Waveform
- getFill()
- getLine()
- framer-motion/AnimatePresence
- framer-motion/LayoutGroup
- framer-motion/motion
- lucide-react/AlertOctagon
- react-i18next/useTranslation
- react/useEffect
- react/useRef
- react/useState
- ../../ui/AppProperties/WaveformData
- ../../../utils/adjustments/DisplayMode

### Community 128 — formatBytes() (13 nodes, cohesion: 0.15)

- SourcePicker
- formatBytes()
- ../../../hooks/useSdImportActions/useSdImportActions
- lucide-react/Film
- lucide-react/FolderOpen
- lucide-react/HardDrive
- lucide-react/Loader2
- lucide-react/RefreshCw
- react/useEffect
- ./ScannerPane/detectScanner
- ../../../store/useImportStore/useImportStore
- ../../../store/useScannerStore/useScannerStore
- ../../ui/Button/Button

### Community 129 — handleToggle() (13 nodes, cohesion: 0.15)

- AdjustmentSubSection
- handleToggle()
- clsx/clsx
- ../../hooks/useCollapsibleHeight/useCollapsibleHeight
- lucide-react/ChevronDown
- react/ReactNode
- ../../store/useSettingsStore/useSettingsStore
- ../../types/typography/TextVariants
- ../ui/Text/Text
- ../../utils/adjustments/ADJUSTMENT_SECTION_TOOLS
- ../../utils/adjustments/getAdjustmentSectionToolIds
- ../../utils/adjustments/withAdjustmentLayout
- zustand/react/shallow/useShallow

### Community 130 — handleReset() (130) (13 nodes, cohesion: 0.18)

- DepthRangePicker
- beginDrag()
- compute()
- getVal()
- handleColor()
- handleReset()
- react-i18next/useTranslation
- react/PointerEvent
- react/useEffect
- react/useRef
- react/useState
- ./Text/Text
- ../../types/typography/TextVariants

### Community 131 — SeamInfo (13 nodes, cohesion: 0.15)

- stitching
- crate::panorama_stitching::ImageInfo
- image::{GrayImage, Rgb, Rgb32FImage}
- nalgebra::{Matrix3, Point3}
- rayon::prelude::*
- std::collections::HashMap
- std::path::Path
- tauri::{AppHandle, Emitter}
- largest_content_rect()
- sample_bilinear()
- SeamContext
- SeamInfo
- SeamOrientation

### Community 132 — zustand/create (132) (13 nodes, cohesion: 0.15)

- useEditorStore
- ../components/adjustments/Curves/ChannelConfig
- ../components/panel/right/CropPanel/OverlayMode
- ../components/panel/right/Masks/ToolType
- ../components/ui/AppProperties/BrushSettings
- ../components/ui/AppProperties/SelectedImage
- ../components/ui/AppProperties/WaveformData
- ../hooks/useImageRenderSize/ImageDimensions
- ../utils/adjustments/Adjustments
- ../utils/adjustments/AiPatch
- ../utils/adjustments/INITIAL_ADJUSTMENTS
- ../utils/adjustments/MaskContainer
- zustand/create

### Community 133 — outcome() (13 nodes, cohesion: 0.18)

- cull_model
- CueKind
- .default_weights()
- default_weights_sum_to_one()
- GroupOutcome
- serde::{Deserialize, Serialize}
- std::collections::HashMap
- std::path::PathBuf
- super::*
- tauri::Manager
- LearningSummary
- mixed_kinds_in_a_burst_are_not_compared()
- outcome()

### Community 134 — RestoreDownIcon() (12 nodes, cohesion: 0.17)

- TitleBar
- handleClose()
- handleMinimize()
- lucide-react/Minus
- lucide-react/Square
- lucide-react/X
- react/useCallback
- react/useEffect
- react/useState
- @tauri-apps/api/window/getCurrentWindow
- @tauri-apps/plugin-os/platform
- RestoreDownIcon()

### Community 135 — useTauriListeners() (12 nodes, cohesion: 0.17)

- useTauriListeners
- ../components/ui/ExportImportProperties/Status
- react-toastify/toast
- react/useEffect
- react/useRef
- ../store/useEditorStore/useEditorStore
- ../store/useLibraryStore/useLibraryStore
- ../store/useProcessStore/useProcessStore
- ../store/useUIStore/useUIStore
- @tauri-apps/api/core/convertFileSrc
- @tauri-apps/api/event/listen
- useTauriListeners()

### Community 136 — WhiteBalance (12 nodes, cohesion: 0.27)

- from_adjustments()
- isotherm_direction()
- WhiteBalance
- .chromaticity_lms()
- .clamped()
- .from_camera_neutral()
- .from_dual_illuminant_camera_neutral()
- .from_xy()
- .from_xyz()
- .reference()
- .shifted()
- .to_xy()

### Community 137 — save_manual_order() (12 nodes, cohesion: 0.21)

- manual_order
- cache_dir()
- serde::{Deserialize, Serialize}
- std::collections::HashMap
- std::fs
- std::path::PathBuf
- super::*
- tauri::{AppHandle, Manager}
- load_manual_order()
- order_path()
- OrderFile
- save_manual_order()

### Community 138 — parse_launch_args() (12 nodes, cohesion: 0.18)

- launch_request
- emit_launch_request()
- ExternalEditSession
- handle_file_open()
- HeadlessExportSession
- crate::export_processing::TiffBitDepth
- serde::{Deserialize, Serialize}
- std::path::PathBuf
- tauri::Emitter
- LaunchPayload
- LaunchRequest
- parse_launch_args()

### Community 139 — handleReset() (139) (12 nodes, cohesion: 0.17)

- Basic
- handleReset()
- clsx/clsx
- framer-motion/motion
- react-i18next/useTranslation
- react/useEffect
- react/useMemo
- react/useRef
- react/useState
- ../ui/Slider/Slider
- ../../utils/adjustments/Adjustments
- ../../utils/adjustments/BasicAdjustment

### Community 140 — parse_point_colors() (12 nodes, cohesion: 0.35)

- calculate_agx_matrices()
- convert_points_to_aligned()
- get_all_adjustments_from_json()
- get_global_adjustments_from_json()
- get_mask_adjustments_from_json()
- GpuMat3
- .default()
- is_section_visible()
- mat3_to_gpu_mat3()
- parse_color_grade_settings()
- parse_hsl_adjustments()
- parse_point_colors()

### Community 141 — regularize_poses() (11 nodes, cohesion: 0.27)

- LensWarp
- .apply()
- .identity()
- .is_plausible()
- .jacobian()
- .max_displacement()
- .predistort()
- .to_vector()
- .with_vector()
- poly_basis()
- regularize_poses()

### Community 142 — summarizeWindow() (11 nodes, cohesion: 0.27)

- replay
- durationsInWindow()
- frameTick()
- median()
- p95()
- startMeasuring()
- statOf()
- stdev()
- stopMeasuring()
- summarizePhaseAcrossIterations()
- summarizeWindow()

### Community 143 — unorientPoint() (11 nodes, cohesion: 0.31)

- cropUtils
- calculateAutoCropForRotation()
- calculateCenteredCrop()
- calculateStraightenAngle()
- fitCropTowards()
- getOrientedDimensions()
- guidedRectToCrop()
- react-image-crop/Crop
- orientPoint()
- rotateCropCenter()
- unorientPoint()

### Community 144 — useImageLoader() (11 nodes, cohesion: 0.18)

- useImageLoader
- ../components/ui/AppProperties/Invokes
- react-toastify/toast
- react/useEffect
- ../store/useEditorStore/useEditorStore
- ../store/useLibraryStore/useLibraryStore
- ../store/useSettingsStore/useSettingsStore
- @tauri-apps/api/core/invoke
- ../utils/adjustments/INITIAL_ADJUSTMENTS
- ../utils/adjustments/normalizeLoadedAdjustments
- useImageLoader()

### Community 145 — reconcileWorkspace() (11 nodes, cohesion: 0.18)

- useUIStore
- ../components/ui/AppProperties/CollapsibleSectionsState
- ../components/ui/AppProperties/CullingSuggestions
- ../components/ui/AppProperties/ImageFile
- ../components/ui/AppProperties/Panel
- ../components/ui/AppProperties/PanelRegion
- ../components/ui/AppProperties/UiVisibility
- ../components/ui/AppProperties/WorkspaceState
- ./useEditorStore/useEditorStore
- zustand/create
- reconcileWorkspace()

### Community 146 — stitch_images() (11 nodes, cohesion: 0.33)

- build_stitching_order()
- compute_gain_compensation()
- compute_pairwise_matches()
- Dsu
- .find()
- .new()
- .union()
- estimate_span_degrees()
- overlap_luminance_ratio()
- prepare_image_info()
- stitch_images()

### Community 147 — runTauriCli (11 nodes, cohesion: 0.20)

- BuildTask
- assemble
- BuildTask
- java.io.File
- org.apache.tools.ant.taskdefs.condition.Os
- org.gradle.api.DefaultTask
- org.gradle.api.GradleException
- org.gradle.api.logging.LogLevel
- org.gradle.api.tasks.Input
- org.gradle.api.tasks.TaskAction
- runTauriCli

### Community 148 — withRelativeWhiteBalance() (11 nodes, cohesion: 0.31)

- whiteBalance
- clamp()
- getRelativeWhiteBalance()
- getWhiteBalanceMode()
- ../components/ui/AppProperties/AppSettings
- resolveWhiteBalance()
- toMired()
- toRelativeUnits()
- toRelativeWhiteBalance()
- withKelvinWhiteBalance()
- withRelativeWhiteBalance()

### Community 149 — load_frames() (11 nodes, cohesion: 0.27)

- as_bytes()
- as_bytes_mut()
- check_dims()
- DiskSource
- .create()
- .dims()
- .drop()
- .get()
- .path()
- .write()
- load_frames()

### Community 150 — zustand/create (150) (11 nodes, cohesion: 0.18)

- useLibraryStore
- ../components/panel/MainLibrary/ColumnWidths
- ../components/ui/AppProperties/AlbumItem
- ../components/ui/AppProperties/FilterCriteria
- ../components/ui/AppProperties/ImageFile
- ../components/ui/AppProperties/RawStatus
- ../components/ui/AppProperties/SortCriteria
- ../components/ui/AppProperties/SortDirection
- ../utils/adjustments/Adjustments
- ../utils/adjustments/INITIAL_ADJUSTMENTS
- zustand/create

### Community 151 — handleVariableClick() (11 nodes, cohesion: 0.18)

- ImportSettingsModal
- handleVariableClick()
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useRef
- react/useState
- ../../types/typography/TextVariants
- ../ui/ExportImportProperties/FILENAME_VARIABLES
- ../ui/Switch/Switch
- ../ui/Text/Text

### Community 152 — ImageLRUCache (11 nodes, cohesion: 0.36)

- ImageLRUCache
- ImageLRUCache
- .cleanupEntry()
- .clear()
- .constructor()
- .delete()
- .deleteByPrefix()
- .get()
- .isProtected()
- .set()
- ./adjustments/Adjustments

### Community 153 — useExportSettings() (11 nodes, cohesion: 0.18)

- useExportSettings
- ../components/ui/ExportImportProperties/BorderBasis
- ../components/ui/ExportImportProperties/DEFAULT_FILENAME_TEMPLATE
- ../components/ui/ExportImportProperties/ExportPreset
- ../components/ui/ExportImportProperties/sanitizeFilenameTemplate
- ../components/ui/ExportImportProperties/TiffBitDepth
- ../components/ui/ExportImportProperties/WatermarkAnchor
- react/useCallback
- react/useMemo
- react/useState
- useExportSettings()

### Community 154 — is_android_content_uri() (11 nodes, cohesion: 0.18)

- android_integration
- get_android_cached_lut_path()
- get_android_internal_library_root()
- jni22::{EnvUnowned as VerifierEnvUnowned, objects::JObject as VerifierJObject}
- jni::{JNIEnv, JavaVM}
- jni::objects::{JObject, JString, JValue}
- ndk_context::android_context
- std::fs
- std::path::PathBuf
- initialize_android()
- is_android_content_uri()

### Community 155 — KeepPolicy (11 nodes, cohesion: 0.29)

- .summary()
- defaults_reproduce_the_old_behaviour()
- keeping_two_per_burst_is_learned()
- keeping_weak_bursts_lowers_the_grade_threshold()
- KeepPolicy
- .grade_prior()
- .groups()
- .keep_grade()
- .keep_rank()
- .rank_prior()
- .typical_keep()

### Community 156 — update_auto_profile_from_image() (10 nodes, cohesion: 0.33)

- flat_gradient()
- kept_warmth_is_not_undone_by_auto_white_balance()
- learn_auto_profile_from_folder()
- learn_style()
- learned_style_changes_what_auto_asks_for()
- luma_median()
- measure_output_style()
- median_f64()
- style_measures_contrast_and_kept_warmth()
- update_auto_profile_from_image()

### Community 157 — resolve_xmp_path() (10 nodes, cohesion: 0.31)

- adjustments_is_negative()
- assign_group_ids()
- enqueue_metadata()
- generate_thumbnail_data()
- is_cloud_placeholder()
- list_images_in_dir()
- list_images_recursive()
- make_group_key()
- resolve_image_metadata()
- resolve_xmp_path()

### Community 158 — ConfirmModal() (10 nodes, cohesion: 0.20)

- ConfirmModal
- ConfirmModal()
- react-dom/createPortal
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useState
- ../../types/typography/TextVariants
- ../ui/Button/Button
- ../ui/Text/Text

### Community 159 — ../../types/typography/VariantConfig (10 nodes, cohesion: 0.20)

- Text
- clsx/clsx
- react/forwardRef
- react/React
- ../../types/typography/TEXT_COLOR_KEYS
- ../../types/typography/TEXT_WEIGHT_KEYS
- ../../types/typography/TextColor
- ../../types/typography/TextVariants
- ../../types/typography/TextWeight
- ../../types/typography/VariantConfig

### Community 160 — v1_model_files_migrate_into_people() (10 nodes, cohesion: 0.27)

- a_changed_cue_set_starts_that_kind_over()
- cue_preferences_learn_per_kind_within_a_burst()
- .default()
- .preference()
- .score()
- Preference
- .add()
- .effective_weights()
- .record()
- v1_model_files_migrate_into_people()

### Community 161 — default_tagging_shortcuts_option() (10 nodes, cohesion: 0.20)

- AppSettings
- .default()
- AutoAdjustOptions
- .default()
- .deserialize()
- default_adjustment_visibility()
- default_export_presets()
- default_linear_raw_mode()
- default_open_tree_sections()
- default_tagging_shortcuts_option()

### Community 162 — handleAdjustmentChange() (10 nodes, cohesion: 0.20)

- Details
- handleAdjustmentChange()
- ./AdjustmentSubSection/AdjustmentSubSection
- react-i18next/useTranslation
- ../ui/AppProperties/AppSettings
- ../ui/Slider/Slider
- ../../utils/adjustments/Adjustments
- ../../utils/adjustments/DetailsAdjustment
- ../../utils/adjustments/getAdjustmentToolOrder
- ../../utils/adjustments/getHiddenAdjustmentTools

### Community 163 — RenameFileModal() (10 nodes, cohesion: 0.20)

- RenameFileModal
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useRef
- react/useState
- ../../types/typography/TextVariants
- ../ui/ExportImportProperties/FILENAME_VARIABLES
- ../ui/Text/Text
- RenameFileModal()

### Community 164 — hydrate_sub_masks() (10 nodes, cohesion: 0.24)

- adjustment_utils
- apply_all_transformations()
- apply_spatial_transformations()
- hydrate_adjustments()
- hydrate_sub_masks()
- crate::app_state::AppState
- crate::image_processing::{
    Crop, IntoCowImage, apply_coarse_rotation, apply_crop, apply_flip, apply_geometry_warp,
    apply_rotation,
}
- image::DynamicImage
- std::borrow::Cow
- std::collections::HashMap

### Community 165 — ycbcr_to_rgb() (10 nodes, cohesion: 0.27)

- apply_denoising()
- .new()
- batch_denoise_images()
- denoise_image()
- gaussian_blur_1ch()
- merge_channels()
- rgb_to_ycbcr()
- run_bm3d()
- split_channels()
- ycbcr_to_rgb()

### Community 166 — uniqueNames() (10 nodes, cohesion: 0.33)

- assistantCull
- decisionLine()
- fileNameOf()
- ../components/ui/AppProperties/Invokes
- @tauri-apps/api/core/invoke
- matchDecision()
- planCullChunks()
- prepareAll()
- runAssistantCull()
- uniqueNames()

### Community 167 — RustPlugin (10 nodes, cohesion: 0.20)

- RustPlugin
- apply
- Config
- com.android.build.api.dsl.ApplicationExtension
- org.gradle.api.DefaultTask
- org.gradle.api.Plugin
- org.gradle.api.Project
- org.gradle.kotlin.dsl.configure
- org.gradle.kotlin.dsl.get
- RustPlugin

### Community 168 — sync_album_path_changes() (10 nodes, cohesion: 0.38)

- albums
- add_to_album()
- AlbumItem
- get_album_images()
- get_albums()
- get_albums_path()
- super::*
- save_albums()
- sort_album_tree()
- sync_album_path_changes()

### Community 169 — pickPrimary() (10 nodes, cohesion: 0.29)

- imageGrouping
- buildImageGroups()
- expandGroupedPaths()
- findGroupVariants()
- getFileExtension()
- getVariantLabel()
- ../components/ui/AppProperties/GroupingMode
- ../components/ui/AppProperties/GroupPreference
- ../components/ui/AppProperties/ImageFile
- pickPrimary()

### Community 170 — write_rrexif_sidecar() (10 nodes, cohesion: 0.40)

- get_rrexif_path()
- load_primary_metadata()
- load_sidecar()
- load_sidecar_with_exif()
- persist_exif_if_missing()
- read_exif_data()
- read_rrexif_sidecar()
- save_exif_to_rrcache()
- save_primary_metadata()
- write_rrexif_sidecar()

### Community 171 — restoreSelection() (9 nodes, cohesion: 0.22)

- useImportStore
- ../components/ui/AppProperties/CullingSettings
- ../components/ui/AppProperties/CullingSuggestions
- ../components/ui/AppProperties/SortDirection
- ../components/views/import/importFilters/FileTypeFilter
- ../utils/importLearning/AiDecision
- zustand/create
- zustand/middleware/persist
- restoreSelection()

### Community 172 — truncate_large_exif() (9 nodes, cohesion: 0.42)

- clean_ascii_value()
- extract_metadata()
- format_lens_spec()
- format_lens_specification()
- insert_missing_lens_fields()
- rational_to_f32_checked()
- rawler_rational_to_f32_checked()
- read_exif_data_from_bytes()
- truncate_large_exif()

### Community 173 — verify_sha256() (9 nodes, cohesion: 0.31)

- build
- download_and_verify()
- sha2::{Digest, Sha256}
- std::env
- std::fs
- std::io::{self, Read}
- std::path::{Path, PathBuf}
- main()
- verify_sha256()

### Community 174 — strip_maker_prefix() (9 nodes, cohesion: 0.44)

- find_lens_by_fuzzy_model()
- get_lensfun_makers()
- Lens
- .get_canonical_model_name()
- .get_display_name()
- .get_full_model_name()
- .get_maker()
- .get_name()
- strip_maker_prefix()

### Community 175 — render_rgba_f32() (9 nodes, cohesion: 0.31)

- configure()
- core_image_raw_available()
- develop_raw9()
- is_dng()
- is_raw9_available()
- is_raw9_supported_for_path()
- make_filter()
- pick_raw9_version()
- render_rgba_f32()

### Community 176 — MetadataManager (9 nodes, cohesion: 0.28)

- .new()
- AiTaskToken
- .cancel()
- .default()
- .is_cancelled()
- .new()
- .wait_for_cancel()
- MetadataManager
- .new()

### Community 177 — scan_dir_lazy() (9 nodes, cohesion: 0.36)

- folder_tree
- FolderNode
- get_folder_children()
- get_folder_tree()
- get_folder_tree_sync()
- get_pinned_folder_trees()
- has_subdirs()
- super::*
- scan_dir_lazy()

### Community 178 — vanishingPoint() (9 nodes, cohesion: 0.44)

- keystone
- autoFitScale()
- clamp()
- fitScaleForParams()
- frameFitsInside()
- invSource()
- lineCoeffs()
- solveKeystone()
- vanishingPoint()

### Community 179 — Plane (9 nodes, cohesion: 0.33)

- catmull_weights()
- collect_samples()
- Plane
- .at()
- .clamped()
- .gradients()
- .new()
- .sample_bilinear()
- .sample_catmull_rom()

### Community 180 — zustand/create (8 nodes, cohesion: 0.25)

- useSettingsStore
- ../components/ui/AppProperties/AppSettings
- ../components/ui/AppProperties/Invokes
- ../components/ui/AppProperties/SupportedTypes
- @tauri-apps/api/core/invoke
- @tauri-apps/plugin-os/platform
- ../utils/themes/DEFAULT_THEME_ID
- zustand/create

### Community 181 — ComfyClient (8 nodes, cohesion: 0.43)

- ComfyClient
- .check_health()
- .execute()
- ._fetch_image()
- ._get_history()
- .__init__()
- ._queue_prompt()
- .get()

### Community 182 — wait() (8 nodes, cohesion: 0.32)

- backToLibrary()
- dispatchMouse()
- dragSlider()
- editTwoSliders()
- openFirstImage()
- scrollLibrary()
- undoSliderEdits()
- wait()

### Community 183 — calculate_visual_hash() (8 nodes, cohesion: 0.43)

- calculate_full_job_hash()
- calculate_geometry_hash()
- calculate_patched_warped_hash()
- calculate_thumbnail_base_hash()
- calculate_transform_hash()
- calculate_visual_hash()
- .get()
- .new()

### Community 184 — verify_sha256() (184) (8 nodes, cohesion: 0.39)

- download_and_verify_model()
- get_models_dir()
- get_or_init_ai_models()
- get_or_init_denoise_model()
- get_or_init_face_model()
- get_or_init_lama_model()
- promote_legacy_model_filename()
- verify_sha256()

### Community 185 — warp_image_homography() (8 nodes, cohesion: 0.43)

- canvas_bounds()
- find_adaptive_seam()
- find_pairwise_seam_dp_horizontal()
- find_pairwise_seam_dp_vertical()
- get_interpolated_pixel()
- progressive_seam_stitcher()
- source_valid()
- warp_image_homography()

### Community 186 — resolveAutoAdjust() (8 nodes, cohesion: 0.25)

- AppProperties
- ./ExportImportProperties/ExportPreset
- ../panel/right/Masks/ToolType
- ../../utils/adjustments/Adjustments
- ../../utils/adjustments/CopyPasteSettings
- ../../utils/whiteBalance/WhiteBalance
- ../../utils/whiteBalance/WhiteBalanceMode
- resolveAutoAdjust()

### Community 187 — useThumbnails() (8 nodes, cohesion: 0.25)

- useThumbnails
- lodash.debounce/debounce
- react/useCallback
- react/useEffect
- react/useMemo
- react/useRef
- @tauri-apps/api/core/invoke
- useThumbnails()

### Community 188 — CullModel (8 nodes, cohesion: 0.43)

- CullModel
- .file_path()
- .from_json()
- .load()
- .preference_mut()
- .record_import()
- .reset()
- .save()

### Community 189 — Num (8 nodes, cohesion: 0.25)

- preset_converter
- crate::file_management::Preset
- crate::white_balance::{MIRED_PER_RELATIVE_UNIT, TINT_PER_RELATIVE_UNIT}
- regex::Regex
- serde_json::{Map, Value, json}
- std::collections::HashMap
- uuid::Uuid
- Num

### Community 190 — reset_cull_feedback() (8 nodes, cohesion: 0.39)

- CullFeedback
- .add()
- .load()
- .path()
- .save()
- only_the_newest_corrections_are_kept()
- record_assistant_cull_feedback()
- reset_cull_feedback()

### Community 191 — nextMessageId() (7 nodes, cohesion: 0.33)

- useAssistantStore
- deriveTitle()
- zustand/create
- zustand/middleware/persist
- makeConversation()
- nextConversationId()
- nextMessageId()

### Community 192 — resolve_android_content_uri_name() (7 nodes, cohesion: 0.57)

- clear_pending_android_exception()
- close_android_closeable()
- get_android_content_resolver()
- map_android_jni_error()
- parse_android_uri()
- read_android_content_uri()
- resolve_android_content_uri_name()

### Community 193 — resolveNudgeStep() (7 nodes, cohesion: 0.38)

- keyboardUtils
- codeToDisplayLabel()
- formatKeyCode()
- getDefaultCombo()
- isValidShortcutKey()
- normalizeCombo()
- resolveNudgeStep()

### Community 194 — update_json_file() (7 nodes, cohesion: 0.43)

- update_translations
- deep_merge()
- json
- pathlib.Path
- main()
- sort_dict_recursively()
- update_json_file()

### Community 195 — useWaveformControls() (7 nodes, cohesion: 0.29)

- useWaveformControls
- react/PointerEvent
- react/useCallback
- react/useState
- ../store/useEditorStore/useEditorStore
- ../store/useSettingsStore/useSettingsStore
- useWaveformControls()

### Community 196 — v() (7 nodes, cohesion: 0.43)

- appends_files_added_since_the_order_was_saved()
- apply_order()
- every_present_file_appears_exactly_once()
- ignores_names_that_are_gone()
- keeps_the_saved_order()
- no_saved_order_leaves_the_folder_untouched()
- v()

### Community 197 — xmp_packet_for_source() (7 nodes, cohesion: 0.33)

- apply_gps_from_kamadak()
- apply_gps_from_rawler()
- apply_sidecar_field_overrides()
- copy_full_exif_from_source()
- write_image_with_metadata()
- write_xmp_only()
- xmp_packet_for_source()

### Community 198 — wiener_filter() (7 nodes, cohesion: 0.33)

- AtomicAccumulator
- .add()
- .to_vec()
- bm3d_process_joint()
- hard_threshold()
- run_bm3d_step_joint()
- wiener_filter()

### Community 199 — TileParams (7 nodes, cohesion: 0.38)

- face_area()
- face_iou()
- generate_face_region_mask()
- run_face_detection()
- run_lama_inpainting()
- TileParams
- .new()

### Community 200 — ../ui/Text/Text (7 nodes, cohesion: 0.29)

- RenameFolderModal
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useState
- ../../types/typography/TextVariants
- ../ui/Text/Text

### Community 201 — useProductivityActions() (7 nodes, cohesion: 0.29)

- useProductivityActions
- ../components/modals/DenoiseModal/DenoiseMethod
- ../components/ui/AppProperties/Invokes
- react/useCallback
- ../store/useUIStore/useUIStore
- @tauri-apps/api/core/invoke
- useProductivityActions()

### Community 202 — ../ui/Text/Text (202) (7 nodes, cohesion: 0.29)

- CreateFolderModal
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useState
- ../../types/typography/TextVariants
- ../ui/Text/Text

### Community 203 — createSubMask() (7 nodes, cohesion: 0.29)

- maskUtils
- createSubMask()
- ../components/panel/right/Masks/formatMaskTypeName
- ../components/panel/right/Masks/Mask
- ../components/panel/right/Masks/SubMaskMode
- ../hooks/useImageRenderSize/ImageDimensions
- uuid/v4

### Community 204 — handleSelectFile() (7 nodes, cohesion: 0.29)

- ImagePicker
- handleSelectFile()
- lucide-react/X
- react-i18next/useTranslation
- @tauri-apps/plugin-dialog/open
- ./Text/Text
- ../../types/typography/TextVariants

### Community 205 — PinchZoomDisablePlugin (7 nodes, cohesion: 0.33)

- window_customizer
- apply_macos_window_rounding()
- tauri::{Runtime, Webview, plugin::Plugin}
- PinchZoomDisablePlugin
- .default()
- .name()
- .webview_created()

### Community 206 — tiff_keywords_embed_and_the_image_still_decodes() (7 nodes, cohesion: 0.48)

- build_xmp_packet()
- encode_tiff()
- insert_xmp_into_tiff()
- round_trips_through_the_apps_own_tag_parser()
- tiff_ifd_stays_sorted_and_gains_exactly_one_entry()
- tiff_is_not_double_tagged_on_re_export()
- tiff_keywords_embed_and_the_image_still_decodes()

### Community 207 — save_image_bytes_to_android_gallery() (6 nodes, cohesion: 0.33)

- delete_android_media_store_item()
- put_android_content_value_int()
- put_android_content_value_string()
- save_bytes_to_android_media_store()
- save_file_bytes_to_android_downloads()
- save_image_bytes_to_android_gallery()

### Community 208 — to_rgba_f16() (6 nodes, cohesion: 0.33)

- .build_guided_coeffs()
- process_and_get_dynamic_image()
- process_and_get_dynamic_image_inner()
- process_and_get_dynamic_image_with_analytics()
- process_and_get_dynamic_image_with_precision()
- to_rgba_f16()

### Community 209 — solve_level() (6 nodes, cohesion: 0.40)

- .update()
- .mean()
- solve_level()
- WarpModel
- .n_params()
- .prior()

### Community 210 — zustand/create (210) (6 nodes, cohesion: 0.33)

- useCloudStore
- @clerk/clerk-js/Clerk
- @tauri-apps/plugin-http/fetch
- @tauri-apps/plugin-os/platform
- tauri-plugin-clerk/initClerk
- zustand/create

### Community 211 — useCloudUsage() (6 nodes, cohesion: 0.33)

- useCloudUsage
- react/useCallback
- react/useEffect
- ../store/useCloudStore/useCloudStore
- ../store/useSettingsStore/useSettingsStore
- useCloudUsage()

### Community 212 — ./utils/frontendLogBridge/installFrontendLogBridge (6 nodes, cohesion: 0.33)

- main
- ./App/App
- react-dom/client/createRoot
- react/React
- ./styles.css
- ./utils/frontendLogBridge/installFrontendLogBridge

### Community 213 — ../../types/typography/TextVariants (6 nodes, cohesion: 0.33)

- Switch
- clsx/clsx
- framer-motion/motion
- react/React
- ./Text/Text
- ../../types/typography/TextVariants

### Community 214 — parse_num() (6 nodes, cohesion: 0.33)

- convert_xmp_to_preset()
- extract_tone_curve_points()
- extract_xmp_name()
- get_attr_as_f64()
- num_to_json()
- parse_num()

### Community 215 — zustand/create (215) (6 nodes, cohesion: 0.33)

- useProcessStore
- ../components/ui/AppProperties/Progress
- ../components/ui/ExportImportProperties/ExportState
- ../components/ui/ExportImportProperties/ImportState
- ../components/ui/ExportImportProperties/Status
- zustand/create

### Community 216 — default_shadow_mass() (6 nodes, cohesion: 0.33)

- AutoProfile
- .default()
- default_highlight_mass()
- default_range_target()
- default_saturation_target()
- default_shadow_mass()

### Community 217 — ../../store/useProcessStore/ExternalEditSession (6 nodes, cohesion: 0.33)

- ExternalEditBar 2
- ./Button/Button
- lucide-react/Check
- lucide-react/Loader
- react-i18next/useTranslation
- ../../store/useProcessStore/ExternalEditSession

### Community 218 — inverse_transform_mask() (6 nodes, cohesion: 0.47)

- .into_cow()
- apply_coarse_rotation()
- apply_crop()
- apply_flip()
- apply_rotation()
- inverse_transform_mask()

### Community 219 — read_texture_data_roi() (6 nodes, cohesion: 0.47)

- GpuProcessor
- .high_precision_pipeline()
- .high_precision_tile()
- .run()
- high_precision_shader_source()
- read_texture_data_roi()

### Community 220 — generate_ai_subject_mask() (6 nodes, cohesion: 0.33)

- encode_to_base64_png()
- generate_ai_depth_mask()
- generate_ai_face_region_mask()
- generate_ai_foreground_mask()
- generate_ai_sky_mask()
- generate_ai_subject_mask()

### Community 221 — SourceCache (6 nodes, cohesion: 0.53)

- SourceCache
- .add()
- ._delete()
- ._enforce_limits()
- .__init__()
- ._sync()

### Community 222 — Settings (6 nodes, cohesion: 0.33)

- Settings
- .comfy_url()
- .http_url()
- .sent_cache_dir()
- .source_cache_dir()
- .ws_url()

### Community 223 — start_analytics_worker() (6 nodes, cohesion: 0.33)

- available_monitor_bounds()
- register_exit_handler()
- run()
- saved_window_state_is_usable()
- setup_logging()
- start_analytics_worker()

### Community 224 — getImageFlag() (6 nodes, cohesion: 0.33)

- imageFlags
- getImageFlag()
- ../components/ui/AppProperties/ImageFile
- ../components/ui/AppProperties/ImageFlag
- lucide-react/Ban
- lucide-react/Flag

### Community 225 — xyz_tag() (6 nodes, cohesion: 0.40)

- build_srgb_profile()
- desc_tag()
- s15_fixed16()
- text_tag()
- trc_tag()
- xyz_tag()

### Community 226 — useSortedLibrary() (6 nodes, cohesion: 0.33)

- computeGroupedLibrary()
- computeSortedLibrary()
- parseAperture()
- parseFocalLength()
- parseShutter()
- useSortedLibrary()

### Community 227 — ir_defect_mask() (5 nodes, cohesion: 0.50)

- box_blur()
- dilate()
- downsample2()
- ir_clean_scan()
- ir_defect_mask()

### Community 228 — is_image_edited() (5 nodes, cohesion: 0.60)

- apply_geometry_warp()
- apply_unwarp_geometry()
- get_geometry_params_from_json()
- is_geometry_identity()
- is_image_edited()

### Community 229 — generate_ai_subject_bitmap() (5 nodes, cohesion: 0.40)

- generate_ai_bitmap_from_base64()
- generate_ai_bitmap_from_full_mask()
- generate_ai_depth_bitmap()
- generate_ai_foreground_bitmap()
- generate_ai_subject_bitmap()

### Community 230 — sidecar_tags_reach_the_exported_jpeg() (5 nodes, cohesion: 0.40)

- a_format_we_cannot_embed_leaves_the_bytes_untouched()
- get_primary_sidecar_path()
- keep_metadata_off_writes_no_keywords()
- png_keywords_land_in_an_itxt_chunk_before_iend()
- sidecar_tags_reach_the_exported_jpeg()

### Community 231 — load_settings() (5 nodes, cohesion: 0.60)

- all_available_adjustments()
- CopyPasteSettings
- .default()
- default_included_adjustments()
- load_settings()

### Community 232 — MemorySource (5 nodes, cohesion: 0.40)

- FrameSource
- MemorySource
- .dims()
- .get()
- .len()

### Community 233 — ScanLook (5 nodes, cohesion: 0.50)

- compress_embeds_film_metadata()
- compress_scan()
- compress_scan_quantizes_shrinks_and_round_trips()
- ScanLook
- .default()

### Community 234 — ../../../store/useProcessStore/useProcessStore (5 nodes, cohesion: 0.40)

- LazyThumb
- ../../../hooks/useThumbnails/useThumbnails
- react/useEffect
- react/useRef
- ../../../store/useProcessStore/useProcessStore

### Community 235 — Button() (5 nodes, cohesion: 0.40)

- Button
- Button()
- clsx/clsx
- react/ButtonHTMLAttributes
- react/ReactNode

### Community 236 — non_maximal_suppression() (5 nodes, cohesion: 0.40)

- compute_brief_descriptor()
- convert_gray_u8_to_f32()
- find_features()
- find_features_tuned()
- non_maximal_suppression()

### Community 237 — generate_sub_mask_bitmap() (5 nodes, cohesion: 0.40)

- generate_ai_sky_bitmap()
- generate_all_bitmap()
- generate_linear_bitmap()
- generate_radial_bitmap()
- generate_sub_mask_bitmap()

### Community 238 — parse_lut_file() (5 nodes, cohesion: 0.40)

- load_and_parse_lut()
- parse_3dl()
- parse_cube()
- parse_hald()
- parse_lut_file()

### Community 239 — useAndroidBackHandler() (5 nodes, cohesion: 0.40)

- useAndroidBackHandler 2
- react/useEffect
- ../store/useSettingsStore/useSettingsStore
- ../store/useUIStore/useUIStore
- useAndroidBackHandler()

### Community 240 — srgb_linearize() (5 nodes, cohesion: 0.50)

- parse_input_profile()
- parse_trc_tag()
- parse_xyz_tag()
- read_s15_fixed16()
- srgb_linearize()

### Community 241 — get_lens_distortion_params() (5 nodes, cohesion: 0.40)

- extract_dist_params()
- extract_tca_params()
- extract_vig_params()
- get_lens_distortion_params()
- .get_distortion_params()

### Community 242 — white_balance_and_effects_when_enabled() (5 nodes, cohesion: 0.60)

- auto_results_to_json()
- each_slider_switch_is_independent()
- only_enabled_tools_are_written()
- results()
- white_balance_and_effects_when_enabled()

### Community 243 — stemKey() (5 nodes, cohesion: 0.70)

- importFilters
- computeVisible()
- computeVisibleSet()
- extOf()
- stemKey()

### Community 244 — is_supported_image_file() (5 nodes, cohesion: 0.40)

- formats
- std::convert::AsRef
- std::path::Path
- is_raw_file()
- is_supported_image_file()

### Community 245 — zoomCrop() (5 nodes, cohesion: 0.50)

- calculateAreaPreservingCrop()
- forceCropInBounds()
- isCropWithinBounds()
- moveCropInsideBounds()
- zoomCrop()

### Community 246 — WgpuDisplay (5 nodes, cohesion: 0.40)

- get_or_init_gpu_context()
- .new()
- WgpuDisplay
- .apply_pending_size()
- .render()

### Community 247 — run() (5 nodes, cohesion: 0.40)

- handleCapture()
- handleDetect()
- handleDisconnect()
- handleLiveView()
- run()

### Community 248 — grayscale_erode() (5 nodes, cohesion: 0.40)

- apply_grow_and_feather()
- generate_color_bitmap()
- generate_luminance_bitmap()
- grayscale_dilate()
- grayscale_erode()

### Community 249 — identical_frames_shot_far_apart_stay_separate() (5 nodes, cohesion: 0.40)

- a_borderline_match_needs_the_frames_to_be_close_in_time()
- a_matching_hash_links_frames_whose_palettes_differ()
- a_stray_frame_between_two_takes_does_not_split_them()
- frame()
- identical_frames_shot_far_apart_stay_separate()

### Community 250 — ../../utils/imageFlags/FLAG_ICONS (5 nodes, cohesion: 0.40)

- FlagToggles
- ./AppProperties/ImageFlag
- clsx/clsx
- react-i18next/useTranslation
- ../../utils/imageFlags/FLAG_ICONS

### Community 251 — react/PointerEventHandler (5 nodes, cohesion: 0.40)

- Resizer
- ./AppProperties/Orientation
- clsx/clsx
- react/MouseEventHandler
- react/PointerEventHandler

### Community 252 — neutralize_wb_if_multiexposure() (4 nodes, cohesion: 0.83)

- multi_exposure 2
- _find_ifd_entry()
- is_incamera_multiexposure_canon()
- neutralize_wb_if_multiexposure()

### Community 253 — useCollapsibleHeight() (4 nodes, cohesion: 0.50)

- useCollapsibleHeight
- react/useEffect
- react/useRef
- useCollapsibleHeight()

### Community 254 — useImportKeyboard() (4 nodes, cohesion: 0.50)

- useImportKeyboard
- react/useEffect
- ./useSdImportActions/useSdImportActions
- useImportKeyboard()

### Community 255 — node:assert/assert (4 nodes, cohesion: 0.50)

- keystone.test
- ./keystone/solveKeystone
- ./keystone/vanishingPoint
- node:assert/assert

### Community 256 — useImageRenderSize() (4 nodes, cohesion: 0.50)

- useImageRenderSize
- react/useLayoutEffect
- react/useState
- useImageRenderSize()

### Community 257 — ../../utils/imageFlags/FLAG_ICONS (257) (4 nodes, cohesion: 0.50)

- FlagBadges
- ./AppProperties/ImageFlag
- clsx/clsx
- ../../utils/imageFlags/FLAG_ICONS

### Community 258 — suggestProfile() (4 nodes, cohesion: 0.50)

- filmProfiles
- allProfiles()
- p()
- suggestProfile()

### Community 259 — useOsPlatform() (4 nodes, cohesion: 0.50)

- useOsPlatform
- react/useMemo
- @tauri-apps/plugin-os/platform
- useOsPlatform()

### Community 260 — ImageLoaderManager() (3 nodes, cohesion: 0.67)

- ImageLoaderManager
- ImageLoaderManager()
- ../../hooks/useImageLoader/useImageLoader

### Community 261 — java.io.ByteArrayOutputStream (3 nodes, cohesion: 0.67)

- build.gradle
- groovy.json.JsonSlurper
- java.io.ByteArrayOutputStream

### Community 262 — react/SVGProps (3 nodes, cohesion: 0.67)

- ExifIcons
- react/React
- react/SVGProps

### Community 263 — ImageProcessingManager() (3 nodes, cohesion: 0.67)

- ImageProcessingManager
- ImageProcessingManager()
- ../../hooks/useImageProcessing/useImageProcessing

### Community 264 — editorCanvasRgb() (3 nodes, cohesion: 0.67)

- themes
- editorCanvasRgb()
- ../components/ui/AppProperties/Theme

### Community 265 — hierarchy (3 nodes, cohesion: 0.67)

- hierarchy
- once_cell::sync::Lazy
- std::collections::HashMap

### Community 266 — sanitizeFilenameTemplate() (3 nodes, cohesion: 0.67)

- ExportImportProperties
- ./AppProperties/Progress
- sanitizeFilenameTemplate()

### Community 267 — i18next (3 nodes, cohesion: 0.67)

- i18next.d
- ../i18n/locales/en.json/en
- i18next

### Community 268 — react/React (3 nodes, cohesion: 0.67)

- Input
- clsx/clsx
- react/React

### Community 269 — zustand/middleware/persist (3 nodes, cohesion: 0.67)

- useScannerStore
- zustand/create
- zustand/middleware/persist

### Community 270 — lucide-react/Star (3 nodes, cohesion: 0.67)

- RatingColor
- ./importFilters/COLOR_HEX
- lucide-react/Star

### Community 271 — ../../right/CropPanel/OverlayMode (3 nodes, cohesion: 0.67)

- CompositionOverlays
- react/React
- ../../right/CropPanel/OverlayMode

### Community 272 — zustand/create (272) (2 nodes, cohesion: 1.00)

- useTetherStore
- zustand/create

### Community 273 — i18next-cli/defineConfig (2 nodes, cohesion: 1.00)

- i18next.config
- i18next-cli/defineConfig

### Community 274 — main() (2 nodes, cohesion: 1.00)

- main
- main()

### Community 275 — react/JSX (2 nodes, cohesion: 1.00)

- CollageVariants
- react/JSX

### Community 276 — java.util.Properties (2 nodes, cohesion: 1.00)

- build.gradle
- java.util.Properties

### Community 277 — zustand/create (277) (2 nodes, cohesion: 1.00)

- useTetheringStore
- zustand/create

### Community 278 — typography (1 nodes, cohesion: 1.00)

- typography

### Community 279 — .into_cow() (279) (1 nodes, cohesion: 1.00)

- .into_cow()

### Community 280 — .into_cow() (1 nodes, cohesion: 1.00)

- .into_cow()

### Community 281 — build.gradle (1 nodes, cohesion: 1.00)

- build.gradle

### Community 282 — .drop() (1 nodes, cohesion: 1.00)

- .drop()

### Community 283 — .into_cow() (283) (1 nodes, cohesion: 1.00)

- .into_cow()

### Community 284 — .from_u8() (1 nodes, cohesion: 1.00)

- .from_u8()

### Community 285 — .from_u8() (285) (1 nodes, cohesion: 1.00)

- .from_u8()

### Community 286 — mod (286) (1 nodes, cohesion: 1.00)

- mod

### Community 287 — candidates (1 nodes, cohesion: 1.00)

- candidates

### Community 288 — eslint.config (1 nodes, cohesion: 1.00)

- eslint.config

### Community 289 — mod (1 nodes, cohesion: 1.00)

- mod

## 🕳️ Knowledge Gaps

**Isolated nodes** (12):
- eslint.config
- typography
- build.gradle
- .drop()
- .from_u8()
- .from_u8()
- .into_cow()
- .into_cow()
- .into_cow()
- mod
- candidates
- mod

**Thin communities** (< 3 nodes): 18 communities

## 💰 Token Cost

| File | Tokens |
|------|--------|
| input | 0 |
| output | 0 |
| **Total** | **0** |

## ❓ Suggested Questions

1. How does 'src_tauri_src_focus_stacking_rs_erode_mask' relate to 3 different communities (warp_or_copy(), Plane, weighted_median_labels())?
1. How does 'src_tauri_src_ai_processing_rs_get_or_init_ai_models' relate to 3 different communities (transpose(), TileParams, verify_sha256() (184))?
1. How does 'src_tauri_src_focus_stacking_rs_load_frames' relate to 3 different communities (weighted_median_labels(), load_frames(), warp_or_copy())?
1. How does 'src_tauri_src_scanning_rs' relate to 3 different communities (write_scan_sidecar(), ir_defect_mask(), ScanLook)?
1. How does 'src_tauri_src_exif_processing_rs_build_xmp_packet' relate to 3 different communities (tiff_keywords_embed_and_the_image_still_decodes(), xmp_packet_for_source(), try_get_exif_creation_date())?
1. How does 'src_tauri_src_exif_processing_rs_a_format_we_cannot_embed_leaves_the_bytes_untouched' relate to 3 different communities (sidecar_tags_reach_the_exported_jpeg(), xmp_packet_for_source(), try_get_exif_creation_date())?
1. How does 'src_tauri_src_mask_generation_rs_generate_color_bitmap' relate to 3 different communities (TransformParams, grayscale_erode(), generate_sub_mask_bitmap())?

---
_Generated by graphify-rs_
