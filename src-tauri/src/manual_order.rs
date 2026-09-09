// Per-folder manual sort order.
//
// Stored in the app cache directory rather than beside the images: upstream
// deliberately moved folder-level state (the EXIF cache) out of image folders,
// and a photo directory should not accumulate app bookkeeping files.
//
// Only filenames are stored, not full paths, so a folder keeps its order if it
// is moved or the drive letter changes.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Default)]
struct OrderFile {
    /// Filenames in user-chosen order.
    order: Vec<String>,
}

fn cache_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_handle
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("manual_order");
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(dir)
}

/// One file per folder, named by a hash of the folder path — the path itself is
/// not a legal filename, and hashing keeps the name a fixed length.
fn order_path(app_handle: &AppHandle, folder: &str) -> Result<PathBuf, String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Case-insensitive on Windows, where the same folder can arrive with
    // different casing and would otherwise get two separate orders.
    let key = if cfg!(windows) {
        folder.to_lowercase()
    } else {
        folder.to_string()
    };
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    Ok(cache_dir(app_handle)?.join(format!("{:016x}.json", hasher.finish())))
}

/// The saved order for a folder, or an empty list when it has never been sorted
/// manually. Never an error for "no order yet" — the caller treats empty as
/// "fall back to the current sort".
#[tauri::command]
pub fn load_manual_order(folder: String, app_handle: AppHandle) -> Result<Vec<String>, String> {
    let path = order_path(&app_handle, &folder)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(serde_json::from_str::<OrderFile>(&text)
        .map(|f| f.order)
        .unwrap_or_default())
}

#[tauri::command]
pub fn save_manual_order(
    folder: String,
    order: Vec<String>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let path = order_path(&app_handle, &folder)?;
    let text = serde_json::to_string(&OrderFile { order }).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| e.to_string())
}

/// Merge a saved order with what is actually in the folder now.
///
/// Files added since the order was saved are appended rather than dropped, and
/// names that have gone are ignored, so an order survives imports, deletions and
/// renames without needing to be rebuilt.
pub fn apply_order(saved: &[String], present: &[String]) -> Vec<String> {
    let rank: HashMap<&str, usize> = saved
        .iter()
        .enumerate()
        .map(|(i, name)| (name.as_str(), i))
        .collect();

    let mut known: Vec<&String> = Vec::new();
    let mut added: Vec<&String> = Vec::new();
    for name in present {
        if rank.contains_key(name.as_str()) {
            known.push(name);
        } else {
            added.push(name);
        }
    }
    known.sort_by_key(|n| rank[n.as_str()]);
    // New arrivals go to the end, in whatever order they came in.
    known.into_iter().chain(added).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn keeps_the_saved_order() {
        let got = apply_order(&v(&["c.cr2", "a.cr2", "b.cr2"]), &v(&["a.cr2", "b.cr2", "c.cr2"]));
        assert_eq!(got, v(&["c.cr2", "a.cr2", "b.cr2"]));
    }

    #[test]
    fn appends_files_added_since_the_order_was_saved() {
        let got = apply_order(&v(&["b.cr2", "a.cr2"]), &v(&["a.cr2", "b.cr2", "new.cr2"]));
        assert_eq!(got, v(&["b.cr2", "a.cr2", "new.cr2"]), "new file must not be dropped");
    }

    #[test]
    fn ignores_names_that_are_gone() {
        let got = apply_order(&v(&["b.cr2", "deleted.cr2", "a.cr2"]), &v(&["a.cr2", "b.cr2"]));
        assert_eq!(got, v(&["b.cr2", "a.cr2"]));
    }

    #[test]
    fn no_saved_order_leaves_the_folder_untouched() {
        let present = v(&["a.cr2", "b.cr2", "c.cr2"]);
        assert_eq!(apply_order(&[], &present), present);
    }

    #[test]
    fn every_present_file_appears_exactly_once() {
        let present = v(&["a", "b", "c", "d"]);
        let got = apply_order(&v(&["d", "x", "b"]), &present);
        assert_eq!(got.len(), present.len());
        let mut sorted = got.clone();
        sorted.sort();
        let mut expect = present.clone();
        expect.sort();
        assert_eq!(sorted, expect, "no file may be lost or duplicated");
    }
}
