// SD-card (or any folder) import module: drive detection + recursive image scan.
//
// This module is intentionally thin and self-contained. The heavy lifting is reused:
//   - similarity grouping / quality / blurry analysis -> `crate::culling::run_culling`
//   - the actual copy/rename/organize import           -> `file_management::import_files`
// Only the source discovery (removable drives + recursive scan) is new here.

use base64::{Engine as _, engine::general_purpose};
use rawler::decoders::RawDecodeParams;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use sysinfo::Disks;
use tauri::{AppHandle, Emitter, State};
use walkdir::WalkDir;

use crate::culling::{
    CullingSettings, CullingSuggestions, ImageAnalysisData, analyze_paths, group_analyses,
    group_by_time,
};
use crate::formats::{is_raw_file, is_supported_image_file};

// Cache of the last import analysis so the similarity slider can re-group instantly
// (group_for_import) without re-decoding every photo.
static IMPORT_ANALYSIS: Mutex<Option<Vec<ImageAnalysisData>>> = Mutex::new(None);
static IMPORT_FAILED: Mutex<Vec<String>> = Mutex::new(Vec::new());
// Per-photo interpretable cue features from the last scoring run, keyed by path. Used to
// fold your keep/skip decisions into the "learn from my picks" model (record_cull_picks).
static IMPORT_FEATURES: Mutex<
    Option<std::collections::HashMap<String, [f64; crate::cull_model::N_FEATURES]>>,
> = Mutex::new(None);
// Capture timestamps (epoch seconds) per scanned path, for time-based (burst) grouping.
static IMPORT_TIMES: Mutex<Option<std::collections::HashMap<String, i64>>> = Mutex::new(None);

/// Group the cached analyses by the requested mode: "time" → burst-by-capture-time using the
/// timestamp cache; anything else → visual perceptual-hash similarity.
fn group_cached(
    analyses: &[ImageAnalysisData],
    failed: Vec<String>,
    settings: &CullingSettings,
    mode: &str,
    time_gap_seconds: i64,
) -> CullingSuggestions {
    if mode == "time" {
        let guard = IMPORT_TIMES.lock().unwrap();
        let empty = std::collections::HashMap::new();
        let times = guard.as_ref().unwrap_or(&empty);
        group_by_time(analyses, failed, times, time_gap_seconds)
    } else {
        group_analyses(analyses, failed, settings)
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    pub name: String,
    pub path: String,
    pub is_removable: bool,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

/// List mounted volumes, removable ones first. We include non-removable volumes too,
/// because many USB card readers / cameras report as fixed disks; the frontend labels
/// removable ones but still lets the user pick any.
#[tauri::command]
pub fn list_source_drives() -> Result<Vec<DriveInfo>, String> {
    let disks = Disks::new_with_refreshed_list();

    let mut drives: Vec<DriveInfo> = disks
        .list()
        .iter()
        .map(|disk| {
            let mount = disk.mount_point().to_string_lossy().to_string();
            let raw_name = disk.name().to_string_lossy().to_string();
            // Fall back to the last path component of the mount point for a friendly name.
            let name = if raw_name.trim().is_empty() {
                disk.mount_point()
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| mount.clone())
            } else {
                raw_name
            };

            DriveInfo {
                name,
                path: mount,
                is_removable: disk.is_removable(),
                total_bytes: disk.total_space(),
                available_bytes: disk.available_space(),
            }
        })
        .collect();

    // Hide obvious system/pseudo mounts that are never an import source.
    drives.retain(|d| {
        let p = d.path.as_str();
        !(p == "/dev" || p.starts_with("/System/Volumes/") || p.starts_with("/private/var/vm"))
    });

    drives.sort_by(|a, b| {
        b.is_removable
            .cmp(&a.is_removable)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(drives)
}

/// Recursively collect supported image paths under `path` (SD cards nest images under
/// `DCIM/100XXXXX/`, so a recursive walk is required).
#[tauri::command]
pub fn scan_source_images(path: String) -> Result<Vec<String>, String> {
    let root = Path::new(&path);
    if !root.exists() {
        return Err(format!("Source path does not exist: {path}"));
    }

    let mut paths: Vec<String> = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        // Skip macOS AppleDouble resource-fork stubs (`._NAME.JPG`) that get written to
        // FAT/exFAT cards. They share a real image extension but are tiny metadata files
        // that only ever fail to decode.
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|n| !n.starts_with("._"))
                .unwrap_or(true)
        })
        .filter(|entry| is_supported_image_file(entry.path()))
        .map(|entry| entry.path().to_string_lossy().to_string())
        .collect();

    paths.sort();
    Ok(paths)
}

