# 📊 Graph Analysis Report

**Root:** `.`

## Summary

| Metric | Value |
|--------|-------|
| Nodes | 4473 |
| Edges | 5335 |
| Communities | 247 |
| Hyperedges | 0 |

### Confidence Breakdown

| Level | Count | Percentage |
|-------|-------|------------|
| EXTRACTED | 4332 | 81.2% |
| INFERRED | 1003 | 18.8% |
| AMBIGUOUS | 0 | 0.0% |

## 🌟 God Nodes (Most Connected)

| Node | Degree | Community |
|------|--------|-----------|
| file_management | 136 | 0 |
| MasksPanel | 92 | 1 |
| SettingsPanel | 84 | 2 |
| lib | 79 | 5 |
| App | 78 | 3 |
| image_processing | 76 | 9 |
| AIPanel | 75 | 4 |
| exif_processing | 74 | 41 |
| useAppContextMenus | 67 | 6 |
| CropPanel | 64 | 7 |

## 🔮 Surprising Connections

- **src_components_panel_library_tethermenu_tsx_handleconfig** → **src_components_panel_library_tethermenu_tsx_run** (calls)
- **src_components_panel_right_snapshotssection_tsx_handledelete** → **src_components_panel_right_snapshotssection_tsx_updatesnapshots** (calls)
- **src_components_panel_right_snapshotssection_tsx_handlerowcontextmenu** → **src_components_panel_right_snapshotssection_tsx_handleoverwrite** (calls)
- **src_hooks_usesdimportactions_ts_usesdimportactions** → **src_hooks_usesdimportactions_ts_visiblenow** (calls)
- **src_hooks_usesdimportactions_ts_usesdimportactions** → **src_hooks_usesdimportactions_ts_rawexts** (calls)

## 🏘️ Communities

### Community 0 — update_thumbnail_queue() (95 nodes, cohesion: 0.03)

- file_management
- adjustments_is_negative()
- apply_auto_lens_correction()
- clear_all_sidecars()
- clear_thumbnail_cache()
- copy_files()
- create_folder()
- create_virtual_copy()
- delete_files_from_disk()
- delete_files_with_associated()
- delete_folder()
- duplicate_file()
- emit_image_metadata_loaded()
- ExportPresetFile
- extension_is_preserved_and_stem_not_mangled()
- extract_xmp_label()
- extract_xmp_rating()
- extract_xmp_tags()
- find_all_associated_files()
- free_name_is_returned_unchanged()
- _…and 75 more_

### Community 1 — setCombinedRef() (1) (93 nodes, cohesion: 0.02)

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
- clsx/clsx
- ../../../context/ContextMenuContext/useContextMenu
- @dnd-kit/core/DndContext
- @dnd-kit/core/DragEndEvent
- @dnd-kit/core/DragOverlay
- @dnd-kit/core/DragStartEvent
- @dnd-kit/core/PointerSensor
- @dnd-kit/core/pointerWithin
- @dnd-kit/core/useDraggable
- @dnd-kit/core/useDroppable
- _…and 73 more_

### Community 2 — refreshAssistantModels() (85 nodes, cohesion: 0.02)

- SettingsPanel
- async()
- closeConfirmModal()
- clsx()
- executeClearAiTags()
- executeClearCache()
- executeClearSidecars()
- executeClearTags()
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
- handleKeybindSave()
- _…and 65 more_

### Community 3 — insertChildrenIntoTree() (79 nodes, cohesion: 0.03)

- App
- createResizeHandler()
- getDynamicCompactPanelHeight()
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
- ./components/panel/right/PresetsPanel/PresetsPanel
- ./components/panel/SettingsPanel/SettingsPanel
- _…and 59 more_

### Community 4 — setCombinedRef() (76 nodes, cohesion: 0.03)

- AIPanel
- handleMouseEnter()
- handleMouseLeave()
- handleRenameSubmit()
- if()
- @clerk/react/useAuth
- @clerk/react/useUser
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
- _…and 56 more_

### Community 5 — WgpuTransformPayload (74 nodes, cohesion: 0.03)

- lib
- apply_adjustments()
- cancel_thumbnail_generation()
- CommunityPreset
- compute_preview_transformed()
- fetch_community_presets()
- force_exit()
- frontend_log()
- frontend_ready()
- generate_all_community_previews()
- generate_original_transformed_preview()
- generate_preset_preview()
- generate_preview_for_path()
- generate_transformed_preview()
- generate_uncropped_preview()
- get_cached_full_warped_image()
- get_image_dimensions()
- get_log_file_path()
- get_original_image()
- ImageDimensions
- _…and 54 more_

### Community 6 — useAppContextMenus() (68 nodes, cohesion: 0.03)

- useAppContextMenus
- ../components/ui/AppProperties/Album
- ../components/ui/AppProperties/AlbumGroup
- ../components/ui/AppProperties/AlbumItem
- ../components/ui/AppProperties/Invokes
- ../components/ui/AppProperties/Option
- ../components/ui/AppProperties/OPTION_SEPARATOR
- ../components/ui/AppProperties/Panel
- ../context/ContextMenuContext/useContextMenu
- ../context/TaggingSubMenu/TaggingSubMenu
- lucide-react/Album
- lucide-react/Aperture
- lucide-react/Briefcase
- lucide-react/Camera
- lucide-react/Car
- lucide-react/Check
- lucide-react/ClipboardPaste
- lucide-react/Copy
- lucide-react/CopyPlus
- lucide-react/Edit
- _…and 48 more_

### Community 7 — resetPerspective() (65 nodes, cohesion: 0.03)

- CropPanel
- applyGuided()
- enterGuided()
- exitGuided()
- geoValue()
- getOrientationTooltip()
- getOverlayTooltip()
- handleApplyCustomRatio()
- handleApplySavedPreset()
- handleCustomInputChange()
- handleCustomInputFocus()
- handleCustomInputKeyDown()
- handleDeleteSavedPreset()
- handleFineRotationChange()
- handleGeoChange()
- handleOverlayCycle()
- handlePresetClick()
- handleReset()
- handleSaveCropPreset()
- handleStepRotate()
- _…and 45 more_

### Community 8 — setActiveItem() (65 nodes, cohesion: 0.03)

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
- _…and 45 more_

### Community 9 — yc_to_rgb() (62 nodes, cohesion: 0.04)

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
- calculate_agx_matrices()
- calculate_agx_matrices_glam()
- ColorCalibrationSettings
- ColorGradeSettings
- compute_lens_auto_crop_scale()
- Crop
- downscale_f32_image()
- GeometryParams
- .default()
- _…and 42 more_

### Community 10 — startRename() (10) (59 nodes, cohesion: 0.04)

- AssistantPanel
- applyScannerPatch()
- blobUrlToImage()
- commitRename()
- dataUrlToImage()
- downscaleBlob()
- fileToAttachment()
- formatMetadata()
- formatPatch()
- handleKeyDown()
- clsx/clsx
- ../../../hooks/useEditorActions/useEditorActions
- ../../../hooks/useLibraryActions/useLibraryActions
- lucide-react/AlertTriangle
- lucide-react/Bot
- lucide-react/Check
- lucide-react/History
- lucide-react/Layers
- lucide-react/Loader2
- lucide-react/MessageSquare
- _…and 39 more_

### Community 11 — toggleSection() (58 nodes, cohesion: 0.03)

- FolderTree
- filterAlbumTree()
- filterTree()
- getAutoExpandedAlbumGroups()
- getAutoExpandedPaths()
- handleEmptyAreaContextMenu()
- handleFolderIconClick()
- handleNameClick()
- handleNameDoubleClick()
- clsx/clsx
- framer-motion/AnimatePresence
- framer-motion/LayoutGroup
- framer-motion/motion
- lucide-react/Album
- lucide-react/ArrowUpDown
- lucide-react/Briefcase
- lucide-react/Camera
- lucide-react/Car
- lucide-react/Check
- lucide-react/ChevronDown
- _…and 38 more_

### Community 12 — setLibraryDisplayMode() (57 nodes, cohesion: 0.04)

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
- _…and 37 more_

### Community 13 — write_scan_sidecar() (52 nodes, cohesion: 0.08)

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

### Community 14 — toggleCompare() (52 nodes, cohesion: 0.04)

- LensCorrectionModal
- clsx()
- fetchDistortionParams()
- handleAmountChange()
- handleApply()
- handleAutoDetect()
- handleMakerChange()
- handleModelChange()
- handleMouseDown()
- handleMyLensSelect()
- handleReset()
- handleResetZoom()
- handleToggleChange()
- handleWheel()
- clsx/clsx
- framer-motion/AnimatePresence
- framer-motion/motion
- lodash.throttle/throttle
- lucide-react/Activity
- lucide-react/Check
- _…and 32 more_

### Community 15 — Section() (51 nodes, cohesion: 0.04)

- ExportPanel
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
- react/useMemo
- _…and 31 more_

### Community 16 — toggleMode() (51 nodes, cohesion: 0.04)

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
- _…and 31 more_

### Community 17 — select_tile_params() (46 nodes, cohesion: 0.06)

