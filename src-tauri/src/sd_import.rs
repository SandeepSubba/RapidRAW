// SD-card (or any folder) import module: drive detection + recursive image scan.
//
// This module is intentionally thin and self-contained. The heavy lifting is reused:
//   - similarity grouping / quality / blurry analysis -> `crate::culling::run_culling`
//   - the actual copy/rename/organize import           -> `file_management::import_files`
// Only the source discovery (removable drives + recursive scan) is new here.

use base64::{Engine as _, engine::general_purpose};
use rawler::decoders::RawDecodeParams;
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;
use sysinfo::Disks;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

use crate::culling::{
    CullingSettings, CullingSuggestions, ImageAnalysisData, analyze_paths, group_analyses,
};
use crate::formats::{is_raw_file, is_supported_image_file};

// Cache of the last import analysis so the similarity slider can re-group instantly
// (group_for_import) without re-decoding every photo.
static IMPORT_ANALYSIS: Mutex<Option<Vec<ImageAnalysisData>>> = Mutex::new(None);
static IMPORT_FAILED: Mutex<Vec<String>> = Mutex::new(Vec::new());

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
pub async fn analyze_for_import(paths: Vec<String>, app_handle: AppHandle) -> Result<usize, String> {
    let (analyses, failed) = analyze_paths(paths, app_handle, "sd-import-cull", false).await?;
    let count = analyses.len();
    *IMPORT_ANALYSIS.lock().unwrap() = Some(analyses);
    *IMPORT_FAILED.lock().unwrap() = failed;
    Ok(count)
}

/// Group the cached analysis at the given similarity threshold. Cheap (no I/O), so the
/// similarity slider can call it live on every change. Ranks each group by quality score
/// (0 until the scoring step has run, in which case order is just the scan order).
#[tauri::command]
pub fn group_for_import(settings: CullingSettings) -> Result<CullingSuggestions, String> {
    let guard = IMPORT_ANALYSIS.lock().unwrap();
    let failed = IMPORT_FAILED.lock().unwrap().clone();
    match guard.as_ref() {
        Some(analyses) => Ok(group_analyses(analyses, failed, &settings)),
        None => Ok(CullingSuggestions::default()),
    }
}

/// Scoring step (the "AI score"): compute quality metrics for the cached photos and fill
/// them into the cached analysis. Emits `sd-import-score-*` progress. After this, call
/// `group_for_import` again to get groups ranked + a "best of group" pick. This is where
/// future AI metrics (eyes-closed, composition, content) will be added.
#[tauri::command]
pub async fn score_for_import(app_handle: AppHandle) -> Result<usize, String> {
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

    let settings = crate::app_settings::load_settings(app_handle.clone()).unwrap_or_default();
    let total = paths.len();
    let done = Arc::new(AtomicUsize::new(0));
    let _ = app_handle.emit("sd-import-score-start", total);

    let scores: std::collections::HashMap<String, (f64, f64, f64, f64)> = paths
        .par_iter()
        .filter_map(|p| {
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = app_handle.emit("sd-import-score-progress", serde_json::json!({ "current": n, "total": total, "stage": "Scoring photos…" }));
            crate::culling::score_image(p, &settings).ok().map(|s| (p.clone(), s))
        })
        .collect();

    {
        let mut guard = IMPORT_ANALYSIS.lock().unwrap();
        if let Some(v) = guard.as_mut() {
            for d in v.iter_mut() {
                if let Some(&(q, sh, cf, ex)) = scores.get(&d.result_path()) {
                    d.set_scores(q, sh, cf, ex);
                }
            }
        }
    }

    let _ = app_handle.emit("sd-import-score-complete", scores.len());
    Ok(scores.len())
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
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = mount_point;
        Err("Eject is not supported on this platform.".into())
    }
}

/// Given source files and a destination folder, return the subset of source paths whose
/// filename already exists anywhere under the destination (recursively). Used to skip
/// re-importing photos that are already in the library.
#[tauri::command]
pub fn find_existing_in_destination(
    source_paths: Vec<String>,
    destination_folder: String,
) -> Result<Vec<String>, String> {
    let dest = Path::new(&destination_folder);
    if !dest.exists() {
        return Ok(vec![]);
    }

    // Match by filename STEM (base name without extension) so a shot already imported
    // as RAW also flags its JPEG counterpart on the card (and vice-versa).
    let mut existing: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in WalkDir::new(dest).follow_links(false).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() && is_supported_image_file(entry.path()) {
            if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                existing.insert(stem.to_lowercase());
            }
        }
    }

    let matches = source_paths
        .into_iter()
        .filter(|p| {
            Path::new(p)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| existing.contains(&s.to_lowercase()))
                .unwrap_or(false)
        })
        .collect();
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
            return Ok(format!("data:image/jpeg;base64,{}", general_purpose::STANDARD.encode(&jpeg)));
        }
        // Other raw formats: let rawler pull the embedded preview, then re-encode to JPEG.
        if let Ok(img) = rawler::analyze::extract_preview_pixels(&path, &RawDecodeParams::default()) {
            let mut buf = std::io::Cursor::new(Vec::new());
            img.write_to(&mut buf, image::ImageFormat::Jpeg).map_err(|e| e.to_string())?;
            return Ok(format!("data:image/jpeg;base64,{}", general_purpose::STANDARD.encode(buf.get_ref())));
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
    Ok(format!("data:{};base64,{}", mime, general_purpose::STANDARD.encode(&bytes)))
}
