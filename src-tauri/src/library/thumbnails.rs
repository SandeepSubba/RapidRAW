use crate::app::settings::{AppSettings, load_settings};
use crate::app::state::{AppState, ThumbnailSource};
use crate::decode::as_shot::as_shot;
use crate::decode::formats::is_raw_file;
use crate::decode::image_loader;
use crate::library::file_ops::{is_cloud_placeholder, read_file_mapped};
use crate::library::listing::enqueue_metadata;
use crate::library::sidecars::{parse_virtual_path, run_batch_workers};
use crate::masks::generation::MaskDefinition;
use crate::metadata::exif_processing;
use crate::metadata::sidecar::{ImageMetadata, is_image_edited};
use crate::render::adjustment_utils::{effective_baseline_exposure, is_default_adjustments};
use crate::render::geometry::{
    Crop, apply_coarse_rotation, get_geometry_params_from_json, is_geometry_identity,
};
use crate::render::gpu::context::{GpuContext, get_or_init_gpu_context};
use crate::render::gpu::output::process_and_get_dynamic_image;
use crate::render::gpu::processor::RenderRequest;
use crate::render::image_processing::{apply_cpu_default_raw_processing, apply_exposure};
use crate::render::options::{resolve_render_options, resolve_tonemapper_override};
use crate::render::preview::cached_patched_image;
use crate::render::source::{Frame, RenderSource, SourceTransform};
use crate::render::uniforms::get_all_adjustments_from_json;
use anyhow::Result;
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, GenericImageView};
use std::borrow::Cow;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
pub fn cancel_thumbnail_generation(
    state: tauri::State<AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    state
        .thumbnail_cancellation_token
        .store(true, Ordering::SeqCst);

    let mut tracker = state.thumbnail_progress.lock().unwrap();
    tracker.total = 0;
    tracker.completed = 0;
    drop(tracker);

    let _ = app_handle.emit(
        "thumbnail-progress",
        serde_json::json!({ "current": 0, "total": 0 }),
    );
    Ok(())
}

pub fn resolve_thumbnail_cache_dir(app_handle: &AppHandle) -> std::result::Result<PathBuf, String> {
    let cache_dir = app_handle
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?;
    let thumb_cache_dir = cache_dir.join("thumbnails");
    if !thumb_cache_dir.exists() {
        fs::create_dir_all(&thumb_cache_dir).map_err(|e| e.to_string())?;
    }
    Ok(thumb_cache_dir)
}

pub fn emit_thumbnail_cache_setup_error(app_handle: &AppHandle, path: &str, reason: &str) {
    let _ = app_handle.emit(
        "thumbnail-generation-error",
        serde_json::json!({ "path": path, "reason": reason }),
    );
}

fn thumbnail_render_key(settings: &AppSettings, is_raw: bool) -> String {
    let render = resolve_render_options(settings, is_raw);
    format!(
        "RenderOptions {{ tonemapper_override: {:?}, non_raw_tonemapper_override: {:?}, \
         baseline: {:?} }}{:?}",
        render.tonemapper_override,
        render.non_raw_tonemapper_override,
        render.baseline,
        settings.always_decode_raw_thumbnails
    )
}

fn compute_thumbnail_cache_hash(
    path_str: &str,
    adjustments_bytes: &[u8],
    settings: &AppSettings,
) -> Option<String> {
    let (source_path, _) = parse_virtual_path(path_str);
    let render_key = thumbnail_render_key(settings, is_raw_file(&source_path));

    let img_mod_time = fs::metadata(&source_path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();

    let mut hasher = blake3::Hasher::new();
    hasher.update(path_str.as_bytes());
    hasher.update(&img_mod_time.to_le_bytes());
    hasher.update(adjustments_bytes);
    // Unedited RAF thumbnails cached before the embedded-JPEG orientation was
    // read are sideways for portrait shots; this salt retires them (fork). Only
    // unedited ones: a RAF with adjustments is thumbnailed by the full decode,
    // which always rotated correctly, and redoing those costs a RAW decode each.
    let unedited = adjustments_bytes.is_empty() || adjustments_bytes == b"null";
    if unedited && source_path.extension().is_some_and(|e| e.eq_ignore_ascii_case("raf")) {
        hasher.update(b"raf-orientation-v2");
    }
    hasher.update(render_key.as_bytes());
    Some(hasher.finalize().to_hex().to_string())
}

const THUMBNAIL_SOURCE_CACHE_ENTRIES: usize = 8;

const THUMBNAIL_SOURCE_CACHE_BYTES: usize = 256 << 20;

fn find_embedded_jpeg<'a>(
    exif: &'a exif::Exif,
    file_bytes: &'a [u8],
    ifd: exif::In,
) -> Option<&'a [u8]> {
    let offset = exif
        .get_field(exif::Tag::JPEGInterchangeFormat, ifd)?
        .value
        .get_uint(0)? as usize;
    let len = exif
        .get_field(exif::Tag::JPEGInterchangeFormatLength, ifd)?
        .value
        .get_uint(0)? as usize;
    let tiff = if exif_processing::is_tiff(file_bytes) {
        file_bytes
    } else {
        exif.buf()
    };
    tiff.get(offset..offset + len)
}

fn apply_exif_orientation(img: DynamicImage, orientation: u32) -> DynamicImage {
    match orientation {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        7 => img.rotate270().fliph(),
        8 => img.rotate270(),
        _ => img,
    }
}

fn exif_embedded_preview(exif: &exif::Exif, file_bytes: &[u8]) -> Option<DynamicImage> {
    let (jpeg_bytes, ifd) = find_embedded_jpeg(exif, file_bytes, exif::In::PRIMARY)
        .map(|b| (b, exif::In::PRIMARY))
        .or_else(|| {
            find_embedded_jpeg(exif, file_bytes, exif::In::THUMBNAIL)
                .map(|b| (b, exif::In::THUMBNAIL))
        })?;

    let img = image::load_from_memory_with_format(jpeg_bytes, image::ImageFormat::Jpeg).ok()?;

    let orientation = exif
        .get_field(exif::Tag::Orientation, ifd)
        .and_then(|f| f.value.get_uint(0))
        .unwrap_or(1);

    Some(apply_exif_orientation(img, orientation))
}