- ai_processing
- accumulator_to_rgb32f()
- AiDepthMaskParameters
- AiForegroundMaskParameters
- AiModels
- AiSkyMaskParameters
- AiState
- AiSubjectMaskParameters
- apply_seamless()
- CachedDepthMap
- ClipModels
- edt_1d()
- edt_2d()
- extract_tile_mirror()
- face_area()
- face_iou()
- FaceBox
- generate_face_region_mask()
- generate_image_embeddings()
- ImageEmbeddings
- _…and 26 more_

### Community 18 — handleToggleVisibility() (46 nodes, cohesion: 0.04)

- ControlsPanel
- clsx()
- handleResetAdjustments()
- handleSectionContextMenu()
- handleToggleSection()
- handleToggleVisibility()
- ../../adjustments/Basic/BasicAdjustments
- ../../adjustments/Color/ColorPanel
- ../../adjustments/Curves/CurveGraph
- ../../adjustments/Details/DetailsPanel
- ../../adjustments/Effects/EffectsPanel
- ../../adjustments/FilmPanel/FilmPanel
- clsx/clsx
- ../../../context/ContextMenuContext/useContextMenu
- ../editor/Waveform/Waveform
- framer-motion/AnimatePresence
- framer-motion/motion
- ../../../hooks/useEditorActions/useEditorActions
- ../../../hooks/useWaveformControls/useWaveformControls
- ../library/TetherMenu/CameraSection
- _…and 26 more_

### Community 19 — TransformParams (45 nodes, cohesion: 0.06)

- mask_generation
- AiPatchDefinition
- BrushLine
- BrushMaskParameters
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
- crate::ai_processing::{
    AiDepthMaskParameters, AiForegroundMaskParameters, AiSkyMaskParameters, AiSubjectMaskParameters,
}
- crate::app_state::AppState
- _…and 25 more_

### Community 20 — CullingPreview() (45 nodes, cohesion: 0.04)

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
- _…and 25 more_

### Community 21 — handleStraightenMouseUp() (45 nodes, cohesion: 0.04)

- ImageCanvas
- for()
- getCropDimensions()
- getEdgeFadeStyle()
- handleStraightenMouseDown()
- handleStraightenMouseLeave()
- handleStraightenMouseMove()
- handleStraightenMouseUp()
- ../../../hooks/useImageRenderSize/RenderSize
- ../../../hooks/useOsPlatform/useOsPlatform
- lucide-react/Bandage
- lucide-react/Stamp
- ./overlays/CompositionOverlays/CompositionOverlays
- react-i18next/useTranslation
- react-image-crop/Crop
- react-image-crop/dist/ReactCrop.css
- react-image-crop/PercentCrop
- react-image-crop/ReactCrop
- react-konva/Circle
- react-konva/Ellipse
- _…and 25 more_

### Community 22 — parseDms() (44 nodes, cohesion: 0.05)

- MetadataPanel
- catch()
- clsx()
- formatExifTag()
- handleKeyDown()
- handleSave()
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
- lucide-react/Copy
- _…and 24 more_

### Community 23 — handleWheel() (23) (43 nodes, cohesion: 0.05)

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

### Community 24 — parseRgb() (42 nodes, cohesion: 0.05)

- Editor
- checkCropValid()
- clsx/clsx
- ./editor/EditorToolbar/EditorToolbar
- ./editor/GuidedKeystoneOverlay/GuidedKeystoneOverlay
- ./editor/ImageCanvas/ImageCanvas
- ../../hooks/useAiMasking/useAiMasking
- ../../hooks/useImageRenderSize/ImageDimensions
- ../../hooks/useImageRenderSize/RenderSize
- ../../hooks/useImageRenderSize/useImageRenderSize
- lodash.debounce/debounce
- lucide-react/Loader2
- react-image-crop/Crop
- react-image-crop/PercentCrop
- react-toastify/toast
- react/useCallback
- react/useEffect
- react/useImperativeHandle
- react/useLayoutEffect
- react/useMemo
- _…and 22 more_

### Community 25 — WatermarkSettings (41 nodes, cohesion: 0.06)

- export_processing
- apply_export_resize_and_watermark()
- apply_watermark()
- calculate_resize_target()
- cancel_export()
- component_matches()
- ExportAdjustmentsMode
- ExportCancellationRequest
- ExportSettings
- crate::{
    apply_all_transformations, generate_transformed_preview, get_cached_or_generate_mask,
    hydrate_adjustments, load_settings, resolve_warped_image_for_masks,
}
- crate::AppState
- crate::cache_utils::{calculate_full_job_hash, calculate_transform_hash}
- crate::exif_processing
- crate::file_management::{
    generate_filename_from_template, parse_virtual_path, read_file_mapped,
}
- crate::formats::is_raw_file
- crate::image_loader::{
    composite_patches_on_image, load_and_composite, load_base_image_from_bytes,
}
- crate::image_processing::{
    AllAdjustments, Crop, GpuContext, RenderRequest, downscale_f32_image,
    get_all_adjustments_from_json, get_or_init_gpu_context, process_and_get_dynamic_image,
    resolve_tonemapper_override_from_handle,
}
- crate::lut_processing::{
    convert_image_to_cube_lut, generate_identity_lut_image, get_or_load_lut,
}
- crate::mask_generation::{MaskDefinition, generate_mask_bitmap}
- image::codecs::jpeg::JpegEncoder
- _…and 21 more_

### Community 26 — try_fast_embedded_preview() (41 nodes, cohesion: 0.07)

- culling
- analyze_image()
- analyze_paths()
- best_region_sharpness()
- calculate_exposure_metric()
- calculate_laplacian_variance()
- cull_images()
- CullGroup
- CullingProgress
- CullingSettings
- CullingSuggestions
- fast_raw_preview()
- flush_time_burst()
- group_analyses()
- group_by_time()
- ImageAnalysisData
- .result_path()
- .set_scores()
- ImageAnalysisResult
- crate::app_settings::load_settings
- _…and 21 more_

### Community 27 — updateParametricValue() (40 nodes, cohesion: 0.06)

- Curves
- buildParametricPoints()
- convertParametricToPoints()
- getCurvePath()
- getHistogramPath()
- getSplitterGradient()
- getZeroHistogramPath()
- handleContainerStart()
- handleContextMenu()
- handleDoubleClick()
- handlePointContextMenu()
- handlePointStart()
- handleToggleMode()
- ../../context/ContextMenuContext/useContextMenu
- framer-motion/AnimatePresence
- framer-motion/motion
- lucide-react/ClipboardPaste
- lucide-react/Copy
- lucide-react/RotateCcw
- lucide-react/Settings2
- _…and 20 more_

### Community 28 — UsbCameraState (39 nodes, cohesion: 0.07)

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

### Community 29 — truncate() (37 nodes, cohesion: 0.09)

- assistant
- anthropic_content()
- apply_edits_tool()
- assistant_chat()
- assistant_list_models()
- assistant_test_connection()
- AssistantResponse
- build_cli_prompt()
- build_url()
- call_anthropic()
- call_claude_code()
- call_openai_compatible()
- ChatMessage
- default_endpoint()
- default_model()
- edits_response_format()
- extract()
- fetch_models()
- ImageAttachment
- crate::app_settings
- _…and 17 more_

### Community 30 — walsh_hadamard_1d() (37 nodes, cohesion: 0.07)

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

### Community 31 — srgb_to_linear_lut() (37 nodes, cohesion: 0.06)

- image_loader
- composite_patches_on_image()
- embedded_preview_fallback()
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
- exif::{Reader as ExifReader, Tag}
- image::{DynamicImage, GenericImageView, ImageReader, imageops}
- rawler::Orientation
- rayon::prelude::*
- serde::Deserialize
- serde_json::Value
- _…and 17 more_

### Community 32 — getW() (36 nodes, cohesion: 0.06)

- LibraryItems
- clsx()
- getW()
- clsx/clsx
- ../editor/ExifIcons/IconAperture
- ../editor/ExifIcons/IconFocalLength
- ../editor/ExifIcons/IconIso
- ../editor/ExifIcons/IconShutter
- lucide-react/CloudOff
- lucide-react/Film
- lucide-react/Folder
- lucide-react/FolderOpen
- lucide-react/Image
- lucide-react/Layers
- lucide-react/SlidersHorizontal
- lucide-react/Star
- ../MainLibrary/ColumnWidths
- react-i18next/useTranslation
- react/React
- react/useCallback
- _…and 16 more_

### Community 33 — start_background_indexing() (36 nodes, cohesion: 0.07)

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
- _…and 16 more_

### Community 34 — update_negative_conversion() (36 nodes, cohesion: 0.09)

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

### Community 35 — setConfigCurrent() (35 nodes, cohesion: 0.06)

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

### Community 36 — ScannerPane() (35 nodes, cohesion: 0.06)

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

### Community 37 — shuffleArray() (34 nodes, cohesion: 0.06)

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

### Community 38 — handleButtonKeyDown() (34 nodes, cohesion: 0.06)

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

### Community 39 — WindowState (34 nodes, cohesion: 0.06)

- app_state
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
- image::{DynamicImage, GrayImage}
- serde::{Deserialize, Serialize}
- std::collections::{HashMap, HashSet, VecDeque}
- std::path::PathBuf
- std::sync::{Arc, Condvar, Mutex}
- std::sync::atomic::{AtomicBool, AtomicUsize}
- _…and 14 more_