/// Best-effort EXIF capture date (epoch seconds) per path, so the importer grid can offer a
/// "sort by date" option. Reuses the same date resolution as time-based grouping
/// (`get_creation_date_from_path`): EXIF DateTimeOriginal first, then RAW metadata, falling
/// back to the file's creation time. Read in parallel; never fails (missing dates just sort old).
#[tauri::command]
pub fn get_capture_times(
    paths: Vec<String>,
) -> Result<std::collections::HashMap<String, i64>, String> {
    use rayon::prelude::*;
    let times = paths
        .par_iter()
        .map(|p| {
            let t = crate::exif_processing::get_creation_date_from_path(Path::new(p)).timestamp();
            (p.clone(), t)
        })
        .collect();
    Ok(times)
}

/// Pre-import culling. Identical analysis to `cull_images`, but on an isolated event
/// channel (`sd-import-cull-*`) so running it never pops the post-import CullingModal.
#[tauri::command]
pub async fn cull_images_for_import(
    paths: Vec<String>,
    settings: CullingSettings,
    app_handle: AppHandle,
) -> Result<CullingSuggestions, String> {
    crate::culling::run_culling(paths, settings, app_handle, "sd-import-cull").await
}

/// Analyze the photos for GROUPING only: decode + perceptual hash, no quality scoring.
/// This keeps "Group similar" fast — scoring is a separate, opt-in step. Emits
/// `sd-import-cull-*` progress. Returns how many analyzed successfully.
#[tauri::command]
pub async fn analyze_for_import(
    paths: Vec<String>,
    app_handle: AppHandle,
) -> Result<usize, String> {
    use rayon::prelude::*;
    // Capture timestamps up front (parallel, best-effort) so time-based grouping is instant.
    let times: std::collections::HashMap<String, i64> = paths
        .par_iter()
        .map(|p| {
            let t = crate::exif_processing::get_creation_date_from_path(Path::new(p)).timestamp();
            (p.clone(), t)
        })
        .collect();

    let (analyses, failed) = analyze_paths(paths, app_handle, "sd-import-cull", false).await?;
    let count = analyses.len();
    *IMPORT_ANALYSIS.lock().unwrap() = Some(analyses);
    *IMPORT_FAILED.lock().unwrap() = failed;
    *IMPORT_TIMES.lock().unwrap() = Some(times);
    Ok(count)
}

/// Group the cached analysis. `mode` = "time" → burst-by-capture-time (gap = `time_gap_seconds`);
/// otherwise visual similarity at the slider's threshold. Cheap (no I/O), so the slider /
/// mode toggle can call it live. Ranks each group by quality score.
#[tauri::command]
pub fn group_for_import(
    settings: CullingSettings,
    mode: String,
    time_gap_seconds: i64,
) -> Result<CullingSuggestions, String> {
    let guard = IMPORT_ANALYSIS.lock().unwrap();
    let failed = IMPORT_FAILED.lock().unwrap().clone();
    match guard.as_ref() {
        Some(analyses) => Ok(group_cached(
            analyses,
            failed,
            &settings,
            &mode,
            time_gap_seconds,
        )),
        None => Ok(CullingSuggestions::default()),
    }
}