fn try_load_embedded_raw_preview(source_path: &Path, target_res: u32) -> Option<DynamicImage> {
    let mmap = read_file_mapped(source_path).ok()?;
    let min_size = (target_res as f32 * 0.95) as u32;
    let large_enough = |preview: &DynamicImage| preview.width().max(preview.height()) >= min_size;

    match exif_processing::read_exif(&mmap) {
        Some(exif) => exif_embedded_preview(&exif, &mmap)
            .filter(large_enough)
            .or_else(|| image_loader::tiff_embedded_preview(&mmap).filter(large_enough)),
        None => image_loader::safe_embedded_preview_fallback(&mmap, &source_path.to_string_lossy())
            .filter(large_enough),
    }
}

fn reserve_thumbnail_decode(
    state: &AppState,
    source_path: &Path,
    file_bytes: &[u8],
    is_raw: bool,
) -> Option<crate::app::memory_budget::MemoryReservation> {
    if rayon::current_thread_index().is_some() {
        return None;
    }
    let pixels = if is_raw {
        crate::app::memory_budget::raw_pixel_count(file_bytes)?
    } else {
        crate::app::memory_budget::image_pixel_count(source_path, false)?
    };
    state
        .memory_budget
        .reserve(pixels * if is_raw { 10 } else { 20 }, None)
}

pub fn generate_thumbnail_data(
    path_str: &str,
    gpu_context: Option<&GpuContext>,
    preloaded_image: Option<&DynamicImage>,
    app_handle: &AppHandle,
) -> anyhow::Result<DynamicImage> {
    let metadata = read_thumbnail_metadata(path_str, app_handle);
    let settings = load_settings(app_handle.clone()).unwrap_or_default();
    generate_thumbnail_data_with_metadata(
        path_str,
        gpu_context,
        preloaded_image,
        app_handle,
        metadata.as_ref(),
        &settings,
    )
}

fn read_thumbnail_metadata(path_str: &str, app_handle: &AppHandle) -> Option<ImageMetadata> {
    let (source_path, sidecar_path) = parse_virtual_path(path_str);
    if is_cloud_placeholder(&sidecar_path) {
        enqueue_metadata(
            app_handle,
            path_str.to_string(),
            source_path.clone(),
            sidecar_path.clone(),
        );
        None
    } else {
        let mut metadata = fs::read_to_string(&sidecar_path)
            .ok()
            .and_then(|content| crate::metadata::sidecar::parse_sidecar(&content));
        resolve_unedited_metadata(&mut metadata, path_str);
        metadata
    }
}

fn metadata_adjustments_bytes(metadata: &Option<ImageMetadata>) -> Vec<u8> {
    metadata
        .as_ref()
        .map(|meta| serde_json::to_vec(&meta.adjustments).unwrap_or_default())
        .unwrap_or_default()
}

fn resolve_unedited_metadata(metadata: &mut Option<ImageMetadata>, path_str: &str) -> bool {
    let (source_path, _) = parse_virtual_path(path_str);
    if !crate::adjustments::develop_defaults::is_customized(&source_path) {
        return false;
    }
    let meta = metadata.get_or_insert_with(ImageMetadata::default);
    if !crate::render::adjustment_utils::is_unedited(&meta.adjustments) {
        return false;
    }
    crate::adjustments::develop_defaults::resolve_in_place(&mut meta.adjustments, &source_path);
    true
}