### Community 40 — handleWheel() (33 nodes, cohesion: 0.06)

- DenoiseModal
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
- react-i18next/useTranslation
- _…and 13 more_

### Community 41 — to_ur64() (32 nodes, cohesion: 0.07)

- exif_processing
- declared_segment_length_matches_the_bytes_written()
- decode_user_comment()
- embeddable_formats_are_reported_correctly()
- format_min_max()
- has_embedded_xmp()
- has_embedded_xmp_is_false_for_a_plain_encode()
- heal_cached_user_comment()
- chrono::{DateTime, NaiveDateTime, Utc}
- crate::formats::is_raw_file
- crate::image_processing::ImageMetadata
- exif::{Exif, In, Value}
- little_exif::exif_tag::ExifTag
- little_exif::filetype::FileExtension
- little_exif::metadata::Metadata
- little_exif::rational::{iR64, uR64}
- rawler::decoders::RawMetadata
- std::collections::HashMap
- std::fs
- std::io::{BufReader, Cursor}
- _…and 12 more_

### Community 42 — sync_metadata_to_xmp() (31 nodes, cohesion: 0.18)

- add_to_thumbnail_queue()
- apply_adjustments_to_paths()
- apply_auto_adjustments_to_paths()
- apply_orientation_to_paths()
- assistant_prepare_image()
- compute_thumbnail_cache_hash()
- emit_thumbnail_cache_setup_error()
- emit_thumbnail_generated()
- encode_thumbnail()
- enqueue_metadata()
- generate_single_thumbnail_and_cache()
- generate_thumbnail_data()
- get_cache_key_hash()
- get_cached_or_generate_thumbnail_image()
- get_thumb_cache_dir()
- increment_thumbnail_progress()
- is_cloud_placeholder()
- list_images_in_dir()
- list_images_recursive()
- parse_virtual_path()
- _…and 11 more_

### Community 43 — TetherChip() (31 nodes, cohesion: 0.06)

- BottomBar
- clsx()
- clsx/clsx
- ./Filmstrip/Filmstrip
- framer-motion/AnimatePresence
- framer-motion/motion
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
- ../../store/useEditorStore/useEditorStore
- _…and 11 more_

### Community 44 — upload_source() (31 nodes, cohesion: 0.06)

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

### Community 45 — onDragMove() (30 nodes, cohesion: 0.07)

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
- lucide-react/Crop
- lucide-react/FileInput
- lucide-react/Folder
- lucide-react/Info
- lucide-react/Layers
- lucide-react/LucideIcon
- lucide-react/Paintbrush
- lucide-react/SlidersHorizontal
- lucide-react/SwatchBook
- _…and 10 more_

### Community 46 — unique_lut_destination() (30 nodes, cohesion: 0.08)

- lut_processing
- convert_image_to_cube_lut()
- generate_identity_lut_image()
- generate_lut_previews()
- get_lut_cache_dir()
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
- import_luts_to_dir()
- mozjpeg_rs::{Encoder, Preset}
- serde::Serialize
- std::fs::{File, copy, create_dir_all, read_dir}
- std::io::{BufRead, BufReader, Cursor}
- _…and 10 more_

### Community 47 — WgpuDisplay (30 nodes, cohesion: 0.08)

- gpu_processing
- blur_needs_is_union_of_all_consumers()
- BlurNeeds
- BlurParams
- compute_blur_needs()
- DisplayTransform
- FlareParams
- crate::{AppState, GpuImageCache}
- crate::image_processing::AllAdjustments
- crate::image_processing::{AllAdjustments, GpuContext, MAX_MASKS}
- crate::lut_processing::Lut
- half::f16
- image::{DynamicImage, GenericImageView, ImageBuffer, Luma, Rgba}
- std::num::NonZero
- std::sync::Arc
- std::time::Instant
- super::{compute_blur_needs, processor_side}
- tauri::Manager
- wgpu::util::{DeviceExt, TextureDataOrder}
- process_and_get_dynamic_image()
- _…and 10 more_

### Community 48 — startRename() (29 nodes, cohesion: 0.08)

- SnapshotsSection
- dateLabel()
- handleApply()
- handleDelete()
- handleRowContextMenu()
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
- ../../../types/typography/TextColors
- ../../../types/typography/TextVariants
- ../../../types/typography/TextWeights
- _…and 9 more_

### Community 49 — startEyedropper() (29 nodes, cohesion: 0.07)

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

### Community 50 — ImageThumbnail() (28 nodes, cohesion: 0.07)

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

### Community 51 — handleResize() (28 nodes, cohesion: 0.07)

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
- _…and 8 more_

### Community 52 — closeConfirmModal() (28 nodes, cohesion: 0.07)

- AppModals
- closeConfirmModal()
- ./CollageModal/CollageModal
- ./ConfirmModal/ConfirmModal
- ./CopyPasteSettingsModal/CopyPasteSettingsModal
- ./CreateFolderModal/CreateFolderModal
- ./CullingModal/CullingModal
- ./DenoiseModal/DenoiseModal
- ./HdrModal/HdrModal
- ./ImportSettingsModal/ImportSettingsModal
- ./PanoramaModal/PanoramaModal
- react-i18next/useTranslation
- react-toastify/toast
- ./RenameFileModal/RenameFileModal
- ./RenameFolderModal/RenameFolderModal
- ../../store/useEditorStore/useEditorStore
- ../../store/useLibraryStore/useLibraryStore
- ../../store/useProcessStore/useProcessStore
- ../../store/useSettingsStore/useSettingsStore
- ../../store/useUIStore/useUIStore
- _…and 8 more_

### Community 53 — handleMouseUp() (28 nodes, cohesion: 0.07)

- Color
- getTransform()
- handleClick()
- handleMouseDown()
- handleMouseEnter()
- handleMouseLeave()
- handleMouseUp()
- framer-motion/AnimatePresence
- framer-motion/motion
- lucide-react/Pipette
- lucide-react/Sliders
- react-i18next/useTranslation
- react/useEffect
- react/useMemo
- react/useState
- ../../types/typography/TextColors
- ../../types/typography/TextVariants
- ../../types/typography/TextWeights
- ../ui/AppProperties/AppSettings
- ../ui/ColorWheel/ColorWheel
- _…and 8 more_

### Community 54 — useAppInitialization() (28 nodes, cohesion: 0.07)

- useAppInitialization
- getDefaultLanguage()
- ../components/ui/AppProperties/EditedStatus
- ../components/ui/AppProperties/FilterCriteria
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
- ../store/useSettingsStore/useSettingsStore
- _…and 8 more_

### Community 55 — ../../utils/adjustments/COLOR_LABELS (28 nodes, cohesion: 0.07)

- Filmstrip
- clsx/clsx
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
- ../../types/typography/TextVariants
- _…and 8 more_

### Community 56 — write_preset_sidecar() (27 nodes, cohesion: 0.08)

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

### Community 57 — clsx() (27 nodes, cohesion: 0.07)

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

### Community 58 — ../../utils/adjustments/Adjustments (27 nodes, cohesion: 0.07)

- TransformModal
- clsx/clsx
- framer-motion/AnimatePresence
- framer-motion/motion
- lodash.throttle/throttle
- lucide-react/Check
- lucide-react/Eye
- lucide-react/EyeOff
- lucide-react/Grid3X3
- lucide-react/Info
- lucide-react/LineChart
- lucide-react/Maximize
- lucide-react/RotateCcw
- lucide-react/ZoomIn
- lucide-react/ZoomOut
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useRef
- react/useState
- _…and 7 more_

### Community 59 — handleReset() (59) (27 nodes, cohesion: 0.07)

- Effects
- handleReset()
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
- @tauri-apps/api/core/invoke
- ../../types/typography/TextVariants
- ../ui/AppProperties/AppSettings
- ../ui/DepthRangePicker/DepthRangePicker
- _…and 7 more_

### Community 60 — save_inputs_for_debug() (26 nodes, cohesion: 0.08)

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

### Community 61 — score_for_import() (26 nodes, cohesion: 0.09)

- sd_import
- analyze_for_import()
- cull_images_for_import()
- DriveInfo
- eject_drive()
- find_existing_in_destination()
- get_capture_times()
- get_import_preview()
- group_cached()
- group_for_import()
- base64::{Engine as _, engine::general_purpose}
- crate::culling::{
    CullingSettings, CullingSuggestions, ImageAnalysisData, analyze_paths, group_analyses,
    group_by_time,
}
- crate::formats::{is_raw_file, is_supported_image_file}
- rawler::decoders::RawDecodeParams
- serde::Serialize
- std::path::Path
- std::sync::Mutex
- sysinfo::Disks
- tauri::{AppHandle, Emitter, State}
- walkdir::WalkDir
- _…and 6 more_

### Community 62 — Cell() (25 nodes, cohesion: 0.08)

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
- lucide-react/Image
- lucide-react/Layers
- lucide-react/Sparkles
- ./RatingColor/RatingColor
- react/useCallback
- react/useEffect
- react/useMemo
- _…and 5 more_

### Community 63 — Vignetting (25 nodes, cohesion: 0.09)