/// Scoring step (the "AI score"): compute quality metrics for the cached photos and fill
/// them into the cached analysis. Emits `sd-import-score-*` progress. After this, call
/// `group_for_import` again to get groups ranked + a "best of group" pick.
///
/// Two passes: (1) fast technical metrics (sharpness / center-focus / exposure) in
/// parallel, then (2) a CLIP zero-shot "people quality" check that rewards frames where
/// everyone looks at the camera with open eyes and penalises looking-away / blinking
/// frames, so the per-group best pick prefers the attentive shot. The CLIP pass is
/// best-effort — if the model isn't available (offline / not downloaded) we keep the
/// technical score unchanged.
#[tauri::command]
pub async fn score_for_import(
    app_handle: AppHandle,
    state: State<'_, crate::AppState>,
    group_settings: CullingSettings,
    mode: String,
    time_gap_seconds: i64,
    personalize: bool,
) -> Result<usize, String> {
    use rayon::prelude::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let paths: Vec<String> = {
        let guard = IMPORT_ANALYSIS.lock().unwrap();
        match guard.as_ref() {
            Some(v) => v.iter().map(|d| d.result_path()).collect(),
            None => return Ok(0),
        }
    };
    if paths.is_empty() {
        return Ok(0);
    }

    // The face/eye factor only changes the per-group "best" ranking — ungrouped singles are
    // auto-kept regardless of score — so we only run the (expensive) face pass on images
    // that actually belong to a multi-image similar group at the current threshold.
    let grouped: std::collections::HashSet<String> = {
        let guard = IMPORT_ANALYSIS.lock().unwrap();
        let failed = IMPORT_FAILED.lock().unwrap().clone();
        match guard.as_ref() {
            Some(analyses) => {
                let sugg = group_cached(analyses, failed, &group_settings, &mode, time_gap_seconds);
                let mut set = std::collections::HashSet::new();
                for g in &sugg.similar_groups {
                    set.insert(g.representative.path.clone());
                    for d in &g.duplicates {
                        set.insert(d.path.clone());
                    }
                }
                set
            }
            None => std::collections::HashSet::new(),
        }
    };

    let settings = crate::app_settings::load_settings(app_handle.clone()).unwrap_or_default();
    let total = paths.len();
    let done = Arc::new(AtomicUsize::new(0));
    let _ = app_handle.emit("sd-import-score-start", total);

    // Pass 1 — technical metrics, in parallel.
    let tech: std::collections::HashMap<String, (f64, f64, f64, f64)> = paths
        .par_iter()
        .filter_map(|p| {
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = app_handle.emit(
                "sd-import-score-progress",
                serde_json::json!({ "current": n, "total": total, "stage": "Scoring photos…" }),
            );
            crate::culling::score_image(p, &settings)
                .ok()
                .map(|s| (p.clone(), s))
        })
        .collect();

    // Pass 2 — face-aware "people quality": detect faces, then run the open-eyes /
    // looking-at-camera check on each cropped face so a single blinker demotes the frame
    // (serial; the CLIP + face sessions are single-threaded). Best-effort: skip silently if
    // either model can't be loaded (offline / not downloaded).
    let clip = crate::ai_processing::get_or_init_clip_models(
        &app_handle,
        &state.ai_state,
        &state.ai_init_lock,
    )
    .await;
    let face = crate::ai_processing::get_or_init_face_model(
        &app_handle,
        &state.ai_state,
        &state.ai_init_lock,
    )
    .await;
    // People shots → per-face cues. Non-people shots (landscape/product/food/still life) →
    // subject-region sharpness + a CLIP aesthetic/composition cue: [overall, best_region, aesthetic].
    let mut people: std::collections::HashMap<String, crate::tagging::FaceCues> =
        std::collections::HashMap::new();
    let mut nonpeople: std::collections::HashMap<String, [f32; 3]> =
        std::collections::HashMap::new();
    match (clip, face) {
        (Ok(clip), Ok(face)) => {
            let face_paths: Vec<&String> = paths.iter().filter(|p| grouped.contains(*p)).collect();
            let face_total = face_paths.len();
            for (i, p) in face_paths.iter().enumerate() {
                let _ = app_handle.emit("sd-import-score-progress", serde_json::json!({ "current": i + 1, "total": face_total, "stage": "Analyzing photos (faces, focus, composition)…" }));
                if let Ok(img) = crate::culling::load_face_image(p, &settings) {
                    match crate::tagging::score_faces(&img, &face, &clip.model, &clip.tokenizer) {
                        Ok(Some(cues)) => {
                            people.insert((*p).clone(), cues);
                        }
                        Ok(None) => {
                            // No faces: rank by subject-region sharpness + composition instead
                            // of the centre-focus assumption.
                            let overall = crate::culling::normalized_sharpness_of(&img);
                            let best = crate::culling::best_region_sharpness(&img);
                            let aesthetic =
                                crate::tagging::clip_aesthetic(&img, &clip.model, &clip.tokenizer)
                                    .unwrap_or(0.5);
                            nonpeople.insert((*p).clone(), [overall, best, aesthetic]);
                        }
                        Err(_) => {}
                    }
                }
            }
        }
        (clip_res, face_res) => {
            if let Err(e) = clip_res {
                eprintln!("AI scoring skipped (CLIP unavailable): {e}");
            }
            if let Err(e) = face_res {
                eprintln!("AI scoring skipped (face model unavailable): {e}");
            }
        }
    };

    // Score people shots via the learned/default cue-weight model and remember each photo's
    // feature vector so record_cull_picks can learn from the final keep/skip decisions.
    let model = crate::cull_model::CullModel::load(&app_handle);
    let mut features_map: std::collections::HashMap<String, [f64; crate::cull_model::N_FEATURES]> =
        std::collections::HashMap::new();
    {
        let mut guard = IMPORT_ANALYSIS.lock().unwrap();
        if let Some(v) = guard.as_mut() {
            for d in v.iter_mut() {
                if let Some(&(q, sh, cf, ex)) = tech.get(&d.result_path()) {
                    // For people shots (faces detected) the faces drive the score via the cue
                    // model: technical + face-sharpness + looking-at-camera + expression +
                    // eyes-open. Photos with no faces keep the technical score.
                    let q_adj = if let Some(c) = people.get(&d.result_path()) {
                        let f = [
                            q,
                            c.face_sharp as f64,
                            c.look_mean as f64,
                            c.look_worst as f64,
                            c.expr_mean as f64,
                            c.eyes_worst as f64,
                        ];
                        features_map.insert(d.result_path(), f);
                        model.score(&f, personalize)
                    } else if let Some(&[overall, best, aesthetic]) =
                        nonpeople.get(&d.result_path())
                    {
                        // Non-people: subject-region focus + composition, de-emphasising the
                        // centre-focus assumption that doesn't fit off-centre subjects.
                        (0.30 * overall as f64
                            + 0.30 * best as f64
                            + 0.20 * ex
                            + 0.20 * aesthetic as f64)
                            .clamp(0.0, 1.0)
                    } else {
                        q
                    };
                    d.set_scores(q_adj, sh, cf, ex);
                }
            }
        }
    }
    *IMPORT_FEATURES.lock().unwrap() = Some(features_map);

    let _ = app_handle.emit("sd-import-score-complete", tech.len());
    Ok(tech.len())
}