fn generate_thumbnail_data_with_metadata(
    path_str: &str,
    gpu_context: Option<&GpuContext>,
    preloaded_image: Option<&DynamicImage>,
    app_handle: &AppHandle,
    metadata: Option<&ImageMetadata>,
    settings: &AppSettings,
) -> anyhow::Result<DynamicImage> {
    let (source_path, _) = parse_virtual_path(path_str);
    let source_path_str = source_path.to_string_lossy().to_string();
    let is_raw = is_raw_file(&source_path_str);

    let null_adjustments = serde_json::Value::Null;
    let adjustments = metadata.map_or(&null_adjustments, |m| &m.adjustments);
    let cached_patched = preloaded_image
        .and_then(|img| cached_patched_image(&app_handle.state::<AppState>(), img, adjustments));

    let always_decode_raw = settings.always_decode_raw_thumbnails.unwrap_or(false);

    let untouched = || {
        adjustments.is_null()
            || (crate::adjustments::develop_defaults::keeps_embedded_preview(&source_path)
                && is_default_adjustments(adjustments, &source_path))
    };

    if is_raw && preloaded_image.is_none() && !always_decode_raw && untouched() {
        let target_res = settings.medium_thumbnail_resolution.unwrap_or(1280);
        // Fork: culling's fast path first (RAF embedded JPEG with its own
        // orientation, scaled decode). This is what keeps hundreds of large X-Trans
        // raws in the SD-card review grid from each triggering a full decode.
        if let Some(preview) =
            crate::library::culling::fast_raw_preview_scaled(&source_path_str, Some(target_res))
        {
            let (w, h) = preview.dimensions();
            return Ok(if w.max(h) > target_res {
                preview.thumbnail(target_res, target_res)
            } else {
                preview
            });
        }
        if let Some(preview) = try_load_embedded_raw_preview(&source_path, target_res) {
            return Ok(preview);
        }
    }

    if let (Some(context), Some(meta)) = (gpu_context, metadata)
        && !meta.adjustments.is_null()
    {
        let state = app_handle.state::<AppState>();
        let target_res = settings.medium_thumbnail_resolution.unwrap_or(1280);

        let base_cache_hash = crate::render::cache_utils::calculate_thumbnail_source_hash(
            path_str,
            &meta.adjustments,
        );

        let crop_data: Option<Crop> = serde_json::from_value(meta.adjustments["crop"].clone()).ok();

        let thumbnail_transform = |base: &ThumbnailSource| {
            let geometry = get_geometry_params_from_json(path_str, &meta.adjustments);
            let warp = (!is_geometry_identity(&geometry)).then_some(geometry);
            let mut transform = SourceTransform {
                warp,
                working_fit: Some((base.processing_dim, base.processing_dim)),
                effects: true,
                ..Default::default()
            };
            let (w, h) = transform.working_size(base.image.dimensions());
            let scale = if base.full_max_dim > 0 {
                w.max(h) as f32 / base.full_max_dim as f32
            } else {
                1.0
            } * base.raw_scale_factor;
            transform.frame = Frame::from_adjustments(&meta.adjustments).scale_crop(scale as f64);
            (transform, scale)
        };

        let cached_base: Option<Arc<ThumbnailSource>> = {
            let cache = state.thumbnail_source_cache.lock().unwrap();
            cache
                .get(path_str)
                .filter(|base| {
                    let (_, scale) = thumbnail_transform(base);
                    let sufficient_resolution = crop_data
                        .as_ref()
                        .filter(|c| c.width > 0.0 && c.height > 0.0)
                        .is_none_or(|c| {
                            (c.width as f32 * scale).max(c.height as f32 * scale)
                                >= target_res as f32 * 0.95
                        });
                    base.hash == base_cache_hash && sufficient_resolution
                })
                .cloned()
        };

        let base = if let Some(base) = cached_base {
            base
        } else {
            let settings = load_settings(app_handle.clone()).unwrap_or_default();
            let mut raw_scale_factor = 1.0f32;

            let _decode_reservation;
            let composite_image: Cow<DynamicImage> = if let Some(patched) = &cached_patched {
                if let Some(img) = preloaded_image {
                    raw_scale_factor = crate::ai::upscale::scale_between(img, patched);
                }
                Cow::Borrowed(patched)
            } else if let Some(img) = preloaded_image {
                let prepared =
                    image_loader::prepare_base_image_cow(img, &source_path_str, adjustments)?;
                raw_scale_factor = crate::ai::upscale::scale_between(img, &prepared);
                prepared
            } else {
                let mmap_guard;
                let vec_guard;

                let file_slice: &[u8] = match read_file_mapped(&source_path) {
                    Ok(mmap) => {
                        mmap_guard = Some(mmap);
                        mmap_guard.as_ref().unwrap()
                    }
                    Err(e) => {
                        if preloaded_image.is_none() {
                            log::warn!("Fallback read for {}: {}", source_path_str, e);
                        }
                        let bytes = fs::read(&source_path).map_err(|io_err| {
                            anyhow::anyhow!(
                                "Fallback read failed for {}: {}",
                                source_path_str,
                                io_err
                            )
                        })?;
                        vec_guard = Some(bytes);
                        vec_guard.as_ref().unwrap()
                    }
                };

                _decode_reservation =
                    reserve_thumbnail_decode(&state, &source_path, file_slice, is_raw);
                crate::decode::as_shot::as_shot_from_bytes(&source_path_str, file_slice);
                let (img, upscale) = image_loader::load_and_composite(
                    file_slice,
                    &source_path_str,
                    adjustments,
                    true,
                    &settings,
                    None,
                )?;

                raw_scale_factor = upscale;
                if is_raw {
                    raw_scale_factor *=
                        crate::decode::raw_processing::get_fast_demosaic_scale_factor(
                            file_slice,
                            (img.width() as f32 / upscale).round() as u32,
                            (img.height() as f32 / upscale).round() as u32,
                        );
                }
                Cow::Owned(img)
            };

            let full_max_dim = composite_image.width().max(composite_image.height());
            let texture_limit = context.limits.max_texture_dimension_2d;

            let mut processing_dim = target_res;
            if let Some(c) = &crop_data
                && c.width > 0.0
                && c.height > 0.0
            {
                let crop_max_dim_loaded = c.width.max(c.height) * raw_scale_factor as f64;
                if crop_max_dim_loaded > 0.0 {
                    processing_dim = ((target_res as f64 * full_max_dim as f64
                        / crop_max_dim_loaded)
                        .round() as u32)
                        .min(full_max_dim);
                }
            }

            let has_geometry =
                !is_geometry_identity(&get_geometry_params_from_json(path_str, &meta.adjustments));
            let processing_dim = processing_dim.min(texture_limit);
            let working_dim = if has_geometry {
                processing_dim.saturating_mul(2).min(full_max_dim)
            } else {
                processing_dim
            }
            .min(texture_limit);

            let working_image = if full_max_dim > working_dim {
                crate::render::image_processing::downscale_f32_image(
                    &composite_image,
                    working_dim,
                    working_dim,
                )
            } else {
                composite_image.into_owned()
            };

            let base = Arc::new(ThumbnailSource {
                hash: base_cache_hash,
                image: Arc::new(working_image),
                full_max_dim,
                processing_dim,
                raw_scale_factor,
            });
            let mut cache = state.thumbnail_source_cache.lock().unwrap();
            cache.remove(path_str);
            let bytes = |entry: &ThumbnailSource| entry.image.as_bytes().len();
            let mut total: usize = cache.values().map(|entry| bytes(entry)).sum();
            while !cache.is_empty()
                && (cache.len() >= THUMBNAIL_SOURCE_CACHE_ENTRIES
                    || total + bytes(&base) > THUMBNAIL_SOURCE_CACHE_BYTES)
            {
                let key = cache.keys().next().cloned().unwrap();
                total -= cache.remove(&key).map_or(0, |entry| bytes(&entry));
            }
            cache.insert(path_str.to_string(), Arc::clone(&base));
            base
        };

        let (transform, total_scale) = thumbnail_transform(&base);
        let source_id = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::hash::Hash::hash(&(path_str, base.hash), &mut hasher);
            std::hash::Hasher::finish(&hasher)
        };
        let source = RenderSource::new(&base.image, source_id, &meta.adjustments, transform);
        let (preview_w, preview_h) = source.size();
        let unscaled_crop_offset = crop_data.map_or((0.0, 0.0), |c| (c.x as f32, c.y as f32));

        let mask_definitions: Vec<MaskDefinition> = meta
            .adjustments
            .get("masks")
            .and_then(|m| serde_json::from_value(m.clone()).ok())
            .unwrap_or_else(Vec::new);

        let mask_bitmaps = crate::masks::generation::get_cached_or_generate_masks(
            &state,
            path_str,
            &mask_definitions,
            preview_w,
            preview_h,
            total_scale,
            (
                unscaled_crop_offset.0 * total_scale,
                unscaled_crop_offset.1 * total_scale,
            ),
            &meta.adjustments,
        )
        .bitmaps;

        let gpu_adjustments = get_all_adjustments_from_json(
            &meta.adjustments,
            is_raw,
            as_shot(&source_path_str),
            resolve_render_options(settings, is_raw),
        );
        let lut_path = meta.adjustments["lutPath"].as_str();
        let lut = lut_path.and_then(|p| {
            let mut cache = state.lut_cache.lock().unwrap();
            if let Some(cached_lut) = cache.get(p) {
                return Some(cached_lut.clone());
            }
            if let Ok(loaded_lut) = crate::adjustments::lut_processing::parse_lut_file(p) {
                let arc_lut = Arc::new(loaded_lut);
                cache.insert(p.to_string(), arc_lut.clone());
                return Some(arc_lut);
            }
            None
        });

        let processed_image = process_and_get_dynamic_image(
            context,
            &state,
            &source,
            RenderRequest {
                adjustments: gpu_adjustments,
                mask_bitmaps: &mask_bitmaps,
                mask_cache_key: None,
                lut,
                roi: None,
                hsl_probe: false,
                motion_blur: crate::effects::motion_blur::MotionBlurs::new(
                    path_str,
                    &meta.adjustments,
                    (preview_w, preview_h),
                    (
                        unscaled_crop_offset.0 * total_scale,
                        unscaled_crop_offset.1 * total_scale,
                    ),
                    {
                        let (w, h) = source.transform.working_size(base.image.dimensions());
                        w.max(h) as f32
                    },
                ),
            },
            "generate_thumbnail_data",
        );
        return processed_image.map_err(|e| anyhow::anyhow!(e));
    }

    let _decode_reservation;
    let final_image: Cow<DynamicImage> = if let Some(patched) = &cached_patched {
        Cow::Borrowed(patched)
    } else if let Some(img) = preloaded_image {
        image_loader::prepare_base_image_cow(img, &source_path_str, adjustments)?
    } else {
        let mmap_guard;
        let vec_guard;
        let file_slice: &[u8] = match read_file_mapped(&source_path) {
            Ok(mmap) => {
                mmap_guard = mmap;
                &mmap_guard
            }
            Err(e) => {
                log::warn!("Fallback read for {}: {}", source_path_str, e);
                vec_guard = fs::read(&source_path)?;
                &vec_guard
            }
        };
        _decode_reservation = reserve_thumbnail_decode(
            &app_handle.state::<AppState>(),
            &source_path,
            file_slice,
            is_raw,
        );
        if adjustments.is_null() {
            crate::decode::as_shot::as_shot_from_bytes(&source_path_str, file_slice);
        }
        if !is_raw
            && adjustments.is_null()
            && resolve_render_options(settings, is_raw).tonemapper_override != Some(1)
        {
            return image_loader::load_display_image(file_slice, &source_path_str);
        }
        Cow::Owned(
            image_loader::load_and_composite(
                file_slice,
                &source_path_str,
                adjustments,
                true,
                settings,
                None,
            )?
            .0,
        )
    };

    let mut owned_image = final_image.into_owned();
    if adjustments.is_null() {
        let render = resolve_render_options(settings, is_raw);
        owned_image = apply_exposure(
            owned_image,
            effective_baseline_exposure(adjustments, as_shot(&source_path_str), render.baseline),
        );
        let use_agx = render.tonemapper_override == Some(1);

        if use_agx {
            if !is_raw {
                owned_image = crate::render::image_processing::apply_srgb_to_linear(owned_image);
            }
            crate::render::image_processing::apply_cpu_agx_tonemap(&mut owned_image);
        } else if is_raw {
            apply_cpu_default_raw_processing(&mut owned_image);
        }
    }
    crate::render::image_processing::apply_working_display_to_srgb(&mut owned_image, is_raw);

    let fallback_orientation_steps = adjustments["orientationSteps"].as_u64().unwrap_or(0) as u8;
    Ok(apply_coarse_rotation(Cow::Owned(owned_image), fallback_orientation_steps).into_owned())
}

