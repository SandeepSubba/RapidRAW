use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::app_state::AppState;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SortCriteria {
    pub key: String,
    pub order: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FilterCriteria {
    pub rating: i8,
    pub raw_status: String,
    #[serde(default)]
    pub edited_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flag_status: Option<String>,
    #[serde(default)]
    pub colors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rating_operator: Option<String>,
}

impl Default for FilterCriteria {
    fn default() -> Self {
        Self {
            rating: 0,
            raw_status: "all".to_string(),
            edited_status: Some("all".to_string()),
            flag_status: None,
            colors: Vec::new(),
            rating_operator: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FolderTreeSort {
    pub key: String,
    pub order: String,
}

impl Default for FolderTreeSort {
    fn default() -> Self {
        Self {
            key: "name".to_string(),
            order: "asc".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LastFolderState {
    #[serde(default)]
    pub current_folder_path: Option<String>,
    #[serde(default)]
    pub expanded_folders: Vec<String>,
    #[serde(default)]
    pub active_album_id: Option<String>,
    #[serde(default)]
    pub expanded_album_groups: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MyLens {
    pub maker: String,
    pub model: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CustomAspectRatio {
    pub width: f64,
    pub height: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PasteMode {
    Merge,
    Replace,
}

pub fn all_available_adjustments() -> HashSet<String> {
    [
        "exposure",
        "brightness",
        "contrast",
        "curves",
        "pointCurves",
        "parametricCurve",
        "curveMode",
        "highlights",
        "shadows",
        "whites",
        "blacks",
        "toneMapper",
        "temperature",
        "tint",
        "whiteBalance",
        "saturation",
        "vibrance",
        "hsl",
        "hue",
        "colorGrading",
        "colorCalibration",
        "clarity",
        "structure",
        "dehaze",
        "sharpness",
        "sharpnessThreshold",
        "centré",
        "centréX",
        "centréY",
        "lumaNoiseReduction",
        "colorNoiseReduction",
        "aiDenoiseEnabled",
        "aiDenoiseMethod",
        "aiDenoiseAmount",
        "aiDenoiseDetail",
        "chromaticAberrationRedCyan",
        "chromaticAberrationBlueYellow",
        "negativeConversionEnabled",
        "negativeRedWeight",
        "negativeGreenWeight",
        "negativeBlueWeight",
        "negativeExposure",
        "negativeContrast",
        "vignetteAmount",
        "vignetteFeather",
        "vignetteMidpoint",
        "vignetteRoundness",
        "vignetteCenterX",
        "vignetteCenterY",
        "grainAmount",
        "grainRoughness",
        "grainSize",
        "lutIntensity",
        "lutName",
        "lutPath",
        "lutSize",
        "lutData",
        "lutIsSceneReferred",
        "glowAmount",
        "halationAmount",
        "flareAmount",
        "lensBlurEnabled",
        "lensBlurAmount",
        "lensBlurDiffusion",
        "lensBlurShape",
        "lensBlurMinDepth",
        "lensBlurMaxDepth",
        "lensBlurMinFade",
        "lensBlurMaxFade",
        "fogEnabled",
        "fogAmount",
        "fogStart",
        "fogDensity",
        "fogHeight",
        "fogVariation",
        "fogGlow",
        "fogTemperature",
        "fogTint",
        "relightEnabled",
        "relightLights",
        "relightAmbient",
        "relightSoftness",
        "relightShine",
        "relightShadows",
        "relightShadowSoftness",
        "crop",
        "aspectRatio",
        "rotation",
        "flipHorizontal",
        "flipVertical",
        "orientationSteps",
        "transformDistortion",
        "transformVertical",
        "transformHorizontal",
        "transformRotate",
        "transformAspect",
        "transformScale",
        "transformXOffset",
        "transformYOffset",
        "masks",
        "lensCorrectionMode",
        "lensMaker",
        "lensModel",
        "lensDistortionAmount",
        "lensVignetteAmount",
        "lensTcaAmount",
        "lensDistortionEnabled",
        "lensTcaEnabled",
        "lensVignetteEnabled",
        "guidedPerspective",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

pub fn default_included_adjustments() -> HashSet<String> {
    let mut defaults = all_available_adjustments();

    let off_by_default = [
        "crop",
        "aspectRatio",
        "rotation",
        "flipHorizontal",
        "flipVertical",
        "orientationSteps",
        "transformDistortion",
        "transformVertical",
        "transformHorizontal",
        "transformRotate",
        "transformAspect",
        "transformScale",
        "transformXOffset",
        "transformYOffset",
        "masks",
        "lensCorrectionMode",
        "lensMaker",
        "lensModel",
        "lensDistortionAmount",
        "lensVignetteAmount",
        "lensTcaAmount",
        "lensDistortionEnabled",
        "lensTcaEnabled",
        "lensVignetteEnabled",
        "guidedPerspective",
    ];

    for item in off_by_default.iter() {
        defaults.remove(*item);
    }

    defaults
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CopyPasteSettings {
    pub mode: PasteMode,
    #[serde(default = "default_included_adjustments")]
    pub included_adjustments: HashSet<String>,
    #[serde(default)]
    pub known_adjustments: HashSet<String>,
    #[serde(default)]
    pub auto_sync: bool,
}

impl Default for CopyPasteSettings {
    fn default() -> Self {
        Self {
            mode: PasteMode::Merge,
            included_adjustments: default_included_adjustments(),
            known_adjustments: all_available_adjustments(),
            auto_sync: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreset {
    pub id: String,
    pub name: String,
    pub file_format: String,
    pub jpeg_quality: u8,
    pub enable_resize: bool,
    pub resize_mode: String,
    pub resize_value: u32,
    pub dont_enlarge: bool,
    pub keep_metadata: bool,
    pub strip_gps: bool,
    pub filename_template: String,
    pub enable_watermark: bool,
    pub watermark_path: Option<String>,
    pub watermark_anchor: Option<String>,
    pub watermark_scale: u32,
    pub watermark_spacing: u32,
    pub watermark_opacity: u32,
    #[serde(default)]
    pub export_masks: Option<bool>,
    #[serde(default)]
    pub preserve_folders: Option<bool>,
    #[serde(default)]
    pub enable_pad: Option<bool>,
    #[serde(default)]
    pub pad_ratio_width: Option<f32>,
    #[serde(default)]
    pub pad_ratio_height: Option<f32>,
    #[serde(default)]
    pub pad_color: Option<String>,
    #[serde(default)]
    pub enable_border: Option<bool>,
    #[serde(default)]
    pub border_basis: Option<String>,
    #[serde(default)]
    pub border_horizontal_percent: Option<f32>,
    #[serde(default)]
    pub border_vertical_percent: Option<f32>,
    #[serde(default)]
    pub border_color: Option<String>,
    #[serde(default)]
    pub last_export_path: Option<String>,
    #[serde(default)]
    pub destination_type: Option<String>,
    #[serde(default)]
    pub subfolder: Option<String>,
    #[serde(default)]
    pub tiff_bit_depth: Option<u8>,
    #[serde(default)]
    pub preserve_timestamps: Option<bool>,
}

pub fn default_export_presets() -> Vec<ExportPreset> {
    vec![
        ExportPreset {
            id: "default-hq".to_string(),
            name: "High Quality".to_string(),
            file_format: "jpeg".to_string(),
            jpeg_quality: 95,
            enable_resize: false,
            resize_mode: "longEdge".to_string(),
            resize_value: 2048,
            dont_enlarge: true,
            keep_metadata: true,
            strip_gps: false,
            filename_template: "{original_filename}".to_string(),
            enable_watermark: false,
            watermark_path: None,
            watermark_anchor: Some("bottomRight".to_string()),
            watermark_scale: 10,
            watermark_spacing: 5,
            watermark_opacity: 75,
            export_masks: Some(false),
            preserve_folders: Some(false),
            enable_pad: Some(false),
            pad_ratio_width: Some(1.0),
            pad_ratio_height: Some(1.0),
            pad_color: Some("#ffffff".to_string()),
            enable_border: Some(false),
            border_basis: Some("longEdge".to_string()),
            border_horizontal_percent: Some(2.0),
            border_vertical_percent: Some(2.0),
            border_color: Some("#ffffff".to_string()),
            last_export_path: None,
            destination_type: Some("customFolder".to_string()),
            subfolder: Some("".to_string()),
            tiff_bit_depth: Some(16),
            preserve_timestamps: Some(false),
        },
        ExportPreset {
            id: "default-fast".to_string(),
            name: "Fast (Web)".to_string(),
            file_format: "jpeg".to_string(),
            jpeg_quality: 80,
            enable_resize: true,
            resize_mode: "width".to_string(),
            resize_value: 2048,
            dont_enlarge: true,
            keep_metadata: false,
            strip_gps: true,
            filename_template: "{original_filename}_web".to_string(),
            enable_watermark: false,
            watermark_path: None,
            watermark_anchor: Some("bottomRight".to_string()),
            watermark_scale: 10,
            watermark_spacing: 5,
            watermark_opacity: 75,
            export_masks: Some(false),
            preserve_folders: Some(false),
            enable_pad: Some(false),
            pad_ratio_width: Some(1.0),
            pad_ratio_height: Some(1.0),
            pad_color: Some("#ffffff".to_string()),
            enable_border: Some(false),
            border_basis: Some("longEdge".to_string()),
            border_horizontal_percent: Some(2.0),
            border_vertical_percent: Some(2.0),
            border_color: Some("#ffffff".to_string()),
            last_export_path: None,
            destination_type: Some("customFolder".to_string()),
            subfolder: Some("".to_string()),
            tiff_bit_depth: Some(16),
            preserve_timestamps: Some(false),
        },
    ]
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AdjustmentLayout {
    pub section_order: Vec<String>,
    pub hidden_sections: Vec<String>,
    pub open_sections: BTreeMap<String, bool>,
    pub tool_order: HashMap<String, Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden_tools: Option<Vec<String>>,
    pub collapsed_tools: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceState {
    pub left_panel_width: u32,
    pub right_panel_width: u32,
    pub left_top_height: u32,
    pub right_top_height: u32,
    pub panel_layout: HashMap<String, Vec<String>>,
    pub active_panels: HashMap<String, Option<String>>,
    pub panel_switcher_placement: HashMap<String, String>,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        let mut panel_layout = HashMap::new();
        panel_layout.insert(
            "leftTop".to_string(),
            vec![
                "metadata".to_string(),
                "folderTree".to_string(),
                "export".to_string(),
            ],
        );
        // The assistant gets its own bottom-left region (matches the frontend's
        // reconcileWorkspace default), so Metadata stays visible above it.
        panel_layout.insert("leftBottom".to_string(), vec!["agent".to_string()]);

        panel_layout.insert(
            "rightTop".to_string(),
            vec![
                "adjustments".to_string(),
                "crop".to_string(),
                "masks".to_string(),
                "ai".to_string(),
                "presets".to_string(),
            ],
        );
        panel_layout.insert("rightBottom".to_string(), vec![]);

        let mut active_panels = HashMap::new();
        active_panels.insert("leftTop".to_string(), Some("folderTree".to_string()));
        active_panels.insert("leftBottom".to_string(), Some("agent".to_string()));
        active_panels.insert("rightTop".to_string(), Some("adjustments".to_string()));
        active_panels.insert("rightBottom".to_string(), None);

        let mut panel_switcher_placement = HashMap::new();
        panel_switcher_placement.insert("leftTop".to_string(), "bottom".to_string());
        panel_switcher_placement.insert("leftBottom".to_string(), "bottom".to_string());
        panel_switcher_placement.insert("rightTop".to_string(), "right".to_string());
        panel_switcher_placement.insert("rightBottom".to_string(), "right".to_string());

        Self {
            left_panel_width: 320,
            right_panel_width: 320,
            left_top_height: 450,
            right_top_height: 450,
            panel_layout,
            active_panels,
            panel_switcher_placement,
        }
    }
}

pub fn default_linear_raw_mode() -> String {
    "auto".to_string()
}

pub fn default_baseline_exposure() -> f32 {
    0.7
}

pub fn default_compensate_highlight_preservation() -> bool {
    true
}

pub fn default_ai_gpu_acceleration() -> bool {
    true
}

pub fn default_ai_cache_limit_gb() -> u32 {
    10
}

pub fn default_ai_maps_cache_limit_gb() -> u32 {
    4
}

pub fn default_thumbnail_cache_limit_gb() -> u32 {
    10
}

pub fn default_tagging_shortcuts_option() -> Option<Vec<String>> {
    Some(vec![
        "portrait".to_string(),
        "landscape".to_string(),
        "architecture".to_string(),
        "travel".to_string(),
        "street".to_string(),
        "family".to_string(),
        "nature".to_string(),
        "food".to_string(),
        "event".to_string(),
    ])
}

pub fn default_adjustment_visibility() -> HashMap<String, bool> {
    let mut map = HashMap::new();
    map.insert("sharpening".to_string(), true);
    map.insert("presence".to_string(), true);
    map.insert("noiseReduction".to_string(), true);
    map.insert("chromaticAberration".to_string(), false);
    map.insert("vignette".to_string(), true);
    map.insert("colorCalibration".to_string(), false);
    map.insert("grain".to_string(), true);
    map
}

pub fn default_open_tree_sections() -> Vec<String> {
    vec!["current".to_string()]
}

/// Personal auto-correct targets learned from the user's own edits: cached
/// thumbnails are rendered with adjustments applied, so measuring them tells us
/// how bright this user actually likes faces and midtones.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AutoProfile {
    pub face_target_luma: f64,
    pub midtone_target_luma: f64,
    pub face_samples: u32,
    pub global_samples: u32,
    pub learned_at: u64,
    // The look of the user's finished photos (v2 — older profiles get the stock
    // values and start learning these on the next edit).
    /// Finished photos measured for the style targets below.
    #[serde(default)]
    pub style_samples: u32,
    /// 1st→99th percentile luma span the user ends up with (stock 220).
    #[serde(default = "default_range_target")]
    pub range_target: f64,
    /// Share of pixels left in deep shadow (< 32) in finished photos (stock 0.05).
    #[serde(default = "default_shadow_mass")]
    pub shadow_mass: f64,
    /// Share of pixels left near white (≥ 240) in finished photos (stock 0.02).
    #[serde(default = "default_highlight_mass")]
    pub highlight_mass: f64,
    /// Mean saturation of finished photos (stock 0.20).
    #[serde(default = "default_saturation_target")]
    pub saturation_target: f64,
    /// Cast the user keeps on purpose, in Temperature / Tint slider units
    /// (+ = warm / magenta) — Auto aims for it instead of dead neutral.
    #[serde(default)]
    pub wb_residual_temperature: f64,
    #[serde(default)]
    pub wb_residual_tint: f64,
}

fn default_range_target() -> f64 {
    220.0
}
fn default_shadow_mass() -> f64 {
    0.05
}
fn default_highlight_mass() -> f64 {
    0.02
}
fn default_saturation_target() -> f64 {
    0.20
}

impl Default for AutoProfile {
    fn default() -> Self {
        Self {
            face_target_luma: 165.0,
            midtone_target_luma: 128.0,
            face_samples: 0,
            global_samples: 0,
            learned_at: 0,
            style_samples: 0,
            range_target: default_range_target(),
            shadow_mass: default_shadow_mass(),
            highlight_mass: default_highlight_mass(),
            saturation_target: default_saturation_target(),
            wb_residual_temperature: 0.0,
            wb_residual_tint: 0.0,
        }
    }
}

/// How hard Auto pushes (upstream v1.6.7's strength levels): its corrections are
/// scaled ×0.5 / ×1 / ×1.5.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AutoAdjustMode {
    Subtle,
    #[default]
    Normal,
    Strong,
}

impl AutoAdjustMode {
    pub fn factor(self) -> f64 {
        match self {
            AutoAdjustMode::Subtle => 0.5,
            AutoAdjustMode::Normal => 1.0,
            AutoAdjustMode::Strong => 1.5,
        }
    }
}

/// Which sliders the Auto button is allowed to touch (Capture One's "Auto Adjust"
/// checklist, one switch per slider), and whether it corrects the photo as it
/// currently looks (on top of presets and edits) or starts from the untouched photo.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutoAdjustOptions {
    pub exposure: bool,
    pub brightness: bool,
    pub contrast: bool,
    pub highlights: bool,
    pub shadows: bool,
    pub whites: bool,
    pub blacks: bool,
    /// Temperature + Tint.
    pub white_balance: bool,
    pub vibrance: bool,
    // Creative, so off by default: they are what erased a preset's look.
    pub clarity: bool,
    pub dehaze: bool,
    /// Vignette + Centré (one effect, two sliders).
    pub vignette: bool,
    /// Analyse the rendered photo and add the correction to the current values
    /// instead of replacing them with values measured on the untouched photo.
    pub keep_look: bool,
    pub strength: AutoAdjustMode,
}

impl Default for AutoAdjustOptions {
    fn default() -> Self {
        Self {
            exposure: true,
            brightness: true,
            contrast: true,
            highlights: true,
            shadows: true,
            whites: true,
            blacks: true,
            white_balance: true,
            vibrance: false,
            clarity: false,
            dehaze: false,
            vignette: false,
            keep_look: true,
            strength: AutoAdjustMode::Normal,
        }
    }
}

/// On-disk form: every switch optional (missing = default) plus the grouped
/// switches the first version saved — `tone` (highlights + shadows), `levels`
/// (whites + blacks) and `effects` (clarity + dehaze + vignette) — so a settings
/// file written by it keeps its choices.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct AutoAdjustOptionsWire {
    exposure: Option<bool>,
    brightness: Option<bool>,
    contrast: Option<bool>,
    highlights: Option<bool>,
    shadows: Option<bool>,
    whites: Option<bool>,
    blacks: Option<bool>,
    white_balance: Option<bool>,
    vibrance: Option<bool>,
    clarity: Option<bool>,
    dehaze: Option<bool>,
    vignette: Option<bool>,
    keep_look: Option<bool>,
    strength: Option<AutoAdjustMode>,
    tone: Option<bool>,
    levels: Option<bool>,
    effects: Option<bool>,
}

impl<'de> Deserialize<'de> for AutoAdjustOptions {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let w = AutoAdjustOptionsWire::deserialize(d)?;
        let base = AutoAdjustOptions::default();
        Ok(AutoAdjustOptions {
            exposure: w.exposure.unwrap_or(base.exposure),
            brightness: w.brightness.or(w.exposure).unwrap_or(base.brightness),
            contrast: w.contrast.unwrap_or(base.contrast),
            highlights: w.highlights.or(w.tone).unwrap_or(base.highlights),
            shadows: w.shadows.or(w.tone).unwrap_or(base.shadows),
            whites: w.whites.or(w.levels).unwrap_or(base.whites),
            blacks: w.blacks.or(w.levels).unwrap_or(base.blacks),
            white_balance: w.white_balance.unwrap_or(base.white_balance),
            vibrance: w.vibrance.unwrap_or(base.vibrance),
            clarity: w.clarity.or(w.effects).unwrap_or(base.clarity),
            dehaze: w.dehaze.or(w.effects).unwrap_or(base.dehaze),
            vignette: w.vignette.or(w.effects).unwrap_or(base.vignette),
            keep_look: w.keep_look.unwrap_or(base.keep_look),
            strength: w.strength.unwrap_or(base.strength),
        })
    }
}

#[cfg(test)]
mod auto_adjust_options_tests {
    use super::*;

    #[test]
    fn grouped_switches_from_the_first_version_still_apply() {
        let saved = r#"{"exposure":true,"contrast":false,"tone":false,"levels":true,"whiteBalance":false,"vibrance":true,"effects":true,"keepLook":false}"#;
        let o: AutoAdjustOptions = serde_json::from_str(saved).unwrap();
        assert!(!o.contrast && !o.highlights && !o.shadows);
        assert!(o.whites && o.blacks);
        assert!(!o.white_balance && o.vibrance);
        assert!(o.clarity && o.dehaze && o.vignette);
        assert!(!o.keep_look);
    }

    #[test]
    fn per_slider_switches_win_over_grouped_ones_and_round_trip() {
        let saved = r#"{"tone":false,"shadows":true,"whites":false}"#;
        let o: AutoAdjustOptions = serde_json::from_str(saved).unwrap();
        assert!(!o.highlights && o.shadows && !o.whites && o.blacks);
        let again: AutoAdjustOptions = serde_json::from_str(&serde_json::to_string(&o).unwrap()).unwrap();
        assert_eq!(again, o);
        let empty: AutoAdjustOptions = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, AutoAdjustOptions::default());
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub film_panel_advanced: Option<bool>,
    #[serde(default)]
    pub display_negative_icon: Option<bool>,
    // Named film-stock conversion profiles (frontend-owned; params only, never
    // bounds). Kept opaque — the backend just round-trips it.
    #[serde(default)]
    pub negative_profiles: Option<serde_json::Value>,
    // Named user crop ratios (frontend-owned; {name, width, height}). Kept
    // opaque — the backend just round-trips it, same as negative_profiles.
    #[serde(default)]
    pub crop_presets: Option<serde_json::Value>,
    // Last aspect ratio + orientation used in the crop panel, so it reopens
    // where the user left it instead of snapping back to the image's own ratio.
    #[serde(default)]
    pub last_crop_ratio: Option<f64>,
    #[serde(default)]
    pub last_crop_prefer_portrait: Option<bool>,
    pub last_root_path: Option<String>,
    #[serde(default)]
    pub root_folders: Vec<String>,
    #[serde(default)]
    pub pinned_folders: Vec<String>,
    pub editor_preview_resolution: Option<u32>,
    #[serde(default)]
    pub small_thumbnail_resolution: Option<u32>,
    #[serde(default)]
    pub medium_thumbnail_resolution: Option<u32>,
    #[serde(default)]
    pub enable_zoom_hifi: Option<bool>,
    #[serde(default)]
    pub use_full_dpi_rendering: Option<bool>,
    #[serde(default)]
    pub high_res_zoom_multiplier: Option<f32>,
    #[serde(default)]
    pub enable_live_previews: Option<bool>,
    #[serde(default)]
    pub live_preview_quality: Option<String>,
    pub sort_criteria: Option<SortCriteria>,
    pub filter_criteria: Option<FilterCriteria>,
    pub theme: Option<String>,
    #[serde(default)]
    pub font_family: Option<String>,
    pub decorations: Option<bool>,
    #[serde(alias = "comfyuiAddress")]
    pub ai_connector_address: Option<String>,
    pub last_folder_state: Option<LastFolderState>,
    pub ui_visibility: Option<Value>,
    pub enable_ai_tagging: Option<bool>,
    pub tagging_thread_count: Option<u32>,
    #[serde(default = "default_tagging_shortcuts_option")]
    pub tagging_shortcuts: Option<Vec<String>>,
    #[serde(default)]
    pub custom_ai_tags: Option<Vec<String>>,
    #[serde(default)]
    pub ai_tag_count: Option<u32>,
    pub thumbnail_size: Option<String>,
    pub thumbnail_aspect_ratio: Option<String>,
    pub ai_provider: Option<String>,
    // AI Assistant chat: which LLM backend to talk to and how.
    #[serde(default)]
    pub assistant_provider: Option<String>, // "lmstudio" | "openai" | "anthropic"
    #[serde(default)]
    pub assistant_endpoint: Option<String>, // optional base URL override
    #[serde(default)]
    pub assistant_api_key: Option<String>,
    #[serde(default)]
    pub assistant_model: Option<String>,
    #[serde(default)]
    // Extended-thinking level for the assistant: "off" | "low" | "medium" | "high".
    pub assistant_thinking: Option<String>,
    // Developer mode: path to the user's RapidRAW checkout that the assistant
    // may modify (claudecode provider only).
    #[serde(default)]
    pub assistant_dev_repo_path: Option<String>,
    // "Edit in external editor": absolute path of the editor executable (blank
    // = OS default app) and the interchange format rendered for it.
    #[serde(default)]
    pub external_editor_path: Option<String>,
    #[serde(default)]
    pub external_editor_format: Option<String>,
    #[serde(default = "default_adjustment_visibility")]
    pub adjustment_visibility: HashMap<String, bool>,
    #[serde(default = "default_ai_gpu_acceleration")]
    pub ai_gpu_acceleration: bool,
    #[serde(default = "default_open_tree_sections")]
    pub open_tree_sections: Vec<String>,
    #[serde(default)]
    pub copy_paste_settings: CopyPasteSettings,
    #[serde(default)]
    pub raw_highlight_compression: Option<f32>,
    #[serde(default)]
    pub processing_backend: Option<String>,
    #[serde(default)]
    pub linux_gpu_optimization: Option<bool>,
    #[serde(default)]
    pub linux_gpu_optimization_migrated_v1: Option<bool>,
    #[serde(default)]
    pub library_view_mode: Option<String>,
    #[serde(default = "default_export_presets")]
    pub export_presets: Vec<ExportPreset>,
    #[serde(default)]
    pub my_lenses: Option<Vec<MyLens>>,
    #[serde(default)]
    pub enable_folder_image_counts: Option<bool>,
    #[serde(default)]
    pub display_edit_icon: Option<bool>,
    #[serde(default)]
    pub auto_profile: Option<AutoProfile>,
    #[serde(default)]
    pub auto_adjust: Option<AutoAdjustOptions>,
    #[serde(default = "default_linear_raw_mode")]
    pub linear_raw_mode: String,
    #[serde(default)]
    pub enable_xmp_sync: Option<bool>,
    #[serde(default)]
    pub create_xmp_if_missing: Option<bool>,
    #[serde(default)]
    pub is_waveform_visible: Option<bool>,
    #[serde(default)]
    pub waveform_height: Option<u32>,
    #[serde(default)]
    pub active_waveform_channel: Option<String>,
    #[serde(default)]
    pub use_wgpu_renderer: Option<bool>,
    #[serde(default)]
    pub editor_neutral_grey_bg: Option<bool>,
    // Editor canvas preset (see EDITOR_CANVAS_COLORS): "white" | "lightGrey" |
    // "midGrey" | "darkGrey" | "black". Unset falls back to the grey toggle.
    #[serde(default)]
    pub editor_canvas_color: Option<String>,
    #[serde(default)]
    pub canvas_input_mode: Option<String>,
    #[serde(default)]
    pub compare_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub white_balance_mode: Option<String>,
    #[serde(default)]
    pub zoom_speed_multiplier: Option<f32>,
    #[serde(default)]
    pub zoom_photo_to_pixel_click: Option<bool>,
    #[serde(default)]
    pub keybinds: HashMap<String, Vec<String>>,
    // Per-adjustment nudge step magnitude, keyed by adjustment key (e.g.
    // "brightness", "contrast"). Overrides the built-in default step for the
    // keyboard increase/decrease shortcuts; the direction stays fixed by the
    // shortcut. Empty map = use built-in defaults.
    #[serde(default)]
    pub adjustment_steps: HashMap<String, f64>,
    #[serde(default)]
    pub thumbnail_worker_threads: Option<u32>,
    #[serde(default)]
    pub image_cache_size: Option<u32>,
    #[serde(default)]
    pub ai_cache_limit_gb: Option<u32>,
    #[serde(default)]
    pub thumbnail_cache_limit_gb: Option<u32>,
    #[serde(default)]
    pub ai_maps_cache_limit_gb: Option<u32>,
    #[serde(default)]
    pub export_worker_threads: Option<u32>,
    #[serde(default)]
    pub export_memory_limit: Option<u32>,
    #[serde(default)]
    pub enable_baseline_exposure: bool,
    #[serde(default = "default_baseline_exposure")]
    pub baseline_exposure: f32,
    #[serde(default = "default_compensate_highlight_preservation")]
    pub compensate_highlight_preservation: bool,
    #[serde(default)]
    pub tonemapper_override_enabled: Option<bool>,
    #[serde(default)]
    pub default_raw_tonemapper: Option<String>,
    #[serde(default)]
    pub default_non_raw_tonemapper: Option<String>,
    #[serde(default)]
    pub enable_focus_mode: Option<bool>,
    #[serde(default)]
    pub wrap_image_navigation: Option<bool>,
    #[serde(default)]
    pub enable_tool_focus_mode: Option<bool>,
    #[serde(default)]
    pub folder_icons: Option<HashMap<String, String>>,
    #[serde(default)]
    pub raw_preprocessing_color_nr: Option<f32>,
    #[serde(default)]
    pub raw_preprocessing_sharpening: Option<f32>,
    #[serde(default)]
    pub apply_preprocessing_to_non_raws: Option<bool>,
    #[serde(default)]
    pub use_apple_raw9: Option<bool>,
    #[serde(default)]
    pub exif_overlay: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub folder_tree_sort: Option<FolderTreeSort>,
    #[serde(default)]
    pub library_display_mode: Option<String>,
    #[serde(default)]
    pub grouping: Option<String>,
    #[serde(default)]
    pub require_matching_exif: Option<bool>,
    #[serde(default)]
    pub group_edited_files: Option<bool>,
    #[serde(default, skip_serializing)] // legacy
    #[allow(dead_code)]
    pub group_associated_files: Option<bool>,
    #[serde(default, skip_serializing)] // legacy
    #[allow(dead_code)]
    pub group_preferred_type: Option<String>,
    #[serde(default)]
    pub always_decode_raw_thumbnails: Option<bool>,
    #[serde(default)]
    pub custom_aspect_ratios: Vec<CustomAspectRatio>,
    #[serde(default)]
    pub adjustment_layout: AdjustmentLayout,
    #[serde(default)]
    pub workspace: WorkspaceState,
    #[serde(default)]
    pub immich: crate::immich::ImmichSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            film_panel_advanced: None,
            display_negative_icon: None,
            negative_profiles: None,
            crop_presets: None,
            last_crop_ratio: None,
            last_crop_prefer_portrait: None,
            last_root_path: None,
            root_folders: Vec::new(),
            pinned_folders: Vec::new(),
            small_thumbnail_resolution: Some(480),
            medium_thumbnail_resolution: Some(1280),
            #[cfg(target_os = "android")]
            editor_preview_resolution: Some(1280),
            #[cfg(not(target_os = "android"))]
            editor_preview_resolution: Some(1920),
            enable_zoom_hifi: Some(true),
            use_full_dpi_rendering: Some(false),
            enable_live_previews: Some(true),
            live_preview_quality: Some("high".to_string()),
            sort_criteria: None,
            filter_criteria: None,
            theme: Some("dark".to_string()),
            font_family: None,
            decorations: Some(false),
            ai_connector_address: None,
            last_folder_state: None,
            ui_visibility: None,
            enable_ai_tagging: Some(false),
            tagging_thread_count: Some(3),
            tagging_shortcuts: default_tagging_shortcuts_option(),
            custom_ai_tags: Some(Vec::new()),
            ai_tag_count: Some(10),
            #[cfg(target_os = "android")]
            thumbnail_size: Some("small".to_string()),
            #[cfg(not(target_os = "android"))]
            thumbnail_size: Some("medium".to_string()),
            thumbnail_aspect_ratio: Some("contain".to_string()),
            ai_provider: Some("cpu".to_string()),
            assistant_provider: Some("lmstudio".to_string()),
            assistant_endpoint: None,
            assistant_api_key: None,
            assistant_model: None,
            assistant_thinking: None,
            assistant_dev_repo_path: None,
            external_editor_path: None,
            external_editor_format: None,
            adjustment_visibility: default_adjustment_visibility(),
            ai_gpu_acceleration: default_ai_gpu_acceleration(),
            open_tree_sections: default_open_tree_sections(),
            copy_paste_settings: CopyPasteSettings::default(),
            raw_highlight_compression: Some(2.5),
            processing_backend: Some("auto".to_string()),
            linux_gpu_optimization: Some(false),
            linux_gpu_optimization_migrated_v1: Some(true),
            library_view_mode: Some("flat".to_string()),
            export_presets: default_export_presets(),
            my_lenses: Some(Vec::new()),
            #[cfg(target_os = "android")]
            high_res_zoom_multiplier: Some(0.75),
            #[cfg(not(target_os = "android"))]
            high_res_zoom_multiplier: Some(1.0),
            enable_folder_image_counts: Some(false),
            display_edit_icon: Some(true),
            auto_profile: None,
            auto_adjust: None,
            linear_raw_mode: default_linear_raw_mode(),
            enable_xmp_sync: Some(true),
            create_xmp_if_missing: Some(false),
            is_waveform_visible: Some(false),
            waveform_height: Some(220),
            active_waveform_channel: Some("luma".to_string()),
            #[cfg(any(target_os = "linux", target_os = "android"))]
            use_wgpu_renderer: Some(false),
            #[cfg(not(any(target_os = "linux", target_os = "android")))]
            use_wgpu_renderer: Some(true),
            editor_neutral_grey_bg: Some(false),
            editor_canvas_color: None,
            canvas_input_mode: Some("mouse".to_string()),
            compare_mode: None,
            white_balance_mode: None,
            zoom_speed_multiplier: Some(1.0),
            zoom_photo_to_pixel_click: Some(false),
            keybinds: HashMap::new(),
            adjustment_steps: HashMap::new(),
            #[cfg(target_os = "android")]
            thumbnail_worker_threads: Some(2),
            #[cfg(not(target_os = "android"))]
            thumbnail_worker_threads: Some(4),
            #[cfg(target_os = "android")]
            image_cache_size: Some(2),
            #[cfg(not(target_os = "android"))]
            image_cache_size: Some(5),
            ai_cache_limit_gb: Some(default_ai_cache_limit_gb()),
            thumbnail_cache_limit_gb: Some(default_thumbnail_cache_limit_gb()),
            ai_maps_cache_limit_gb: Some(default_ai_maps_cache_limit_gb()),
            export_worker_threads: None,
            export_memory_limit: None,
            enable_baseline_exposure: false,
            baseline_exposure: default_baseline_exposure(),
            compensate_highlight_preservation: default_compensate_highlight_preservation(),
            tonemapper_override_enabled: Some(false),
            default_raw_tonemapper: Some("agx".to_string()),
            default_non_raw_tonemapper: Some("basic".to_string()),
            enable_focus_mode: Some(false),
            wrap_image_navigation: Some(true),
            enable_tool_focus_mode: Some(false),
            folder_icons: Some(HashMap::new()),
            raw_preprocessing_color_nr: Some(0.5),
            raw_preprocessing_sharpening: Some(0.35),
            apply_preprocessing_to_non_raws: Some(false),
            use_apple_raw9: Some(false),
            exif_overlay: Some("off".to_string()),
            language: Some("en".to_string()),
            folder_tree_sort: Some(FolderTreeSort::default()),
            library_display_mode: Some("grid".to_string()),
            grouping: Some("off".to_string()),
            require_matching_exif: Some(false),
            group_edited_files: Some(true),
            group_associated_files: Some(false),
            group_preferred_type: Some("raw".to_string()),
            always_decode_raw_thumbnails: Some(false),
            custom_aspect_ratios: Vec::new(),
            adjustment_layout: AdjustmentLayout::default(),
            workspace: WorkspaceState::default(),
            immich: crate::immich::ImmichSettings::default(),
        }
    }
}

pub fn get_settings_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let settings_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;

    if !settings_dir.exists() {
        fs::create_dir_all(&settings_dir).map_err(|e| e.to_string())?;
    }

    Ok(settings_dir.join("settings.json"))
}

#[tauri::command]
pub fn is_tethering_supported() -> bool {
    cfg!(feature = "tether-usb")
}

static SETTINGS_CACHE: LazyLock<Mutex<Option<AppSettings>>> = LazyLock::new(|| Mutex::new(None));

fn settings_cache() -> std::sync::MutexGuard<'static, Option<AppSettings>> {
    SETTINGS_CACHE.lock().unwrap_or_else(|e| e.into_inner())
}

#[tauri::command]
pub fn load_settings(app_handle: AppHandle) -> Result<AppSettings, String> {
    if let Some(settings) = settings_cache().as_ref() {
        return Ok(settings.clone());
    }

    let settings = read_settings_from_disk(&app_handle)?;
    *settings_cache() = Some(settings.clone());
    Ok(settings)
}

fn read_settings_from_disk(app_handle: &AppHandle) -> Result<AppSettings, String> {
    let path = get_settings_path(app_handle)?;

    let mut settings: AppSettings = if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        AppSettings::default()
    };

    let all_current_keys = all_available_adjustments();
    let default_included = default_included_adjustments();
    let mut settings_modified = false;

    if settings.root_folders.is_empty()
        && let Some(last) = &settings.last_root_path
    {
        settings.root_folders.push(last.clone());
        settings_modified = true;
    }

    #[cfg(target_os = "linux")]
    if !settings.linux_gpu_optimization_migrated_v1.unwrap_or(false) {
        if settings.linux_gpu_optimization == Some(true) {
            settings.linux_gpu_optimization = Some(false);
        }
        settings.linux_gpu_optimization_migrated_v1 = Some(true);
        settings_modified = true;
    }

    let is_first_migration = settings.copy_paste_settings.known_adjustments.is_empty();

    if is_first_migration {
        settings.copy_paste_settings.included_adjustments = default_included;
        settings.copy_paste_settings.known_adjustments = all_current_keys.clone();
        settings_modified = true;
    } else {
        let new_features: Vec<String> = all_current_keys
            .difference(&settings.copy_paste_settings.known_adjustments)
            .cloned()
            .collect();

        if !new_features.is_empty() {
            for feature in new_features {
                if default_included.contains(&feature) {
                    settings
                        .copy_paste_settings
                        .included_adjustments
                        .insert(feature.clone());
                }
                settings
                    .copy_paste_settings
                    .known_adjustments
                    .insert(feature);
            }
            settings_modified = true;
        }
    }

    if settings_modified && let Ok(json_string) = serde_json::to_string_pretty(&settings) {
        let _ = fs::write(&path, json_string);
    }

    Ok(settings)
}

#[tauri::command]
pub fn save_settings(settings: AppSettings, app_handle: AppHandle) -> Result<(), String> {
    let previous_gpu_acceleration = load_settings(app_handle.clone())
        .map(|previous| previous.ai_gpu_acceleration)
        .ok();
    let path = get_settings_path(&app_handle)?;
    let json_string = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(path, json_string).map_err(|e| e.to_string())?;
    *settings_cache() = None;
    crate::immich::apply_settings(&settings.immich);

    let state = app_handle.state::<AppState>();
    let cache_size = settings.image_cache_size.unwrap_or(5) as usize;
    state
        .decoded_image_cache
        .lock()
        .unwrap()
        .set_capacity(cache_size);
    crate::storage::apply_cache_limits(&settings);

    if previous_gpu_acceleration != Some(settings.ai_gpu_acceleration) {
        crate::ai_processing::reset_ai_sessions(&state.ai_state);
    }
    Ok(())
}