/// Record the user's keep/skip decisions for the just-finished import into the on-device
/// "learn from my picks" model. `kept`/`skipped` are paths from multi-image similar groups
/// (we learn which frame you preferred over the group-mates you rejected). Returns the total
/// number of samples learned so far. No-op unless both a kept and a skipped grouped photo
/// (with cached cue features) are supplied.
#[tauri::command]
pub fn record_cull_picks(
    app_handle: AppHandle,
    kept: Vec<String>,
    skipped: Vec<String>,
) -> Result<u64, String> {
    let guard = IMPORT_FEATURES.lock().unwrap();
    let map = match guard.as_ref() {
        Some(m) => m,
        None => return Ok(0),
    };
    let chosen: Vec<[f64; crate::cull_model::N_FEATURES]> =
        kept.iter().filter_map(|p| map.get(p).copied()).collect();
    let rejected: Vec<[f64; crate::cull_model::N_FEATURES]> =
        skipped.iter().filter_map(|p| map.get(p).copied()).collect();
    if chosen.is_empty() || rejected.is_empty() {
        return Ok(0);
    }
    let mut model = crate::cull_model::CullModel::load(&app_handle);
    model.record(&chosen, &rejected);
    model.save(&app_handle)?;
    Ok(model.sample_count())
}

/// Forget everything the culling model has learned (back to default cue weights).
#[tauri::command]
pub fn reset_cull_model(app_handle: AppHandle) -> Result<(), String> {
    crate::cull_model::CullModel::reset(&app_handle)
}