fn encode_thumbnail(image: &DynamicImage, target_width: u32) -> Result<Vec<u8>> {
    let downscaled = crate::render::image_processing::downscale_f32_image_if_needed(
        image,
        target_width,
        target_width,
    );
    let thumbnail = downscaled.as_ref().unwrap_or(image);
    let mut buf = Cursor::new(Vec::new());
    let mut encoder = JpegEncoder::new_with_quality(&mut buf, 75);
    match thumbnail.as_rgb8() {
        Some(rgb) => encoder.encode_image(rgb)?,
        None => encoder.encode_image(&thumbnail.to_rgb8())?,
    }
    Ok(buf.into_inner())
}

#[allow(clippy::too_many_arguments)]
pub fn generate_single_thumbnail_and_cache(
    path_str: &str,
    thumb_cache_dir: &Path,
    gpu_context: Option<&GpuContext>,
    preloaded_image: Option<&DynamicImage>,
    force_regenerate: bool,
    app_handle: &AppHandle,
    settings: &AppSettings,
    before_generate: Option<&dyn Fn()>,
) -> Option<(String, String, u8, bool)> {
    let (source_path, sidecar_path) = parse_virtual_path(path_str);
    let sidecar_is_placeholder = is_cloud_placeholder(&sidecar_path);

    let mut metadata: Option<ImageMetadata> = None;
    let (rating, is_edited, mut adjustments_bytes) = if sidecar_is_placeholder {
        enqueue_metadata(
            app_handle,
            path_str.to_string(),
            source_path.clone(),
            sidecar_path.clone(),
        );
        (0, false, Vec::new())
    } else if let Ok(content) = fs::read_to_string(&sidecar_path) {
        if let Some(meta) = crate::metadata::sidecar::parse_sidecar(&content) {
            let is_raw = crate::decode::formats::is_raw_file(path_str);
            let tm = resolve_tonemapper_override(settings, is_raw);
            let info = (
                meta.rating,
                is_image_edited(&meta.adjustments, &source_path, is_raw, tm),
                serde_json::to_vec(&meta.adjustments).unwrap_or_default(),
            );
            metadata = Some(meta);
            info
        } else {
            (0, false, Vec::new())
        }
    } else {
        (0, false, Vec::new())
    };

    if !sidecar_is_placeholder && resolve_unedited_metadata(&mut metadata, path_str) {
        adjustments_bytes = metadata_adjustments_bytes(&metadata);
    }

    if let Some(remote) =
        crate::integrations::immich::placeholder_thumbnail(app_handle, path_str, thumb_cache_dir)
    {
        return remote.map(|(small, medium)| (small, medium, rating, is_edited));
    }

    let cache_hash = compute_thumbnail_cache_hash(path_str, &adjustments_bytes, settings)?;

    let small_path = thumb_cache_dir.join(format!("{}_small.jpg", cache_hash));
    let medium_path = thumb_cache_dir.join(format!("{}_medium.jpg", cache_hash));

    if !force_regenerate && small_path.exists() && medium_path.exists() {
        crate::app::storage::mark_used(&small_path);
        return Some((
            small_path.to_string_lossy().into_owned(),
            medium_path.to_string_lossy().into_owned(),
            rating,
            is_edited,
        ));
    }

    if let Some(before_generate) = before_generate {
        before_generate();
    }

    if is_cloud_placeholder(&source_path) {
        return None;
    }

    let target_width_small = settings.small_thumbnail_resolution.unwrap_or(480);
    let target_width_medium = settings.medium_thumbnail_resolution.unwrap_or(1280);

    let thumb_result = if sidecar_is_placeholder {
        generate_thumbnail_data(path_str, gpu_context, preloaded_image, app_handle)
    } else {
        let current_settings = load_settings(app_handle.clone()).unwrap_or_default();
        generate_thumbnail_data_with_metadata(
            path_str,
            gpu_context,
            preloaded_image,
            app_handle,
            metadata.as_ref(),
            &current_settings,
        )
    };

    if let Ok(thumb_image) = thumb_result
        && let (Ok(small_data), Ok(medium_data)) = (
            encode_thumbnail(&thumb_image, target_width_small),
            encode_thumbnail(&thumb_image, target_width_medium),
        )
    {
        let _ = fs::write(&small_path, &small_data);
        let _ = fs::write(&medium_path, &medium_data);
        crate::app::storage::record_thumbnail_write((small_data.len() + medium_data.len()) as u64);
        return Some((
            small_path.to_string_lossy().into_owned(),
            medium_path.to_string_lossy().into_owned(),
            rating,
            is_edited,
        ));
    }
    None
}

