use crate::app_settings::load_settings;
use image::{DynamicImage, GenericImageView, GrayImage, imageops};
use image_hasher::{HashAlg, HasherConfig};
use rawler::decoders::RawDecodeParams;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::{AppHandle, Emitter};

use crate::formats::is_raw_file;
use crate::image_loader;

/// Fast image load for culling analysis. For RAW files we want the embedded camera
/// preview (an already-rendered JPEG) instead of a full demosaic — orders of magnitude
/// faster and plenty for perceptual hashing + sharpness/exposure metrics.
///
/// The tiered strategy minimises I/O, which dominates when scanning a card full of
/// large raws:
///   1. `try_fast_embedded_preview` — lazily mmaps the file and reads ONLY the embedded
///      JPEG bytes (a few MB), instead of faulting the whole tens-of-MB raw off the card.
///   2. rawler's `extract_preview_pixels` — reads the whole file but still no demosaic;
///      covers raw layouts the fast path doesn't recognise.
///   3. full decode — last resort for non-raw or unreadable previews.
fn load_for_analysis(
    path: &str,
    settings: &crate::app_settings::AppSettings,
) -> Result<DynamicImage, String> {
    if is_raw_file(path) {
        if let Some(img) = try_fast_embedded_preview(path) {
            return Ok(img);
        }
        if let Ok(preview) = rawler::analyze::extract_preview_pixels(path, &RawDecodeParams::default()) {
            return Ok(preview);
        }
    }
    let file_bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    image_loader::load_base_image_from_bytes(&file_bytes, path, true, settings, None).map_err(|e| e.to_string())
}

/// Opportunistic, low-I/O embedded-preview extraction. Uses a lazy memory map (no
/// MAP_POPULATE) and reads only the bytes of the embedded JPEG, so analyzing a folder
/// of large raws doesn't read every full file off the (often slow) card. Returns None
/// for layouts it doesn't recognise so the caller can fall back.
fn try_fast_embedded_preview(path: &str) -> Option<DynamicImage> {
    let file = std::fs::File::open(path).ok()?;
    let mmap = unsafe { memmap2::Mmap::map(&file).ok()? };
    let buf: &[u8] = &mmap;

    // Fujifilm RAF: 16-byte magic, then a header storing the embedded full-size JPEG
    // preview's offset/length as big-endian u32s at byte 84/88.
    if buf.starts_with(b"FUJIFILMCCD-RAW") {
        let off = u32::from_be_bytes(buf.get(84..88)?.try_into().ok()?) as usize;
        let len = u32::from_be_bytes(buf.get(88..92)?.try_into().ok()?) as usize;
        let jpeg = buf.get(off..off.checked_add(len)?)?;
        return image::load_from_memory_with_format(jpeg, image::ImageFormat::Jpeg).ok();
    }

    None
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CullingSettings {
    pub similarity_threshold: u32,
    pub blur_threshold: f64,
    pub group_similar: bool,
    pub filter_blurry: bool,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImageAnalysisResult {
    pub path: String,
    pub quality_score: f64,
    pub sharpness_metric: f64,
    pub center_focus_metric: f64,
    pub exposure_metric: f64,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CullGroup {
    pub representative: ImageAnalysisResult,
    pub duplicates: Vec<ImageAnalysisResult>,
}

#[derive(Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct CullingSuggestions {
    pub similar_groups: Vec<CullGroup>,
    pub blurry_images: Vec<ImageAnalysisResult>,
    pub failed_paths: Vec<String>,
}

#[derive(Serialize, Clone)]
struct CullingProgress {
    current: usize,
    total: usize,
    stage: String,
}

#[derive(Clone)]
pub struct ImageAnalysisData {
    hash: image_hasher::ImageHash,
    result: ImageAnalysisResult,
}

const WEIGHT_SHARPNESS: f64 = 0.40;
const WEIGHT_CENTER_FOCUS: f64 = 0.35;
const WEIGHT_EXPOSURE: f64 = 0.25;

fn calculate_laplacian_variance(image: &GrayImage) -> f64 {
    let (width, height) = image.dimensions();
    if width < 3 || height < 3 {
        return 0.0;
    }

    let mut laplacian_values = Vec::with_capacity(((width - 2) * (height - 2)) as usize);
    let mut sum = 0.0;

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let p_center = image.get_pixel(x, y)[0] as i32;
            let p_north = image.get_pixel(x, y - 1)[0] as i32;
            let p_south = image.get_pixel(x, y + 1)[0] as i32;
            let p_west = image.get_pixel(x - 1, y)[0] as i32;
            let p_east = image.get_pixel(x + 1, y)[0] as i32;
            let conv_val = (p_north + p_south + p_west + p_east - 4 * p_center) as f64;
            laplacian_values.push(conv_val);
            sum += conv_val;
        }
    }

    if laplacian_values.is_empty() {
        return 0.0;
    }
    let mean = sum / laplacian_values.len() as f64;

    laplacian_values
        .iter()
        .map(|v| (v - mean).powi(2))
        .sum::<f64>()
        / laplacian_values.len() as f64
}

