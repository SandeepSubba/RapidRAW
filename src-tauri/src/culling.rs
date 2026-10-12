use crate::app_settings::load_settings;
use image::{DynamicImage, GenericImageView, GrayImage, imageops};
use image_hasher::{HashAlg, HasherConfig};
use rawler::decoders::RawDecodeParams;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;
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
    if crate::file_management::is_cloud_placeholder(Path::new(path)) {
        return Err(format!("'{}' is stored in iCloud and not downloaded", path));
    }
    if is_raw_file(path) {
        if let Some(img) = try_fast_embedded_preview(path) {
            return Ok(img);
        }
        if let Ok(preview) =
            rawler::analyze::extract_preview_pixels(path, &RawDecodeParams::default())
        {
            return Ok(preview);
        }
    }
    let file_bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    image_loader::load_base_image_from_bytes(&file_bytes, path, true, settings, None)
        .map_err(|e| e.to_string())
}

/// Cheap RAW preview: the camera's embedded full-size JPEG (RAF fast path) or rawler's
/// extracted preview. Never demosaics the raw — that's the point. Returns None when neither
/// is available so callers can fall back to a full decode. Used both for import analysis and
/// for thumbnail generation, where a folder of large raws was OOMing when every thumbnail
/// triggered a full-resolution X-Trans decode.
/// Largest embedded JPEG preview found by walking the TIFF IFD tree directly.
/// Works even when rawler has no decoder for the file (stripped make/model) or
/// cannot decode its raw data (e.g. DNG 1.7 lossy 8-bit JPEG raws): previews are
/// plain JPEG streams referenced by standard tags, independent of the raw payload.
/// JPEG preview byte ranges found in the TIFF structure, largest first. The
/// caller tries them in order: the raw sensor track is lossless JPEG (SOF3),
/// which ordinary JPEG decoders reject, so it falls through to the next
/// candidate rather than poisoning the pick.
fn tiff_jpeg_preview_candidates(buf: &[u8]) -> Vec<&[u8]> {
    let le = match buf.get(..4) {
        Some([0x49, 0x49, 0x2A, 0x00]) => true,
        Some([0x4D, 0x4D, 0x00, 0x2A]) => false,
        _ => return Vec::new(),
    };
    let rd16 = |o: usize| -> Option<u64> {
        let b: [u8; 2] = buf.get(o..o + 2)?.try_into().ok()?;
        Some(if le {
            u16::from_le_bytes(b)
        } else {
            u16::from_be_bytes(b)
        } as u64)
    };
    let rd32 = |o: usize| -> Option<u64> {
        let b: [u8; 4] = buf.get(o..o + 4)?.try_into().ok()?;
        Some(if le {
            u32::from_le_bytes(b)
        } else {
            u32::from_be_bytes(b)
        } as u64)
    };

    let mut found: Vec<(u64, u64)> = Vec::new();
    let Some(first_ifd) = rd32(4) else {
        return Vec::new();
    };
    let mut queue: Vec<u64> = vec![first_ifd];
    let mut seen = std::collections::HashSet::new();

    while let Some(ifd) = queue.pop() {
        if !seen.insert(ifd) || seen.len() > 64 {
            continue;
        }
        let Some(n) = rd16(ifd as usize) else {
            continue;
        };
        let mut subfile: u64 = u64::MAX; // absent
        let mut compression: u64 = 0;
        let mut strip: Option<(u64, u64)> = None; // 273/279, single strip
        let mut old_jpeg: Option<(u64, u64)> = None; // 513/514
        for i in 0..n {
            let e = ifd as usize + 2 + (i as usize) * 12;
            let (Some(tag), Some(count), Some(val)) = (rd16(e), rd32(e + 4), rd32(e + 8)) else {
                continue;
            };
            match tag {
                254 => subfile = val,
                259 => compression = val,
                273 if count == 1 => strip = Some((val, strip.map_or(0, |s| s.1))),
                279 if count == 1 => strip = strip.map(|s| (s.0, val)).or(Some((0, val))),
                513 => old_jpeg = Some((val, old_jpeg.map_or(0, |s| s.1))),
                514 => old_jpeg = old_jpeg.map(|s| (s.0, val)).or(Some((0, val))),
                330 => {
                    // SubIFD pointers: inline if count == 1, else an offset array
                    if count == 1 {
                        queue.push(val);
                    } else {
                        for j in 0..count.min(8) {
                            if let Some(p) = rd32(val as usize + (j as usize) * 4) {
                                queue.push(p);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        // Strip-based JPEG candidates: reduced-resolution IFDs (254 == 1) and
        // IFDs with no subfile type at all — Canon CR2 stores its full-size
        // preview in IFD0 without tag 254, which a strict `== 1` check skipped,
        // leaving only the 160px thumbnail (seen as unreadable 35px inspect
        // crops). Explicit subfile 0 (the marked main raw image) stays out.
        if matches!(compression, 6 | 7)
            && subfile != 0
            && let Some((off, len)) = strip
        {
            found.push((off, len));
        }
        if let Some((off, len)) = old_jpeg {
            found.push((off, len));
        }
        if let Some(next) = rd32(ifd as usize + 2 + (n as usize) * 12)
            && next != 0
        {
            queue.push(next);
        }
    }

    found.sort_by(|a, b| b.1.cmp(&a.1));
    found
        .into_iter()
        .filter_map(|(off, len)| {
            let end = off.checked_add(len)?;
            buf.get(off as usize..end as usize)
        })
        // A JPEG stream starts with SOI; this skips uncompressed RGB strips.
        .filter(|bytes| bytes.starts_with(&[0xFF, 0xD8]))
        .collect()
}

pub fn largest_tiff_jpeg_preview(buf: &[u8]) -> Option<DynamicImage> {
    tiff_jpeg_preview_candidates(buf)
        .into_iter()
        .find_map(|bytes| image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg).ok())
}

/// Decode a JPEG at the smallest IDCT scale that still covers `max_dim`: a
/// 720px thumbnail of a 22MP embedded preview becomes a 1/4- or 1/8-scale
/// decode instead of a full one. Returns None on anything unusual (CMYK,
/// decode error) so the caller can fall back to the full-quality path.
fn decode_jpeg_scaled(bytes: &[u8], max_dim: u32) -> Option<DynamicImage> {
    let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(bytes));
    let side = max_dim.min(u16::MAX as u32) as u16;
    decoder.scale(side, side).ok()?;
    let pixels = decoder.decode().ok()?;
    let info = decoder.info()?;
    let (w, h) = (info.width as u32, info.height as u32);
    match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => {
            image::RgbImage::from_raw(w, h, pixels).map(DynamicImage::ImageRgb8)
        }
        jpeg_decoder::PixelFormat::L8 => {
            image::GrayImage::from_raw(w, h, pixels).map(DynamicImage::ImageLuma8)
        }
        _ => None,
    }
}

#[cfg(test)]
mod preview_candidate_tests {
    use super::*;

    fn entry(tag: u16, typ: u16, count: u32, value: u32) -> Vec<u8> {
        let mut e = Vec::new();
        e.extend(tag.to_le_bytes());
        e.extend(typ.to_le_bytes());
        e.extend(count.to_le_bytes());
        e.extend(value.to_le_bytes());
        e
    }

    fn jpeg_bytes(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb([120, 60, 30]));
        let mut buf = Vec::new();
        let mut cur = std::io::Cursor::new(&mut buf);
        let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cur, 80);
        image::DynamicImage::ImageRgb8(img).write_with_encoder(enc).unwrap();
        buf
    }

    /// Canon-CR2-shaped TIFF: IFD0 carries the full preview as a strip JPEG
    /// WITHOUT tag 254 (the layout the old `subfile == 1` guard rejected),
    /// IFD1 the 513/514 thumbnail, IFD2 a bigger FFD8+SOF3 blob standing in
    /// for the undecodable lossless-JPEG raw track.
    fn build_cr2_like() -> (Vec<u8>, (u32, u32)) {
        let preview = jpeg_bytes(64, 48);
        let thumb = jpeg_bytes(16, 12);
        let mut raw_track = vec![0xFF, 0xD8, 0xFF, 0xC3];
        raw_track.extend(std::iter::repeat(0xAB).take(preview.len() + thumb.len() + 1000));

        let ifd0_off: u32 = 8;
        let ifd0_len: u32 = 2 + 12 * 3 + 4;
        let ifd1_off = ifd0_off + ifd0_len;
        let ifd1_len: u32 = 2 + 12 * 2 + 4;
        let ifd2_off = ifd1_off + ifd1_len;
        let ifd2_len: u32 = 2 + 12 * 3 + 4;
        let data0 = ifd2_off + ifd2_len;
        let data1 = data0 + preview.len() as u32;
        let data2 = data1 + thumb.len() as u32;

        let mut buf = vec![0x49, 0x49, 0x2A, 0x00];
        buf.extend(ifd0_off.to_le_bytes());
        buf.extend(3u16.to_le_bytes());
        buf.extend(entry(259, 3, 1, 6));
        buf.extend(entry(273, 4, 1, data0));
        buf.extend(entry(279, 4, 1, preview.len() as u32));
        buf.extend(ifd1_off.to_le_bytes());
        buf.extend(2u16.to_le_bytes());
        buf.extend(entry(513, 4, 1, data1));
        buf.extend(entry(514, 4, 1, thumb.len() as u32));
        buf.extend(ifd2_off.to_le_bytes());
        buf.extend(3u16.to_le_bytes());
        buf.extend(entry(259, 3, 1, 6));
        buf.extend(entry(273, 4, 1, data2));
        buf.extend(entry(279, 4, 1, raw_track.len() as u32));
        buf.extend(0u32.to_le_bytes());
        assert_eq!(buf.len() as u32, data0);
        buf.extend(preview);
        buf.extend(thumb);
        buf.extend(raw_track);
        (buf, (64, 48))
    }

    #[test]
    fn canon_layout_full_preview_wins_over_thumb_and_raw_track() {
        let (buf, dims) = build_cr2_like();
        let img = largest_tiff_jpeg_preview(&buf).expect("a decodable preview");
        assert_eq!((img.width(), img.height()), dims);
    }

    #[test]
    fn candidates_are_largest_first_and_all_start_with_soi() {
        let (buf, _) = build_cr2_like();
        let c = tiff_jpeg_preview_candidates(&buf);
        assert_eq!(c.len(), 3);
        assert!(c.windows(2).all(|w| w[0].len() >= w[1].len()));
        assert!(c.iter().all(|b| b.starts_with(&[0xFF, 0xD8])));
    }
}

pub fn fast_raw_preview(path: &str) -> Option<DynamicImage> {
    fast_raw_preview_scaled(path, None)
}

/// With `max_dim` set, the embedded preview is decoded at reduced scale — enough
/// for a thumbnail, far cheaper than a full-resolution JPEG decode. Without it,
/// behaviour is unchanged (full-quality preview for viewing).
pub fn fast_raw_preview_scaled(path: &str, max_dim: Option<u32>) -> Option<DynamicImage> {
    // One lazy mmap; only the embedded JPEG bytes are actually read, so a
    // folder of large raws on a slow card does not pull every full file.
    let mapped = std::fs::File::open(path)
        .ok()
        .and_then(|f| unsafe { memmap2::Mmap::map(&f).ok() });
    let candidates: Vec<&[u8]> = mapped
        .as_ref()
        .map(|buf| match raf_preview_bytes(buf) {
            Some(raf) => vec![raf],
            None => tiff_jpeg_preview_candidates(buf),
        })
        .unwrap_or_default();
    let decoded = candidates.into_iter().find_map(|bytes| {
        max_dim
            .and_then(|dim| decode_jpeg_scaled(bytes, dim))
            .or_else(|| image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg).ok())
    });
    let img = match decoded {
        Some(img) => img,
        None => {
            rawler::analyze::extract_preview_pixels(path, &RawDecodeParams::default()).ok()?
        }
    };
    // Embedded previews are stored unrotated; apply the file's EXIF orientation like
    // the full-decode path does, or portrait raws render sideways in the library grid.
    let orientation = std::fs::File::open(path).ok().and_then(|f| {
        let mut r = std::io::BufReader::new(f);
        exif::Reader::new()
            .read_from_container(&mut r)
            .ok()?
            .get_field(exif::Tag::Orientation, exif::In::PRIMARY)?
            .value
            .get_uint(0)
    });
    Some(match orientation {
        Some(o) if o > 1 => {
            crate::image_processing::apply_orientation(img, rawler::Orientation::from_u16(o as u16))
        }
        _ => img,
    })
}