/// Decode any supported image (RAW or not) at `path` to a base64 JPEG sized for
/// the AI assistant to read (OCR/vision). Reuses the thumbnail decode/encode path
/// but at a much larger dimension than grid thumbnails so small labels stay legible.
#[tauri::command]
pub async fn assistant_prepare_image(
    path: String,
    max_dim: Option<u32>,
    app_handle: AppHandle,
) -> Result<crate::ai::assistant::PreparedImage, String> {
    let max_dim = max_dim.unwrap_or(2000).clamp(256, 4000);
    tauri::async_runtime::spawn_blocking(move || {
        // Prefer a full-quality source over the thumbnail pipeline: the model is
        // often asked to read text off the image, and generate_thumbnail_data
        // caps unedited raws at the thumbnail resolution (~720px). Edited images
        // still go through the thumbnail path so their edits are visible.
        let (source_path, sidecar_path) = parse_virtual_path(&path);
        let has_edits = fs::read_to_string(&sidecar_path)
            .ok()
            .and_then(|c| serde_json::from_str::<ImageMetadata>(&c).ok())
            .map(|m| !m.adjustments.is_null())
            .unwrap_or(false);
        let source_str = source_path.to_string_lossy().to_string();
        let image = if has_edits {
            let state = app_handle.state::<AppState>();
            let gpu_context = get_or_init_gpu_context(&state, &app_handle).ok();
            generate_thumbnail_data(&path, gpu_context.as_ref(), None, &app_handle)
                .map_err(|e| format!("Failed to decode image: {}", e))?
        } else if is_raw_file(&source_str) {
            crate::library::culling::fast_raw_preview(&source_str)
                .ok_or_else(|| "Failed to extract raw preview".to_string())?
        } else {
            image::open(&source_path).map_err(|e| format!("Failed to decode image: {}", e))?
        };
        let attachment = encode_assistant_jpeg(&image, max_dim)?;
        Ok(crate::ai::assistant::PreparedImage {
            media_type: attachment.media_type,
            data: attachment.data,
            // Full pre-downscale dimensions: the pixel space inspect regions use.
            full_width: image.width(),
            full_height: image.height(),
        })
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

/// Reproduce the VIEW the assistant was shown: the sidecar's 90-degree
/// orientation, then its crop. A rectangle for a cropped/rotated image
/// measured against the raw frame otherwise points at backdrop (seen as
/// black inspect crops in batch runs). Fine rotation / perspective are not
/// reproduced — a couple of degrees doesn't move text out of a generously
/// sized rect.
fn view_for_adjustments(
    full: image::DynamicImage,
    adjustments: &serde_json::Value,
    fallback_steps: u32,
) -> image::DynamicImage {
    let steps = adjustments
        .get("orientationSteps")
        .and_then(|v| v.as_u64())
        .map(|v| (v % 4) as u32)
        .unwrap_or(fallback_steps % 4);
    let oriented = match steps {
        1 => full.rotate90(),
        2 => full.rotate180(),
        3 => full.rotate270(),
        _ => full,
    };
    match adjustments.get("crop").filter(|c| c.is_object()) {
        Some(c) => {
            let g = |k: &str| c.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0).max(0.0) as u32;
            let (cx, cy) = (g("x"), g("y"));
            let (cw, ch) = (g("width"), g("height"));
            if cw > 0 && ch > 0 && cx < oriented.width() && cy < oriented.height() {
                let cw = cw.min(oriented.width() - cx);
                let ch = ch.min(oriented.height() - cy);
                oriented.crop_imm(cx, cy, cw, ch)
            } else {
                oriented
            }
        }
        None => oriented,
    }
}

/// Scale a rectangle from the caller's canvas (the dimensions the model was
/// told about) into the view's own pixel space, clamped inside it. The two can
/// differ by resolution — a 720px thumbnail canvas against a full raw preview.
fn map_canvas_rect(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    canvas_width: Option<u32>,
    canvas_height: Option<u32>,
    view_width: u32,
    view_height: u32,
) -> (u32, u32, u32, u32) {
    let sx = canvas_width
        .filter(|&cw| cw > 0)
        .map(|cw| view_width as f64 / cw as f64)
        .unwrap_or(1.0);
    let sy = canvas_height
        .filter(|&ch| ch > 0)
        .map(|ch| view_height as f64 / ch as f64)
        .unwrap_or(1.0);
    let x = ((x as f64 * sx).round() as u32).min(view_width.saturating_sub(1));
    let y = ((y as f64 * sy).round() as u32).min(view_height.saturating_sub(1));
    let w = ((width as f64 * sx).round() as u32).clamp(1, view_width - x);
    let h = ((height as f64 * sy).round() as u32).clamp(1, view_height - y);
    (x, y, w, h)
}

#[cfg(test)]
mod assistant_view_tests {
    use super::{map_canvas_rect, view_for_adjustments};
    use image::DynamicImage;

    // The user's AI226024 sidecar: rotated 90 degrees and cropped.
    #[test]
    fn rotated_and_cropped_sidecar_yields_the_crop_dims() {
        let full = DynamicImage::new_rgb8(5760, 3840);
        let adj = serde_json::json!({
            "orientationSteps": 1,
            "crop": { "x": 832, "y": 2480, "width": 2378, "height": 2662, "unit": "px" }
        });
        let view = view_for_adjustments(full, &adj, 0);
        assert_eq!((view.width(), view.height()), (2378, 2662));
    }

    // The user's AI228788/89/93 sidecars: rotated, no crop.
    #[test]
    fn rotated_uncropped_sidecar_swaps_dimensions() {
        let full = DynamicImage::new_rgb8(5760, 3840);
        let adj = serde_json::json!({ "orientationSteps": 1, "crop": null });
        let view = view_for_adjustments(full, &adj, 0);
        assert_eq!((view.width(), view.height()), (3840, 5760));
    }

    #[test]
    fn view_pixels_match_a_library_rotate_then_crop() {
        let mut img = image::RgbImage::new(8, 6);
        img.put_pixel(5, 2, image::Rgb([255, 0, 0]));
        let full = DynamicImage::ImageRgb8(img);
        let oriented = full.rotate90();
        let adj = serde_json::json!({
            "orientationSteps": 1,
            "crop": { "x": 1, "y": 2, "width": 3, "height": 4 }
        });
        let view = view_for_adjustments(full, &adj, 0);
        let reference = oriented.crop_imm(1, 2, 3, 4);
        assert_eq!(view.to_rgb8().as_raw(), reference.to_rgb8().as_raw());
    }

    #[test]
    fn thumbnail_canvas_rect_scales_into_the_full_view() {
        // 643x720 thumbnail canvas against the 2378x2662 cropped view.
        let (x, y, w, h) = map_canvas_rect(100, 200, 50, 40, Some(643), Some(720), 2378, 2662);
        assert_eq!((x, y, w, h), (370, 739, 185, 148));
    }

    #[test]
    fn missing_canvas_means_no_scaling() {
        assert_eq!(map_canvas_rect(10, 20, 30, 40, None, None, 100, 100), (10, 20, 30, 40));
    }
}

/// Crop a region (in oriented display pixels, the assistant's `_canvas` space)
/// out of the currently loaded editor image at native resolution, for the
/// assistant's inspect loop — reading small text the downscaled attachment
/// can't resolve.
/// The crop the assistant gets back when it asks to inspect the label.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelCrop {
    pub media_type: String,
    pub data: String,
    /// False when nothing card-like was found; the payload is then the whole
    /// view, so the model still gets something to look at rather than an error.
    pub found: bool,
    /// Where the crop came from, in the caller's canvas space, so the model can
    /// narrow it further with a normal inspect if the detector picked wrong.
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Find the printed label in the frame and return a native-resolution crop of it.
///
/// Detection beats asking the model for coordinates: off a downscaled overview it
/// routinely brackets the QR block instead of the text, wasting most of the
/// close-up's pixel budget on a barcode.
#[tauri::command]
pub async fn assistant_prepare_label(
    path: String,
    orientation_steps: Option<u32>,
    max_dim: Option<u32>,
    canvas_width: Option<u32>,
    canvas_height: Option<u32>,
    app_handle: AppHandle,
) -> Result<LabelCrop, String> {
    let max_dim = max_dim.unwrap_or(1568).clamp(256, 4000);
    let steps = orientation_steps.unwrap_or(0) % 4;
    tauri::async_runtime::spawn_blocking(move || {
        let (source_path, sidecar_path) = parse_virtual_path(&path);
        let source_str = source_path.to_string_lossy().to_string();

        let state = app_handle.state::<AppState>();
        let loaded: Option<std::sync::Arc<image::DynamicImage>> = {
            let guard = state.original_image.lock().unwrap();
            guard
                .as_ref()
                .filter(|l| l.path == path || l.path == source_str)
                .map(|l| l.image.clone())
        };
        let full: image::DynamicImage = match loaded {
            Some(arc) => (*arc).clone(),
            None if is_raw_file(&source_str) => crate::library::culling::fast_raw_preview(&source_str)
                .ok_or_else(|| "Failed to extract raw preview".to_string())?,
            None => image::open(&source_path).map_err(|e| format!("Failed to decode: {}", e))?,
        };

        // Same view the model was shown, so the reported rectangle is meaningful
        // in its `_canvas` space.
        let sidecar_adjustments = crate::metadata::sidecar::load_sidecar(&sidecar_path).adjustments;
        let view = view_for_adjustments(full, &sidecar_adjustments, steps);
        let (iw, ih) = (view.width(), view.height());

        let detected = crate::ai::label_detect::detect_label_region(&view);
        let (region, found, rect) = match detected {
            Some(r) => (view.crop_imm(r.x, r.y, r.width, r.height), true, r),
            None => (
                view.clone(),
                false,
                crate::ai::label_detect::Rect {
                    x: 0,
                    y: 0,
                    width: iw,
                    height: ih,
                },
            ),
        };

        // Report the rectangle in the space the model reasons in, not the
        // source's own resolution, which may differ (a 720px thumbnail canvas
        // vs the full raw preview).
        let sx = canvas_width
            .filter(|&cw| cw > 0)
            .map(|cw| cw as f64 / iw as f64)
            .unwrap_or(1.0);
        let sy = canvas_height
            .filter(|&ch| ch > 0)
            .map(|ch| ch as f64 / ih as f64)
            .unwrap_or(1.0);

        let attachment = encode_assistant_jpeg(&region, max_dim)?;
        Ok(LabelCrop {
            media_type: attachment.media_type,
            data: attachment.data,
            found,
            x: (rect.x as f64 * sx).round() as u32,
            y: (rect.y as f64 * sy).round() as u32,
            width: (rect.width as f64 * sx).round() as u32,
            height: (rect.height as f64 * sy).round() as u32,
        })
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

#[tauri::command]
pub async fn assistant_prepare_region(
    path: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    orientation_steps: Option<u32>,
    max_dim: Option<u32>,
    canvas_width: Option<u32>,
    canvas_height: Option<u32>,
    app_handle: AppHandle,
) -> Result<crate::ai::assistant::ImageAttachment, String> {
    let max_dim = max_dim.unwrap_or(1568).clamp(256, 4000);
    let steps = orientation_steps.unwrap_or(0) % 4;
    tauri::async_runtime::spawn_blocking(move || {
        let (source_path, sidecar_path) = parse_virtual_path(&path);
        let source_str = source_path.to_string_lossy().to_string();

        // The open editor image is the common case and already fully decoded.
        let state = app_handle.state::<AppState>();
        let loaded: Option<std::sync::Arc<image::DynamicImage>> = {
            let guard = state.original_image.lock().unwrap();
            guard
                .as_ref()
                .filter(|l| l.path == path || l.path == source_str)
                .map(|l| l.image.clone())
        };
        let full: image::DynamicImage = match loaded {
            Some(arc) => (*arc).clone(),
            None if is_raw_file(&source_str) => crate::library::culling::fast_raw_preview(&source_str)
                .ok_or_else(|| "Failed to extract raw preview".to_string())?,
            None => image::open(&source_path).map_err(|e| format!("Failed to decode: {}", e))?,
        };

        let sidecar_adjustments = crate::metadata::sidecar::load_sidecar(&sidecar_path).adjustments;
        let view = view_for_adjustments(full, &sidecar_adjustments, steps);
        let (x, y, w, h) =
            map_canvas_rect(x, y, width, height, canvas_width, canvas_height, view.width(), view.height());
        let region = view.crop_imm(x, y, w, h);
        encode_assistant_jpeg(&region, max_dim)
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

/// JPEG at quality 90 — the assistant reads text off these, and the thumbnail
/// encoder's q75 visibly smears small glyphs.
fn encode_assistant_jpeg(
    image: &image::DynamicImage,
    max_dim: u32,
) -> Result<crate::ai::assistant::ImageAttachment, String> {
    use base64::Engine as _;
    let (w, h) = (image.width(), image.height());
    let scaled = if w.max(h) > max_dim {
        image.thumbnail(max_dim, max_dim)
    } else {
        image.clone()
    };
    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 90);
    scaled
        .to_rgb8()
        .write_with_encoder(encoder)
        .map_err(|e| format!("Failed to encode image: {}", e))?;
    Ok(crate::ai::assistant::ImageAttachment {
        media_type: "image/jpeg".to_string(),
        data: base64::engine::general_purpose::STANDARD.encode(&buf),
        label: None,
    })
}

fn prefetch_source_file(path_str: &str) {
    let (source_path, _) = parse_virtual_path(path_str);
    if let Ok(mut file) = std::fs::File::open(&source_path) {
        let _ = std::io::copy(&mut file, &mut std::io::sink());
    }
}

pub fn start_thumbnail_workers(app_handle: tauri::AppHandle) {
    let state = app_handle.state::<AppState>();
    let manager = state.thumbnail_manager.clone();
    let settings = load_settings(app_handle.clone()).unwrap_or_default();
    let thread_count = settings.thumbnail_worker_threads.unwrap_or(4).clamp(1, 16);

    for _ in 0..thread_count {
        let app_clone = app_handle.clone();
        let manager_clone = manager.clone();

        std::thread::spawn(move || {
            loop {
                let path_to_process: String = {
                    let mut queue = manager_clone.queue.lock().unwrap();
                    while queue.is_empty() {
                        queue = manager_clone.cvar.wait(queue).unwrap();
                    }
                    let path = queue.pop_back().unwrap();

                    let mut processing = manager_clone.processing_now.lock().unwrap();
                    if processing.contains(&path) {
                        let state = app_clone.state::<AppState>();
                        increment_thumbnail_progress(&state, &app_clone);
                        continue;
                    }
                    processing.insert(path.clone());
                    path
                };

                let state = app_clone.state::<AppState>();
                let gpu_context = get_or_init_gpu_context(&state, &app_clone).ok();

                let current_settings = load_settings(app_clone.clone()).unwrap_or_default();

                if let Ok(cache_dir) = resolve_thumbnail_cache_dir(&app_clone) {
                    let prefetch = || {
                        if manager_clone.rotational_disk.load(Ordering::Relaxed) {
                            let _io_permit = manager_clone.io_gate.lock().unwrap();
                            prefetch_source_file(&path_to_process);
                        }
                    };

                    let result = generate_single_thumbnail_and_cache(
                        &path_to_process,
                        &cache_dir,
                        gpu_context.as_ref(),
                        None,
                        false,
                        &app_clone,
                        &current_settings,
                        Some(&prefetch),
                    );

                    if let Some((small_path, medium_path, rating, is_edited)) = result {
                        emit_thumbnail_generated(
                            &app_clone,
                            &path_to_process,
                            &small_path,
                            &medium_path,
                            rating,
                            is_edited,
                        );
                    }
                    increment_thumbnail_progress(&state, &app_clone);
                }
                manager_clone
                    .processing_now
                    .lock()
                    .unwrap()
                    .remove(&path_to_process);
            }
        });
    }
}

const MAX_QUEUED_THUMBNAILS: usize = 500;

#[tauri::command]
pub fn update_thumbnail_queue(
    paths: Vec<String>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let state = app_handle.state::<AppState>();

    let mut queue = state.thumbnail_manager.queue.lock().unwrap();

    if paths.is_empty() {
        queue.clear();
        let mut tracker = state.thumbnail_progress.lock().unwrap();
        tracker.total = 0;
        tracker.completed = 0;
        drop(tracker);

        let _ = app_handle.emit(
            "thumbnail-progress",
            serde_json::json!({ "current": 0, "total": 0 }),
        );
        state.thumbnail_manager.cvar.notify_all();
        return Ok(());
    }

    let mut unique_paths = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for path in paths {
        if seen.insert(path.clone()) {
            unique_paths.push(path);
        }
    }

    queue.retain(|p| !seen.contains(p));

    unique_paths.truncate(MAX_QUEUED_THUMBNAILS);
    while queue.len() + unique_paths.len() > MAX_QUEUED_THUMBNAILS && queue.pop_front().is_some() {}

    if state
        .thumbnail_manager
        .rotational_disk
        .load(Ordering::Relaxed)
    {
        unique_paths.sort();
        for path in unique_paths.into_iter().rev() {
            queue.push_back(path);
        }
    } else {
        for path in unique_paths {
            queue.push_back(path);
        }
    }

    let queue_len = queue.len();
    drop(queue);

    let mut tracker = state.thumbnail_progress.lock().unwrap();
    tracker.total = tracker.completed + queue_len;

    let current = tracker.completed;
    let total = tracker.total;
    drop(tracker);

    let _ = app_handle.emit(
        "thumbnail-progress",
        serde_json::json!({ "current": current, "total": total }),
    );

    state.thumbnail_manager.cvar.notify_all();
    Ok(())
}

pub fn add_to_thumbnail_queue(state: &AppState, count: usize, app_handle: &AppHandle) {
    let mut tracker = state.thumbnail_progress.lock().unwrap();
    tracker.total += count;
    let current = tracker.completed;
    let total = tracker.total;
    drop(tracker);

    let _ = app_handle.emit(
        "thumbnail-progress",
        serde_json::json!({ "current": current, "total": total }),
    );
}

pub fn increment_thumbnail_progress(state: &AppState, app_handle: &AppHandle) {
    let mut tracker = state.thumbnail_progress.lock().unwrap();
    tracker.completed += 1;
    let current = tracker.completed;
    let total = tracker.total;

    if current >= total {
        tracker.total = 0;
        tracker.completed = 0;
        drop(tracker);

        let _ = app_handle.emit(
            "thumbnail-progress",
            serde_json::json!({ "current": 0, "total": 0 }),
        );
        let _ = app_handle.emit("thumbnail-generation-complete", true);
    } else {
        drop(tracker);
        let _ = app_handle.emit(
            "thumbnail-progress",
            serde_json::json!({ "current": current, "total": total }),
        );
    }
}

pub fn emit_thumbnail_generated(
    app_handle: &AppHandle,
    path: &str,
    small_thumbnail_path: &str,
    medium_thumbnail_path: &str,
    rating: u8,
    is_edited: bool,
) {
    let _ = app_handle.emit(
        "thumbnail-generated",
        serde_json::json!({
            "path": path,
            "thumbnailPath": small_thumbnail_path,
            "previewPath": medium_thumbnail_path,
            "rating": rating,
            "is_edited": is_edited
        }),
    );
}

#[tauri::command]
pub fn clear_thumbnail_cache(app_handle: AppHandle) -> Result<(), String> {
    let cache_dir = app_handle
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?;
    let thumb_cache_dir = cache_dir.join("thumbnails");

    if thumb_cache_dir.exists() {
        fs::remove_dir_all(&thumb_cache_dir)
            .map_err(|e| format!("Failed to remove thumbnail cache: {}", e))?;
    }

    fs::create_dir_all(&thumb_cache_dir)
        .map_err(|e| format!("Failed to recreate thumbnail cache directory: {}", e))?;
    crate::app::storage::reset_thumbnail_estimate();

    Ok(())
}

/// The cached thumbnail of `image_path` rendered WITH its current edits — the
/// medium one, else the small one, else the pre-v1.6.5 single file. None when the
/// cache hasn't been written yet. This is what Auto learns the user's taste from.
pub fn cached_edited_thumbnail(app_handle: &AppHandle, image_path: &str) -> Option<DynamicImage> {
    let dir = resolve_thumbnail_cache_dir(app_handle).ok()?;
    let settings = load_settings(app_handle.clone()).unwrap_or_default();
    let hash = get_cache_key_hash(image_path, &settings)?;
    ["_medium.jpg", "_small.jpg", ".jpg"]
        .iter()
        .map(|suffix| dir.join(format!("{hash}{suffix}")))
        .find(|p| p.exists())
        .and_then(|p| image::open(p).ok())
}

pub fn get_cache_key_hash(path_str: &str, settings: &AppSettings) -> Option<String> {
    let (_, sidecar_path) = parse_virtual_path(path_str);

    let mut metadata = fs::read_to_string(&sidecar_path)
        .ok()
        .and_then(|content| crate::metadata::sidecar::parse_sidecar(&content));
    let mut adjustments_bytes = metadata_adjustments_bytes(&metadata);
    if resolve_unedited_metadata(&mut metadata, path_str) {
        adjustments_bytes = metadata_adjustments_bytes(&metadata);
    }

    compute_thumbnail_cache_hash(path_str, &adjustments_bytes, settings)
}

pub fn get_cached_or_generate_thumbnail_image(
    path_str: &str,
    app_handle: &AppHandle,
    gpu_context: Option<&GpuContext>,
) -> Result<DynamicImage> {
    let thumb_cache_dir =
        resolve_thumbnail_cache_dir(app_handle).map_err(|e| anyhow::anyhow!(e))?;
    let settings = load_settings(app_handle.clone()).unwrap_or_default();
    let target_width_small = settings.small_thumbnail_resolution.unwrap_or(480);
    let target_width_medium = settings.medium_thumbnail_resolution.unwrap_or(1280);

    if let Some(cache_hash) = get_cache_key_hash(path_str, &settings) {
        let cache_path = thumb_cache_dir.join(format!("{}_medium.jpg", cache_hash));

        if cache_path.exists() {
            if let Ok(image) = image::open(&cache_path) {
                crate::app::storage::mark_used(&cache_path);
                return Ok(image);
            }
            eprintln!(
                "Could not open cached thumbnail, regenerating: {:?}",
                cache_path
            );
        }

        let thumb_image = generate_thumbnail_data(path_str, gpu_context, None, app_handle)?;
        if let (Ok(small_data), Ok(medium_data)) = (
            encode_thumbnail(&thumb_image, target_width_small),
            encode_thumbnail(&thumb_image, target_width_medium),
        ) {
            let _ = fs::write(
                thumb_cache_dir.join(format!("{}_small.jpg", cache_hash)),
                &small_data,
            );
            let _ = fs::write(
                thumb_cache_dir.join(format!("{}_medium.jpg", cache_hash)),
                &medium_data,
            );
            crate::app::storage::record_thumbnail_write(
                (small_data.len() + medium_data.len()) as u64,
            );
        }

        Ok(thumb_image)
    } else {
        generate_thumbnail_data(path_str, gpu_context, None, app_handle)
    }
}

pub(crate) fn regenerate_thumbnails(
    paths: impl Iterator<Item = String> + Send,
    app_handle: &AppHandle,
    settings: &AppSettings,
    force_regenerate: bool,
) {
    let state = app_handle.state::<AppState>();
    let thumb_cache_dir = match resolve_thumbnail_cache_dir(app_handle) {
        Ok(dir) => dir,
        Err(e) => {
            log::warn!("Unable to initialize thumbnail cache directory: {}", e);
            for path in paths {
                emit_thumbnail_cache_setup_error(app_handle, &path, &e);
                increment_thumbnail_progress(&state, app_handle);
            }
            return;
        }
    };

    let gpu_context = get_or_init_gpu_context(&state, app_handle).ok();

    run_batch_workers(paths, |path| {
        let path_str = path.as_str();
        let result = generate_single_thumbnail_and_cache(
            path_str,
            &thumb_cache_dir,
            gpu_context.as_ref(),
            None,
            force_regenerate,
            app_handle,
            settings,
            None,
        );

        if let Some((small_path, medium_path, rating, is_edited)) = result {
            emit_thumbnail_generated(
                app_handle,
                path_str,
                &small_path,
                &medium_path,
                rating,
                is_edited,
            );
        }

        increment_thumbnail_progress(&state, app_handle);
    });
}

/// Regenerate grid/filmstrip thumbnails for the given paths and push each to the
/// frontend via `thumbnail-generated` (fork; used by the negative-conversion commands).
pub fn regenerate_thumbnails_for_paths(paths: &[String], app_handle: &AppHandle) {
    let state = app_handle.state::<AppState>();
    let settings = load_settings(app_handle.clone()).unwrap_or_default();
    add_to_thumbnail_queue(&state, paths.len(), app_handle);
    regenerate_thumbnails(paths.iter().cloned(), app_handle, &settings, true);
}