fn calculate_exposure_metric(image: &GrayImage) -> f64 {
    let histogram = imageproc::stats::histogram(image);
    let total_pixels = (image.width() * image.height()) as f64;
    if total_pixels == 0.0 {
        return 0.0;
    }

    let clip_threshold_dark = 5;
    let clip_threshold_bright = 250;

    let dark_pixels = histogram.channels[0][0..clip_threshold_dark]
        .iter()
        .sum::<u32>() as f64;
    let bright_pixels = histogram.channels[0][clip_threshold_bright..256]
        .iter()
        .sum::<u32>() as f64;

    let dark_clip_ratio = dark_pixels / total_pixels;
    let bright_clip_ratio = bright_pixels / total_pixels;

    let penalty = (dark_clip_ratio * 5.0) + (bright_clip_ratio * 5.0);

    (1.0f64 - penalty).max(0.0)
}

fn analyze_image(
    path: &str,
    hasher: &image_hasher::Hasher,
    settings: &crate::app_settings::AppSettings,
) -> Result<ImageAnalysisData, String> {
    const ANALYSIS_DIM: u32 = 720; // FIXME: How should we calculate good focus if it's downscaled?!?
    let img = load_for_analysis(path, settings)?;

    let (width, height) = img.dimensions();
    let thumbnail = img.thumbnail(ANALYSIS_DIM, ANALYSIS_DIM);
    let gray_thumbnail = thumbnail.to_luma8();

    let sharpness_metric = calculate_laplacian_variance(&gray_thumbnail);
    let exposure_metric = calculate_exposure_metric(&gray_thumbnail);

    let (thumb_w, thumb_h) = gray_thumbnail.dimensions();
    let center_crop = imageops::crop_imm(
        &gray_thumbnail,
        thumb_w / 4,
        thumb_h / 4,
        thumb_w / 2,
        thumb_h / 2,
    )
    .to_image();
    let center_focus_metric = calculate_laplacian_variance(&center_crop);

    let normalized_sharpness = ((sharpness_metric + 1.0).log10() / 3.5).min(1.0);
    let normalized_center_focus = ((center_focus_metric + 1.0).log10() / 3.5).min(1.0);

    let quality_score = (normalized_sharpness * WEIGHT_SHARPNESS)
        + (normalized_center_focus * WEIGHT_CENTER_FOCUS)
        + (exposure_metric * WEIGHT_EXPOSURE);

    let hash = hasher.hash_image(&thumbnail);

    Ok(ImageAnalysisData {
        hash,
        result: ImageAnalysisResult {
            path: path.to_string(),
            quality_score,
            sharpness_metric,
            center_focus_metric,
            exposure_metric,
            width,
            height,
        },
    })
}

#[tauri::command]
pub async fn cull_images(
    paths: Vec<String>,
    settings: CullingSettings,
    app_handle: AppHandle,
) -> Result<CullingSuggestions, String> {
    run_culling(paths, settings, app_handle, "culling").await
}