- lens_correction
- Aperture
- Calibration
- CalibrationElement
- Camera
- Distortion
- extract_dist_params()
- extract_tca_params()
- extract_vig_params()
- Focal
- crate::AppState
- fuzzy_matcher::FuzzyMatcher
- include_dir::{Dir, include_dir}
- serde::{Deserialize, Serialize}
- std::cmp::Ordering
- std::fs
- tauri::{Manager, State}
- walkdir::WalkDir
- .get_distortion_params()
- LensDatabase
- _…and 5 more_

### Community 64 — normalize_grayscale() (25 nodes, cohesion: 0.11)

- processing
- are_points_collinear()
- build_integral_images()
- calculate_downscale_dimensions()
- calculate_downscale_dimensions_capped()
- compute_brief_descriptor()
- compute_homography()
- convert_gray_u8_to_f32()
- find_features()
- find_features_tuned()
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
- _…and 5 more_

### Community 65 — handleSwatchClick() (24 nodes, cohesion: 0.08)

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

### Community 66 — max_corner_displacement() (24 nodes, cohesion: 0.20)

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

### Community 67 — save_panorama() (24 nodes, cohesion: 0.08)

- panorama_stitching
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
- tauri::{AppHandle, Emitter}
- _…and 4 more_

### Community 68 — getSubMaskName() (22 nodes, cohesion: 0.10)

- Masks
- formatMaskTypeName()
- getMaskTypeName()
- getSubMaskName()
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
- lucide-react/MoreHorizontal
- lucide-react/RectangleHorizontal
- lucide-react/Smile
- lucide-react/Sparkles
- lucide-react/Stamp
- lucide-react/Sun
- _…and 2 more_

### Community 69 — handleSelectNone() (22 nodes, cohesion: 0.09)

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

### Community 70 — WorkspaceState (22 nodes, cohesion: 0.10)

- app_settings
- AutoProfile
- ExportPreset
- FilterCriteria
- .default()
- FolderTreeSort
- .default()
- get_settings_path()
- crate::app_state::AppState
- serde::{Deserialize, Serialize}
- serde_json::Value
- std::collections::{HashMap, HashSet}
- std::fs
- std::path::PathBuf
- tauri::{AppHandle, Manager}
- LastFolderState
- MyLens
- PasteMode
- save_settings()
- SortCriteria
- _…and 2 more_

### Community 71 — handleSave() (71) (22 nodes, cohesion: 0.09)

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
- ../../types/typography/TextColors
- ../../types/typography/TextVariants
- ../../types/typography/TextWeights
- _…and 2 more_

### Community 72 — set_timestamps_from_exif() (21 nodes, cohesion: 0.18)

- build_single_mask_adjustments()
- encode_grayscale_to_png()
- encode_image_to_bytes()
- ensure_export_not_cancelled()
- estimate_export_sizes()
- export_adjustments_as_lut()
- export_images()
- export_images_impl()
- export_masks_for_image()
- ExportTaskGuard
- .drop()
- .new()
- .with_app_handle()
- finish_export_task()
- mime_type_for_extension()
- register_export_task()
- relative_dir_is_safe()
- relative_export_dir_for_preserved_folders()
- run_headless_export()
- save_image_with_metadata()
- _…and 1 more_

### Community 73 — hasFineAdjustmentModifier() (21 nodes, cohesion: 0.10)

- Slider
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
- react/useMemo
- react/useRef
- _…and 1 more_

### Community 74 — handleSave() (21 nodes, cohesion: 0.10)

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

### Community 75 — visibleNow() (21 nodes, cohesion: 0.10)

- useSdImportActions
- filterState()
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
- ../store/useImportStore/useImportStore
- ../store/useSettingsStore/useSettingsStore
- ../store/useUIStore/useUIStore
- @tauri-apps/api/core/invoke
- @tauri-apps/api/event/listen
- @tauri-apps/plugin-dialog/open
- rawExts()
- _…and 1 more_

### Community 76 — useAppNavigation() (20 nodes, cohesion: 0.10)

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
- ../utils/adjustments/INITIAL_ADJUSTMENTS
- ../utils/adjustments/normalizeLoadedAdjustments
- ../utils/ImageLRUCache/globalImageCache
- useAppNavigation()

### Community 77 — useKeyboardShortcuts() (20 nodes, cohesion: 0.10)

- useKeyboardShortcuts
- ../components/ui/AppProperties/ExifOverlay
- ../components/ui/AppProperties/ImageFile
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
- ../utils/keyboardUtils/KEYBIND_DEFINITIONS
- ../utils/keyboardUtils/normalizeCombo
- ../utils/keyboardUtils/resolveNudgeStep
- useKeyboardShortcuts()

### Community 78 — upload_source_image() (19 nodes, cohesion: 0.13)

- ai_connector
- check_status()
- composite_full_res()
- generate_source_id()
- image_to_base64()
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
- process_inpainting()
- upload_source_image()

### Community 79 — useImageProcessing() (19 nodes, cohesion: 0.11)

- useImageProcessing
- ../components/ui/AppProperties/Invokes
- ../components/ui/AppProperties/Panel
- lodash.debounce/debounce
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

### Community 80 — zustand/react/shallow/useShallow (80) (19 nodes, cohesion: 0.11)

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
- zustand/react/shallow/useShallow

### Community 81 — useContextMenu() (19 nodes, cohesion: 0.11)

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

### Community 82 — clearAll() (19 nodes, cohesion: 0.11)

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

### Community 83 — useEditorActions() (19 nodes, cohesion: 0.11)

- useEditorActions
- ../components/ui/AppProperties/Invokes
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

### Community 84 — handleSelect() (19 nodes, cohesion: 0.11)

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

### Community 85 — onDragMove() (85) (18 nodes, cohesion: 0.11)

- SidePanelArea
- handleContentInteraction()
- clsx/clsx
- @dnd-kit/core/useDndMonitor
- @dnd-kit/core/useDroppable
- framer-motion/AnimatePresence
- framer-motion/motion
- ./PanelSwitcher/PanelSwitcher
- react-i18next/useTranslation
- react/useCallback
- react/useRef
- react/useState
- ../../store/useUIStore/SwitcherPlacement
- ../../store/useUIStore/useUIStore
- ../ui/AppProperties/Panel
- ../ui/AppProperties/PanelRegion
- onDragEnd()
- onDragMove()

### Community 86 — renderBody() (18 nodes, cohesion: 0.11)

- ImportView
- ./CullGroupsGrid/CullGroupsGrid
- ../../../hooks/useImportKeyboard/useImportKeyboard
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
- ../../../store/useProcessStore/useProcessStore
- ../../ui/ExportImportProperties/Status
- zustand/react/shallow/useShallow
- renderBody()

### Community 87 — ../../../utils/adjustments/ADJUSTMENT_GROUPS (18 nodes, cohesion: 0.11)

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

### Community 88 — ../../utils/adjustments/HueSatLum (18 nodes, cohesion: 0.11)

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

### Community 89 — useAiMasking() (17 nodes, cohesion: 0.13)

- useAiMasking
- getTransformAdjustments()
- @clerk/react/useAuth
- ../components/panel/right/Masks/SubMask
- ../components/ui/AppProperties/Invokes
- react-toastify/toast
- react/useCallback
- react/useEffect
- ../store/useEditorStore/useEditorStore
- @tauri-apps/api/core/invoke
- ./useEditorActions/debouncedSetHistory
- ./useEditorActions/useEditorActions
- ../utils/adjustments/Adjustments
- ../utils/adjustments/AiPatch
- ../utils/adjustments/Coord
- ../utils/adjustments/MaskContainer
- useAiMasking()

### Community 90 — useLibraryActions() (17 nodes, cohesion: 0.12)

- useLibraryActions
- ../components/ui/AppProperties/Album
- ../components/ui/AppProperties/AlbumGroup
- ../components/ui/AppProperties/AlbumItem
- ../components/ui/AppProperties/ImageFile
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
- ../utils/ImageLRUCache/globalImageCache
- useLibraryActions()

### Community 91 — clsx() (91) (17 nodes, cohesion: 0.12)

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

### Community 92 — ../../ui/AppProperties/Invokes (17 nodes, cohesion: 0.12)

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

### Community 93 — zustand/react/shallow/useShallow (16 nodes, cohesion: 0.13)

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
- ../../utils/imageGrouping/GroupBadgeInfo
- ../../utils/imageGrouping/GroupId
- zustand/react/shallow/useShallow

### Community 94 — PresetTypeSwitch() (16 nodes, cohesion: 0.13)

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

### Community 95 — dof_to_energy() (16 nodes, cohesion: 0.18)

- lens_blur
- BokehTap
- build_bokeh_kernel()
- depth_to_signed_coc()
- dof_aperture_intensity()
- dof_despeckle()
- dof_downsample_fused()
- dof_med3()
- dof_polygon_scale()
- dof_smoothstep()
- dof_soft_pow()
- dof_to_energy()
- base64::{Engine as _, engine::general_purpose::STANDARD as BASE64}
- image::{DynamicImage, GenericImageView, Rgb32FImage}
- rayon::prelude::*
- std::borrow::Cow

### Community 96 — WaveformData (16 nodes, cohesion: 0.20)

- analysis
- apply_gaussian_smoothing()
- auto_results_to_json()
- calculate_auto_adjustments()
- calculate_histogram_from_image()
- calculate_waveform_from_image()
- detect_face_median_luma()
- HistogramData
- super::*
- learn_auto_profile_from_folder()
- luma_median()
- median_f64()
- normalize_histogram_range()
- perform_auto_analysis()
- update_auto_profile_from_image()
- WaveformData