/// Eject / unmount the removable volume at `mount_point` (e.g. after import).
#[tauri::command]
pub fn eject_drive(mount_point: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("diskutil")
            .args(["eject", &mount_point])
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
    #[cfg(target_os = "linux")]
    {
        let out = std::process::Command::new("umount")
            .arg(&mount_point)
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        // Windows has no simple eject CLI, so drive the same shell "Eject" verb that
        // Explorer's right-click -> Eject uses: it flushes caches and safely removes the
        // volume without elevation. `mount_point` from sysinfo looks like "E:\".
        let drive = mount_point.trim_end_matches(['\\', '/']).to_string();
        if drive.len() < 2 {
            return Err(format!("Invalid drive path: {mount_point}"));
        }

        // Fire the Eject verb, then poll (up to ~3s) until the drive letter disappears so
        // we can report a real success/failure instead of firing blindly.
        let script = format!(
            "$ErrorActionPreference='Stop'; \
             $drive='{drive}'; \
             $item=(New-Object -ComObject Shell.Application).Namespace(17).ParseName($drive); \
             if ($null -eq $item) {{ throw \"Drive $drive not found\" }} \
             $item.InvokeVerb('Eject'); \
             for ($i=0; $i -lt 20; $i++) {{ if (-not (Test-Path -LiteralPath ($drive + '\\'))) {{ exit 0 }} Start-Sleep -Milliseconds 150 }} \
             throw 'The card is still mounted; it may still be in use.'"
        );
        // Pass as UTF-16LE base64 so we don't fight cmd/PowerShell quoting rules.
        let wide: Vec<u8> = script
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        let encoded = general_purpose::STANDARD.encode(&wide);

        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-EncodedCommand", &encoded])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            let msg = String::from_utf8_lossy(&out.stderr);
            let msg = msg.trim();
            if msg.is_empty() {
                Err("Failed to eject the card.".to_string())
            } else {
                Err(msg.to_string())
            }
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = mount_point;
        Err("Eject is not supported on this platform.".into())
    }
}

// Hash a file's bytes so a photo renamed after import is still matched. Streamed
// in chunks rather than read whole: RAW files run to tens of megabytes, and this
// is only reached once a same-size candidate exists.
fn file_content_hash(path: &Path) -> Option<blake3::Hash> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; 128 * 1024];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                hasher.update(&buffer[..read]);
            }
            Err(_) => return None,
        }
    }
    Some(hasher.finalize())
}