/// Opportunistic, low-I/O embedded-preview extraction for non-TIFF layouts.
/// Lazily mmaps and reads only the embedded JPEG bytes.
fn try_fast_embedded_preview(path: &str) -> Option<DynamicImage> {
    let file = std::fs::File::open(path).ok()?;
    let mmap = unsafe { memmap2::Mmap::map(&file).ok()? };
    let jpeg = raf_preview_bytes(&mmap)?;
    image::load_from_memory_with_format(jpeg, image::ImageFormat::Jpeg).ok()
}

/// Fujifilm RAF: 16-byte magic, then a header storing the embedded full-size
/// JPEG preview offset/length as big-endian u32s at byte 84/88.
fn raf_preview_bytes(buf: &[u8]) -> Option<&[u8]> {
    if !buf.starts_with(b"FUJIFILMCCD-RAW") {
        return None;
    }
    let off = u32::from_be_bytes(buf.get(84..88)?.try_into().ok()?) as usize;
    let len = u32::from_be_bytes(buf.get(88..92)?.try_into().ok()?) as usize;
    buf.get(off..off.checked_add(len)?)
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
    color_hist: ColorHist,
    result: ImageAnalysisResult,
}

/// Bits in the perceptual hash: DoubleGradient at 16x16 hashes a 9x9 image as
/// 72 row + 72 column gradients. Must track the `HasherConfig` in `analyze_paths`.
const HASH_BITS: u32 = 144;