### Community 97 — react-i18next/initReactI18next (16 nodes, cohesion: 0.13)

- index
- i18next/i18n
- ./locales/ca.json/ca
- ./locales/de.json/de
- ./locales/en.json/en
- ./locales/es.json/es
- ./locales/fr.json/fr
- ./locales/it.json/it
- ./locales/ja.json/ja
- ./locales/ko.json/ko
- ./locales/pl.json/pl
- ./locales/pt.json/pt
- ./locales/ru.json/ru
- ./locales/zh-CN.json/zhCN
- ./locales/zh-TW.json/zhTW
- react-i18next/initReactI18next

### Community 98 — test_ai_connector_connection() (16 nodes, cohesion: 0.13)

- ai_commands
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
- crate::app_state::AppState
- crate::cache_utils::GEOMETRY_KEYS
- crate::get_cached_full_warped_image
- image::{GrayImage, ImageFormat}
- std::collections::hash_map::DefaultHasher
- std::hash::{Hash, Hasher}
- std::io::Cursor
- precompute_ai_subject_mask()
- test_ai_connector_connection()

### Community 99 — handleRemoveTag() (16 nodes, cohesion: 0.13)

- TaggingSubMenu
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
- @tauri-apps/api/core/invoke
- ../types/typography/TextVariants

### Community 100 — handleVisibilityClick() (15 nodes, cohesion: 0.13)

- CollapsibleSection
- handleMouseEnter()
- handleMouseLeave()
- handleVisibilityClick()
- clsx/clsx
- lucide-react/ChevronDown
- lucide-react/Eye
- lucide-react/EyeOff
- react-i18next/useTranslation
- react/useEffect
- react/useRef
- react/useState
- ./Text/Text
- ../../types/typography/TextVariants
- ../../types/typography/TextWeights

### Community 101 — stringifyArg() (15 nodes, cohesion: 0.27)

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

### Community 102 — invoke_generative_replace_with_mask_def() (15 nodes, cohesion: 0.27)

- inpainting
- generate_manual_cleanup_patch()
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
- serde_json::Value
- std::io::Cursor
- invoke_generative_replace_with_mask_def()

### Community 103 — GlobalTooltip() (15 nodes, cohesion: 0.14)

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

### Community 104 — handleClose() (15 nodes, cohesion: 0.13)

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

### Community 105 — usePresets() (15 nodes, cohesion: 0.14)

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

### Community 106 — CullModel (14 nodes, cohesion: 0.20)

- cull_model
- CullModel
- .default()
- .effective_weights()
- .file_path()
- .load()
- .record()
- .reset()
- .sample_count()
- .save()
- .score()
- serde::{Deserialize, Serialize}
- std::path::PathBuf
- tauri::Manager

### Community 107 — verify_sha256() (107) (14 nodes, cohesion: 0.26)

- download_and_verify_model()
- download_model()
- get_models_dir()
- get_or_init_ai_models()
- get_or_init_clip_models()
- get_or_init_denoise_model()
- get_or_init_face_model()
- get_or_init_lama_model()
- persist_downloaded_asset()
- promote_legacy_model_filename()
- run_lama_inpainting()
- TileParams
- .new()
- verify_sha256()

### Community 108 — useExternalEditSession() (14 nodes, cohesion: 0.29)

- useExternalEditSession 2
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

### Community 109 — wait() (14 nodes, cohesion: 0.25)

- replay
- backToLibrary()
- dispatchMouse()
- dragSlider()
- durationsInWindow()
- editTwoSliders()
- frameTick()
- openFirstImage()
- scrollLibrary()
- startMeasuring()
- stopMeasuring()
- summarizeWindow()
- undoSliderEdits()
- wait()

### Community 110 — handleReset() (13 nodes, cohesion: 0.18)

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

### Community 111 — useFileOperations() (13 nodes, cohesion: 0.15)

- useFileOperations
- ../components/ui/AppProperties/Invokes
- ../components/ui/ExportImportProperties/Status
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

### Community 112 — getLine() (13 nodes, cohesion: 0.15)

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

### Community 113 — onWebViewCreate (13 nodes, cohesion: 0.15)

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

### Community 114 — zustand/create (114) (13 nodes, cohesion: 0.15)

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

### Community 115 — save_image_bytes_to_android_gallery() (13 nodes, cohesion: 0.15)

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
- save_file_bytes_to_android_downloads()
- save_image_bytes_to_android_gallery()

### Community 116 — parse_num() (13 nodes, cohesion: 0.22)

- preset_converter
- convert_xmp_to_preset()
- extract_tone_curve_points()
- extract_xmp_name()
- get_attr_as_f64()
- crate::file_management::Preset
- regex::Regex
- serde_json::{Map, Value, json}
- std::collections::HashMap
- uuid::Uuid
- Num
- num_to_json()
- parse_num()

### Community 117 — ../utils/imageGrouping/GroupId (13 nodes, cohesion: 0.15)

- useSortedLibrary
- ../components/ui/AppProperties/EditedStatus
- ../components/ui/AppProperties/GroupingMode
- ../components/ui/AppProperties/ImageFile
- ../components/ui/AppProperties/NegativeStatus
- ../components/ui/AppProperties/RawStatus
- ../components/ui/AppProperties/SortDirection
- react/useMemo
- ../store/useLibraryStore/useLibraryStore
- ../store/useSettingsStore/useSettingsStore
- ../utils/imageGrouping/buildImageGroups
- ../utils/imageGrouping/GroupBadgeInfo
- ../utils/imageGrouping/GroupId

### Community 118 — formatBytes() (13 nodes, cohesion: 0.15)

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

### Community 119 — save_presets() (12 nodes, cohesion: 0.26)

- collect_top_level_preset_names()
- get_presets_path()
- handle_import_legacy_presets_from_file()
- handle_import_presets_from_file()
- handle_import_presets_from_files()
- import_preset_file_into_library()
- load_presets()
- merge_imported_items()
- parse_preset_file()
- preset_file_display_name()
- save_community_preset()
- save_presets()

### Community 120 — RestoreDownIcon() (12 nodes, cohesion: 0.17)

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

### Community 121 — handleReset() (121) (12 nodes, cohesion: 0.17)

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

### Community 122 — srgb_to_linear() (11 nodes, cohesion: 0.24)

- raw_processing
- develop_internal()
- develop_raw_image()
- get_fast_demosaic_scale_factor()
- anyhow::{Result, anyhow}
- crate::image_processing::apply_orientation
- image::{DynamicImage, ImageBuffer, Rgba}
- rawler::{
    decoders::{Orientation, RawDecodeParams},
    imgop::develop::{DemosaicAlgorithm, Intermediate, ProcessingStep, RawDevelop},
    rawimage::{RawImage, RawPhotometricInterpretation},
    rawsource::RawSource,
}
- std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
}
- is_linear_raw_format()
- srgb_to_linear()

### Community 123 — ImageLRUCache (11 nodes, cohesion: 0.36)

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

### Community 124 — async() (11 nodes, cohesion: 0.18)

- ImportReviewBar
- async()
- ../../../hooks/useSdImportActions/useSdImportActions
- lucide-react/ArrowRight
- lucide-react/FolderInput
- lucide-react/Settings2
- react/useState
- ../../../store/useImportStore/useImportStore
- ../../ui/Button/Button
- ../../ui/Switch/Switch
- zustand/react/shallow/useShallow

### Community 125 — runTauriCli (11 nodes, cohesion: 0.20)

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

### Community 126 — useTauriListeners() (11 nodes, cohesion: 0.18)

- useTauriListeners
- ../components/ui/ExportImportProperties/Status
- react/useEffect
- react/useRef
- ../store/useEditorStore/useEditorStore
- ../store/useLibraryStore/useLibraryStore
- ../store/useProcessStore/useProcessStore
- ../store/useUIStore/useUIStore
- @tauri-apps/api/core/convertFileSrc
- @tauri-apps/api/event/listen
- useTauriListeners()

### Community 127 — clear_session_caches() (11 nodes, cohesion: 0.20)

- cache_utils
- calculate_full_job_hash()
- clear_image_caches()
- clear_session_caches()
- .clear()
- crate::AppState
- image::DynamicImage
- std::collections::hash_map::DefaultHasher
- std::collections::HashMap
- std::hash::{Hash, Hasher}
- std::sync::Arc

### Community 128 — zustand/create (128) (11 nodes, cohesion: 0.18)

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

### Community 129 — save_bytes_to_android_media_store() (11 nodes, cohesion: 0.38)

- clear_pending_android_exception()
- close_android_closeable()
- delete_android_media_store_item()
- get_android_content_resolver()
- map_android_jni_error()
- parse_android_uri()
- put_android_content_value_int()
- put_android_content_value_string()
- read_android_content_uri()
- resolve_android_content_uri_name()
- save_bytes_to_android_media_store()

### Community 130 — handleVariableClick() (11 nodes, cohesion: 0.18)

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

### Community 131 — useImageLoader() (11 nodes, cohesion: 0.18)

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

### Community 132 — parse_launch_args() (11 nodes, cohesion: 0.20)