/// Core culling routine shared by the post-import `cull_images` command and the
/// pre-import SD-card flow. The `channel` prefix isolates progress events
/// (`{channel}-start` / `-progress` / `-complete`) so different callers don't
/// trigger each other's UI (e.g. the import view must not pop the CullingModal).
/// Analyze each path in parallel (decode embedded preview, perceptual hash, quality
/// metrics). Emits `{channel}-start`/`-progress`. Returns the successful analyses plus
/// the paths that failed. This is the slow, I/O-heavy step.
pub async fn analyze_paths(
    paths: Vec<String>,
    app_handle: AppHandle,
    channel: &str,
) -> Result<(Vec<ImageAnalysisData>, Vec<String>), String> {
    if paths.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let app_settings = load_settings(app_handle.clone()).unwrap_or_default();
    let total_count = paths.len();
    let completed_count = Arc::new(AtomicUsize::new(0));
    let _ = app_handle.emit(&format!("{channel}-start"), total_count);

    let hasher = HasherConfig::new()
        .hash_alg(HashAlg::DoubleGradient)
        .hash_size(16, 16)
        .to_hasher();

    let analysis_results: Vec<Result<ImageAnalysisData, (String, String)>> = paths
        .par_iter()
        .map(|path| {
            let completed = completed_count.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = app_handle.emit(
                &format!("{channel}-progress"),
                CullingProgress {
                    current: completed,
                    total: total_count,
                    stage: "Analyzing images...".to_string(),
                },
            );
            analyze_image(path, &hasher, &app_settings).map_err(|e| (path.to_string(), e))
        })
        .collect();

    let mut successful_analyses = Vec::new();
    let mut failed_paths = Vec::new();
    for res in analysis_results {
        match res {
            Ok(data) => successful_analyses.push(data),
            Err((path, error)) => {
                eprintln!("Failed to analyze image {}: {}", path, error);
                failed_paths.push(path);
            }
        }
    }

    Ok((successful_analyses, failed_paths))
}

/// Group already-analyzed images by perceptual-hash similarity (and optionally flag
/// blurry ones). Cheap — no I/O — so it can be re-run live as the similarity threshold
/// changes.
pub fn group_analyses(
    successful_analyses: &[ImageAnalysisData],
    failed_paths: Vec<String>,
    settings: &CullingSettings,
) -> CullingSuggestions {
    let mut suggestions = CullingSuggestions {
        failed_paths,
        ..Default::default()
    };
    let mut processed_indices = vec![false; successful_analyses.len()];

    if settings.group_similar {
        for i in 0..successful_analyses.len() {
            if processed_indices[i] {
                continue;
            }

            let mut current_group_indices = vec![];
            let mut queue = VecDeque::new();
            processed_indices[i] = true;
            current_group_indices.push(i);
            queue.push_back(i);

            while let Some(current_idx) = queue.pop_front() {
                for j in (current_idx + 1)..successful_analyses.len() {
                    if processed_indices[j] {
                        continue;
                    }
                    let dist = successful_analyses[current_idx].hash.dist(&successful_analyses[j].hash);
                    if dist <= settings.similarity_threshold {
                        processed_indices[j] = true;
                        current_group_indices.push(j);
                        queue.push_back(j);
                    }
                }
            }

            if current_group_indices.len() > 1 {
                current_group_indices.sort_by(|&a, &b| {
                    successful_analyses[b]
                        .result
                        .quality_score
                        .partial_cmp(&successful_analyses[a].result.quality_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                let representative_idx = current_group_indices[0];
                let duplicate_indices = &current_group_indices[1..];
                suggestions.similar_groups.push(CullGroup {
                    representative: successful_analyses[representative_idx].result.clone(),
                    duplicates: duplicate_indices
                        .iter()
                        .map(|&idx| successful_analyses[idx].result.clone())
                        .collect(),
                });
            }
        }
    }

    if settings.filter_blurry {
        for i in 0..successful_analyses.len() {
            if !processed_indices[i] {
                let item = &successful_analyses[i];
                if item.result.sharpness_metric < settings.blur_threshold {
                    suggestions.blurry_images.push(item.result.clone());
                }
            }
        }
        suggestions.blurry_images.sort_by(|a, b| {
            a.sharpness_metric
                .partial_cmp(&b.sharpness_metric)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    suggestions
}

pub async fn run_culling(
    paths: Vec<String>,
    settings: CullingSettings,
    app_handle: AppHandle,
    channel: &str,
) -> Result<CullingSuggestions, String> {
    if paths.is_empty() {
        return Ok(CullingSuggestions::default());
    }
    let (analyses, failed) = analyze_paths(paths, app_handle.clone(), channel).await?;
    let total = analyses.len() + failed.len();
    let _ = app_handle.emit(
        &format!("{channel}-progress"),
        CullingProgress { current: total, total, stage: "Grouping similar images...".to_string() },
    );
    let suggestions = group_analyses(&analyses, failed, &settings);
    let _ = app_handle.emit(&format!("{channel}-complete"), &suggestions);
    Ok(suggestions)
}