/// Given source files and a destination folder, return the subset of source paths
/// already present anywhere under the destination (recursively), matched by filename
/// stem or by file content. Used to skip re-importing photos already in the library.
#[tauri::command]
pub fn find_existing_in_destination(
    source_paths: Vec<String>,
    destination_folder: String,
) -> Result<Vec<String>, String> {
    let dest = Path::new(&destination_folder);
    if !dest.exists() {
        return Ok(vec![]);
    }

    // Two independent signals, because either one can be the only match:
    //   - filename STEM, so a shot already imported as RAW also flags its JPEG
    //     counterpart on the card (and vice-versa);
    //   - file CONTENT, so a photo renamed in the library after import (or renamed
    //     on the way in by a filename template) is still recognised. Stem alone
    //     missed those and offered them as new.
    let mut existing_stems: HashSet<String> = HashSet::new();
    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    for entry in WalkDir::new(dest)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file() && is_supported_image_file(entry.path()) {
            if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                existing_stems.insert(stem.to_lowercase());
            }
            if let Ok(meta) = entry.metadata() {
                by_size
                    .entry(meta.len())
                    .or_default()
                    .push(entry.path().to_path_buf());
            }
        }
    }

    let mut dest_hashes: HashMap<PathBuf, blake3::Hash> = HashMap::new();
    let mut matches = Vec::new();

    for source in source_paths {
        let path = Path::new(&source);

        let stem_hit = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| existing_stems.contains(&s.to_lowercase()))
            .unwrap_or(false);
        if stem_hit {
            matches.push(source);
            continue;
        }

        // Only hash when the destination holds a file of exactly the same byte
        // length. The common "nothing matches" case then costs metadata reads
        // alone, which matters when the card holds hundreds of RAWs.
        let Ok(meta) = std::fs::metadata(path) else {
            continue;
        };
        let Some(candidates) = by_size.get(&meta.len()) else {
            continue;
        };
        let Some(source_hash) = file_content_hash(path) else {
            continue;
        };

        let mut content_hit = false;
        for candidate in candidates {
            let hash = match dest_hashes.get(candidate) {
                Some(hash) => Some(*hash),
                None => {
                    let computed = file_content_hash(candidate);
                    if let Some(hash) = computed {
                        dest_hashes.insert(candidate.clone(), hash);
                    }
                    computed
                }
            };
            if hash == Some(source_hash) {
                content_hit = true;
                break;
            }
        }
        if content_hit {
            matches.push(source);
        }
    }

    Ok(matches)
}

/// Read just the embedded full-size JPEG preview bytes from a Fujifilm RAF via a lazy
/// memory map (only the preview region is faulted in). Returns None for other layouts.
fn read_fuji_embedded_jpeg(path: &str) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mmap = unsafe { memmap2::Mmap::map(&file).ok()? };
    let buf: &[u8] = &mmap;
    if buf.starts_with(b"FUJIFILMCCD-RAW") {
        let off = u32::from_be_bytes(buf.get(84..88)?.try_into().ok()?) as usize;
        let len = u32::from_be_bytes(buf.get(88..92)?.try_into().ok()?) as usize;
        let jpeg = buf.get(off..off.checked_add(len)?)?;
        if jpeg.starts_with(&[0xFF, 0xD8]) {
            return Some(jpeg.to_vec());
        }
    }
    None
}

/// Full-resolution preview for the loupe / enlarged view, returned as a data URL the
/// webview can show directly. For raws this is the embedded camera JPEG (full size);
/// for ordinary images it's the file itself. On demand — one image at a time.
#[tauri::command]
pub fn get_import_preview(path: String) -> Result<String, String> {
    if is_raw_file(&path) {
        if let Some(jpeg) = read_fuji_embedded_jpeg(&path) {
            return Ok(format!(
                "data:image/jpeg;base64,{}",
                general_purpose::STANDARD.encode(&jpeg)
            ));
        }
        // Other raw formats: let rawler pull the embedded preview, then re-encode to JPEG.
        if let Ok(img) = rawler::analyze::extract_preview_pixels(&path, &RawDecodeParams::default())
        {
            let mut buf = std::io::Cursor::new(Vec::new());
            img.write_to(&mut buf, image::ImageFormat::Jpeg)
                .map_err(|e| e.to_string())?;
            return Ok(format!(
                "data:image/jpeg;base64,{}",
                general_purpose::STANDARD.encode(buf.get_ref())
            ));
        }
        return Err("Could not extract a preview from this raw file".into());
    }

    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let mime = match path.rsplit('.').next().map(|e| e.to_lowercase()).as_deref() {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        _ => "image/jpeg",
    };
    Ok(format!(
        "data:{};base64,{}",
        mime,
        general_purpose::STANDARD.encode(&bytes)
    ))
}