- launch_request
- emit_launch_request()
- ExternalEditSession
- handle_file_open()
- HeadlessExportSession
- serde::{Deserialize, Serialize}
- std::path::PathBuf
- tauri::Emitter
- LaunchPayload
- LaunchRequest
- parse_launch_args()

### Community 133 — SeamInfo (11 nodes, cohesion: 0.18)

- stitching
- crate::panorama_stitching::ImageInfo
- image::{GrayImage, Rgb, Rgb32FImage}
- nalgebra::{Matrix3, Point3}
- rayon::prelude::*
- std::collections::HashMap
- std::path::Path
- tauri::{AppHandle, Emitter}
- SeamContext
- SeamInfo
- SeamOrientation

### Community 134 — sync_album_path_changes() (10 nodes, cohesion: 0.38)

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

### Community 135 — RenameFileModal() (10 nodes, cohesion: 0.20)

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

### Community 136 — ../../types/typography/VariantConfig (10 nodes, cohesion: 0.20)

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

### Community 137 — normalizeLoadedAdjustments() (10 nodes, cohesion: 0.29)

- adjustments
- deepCloneCurves()
- deepCloneParametric()
- getDefaultCurves()
- getDefaultParametricCurve()
- ../components/panel/right/Masks/SubMask
- ../components/panel/right/Masks/SubMaskMode
- react-image-crop/Crop
- uuid/v4
- normalizeLoadedAdjustments()

### Community 138 — RustPlugin (10 nodes, cohesion: 0.20)

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

### Community 139 — ConfirmModal() (9 nodes, cohesion: 0.22)

- ConfirmModal
- ConfirmModal()
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useState
- ../../types/typography/TextVariants
- ../ui/Button/Button
- ../ui/Text/Text

### Community 140 — scan_dir_lazy() (9 nodes, cohesion: 0.36)

- folder_tree
- FolderNode
- get_folder_children()
- get_folder_tree()
- get_folder_tree_sync()
- get_pinned_folder_trees()
- has_subdirs()
- super::*
- scan_dir_lazy()

### Community 141 — verify_sha256() (9 nodes, cohesion: 0.31)

- build
- download_and_verify()
- sha2::{Digest, Sha256}
- std::env
- std::fs
- std::io::{self, Read}
- std::path::{Path, PathBuf}
- main()
- verify_sha256()

### Community 142 — DecodedImageCache (9 nodes, cohesion: 0.33)

- calculate_geometry_hash()
- calculate_thumbnail_base_hash()
- calculate_transform_hash()
- calculate_visual_hash()
- DecodedImageCache
- .get()
- .insert()
- .new()
- .set_capacity()

### Community 143 — strip_maker_prefix() (9 nodes, cohesion: 0.33)

- autodetect_lens()
- find_best_lens_match()
- get_lens_distortion_params()
- get_lensfun_lenses_for_maker()
- .get_canonical_model_name()
- .get_display_name()
- lenses_for_maker()
- resolve_lens_params()
- strip_maker_prefix()

### Community 144 — vanishingPoint() (9 nodes, cohesion: 0.44)

- keystone
- autoFitScale()
- clamp()
- fitScaleForParams()
- frameFitsInside()
- invSource()
- lineCoeffs()
- solveKeystone()
- vanishingPoint()

### Community 145 — hydrate_sub_masks() (9 nodes, cohesion: 0.25)

- adjustment_utils
- apply_all_transformations()
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

### Community 146 — handleAdjustmentChange() (9 nodes, cohesion: 0.22)

- Details
- handleAdjustmentChange()
- react-i18next/useTranslation
- ../../types/typography/TextVariants
- ../ui/AppProperties/AppSettings
- ../ui/Slider/Slider
- ../ui/Text/Text
- ../../utils/adjustments/Adjustments
- ../../utils/adjustments/DetailsAdjustment

### Community 147 — useExportSettings() (9 nodes, cohesion: 0.22)

- useExportSettings
- ../components/ui/ExportImportProperties/DEFAULT_FILENAME_TEMPLATE
- ../components/ui/ExportImportProperties/ExportPreset
- ../components/ui/ExportImportProperties/sanitizeFilenameTemplate
- ../components/ui/ExportImportProperties/WatermarkAnchor
- react/useCallback
- react/useMemo
- react/useState
- useExportSettings()

### Community 148 — xmp_packet_for_source() (9 nodes, cohesion: 0.36)

- build_xmp_packet()
- encode_tiff()
- insert_xmp_into_tiff()
- round_trips_through_the_apps_own_tag_parser()
- tiff_ifd_stays_sorted_and_gains_exactly_one_entry()
- tiff_is_not_double_tagged_on_re_export()
- tiff_keywords_embed_and_the_image_still_decodes()
- write_xmp_only()
- xmp_packet_for_source()

### Community 149 — parse_hsl_adjustments() (8 nodes, cohesion: 0.54)

- convert_points_to_aligned()
- get_all_adjustments_from_json()
- get_global_adjustments_from_json()
- get_mask_adjustments_from_json()
- GpuMat3
- .default()
- parse_color_grade_settings()
- parse_hsl_adjustments()

### Community 150 — ComfyClient (8 nodes, cohesion: 0.43)

- ComfyClient
- .check_health()
- .execute()
- ._fetch_image()
- ._get_history()
- .__init__()
- ._queue_prompt()
- .get()

### Community 151 — pickPrimary() (8 nodes, cohesion: 0.36)

- imageGrouping
- buildImageGroups()
- findGroupVariants()
- getFileExtension()
- getVariantLabel()
- ../components/ui/AppProperties/GroupPreference
- ../components/ui/AppProperties/ImageFile
- pickPrimary()

### Community 152 — truncate_large_exif() (8 nodes, cohesion: 0.36)

- clean_ascii_value()
- extract_metadata()
- format_lens_specification()
- load_sidecar()
- rational_to_f32_checked()
- rawler_rational_to_f32_checked()
- read_exif_data_from_bytes()
- truncate_large_exif()

### Community 153 — useThumbnails() (8 nodes, cohesion: 0.25)

- useThumbnails
- lodash.debounce/debounce
- react/useCallback
- react/useEffect
- react/useMemo
- react/useRef
- @tauri-apps/api/core/invoke
- useThumbnails()

### Community 154 — zustand/create (8 nodes, cohesion: 0.25)

- useSettingsStore
- ../components/ui/AppProperties/AppSettings
- ../components/ui/AppProperties/Invokes
- ../components/ui/AppProperties/SupportedTypes
- @tauri-apps/api/core/invoke
- @tauri-apps/plugin-os/platform
- ../utils/themes/DEFAULT_THEME_ID
- zustand/create

### Community 155 — try_get_exif_creation_date() (8 nodes, cohesion: 0.36)

- clean_creation_datetime_str()
- fmt_date_str()
- get_creation_date_from_path()
- normalize_creation_datetime()
- parse_creation_datetime()
- parse_creation_field()
- parse_raw_creation_date()
- try_get_exif_creation_date()

### Community 156 — useWaveformControls() (7 nodes, cohesion: 0.29)

- useWaveformControls
- react/PointerEvent
- react/useCallback
- react/useState
- ../store/useEditorStore/useEditorStore
- ../store/useSettingsStore/useSettingsStore
- useWaveformControls()

### Community 157 — handleSelectFile() (7 nodes, cohesion: 0.29)

- ImagePicker
- handleSelectFile()
- lucide-react/X
- react-i18next/useTranslation
- @tauri-apps/plugin-dialog/open
- ./Text/Text
- ../../types/typography/TextVariants

### Community 158 — ../ui/Text/Text (158) (7 nodes, cohesion: 0.29)

- CreateFolderModal
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useState
- ../../types/typography/TextVariants
- ../ui/Text/Text

### Community 159 — createSubMask() (7 nodes, cohesion: 0.29)

- maskUtils
- createSubMask()
- ../components/panel/right/Masks/formatMaskTypeName
- ../components/panel/right/Masks/Mask
- ../components/panel/right/Masks/SubMaskMode
- ../hooks/useImageRenderSize/ImageDimensions
- uuid/v4

### Community 160 — ../ui/Text/Text (7 nodes, cohesion: 0.29)

- RenameFolderModal
- react-i18next/useTranslation
- react/useCallback
- react/useEffect
- react/useState
- ../../types/typography/TextVariants
- ../ui/Text/Text

### Community 161 — rotateCropCenter() (7 nodes, cohesion: 0.43)

- cropUtils
- calculateAreaPreservingCrop()
- calculateCenteredCrop()
- getOrientedDimensions()
- react-image-crop/Crop
- isCropWithinBounds()
- rotateCropCenter()

### Community 162 — default_tagging_shortcuts_option() (7 nodes, cohesion: 0.29)

- AppSettings
- .default()
- default_adjustment_visibility()
- default_export_presets()
- default_linear_raw_mode()
- default_open_tree_sections()
- default_tagging_shortcuts_option()

### Community 163 — zustand/middleware/persist (163) (7 nodes, cohesion: 0.29)

- useImportStore
- ../components/ui/AppProperties/CullingSettings
- ../components/ui/AppProperties/CullingSuggestions
- ../components/ui/AppProperties/SortDirection
- ../components/views/import/importFilters/FileTypeFilter
- zustand/create
- zustand/middleware/persist