const COLOR_HIST_SIDE: u32 = 16;
const COLOR_HIST_SAMPLES: u32 = COLOR_HIST_SIDE * COLOR_HIST_SIDE;
type ColorHist = [u16; 64];

/// Coarse colour signature: a 4x4x4 RGB histogram of the photo shrunk to 16x16.
/// Unlike the gradient hash it barely moves when the same subjects shift pose
/// or the framing drifts, which is what separates "another take of this shot"
/// from "a different shot seconds later".
fn color_histogram(thumbnail: &DynamicImage) -> ColorHist {
    let small = thumbnail
        .resize_exact(COLOR_HIST_SIDE, COLOR_HIST_SIDE, imageops::FilterType::Triangle)
        .to_rgb8();
    let mut hist = [0u16; 64];
    for px in small.pixels() {
        let [r, g, b] = px.0;
        hist[(r >> 6) as usize * 16 + (g >> 6) as usize * 4 + (b >> 6) as usize] += 1;
    }
    hist
}

/// Histogram-intersection distance: 0 = same palette, `COLOR_HIST_SAMPLES` = nothing shared.
fn color_hist_distance(a: &ColorHist, b: &ColorHist) -> u32 {
    COLOR_HIST_SAMPLES - a.iter().zip(b).map(|(x, y)| u32::from(*x.min(y))).sum::<u32>()
}