### Community 164 — wiener_filter() (7 nodes, cohesion: 0.33)

- AtomicAccumulator
- .add()
- .to_vec()
- bm3d_process_joint()
- hard_threshold()
- run_bm3d_step_joint()
- wiener_filter()

### Community 165 — nextMessageId() (7 nodes, cohesion: 0.33)

- useAssistantStore
- deriveTitle()
- zustand/create
- zustand/middleware/persist
- makeConversation()
- nextConversationId()
- nextMessageId()

### Community 166 — stitch_panorama() (7 nodes, cohesion: 0.52)

- build_stitching_order()
- Dsu
- .find()
- .new()
- .union()
- stitch_images()
- stitch_panorama()

### Community 167 — write_rrexif_sidecar() (7 nodes, cohesion: 0.57)

- get_rrexif_path()
- load_primary_metadata()
- persist_exif_if_missing()
- read_exif_data()
- read_rrexif_sidecar()
- save_primary_metadata()
- write_rrexif_sidecar()

### Community 168 — zustand/create (168) (7 nodes, cohesion: 0.29)

- useUIStore
- ../components/ui/AppProperties/CullingSuggestions
- ../components/ui/AppProperties/ImageFile
- ../components/ui/AppProperties/Panel
- ../components/ui/AppProperties/PanelRegion
- ../components/ui/AppProperties/UiVisibility
- zustand/create

### Community 169 — PinchZoomDisablePlugin (7 nodes, cohesion: 0.33)

- window_customizer
- apply_macos_window_rounding()
- tauri::{Runtime, Webview, plugin::Plugin}
- PinchZoomDisablePlugin
- .default()
- .name()
- .webview_created()

### Community 170 — update_json_file() (7 nodes, cohesion: 0.43)

- update_translations
- deep_merge()
- json
- pathlib.Path
- main()
- sort_dict_recursively()
- update_json_file()

### Community 171 — resolveNudgeStep() (7 nodes, cohesion: 0.38)

- keyboardUtils
- arraysEqual()
- codeToDisplayLabel()
- formatKeyCode()
- isValidShortcutKey()
- normalizeCombo()
- resolveNudgeStep()

### Community 172 — useProductivityActions() (6 nodes, cohesion: 0.33)

- useProductivityActions
- ../components/ui/AppProperties/Invokes
- react/useCallback
- ../store/useUIStore/useUIStore
- @tauri-apps/api/core/invoke
- useProductivityActions()

### Community 173 — ../../types/typography/TextVariants (6 nodes, cohesion: 0.33)

- Switch
- clsx/clsx
- framer-motion/motion
- react/React
- ./Text/Text
- ../../types/typography/TextVariants

### Community 174 — generate_ai_subject_mask() (6 nodes, cohesion: 0.33)

- encode_to_base64_png()
- generate_ai_depth_mask()
- generate_ai_face_region_mask()
- generate_ai_foreground_mask()
- generate_ai_sky_mask()
- generate_ai_subject_mask()

### Community 175 — ./utils/frontendLogBridge/installFrontendLogBridge (6 nodes, cohesion: 0.33)

- main
- ./App/App
- react-dom/client/createRoot
- react/React
- ./styles.css
- ./utils/frontendLogBridge/installFrontendLogBridge

### Community 176 — isVertical() (6 nodes, cohesion: 0.40)

- GuidedKeystoneOverlay
- GuidedKeystoneOverlay()
- react/useRef
- react/useState
- ../../../store/useEditorStore/useEditorStore
- isVertical()

### Community 177 — remove_lut() (6 nodes, cohesion: 0.67)

- combined_lut_list()
- get_luts_dir()
- import_luts()
- list_luts()
- list_luts_in_dir()
- remove_lut()

### Community 178 — ../../store/useProcessStore/ExternalEditSession (6 nodes, cohesion: 0.67)

- ExternalEditBar 2
- ./Button/Button
- lucide-react/Check
- lucide-react/Loader
- react-i18next/useTranslation
- ../../store/useProcessStore/ExternalEditSession

### Community 179 — useSortedLibrary() (6 nodes, cohesion: 0.33)

- computeGroupedLibrary()
- computeSortedLibrary()
- parseAperture()
- parseFocalLength()
- parseShutter()
- useSortedLibrary()

### Community 180 — warp_image_homography() (6 nodes, cohesion: 0.47)

- find_adaptive_seam()
- find_pairwise_seam_dp_horizontal()
- find_pairwise_seam_dp_vertical()
- get_interpolated_pixel()
- progressive_seam_stitcher()
- warp_image_homography()

### Community 181 — SourceCache (6 nodes, cohesion: 0.53)

- SourceCache
- .add()
- ._delete()
- ._enforce_limits()
- .__init__()
- ._sync()

### Community 182 — zustand/create (182) (6 nodes, cohesion: 0.33)

- useProcessStore
- ../components/ui/AppProperties/Progress
- ../components/ui/ExportImportProperties/ExportState
- ../components/ui/ExportImportProperties/ImportState
- ../components/ui/ExportImportProperties/Status
- zustand/create

### Community 183 — start_analytics_worker() (6 nodes, cohesion: 0.33)

- available_monitor_bounds()
- register_exit_handler()
- run()
- saved_window_state_is_usable()
- setup_logging()
- start_analytics_worker()

### Community 184 — write_image_with_metadata() (6 nodes, cohesion: 0.53)

- a_format_we_cannot_embed_leaves_the_bytes_untouched()
- get_primary_sidecar_path()
- keep_metadata_off_writes_no_keywords()
- png_keywords_land_in_an_itxt_chunk_before_iend()
- sidecar_tags_reach_the_exported_jpeg()
- write_image_with_metadata()

### Community 185 — Settings (6 nodes, cohesion: 0.33)

- Settings
- .comfy_url()
- .http_url()
- .sent_cache_dir()
- .source_cache_dir()
- .ws_url()

### Community 186 — inverse_transform_mask() (6 nodes, cohesion: 0.47)

- .into_cow()
- apply_coarse_rotation()
- apply_crop()
- apply_flip()
- apply_rotation()
- inverse_transform_mask()

### Community 187 — load_image_with_orientation() (5 nodes, cohesion: 0.40)

- classify_raw_develop_error()
- linearize_embedded_preview()
- load_base_image_raw()
- load_base_image_with_fallback_raw()
- load_image_with_orientation()

### Community 188 — merge_channels() (5 nodes, cohesion: 0.50)

- apply_denoising()
- .new()
- batch_denoise_images()
- denoise_image()
- merge_channels()

### Community 189 — Lens (5 nodes, cohesion: 0.60)

- get_lensfun_makers()
- Lens
- .get_full_model_name()
- .get_maker()
- .get_name()

### Community 190 — useAndroidBackHandler() (5 nodes, cohesion: 0.80)

- useAndroidBackHandler 2
- react/useEffect
- ../store/useSettingsStore/useSettingsStore
- ../store/useUIStore/useUIStore
- useAndroidBackHandler()

### Community 191 — generate_ai_subject_bitmap() (5 nodes, cohesion: 0.40)

- generate_ai_bitmap_from_base64()
- generate_ai_bitmap_from_full_mask()
- generate_ai_depth_bitmap()
- generate_ai_foreground_bitmap()
- generate_ai_subject_bitmap()

### Community 192 — grayscale_erode() (5 nodes, cohesion: 0.40)

- apply_grow_and_feather()
- generate_color_bitmap()
- generate_luminance_bitmap()
- grayscale_dilate()
- grayscale_erode()

### Community 193 — Button() (5 nodes, cohesion: 0.40)

- Button
- Button()
- clsx/clsx
- react/ButtonHTMLAttributes
- react/ReactNode

### Community 194 — read_raw_metadata() (5 nodes, cohesion: 0.60)

- get_creation_date_from_bytes()
- read_exif()
- read_exposure_time_secs()
- read_iso()
- read_raw_metadata()

### Community 195 — ScanLook (5 nodes, cohesion: 0.50)

- compress_embeds_film_metadata()
- compress_scan()
- compress_scan_quantizes_shrinks_and_round_trips()
- ScanLook
- .default()

### Community 196 — summarizePhaseAcrossIterations() (5 nodes, cohesion: 0.40)

- median()
- p95()
- statOf()
- stdev()
- summarizePhaseAcrossIterations()

### Community 197 — ycbcr_to_rgb() (5 nodes, cohesion: 0.40)

- gaussian_blur_1ch()
- rgb_to_ycbcr()
- run_bm3d()
- split_channels()
- ycbcr_to_rgb()

### Community 198 — load_settings() (5 nodes, cohesion: 0.60)

- all_available_adjustments()
- CopyPasteSettings
- .default()
- default_included_adjustments()
- load_settings()

### Community 199 — ../../../store/useProcessStore/useProcessStore (5 nodes, cohesion: 0.40)

- LazyThumb
- ../../../hooks/useThumbnails/useThumbnails
- react/useEffect
- react/useRef
- ../../../store/useProcessStore/useProcessStore

### Community 200 — parse_lut_file() (5 nodes, cohesion: 0.40)

- load_and_parse_lut()
- parse_3dl()
- parse_cube()
- parse_hald()
- parse_lut_file()

### Community 201 — run() (5 nodes, cohesion: 0.40)

- handleCapture()
- handleDetect()
- handleDisconnect()
- handleLiveView()
- run()

### Community 202 — ../../utils/adjustments/CopyPasteSettings (5 nodes, cohesion: 0.40)

- AppProperties
- ./ExportImportProperties/ExportPreset
- ../panel/right/Masks/ToolType
- ../../utils/adjustments/Adjustments
- ../../utils/adjustments/CopyPasteSettings

### Community 203 — dof_depth_to_f32() (5 nodes, cohesion: 0.50)

- blur_layer_bokeh()
- build_coc_field()
- build_guided_model()
- dof_box_filter()
- dof_depth_to_f32()

### Community 204 — is_image_edited() (5 nodes, cohesion: 0.60)

- apply_geometry_warp()
- apply_unwarp_geometry()
- get_geometry_params_from_json()
- is_geometry_identity()
- is_image_edited()

### Community 205 — writeCullingMetadata() (5 nodes, cohesion: 0.50)

- computeDefaultKeepers()
- groupArgs()
- groupSettings()
- useSdImportActions()
- writeCullingMetadata()

### Community 206 — generate_sub_mask_bitmap() (5 nodes, cohesion: 0.40)

- generate_ai_sky_bitmap()
- generate_all_bitmap()
- generate_linear_bitmap()
- generate_radial_bitmap()
- generate_sub_mask_bitmap()

### Community 207 — is_supported_image_file() (5 nodes, cohesion: 0.40)

- formats
- std::convert::AsRef
- std::path::Path
- is_raw_file()
- is_supported_image_file()

### Community 208 — updateSnapshots() (5 nodes, cohesion: 0.50)

- commitRename()
- handleOverwrite()
- handleSave()
- snapshotState()
- updateSnapshots()

### Community 209 — render_depth_of_field() (5 nodes, cohesion: 0.50)

- apply_lens_blur()
- dof_composite_stack()
- dof_dilate_spans()
- dof_tent()
- render_depth_of_field()

### Community 210 — ir_defect_mask() (5 nodes, cohesion: 0.50)

- box_blur()
- dilate()
- downsample2()
- ir_clean_scan()
- ir_defect_mask()

### Community 211 — stemKey() (5 nodes, cohesion: 0.70)

- importFilters
- computeVisible()
- computeVisibleSet()
- extOf()
- stemKey()

### Community 212 — read_texture_data_roi() (5 nodes, cohesion: 0.50)

- get_or_init_gpu_context()
- GpuProcessor
- .new()
- .run()
- read_texture_data_roi()

### Community 213 — node:assert/assert (4 nodes, cohesion: 0.50)

- keystone.test
- ./keystone/solveKeystone
- ./keystone/vanishingPoint
- node:assert/assert

### Community 214 — @vitejs/plugin-react/react (4 nodes, cohesion: 0.50)

- vite.config
- @tailwindcss/vite/tailwindcss
- vite/defineConfig
- @vitejs/plugin-react/react

### Community 215 — useImportKeyboard() (4 nodes, cohesion: 0.50)

- useImportKeyboard
- react/useEffect
- ./useSdImportActions/useSdImportActions
- useImportKeyboard()

### Community 216 — useOsPlatform() (4 nodes, cohesion: 0.50)

- useOsPlatform
- react/useMemo
- @tauri-apps/plugin-os/platform
- useOsPlatform()

### Community 217 — react/PointerEventHandler (4 nodes, cohesion: 0.50)

- Resizer
- ./AppProperties/Orientation
- clsx/clsx
- react/PointerEventHandler

### Community 218 — neutralize_wb_if_multiexposure() (4 nodes, cohesion: 1.67)

- multi_exposure 2
- _find_ifd_entry()
- is_incamera_multiexposure_canon()
- neutralize_wb_if_multiexposure()

### Community 219 — useImageRenderSize() (4 nodes, cohesion: 0.50)

- useImageRenderSize
- react/useLayoutEffect
- react/useState
- useImageRenderSize()

### Community 220 — suggestProfile() (4 nodes, cohesion: 0.50)

- filmProfiles
- allProfiles()
- p()
- suggestProfile()

### Community 221 — i18next (3 nodes, cohesion: 0.67)

- i18next.d
- ../i18n/locales/en.json/en
- i18next

### Community 222 — react/SVGProps (3 nodes, cohesion: 0.67)

- ExifIcons
- react/React
- react/SVGProps

### Community 223 — sanitizeFilenameTemplate() (3 nodes, cohesion: 0.67)

- ExportImportProperties
- ./AppProperties/Progress
- sanitizeFilenameTemplate()

### Community 224 — ImageProcessingManager() (3 nodes, cohesion: 0.67)

- ImageProcessingManager
- ImageProcessingManager()
- ../../hooks/useImageProcessing/useImageProcessing

### Community 225 — zustand/middleware/persist (3 nodes, cohesion: 0.67)

- useScannerStore
- zustand/create
- zustand/middleware/persist

### Community 226 — ImageLoaderManager() (3 nodes, cohesion: 0.67)

- ImageLoaderManager
- ImageLoaderManager()
- ../../hooks/useImageLoader/useImageLoader

### Community 227 — react/React (3 nodes, cohesion: 0.67)

- Input
- clsx/clsx
- react/React

### Community 228 — hierarchy (3 nodes, cohesion: 0.67)

- hierarchy
- once_cell::sync::Lazy
- std::collections::HashMap

### Community 229 — ../../right/CropPanel/OverlayMode (3 nodes, cohesion: 0.67)

- CompositionOverlays
- react/React
- ../../right/CropPanel/OverlayMode

### Community 230 — java.io.ByteArrayOutputStream (3 nodes, cohesion: 0.67)

- build.gradle
- groovy.json.JsonSlurper
- java.io.ByteArrayOutputStream

### Community 231 — lucide-react/Star (3 nodes, cohesion: 0.67)

- RatingColor
- ./importFilters/COLOR_HEX
- lucide-react/Star

### Community 232 — zustand/create (232) (2 nodes, cohesion: 1.00)

- useTetherStore
- zustand/create

### Community 233 — ../components/ui/AppProperties/Theme (2 nodes, cohesion: 1.00)

- themes
- ../components/ui/AppProperties/Theme

### Community 234 — java.util.Properties (2 nodes, cohesion: 1.00)

- build.gradle
- java.util.Properties

### Community 235 — main() (2 nodes, cohesion: 1.00)

- main
- main()

### Community 236 — react/JSX (2 nodes, cohesion: 1.00)

- CollageVariants
- react/JSX

### Community 237 — i18next-cli/defineConfig (2 nodes, cohesion: 1.00)

- i18next.config
- i18next-cli/defineConfig

### Community 238 — eslint.config (1 nodes, cohesion: 1.00)

- eslint.config

### Community 239 — candidates (1 nodes, cohesion: 1.00)

- candidates

### Community 240 — .into_cow() (240) (1 nodes, cohesion: 1.00)

- .into_cow()

### Community 241 — mod (241) (1 nodes, cohesion: 1.00)

- mod

### Community 242 — mod (1 nodes, cohesion: 1.00)

- mod

### Community 243 — .into_cow() (243) (1 nodes, cohesion: 1.00)

- .into_cow()

### Community 244 — .into_cow() (1 nodes, cohesion: 1.00)

- .into_cow()

### Community 245 — build.gradle (1 nodes, cohesion: 1.00)

- build.gradle

### Community 246 — typography (1 nodes, cohesion: 1.00)

- typography

## 🕳️ Knowledge Gaps

**Isolated nodes** (9):
- eslint.config
- typography
- build.gradle
- .into_cow()
- .into_cow()
- .into_cow()
- mod
- candidates
- mod

**Thin communities** (< 3 nodes): 15 communities

## 💰 Token Cost

| File | Tokens |
|------|--------|
| input | 0 |
| output | 0 |
| **Total** | **0** |

## ❓ Suggested Questions

1. How does 'src_tauri_src_scanning_rs_open_image' relate to 3 different communities (ScanLook, ir_defect_mask(), write_scan_sidecar())?
1. How does 'src_tauri_src_lut_processing_rs_import_luts_to_dir' relate to 3 different communities (parse_lut_file(), unique_lut_destination(), remove_lut())?
1. How does 'src_tauri_src_lens_blur_rs_blur_layer_bokeh' relate to 3 different communities (render_depth_of_field(), dof_to_energy(), dof_depth_to_f32())?
1. How does 'src_tauri_src_denoising_rs_run_bm3d_step_joint' relate to 3 different communities (walsh_hadamard_1d(), merge_channels(), wiener_filter())?
1. How does 'src_tauri_src_mask_generation_rs_generate_ai_depth_bitmap' relate to 4 different communities (grayscale_erode(), generate_ai_subject_bitmap(), generate_sub_mask_bitmap(), TransformParams)?
1. How does 'src_tauri_src_exif_processing_rs_read_exif_data_from_bytes' relate to 4 different communities (truncate_large_exif(), to_ur64(), write_rrexif_sidecar(), read_raw_metadata())?
1. How does 'src_tauri_src_mask_generation_rs_generate_luminance_bitmap' relate to 3 different communities (generate_sub_mask_bitmap(), grayscale_erode(), TransformParams)?

---
_Generated by graphify-rs_