impl ImageAnalysisData {
    pub fn result_path(&self) -> String {
        self.result.path.clone()
    }
    pub fn set_scores(&mut self, quality: f64, sharpness: f64, center_focus: f64, exposure: f64) {
        self.result.quality_score = quality;
        self.result.sharpness_metric = sharpness;
        self.result.center_focus_metric = center_focus;
        self.result.exposure_metric = exposure;
    }
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

const ANALYSIS_DIM: u32 = 720; // FIXME: How should we calculate good focus if it's downscaled?!?

/// Quality metrics from an already-downscaled thumbnail: (quality, sharpness, center
/// focus, exposure). This is the "scoring" work — separable from the grouping hash.
fn quality_metrics(thumbnail: &image::DynamicImage) -> (f64, f64, f64, f64) {
    let gray = thumbnail.to_luma8();
    let sharpness_metric = calculate_laplacian_variance(&gray);
    let exposure_metric = calculate_exposure_metric(&gray);

    let (tw, th) = gray.dimensions();
    let center_crop = imageops::crop_imm(&gray, tw / 4, th / 4, tw / 2, th / 2).to_image();
    let center_focus_metric = calculate_laplacian_variance(&center_crop);

    let normalized_sharpness = ((sharpness_metric + 1.0).log10() / 3.5).min(1.0);
    let normalized_center_focus = ((center_focus_metric + 1.0).log10() / 3.5).min(1.0);
    let quality_score = (normalized_sharpness * WEIGHT_SHARPNESS)
        + (normalized_center_focus * WEIGHT_CENTER_FOCUS)
        + (exposure_metric * WEIGHT_EXPOSURE);

    (
        quality_score,
        sharpness_metric,
        center_focus_metric,
        exposure_metric,
    )
}

/// Decode + perceptual hash, optionally computing quality metrics. When `compute_score`
/// is false (the grouping pass) the quality fields are left at 0 — they're filled in
/// later by the separate scoring step.
fn analyze_image(
    path: &str,
    hasher: &image_hasher::Hasher,
    settings: &crate::app_settings::AppSettings,
    compute_score: bool,
) -> Result<ImageAnalysisData, String> {
    let img = load_for_analysis(path, settings)?;
    let (width, height) = img.dimensions();
    let thumbnail = img.thumbnail(ANALYSIS_DIM, ANALYSIS_DIM);
    let hash = hasher.hash_image(&thumbnail);
    let color_hist = color_histogram(&thumbnail);

    let (quality_score, sharpness_metric, center_focus_metric, exposure_metric) = if compute_score {
        quality_metrics(&thumbnail)
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };

    Ok(ImageAnalysisData {
        hash,
        color_hist,
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

/// Score one image (decode + quality metrics, no hash). Used by the standalone scoring
/// step that fills in scores after grouping.
pub fn score_image(
    path: &str,
    settings: &crate::app_settings::AppSettings,
) -> Result<(f64, f64, f64, f64), String> {
    let img = load_for_analysis(path, settings)?;
    let thumbnail = img.thumbnail(ANALYSIS_DIM, ANALYSIS_DIM);
    Ok(quality_metrics(&thumbnail))
}

/// Decode + downscale to the analysis thumbnail (the same image the scoring pass uses).
/// Exposed so other scorers — e.g. the CLIP "eyes open/closed" check in `sd_import` —
/// can reuse one decode instead of re-loading the raw.
pub fn load_analysis_thumbnail(
    path: &str,
    settings: &crate::app_settings::AppSettings,
) -> Result<image::DynamicImage, String> {
    let img = load_for_analysis(path, settings)?;
    Ok(img.thumbnail(ANALYSIS_DIM, ANALYSIS_DIM))
}

/// Normalized sharpness (Laplacian variance) of an image in [0, 1] — the same curve the
/// technical score uses for whole-image sharpness. Exposed so the face pass can measure
/// how sharp each cropped face is (the most relevant focus cue for people bursts).
pub fn normalized_sharpness_of(image: &image::DynamicImage) -> f32 {
    let gray = image.to_luma8();
    let var = calculate_laplacian_variance(&gray);
    (((var + 1.0).log10() / 3.5).min(1.0)) as f32
}

/// Sharpness of the *sharpest region* of the image (max over a 3×3 grid of the Laplacian
/// variance), normalized to [0,1]. A position-agnostic "is the subject in focus somewhere"
/// cue — better than center-focus for off-centre subjects (landscapes, product, food, still
/// life) where the point of interest isn't in the middle.
pub fn best_region_sharpness(image: &image::DynamicImage) -> f32 {
    let gray = image.to_luma8();
    let (w, h) = gray.dimensions();
    if w < 9 || h < 9 {
        return normalized_sharpness_of(image);
    }
    let (gx, gy) = (3u32, 3u32);
    let (cw, ch) = (w / gx, h / gy);
    let mut best = 0.0f64;
    for ry in 0..gy {
        for rx in 0..gx {
            let cell = imageops::crop_imm(&gray, rx * cw, ry * ch, cw, ch).to_image();
            best = best.max(calculate_laplacian_variance(&cell));
        }
    }
    (((best + 1.0).log10() / 3.5).min(1.0)) as f32
}

const FACE_DIM: u32 = 1920; // large enough that cropped faces keep detail for CLIP gaze/eyes

/// Decode + downscale to a larger thumbnail for the face/eye pass. Faces in a group shot
/// are small, so we keep more resolution here than the technical-scoring thumbnail.
pub fn load_face_image(
    path: &str,
    settings: &crate::app_settings::AppSettings,
) -> Result<image::DynamicImage, String> {
    let img = load_for_analysis(path, settings)?;
    Ok(img.thumbnail(FACE_DIM, FACE_DIM))
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
    compute_score: bool,
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
            analyze_image(path, &hasher, &app_settings, compute_score)
                .map_err(|e| (path.to_string(), e))
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
                    let dist = successful_analyses[current_idx]
                        .hash
                        .dist(&successful_analyses[j].hash);
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

fn flush_time_burst(
    analyses: &[ImageAnalysisData],
    burst: &mut Vec<usize>,
    suggestions: &mut CullingSuggestions,
) {
    if burst.len() > 1 {
        burst.sort_by(|&a, &b| {
            analyses[b]
                .result
                .quality_score
                .partial_cmp(&analyses[a].result.quality_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        suggestions.similar_groups.push(CullGroup {
            representative: analyses[burst[0]].result.clone(),
            duplicates: burst[1..]
                .iter()
                .map(|&idx| analyses[idx].result.clone())
                .collect(),
        });
    }
    burst.clear();
}

/// Group photos into **bursts captured close together in time** (true burst detection),
/// independent of visual similarity. `times` maps result path -> capture epoch seconds; a new
/// burst starts whenever the gap to the previous photo exceeds `max_gap_seconds`. Each burst
/// of >=2 becomes a group (representative = highest quality). Photos without a timestamp are
/// left ungrouped (they fall into the frontend's "Other images").
pub fn group_by_time(
    successful_analyses: &[ImageAnalysisData],
    failed_paths: Vec<String>,
    times: &std::collections::HashMap<String, i64>,
    max_gap_seconds: i64,
) -> CullingSuggestions {
    let mut suggestions = CullingSuggestions {
        failed_paths,
        ..Default::default()
    };

    let mut timed: Vec<(i64, usize)> = successful_analyses
        .iter()
        .enumerate()
        .filter_map(|(i, a)| times.get(&a.result.path).map(|&t| (t, i)))
        .collect();
    timed.sort_by_key(|&(t, _)| t);

    let mut burst: Vec<usize> = Vec::new();
    let mut prev_t: Option<i64> = None;
    for (t, idx) in timed {
        if let Some(pt) = prev_t {
            if t - pt > max_gap_seconds {
                flush_time_burst(successful_analyses, &mut burst, &mut suggestions);
            }
        }
        burst.push(idx);
        prev_t = Some(t);
    }
    flush_time_burst(successful_analyses, &mut burst, &mut suggestions);

    suggestions
}

/// How far ahead in capture order a frame looks for a match, so one stray frame
/// between two takes of the same composition doesn't split them.
const SCENE_LOOKAHEAD: usize = 3;
/// Frames shot back-to-back are far likelier to be the same scene, so the match
/// is this much looser at a gap of zero, fading to no bonus at
/// `SCENE_TIME_BONUS_FADE_SECONDS`.
const SCENE_TIME_BONUS: f64 = 0.5;
const SCENE_TIME_BONUS_FADE_SECONDS: f64 = 10.0;

/// Group photos into **scenes**: runs of frames shot close together that look alike.
///
/// Comparing every photo with every other one (`group_analyses`) has to be strict
/// to keep unrelated shots apart, and so misses most of a burst. Here a frame is
/// only compared with the next few frames in capture order, which lets the match
/// be much looser: two frames link when their colour signatures *or* their
/// perceptual hashes are within the threshold. `settings.similarity_threshold` is
/// in hash bits; the colour signature gets the same fraction of its own range.
/// Capture time counts twice: frames more than `max_gap_seconds` apart are never
/// linked directly, and the closer two frames are the looser the match
/// (`SCENE_TIME_BONUS`). Linked frames chain, so a long sequence in one setup
/// becomes one group.
pub fn group_by_scene(
    analyses: &[ImageAnalysisData],
    failed_paths: Vec<String>,
    times: &std::collections::HashMap<String, i64>,
    settings: &CullingSettings,
    max_gap_seconds: i64,
) -> CullingSuggestions {
    let mut suggestions = CullingSuggestions {
        failed_paths,
        ..Default::default()
    };
    let hash_threshold = settings.similarity_threshold;
    let hist_threshold = hash_threshold * COLOR_HIST_SAMPLES / HASH_BITS;

    let time_of = |i: usize| times.get(&analyses[i].result.path).copied();
    let mut order: Vec<usize> = (0..analyses.len()).collect();
    order.sort_by(|&a, &b| {
        time_of(a)
            .cmp(&time_of(b))
            .then_with(|| analyses[a].result.path.cmp(&analyses[b].result.path))
    });

    fn root(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    let mut parent: Vec<usize> = (0..analyses.len()).collect();
    for (k, &a) in order.iter().enumerate() {
        for &b in order.iter().skip(k + 1).take(SCENE_LOOKAHEAD) {
            // No timestamps (shouldn't happen for an analysed scan): no cut-off, no bonus.
            let gap = time_of(a).zip(time_of(b)).map(|(ta, tb)| tb - ta);
            if gap.is_some_and(|g| g > max_gap_seconds) {
                break;
            }
            let closeness = gap.map_or(0.0, |g| (1.0 - g as f64 / SCENE_TIME_BONUS_FADE_SECONDS).max(0.0));
            let leniency = 1.0 + SCENE_TIME_BONUS * closeness;
            let within = |distance: u32, threshold: u32| f64::from(distance) <= f64::from(threshold) * leniency;
            if within(color_hist_distance(&analyses[a].color_hist, &analyses[b].color_hist), hist_threshold)
                || within(analyses[a].hash.dist(&analyses[b].hash), hash_threshold)
            {
                let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
                parent[ra] = rb;
            }
        }
    }

    // Emit groups in capture order of their first frame.
    let mut slot_of_root = std::collections::HashMap::new();
    let mut scenes: Vec<Vec<usize>> = Vec::new();
    for &i in &order {
        let r = root(&mut parent, i);
        let slot = *slot_of_root.entry(r).or_insert_with(|| {
            scenes.push(Vec::new());
            scenes.len() - 1
        });
        scenes[slot].push(i);
    }
    for mut scene in scenes {
        flush_time_burst(analyses, &mut scene, &mut suggestions);
    }

    suggestions
}

#[cfg(test)]
mod scene_grouping_tests {
    use super::*;

    /// A frame whose whole palette sits in one histogram bin, with a constant-byte hash.
    fn frame(path: &str, bin: usize, hash_byte: u8) -> ImageAnalysisData {
        let mut color_hist = [0u16; 64];
        color_hist[bin] = COLOR_HIST_SAMPLES as u16;
        ImageAnalysisData {
            hash: image_hasher::ImageHash::from_bytes(&[hash_byte; (HASH_BITS / 8) as usize]).unwrap(),
            color_hist,
            result: ImageAnalysisResult {
                path: path.to_string(),
                quality_score: 0.0,
                sharpness_metric: 0.0,
                center_focus_metric: 0.0,
                exposure_metric: 0.0,
                width: 1,
                height: 1,
            },
        }
    }

    const MAX_GAP: i64 = 20;

    fn groups(frames: &[ImageAnalysisData], times: &[i64]) -> Vec<Vec<String>> {
        let times = frames.iter().zip(times).map(|(f, &t)| (f.result.path.clone(), t)).collect();
        let settings = CullingSettings {
            similarity_threshold: 29,
            blur_threshold: 100.0,
            group_similar: true,
            filter_blurry: false,
        };
        group_by_scene(frames, vec![], &times, &settings, MAX_GAP)
            .similar_groups
            .into_iter()
            .map(|g| {
                let mut paths: Vec<String> = std::iter::once(g.representative.path)
                    .chain(g.duplicates.into_iter().map(|d| d.path))
                    .collect();
                paths.sort();
                paths
            })
            .collect()
    }

    #[test]
    fn a_stray_frame_between_two_takes_does_not_split_them() {
        // 0x00 vs 0xFF hashes differ in every bit, so only the palette can link.
        let frames = [frame("a1", 0, 0x00), frame("x", 5, 0xFF), frame("a2", 0, 0x0F)];
        assert_eq!(groups(&frames, &[0, 2, 4]), vec![vec!["a1".to_string(), "a2".to_string()]]);
    }

    #[test]
    fn identical_frames_shot_far_apart_stay_separate() {
        let frames = [frame("a1", 0, 0x00), frame("a2", 0, 0x00)];
        assert!(groups(&frames, &[0, MAX_GAP + 1]).is_empty());
    }

    #[test]
    fn a_borderline_match_needs_the_frames_to_be_close_in_time() {
        // Palette distance 60: over the plain threshold (51), inside it with the time bonus.
        let mut b = frame("b", 0, 0xFF);
        b.color_hist[0] -= 60;
        b.color_hist[1] += 60;
        let frames = [frame("a", 0, 0x00), b];
        assert_eq!(groups(&frames, &[0, 1]).len(), 1);
        assert!(groups(&frames, &[0, 15]).is_empty());
    }

    #[test]
    fn a_matching_hash_links_frames_whose_palettes_differ() {
        let frames = [frame("a1", 0, 0xAA), frame("a2", 9, 0xAA)];
        assert_eq!(groups(&frames, &[0, 1]).len(), 1);
    }
}

/// One photo as the assistant's culling pass sees it: the technical hints sent
/// alongside the image so the model can weigh them against what it sees.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ShotInfo {
    pub path: String,
    pub quality: f64,
    pub sharpness: f64,
}

#[derive(Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShotGroups {
    /// Bursts / near-duplicates, best-scoring first within each group.
    pub groups: Vec<Vec<ShotInfo>>,
    pub singles: Vec<ShotInfo>,
    pub failed: Vec<String>,
}

fn shot_info(r: &ImageAnalysisResult) -> ShotInfo {
    ShotInfo {
        path: r.path.clone(),
        quality: r.quality_score,
        sharpness: r.sharpness_metric,
    }
}

/// Split photos into groups of similar shots for the assistant's comparative
/// cull. Same analysis as the Cull dialog, on its own event channel
/// (`assistant-cull-*`) so the dialog's listeners never fire.
#[tauri::command]
pub async fn assistant_group_images(
    paths: Vec<String>,
    similarity_threshold: Option<u32>,
    app_handle: AppHandle,
) -> Result<ShotGroups, String> {
    let (analyses, failed) = analyze_paths(paths, app_handle, "assistant-cull", true).await?;
    let settings = CullingSettings {
        similarity_threshold: similarity_threshold.unwrap_or(28),
        blur_threshold: 0.0,
        group_similar: true,
        filter_blurry: false,
    };
    let suggestions = group_analyses(&analyses, failed, &settings);

    let mut grouped = std::collections::HashSet::new();
    let groups: Vec<Vec<ShotInfo>> = suggestions
        .similar_groups
        .iter()
        .map(|g| {
            std::iter::once(&g.representative)
                .chain(g.duplicates.iter())
                .map(|r| {
                    grouped.insert(r.path.clone());
                    shot_info(r)
                })
                .collect()
        })
        .collect();
    let singles = analyses
        .iter()
        .map(|a| &a.result)
        .filter(|r| !grouped.contains(&r.path))
        .map(shot_info)
        .collect();
    Ok(ShotGroups { groups, singles, failed: suggestions.failed_paths })
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
    let (analyses, failed) = analyze_paths(paths, app_handle.clone(), channel, true).await?;
    let total = analyses.len() + failed.len();
    let _ = app_handle.emit(
        &format!("{channel}-progress"),
        CullingProgress {
            current: total,
            total,
            stage: "Grouping similar images...".to_string(),
        },
    );
    let suggestions = group_analyses(&analyses, failed, &settings);
    let _ = app_handle.emit(&format!("{channel}-complete"), &suggestions);
    Ok(suggestions)
}
