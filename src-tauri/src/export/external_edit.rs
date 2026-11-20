//! Fork: "Edit in External Editor" — render the image's current adjustments
//! to an interchange file next to the original, open it in the configured
//! editor and watch it for saves. Upstream's "Edit In" apps
//! (`integrations::edit_in`) are the newer round-trip; both are offered.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use serde_json::Value;
use tauri::Emitter;

use crate::app::settings::load_settings;
use crate::app::state::AppState;
use crate::export::export_images_and_wait;
use crate::export::settings::{ExportAdjustmentsMode, ExportSettings, TiffBitDepth};
use crate::library::sidecars::parse_virtual_path;

// ---------------------------------------------------------------------------
// Edit in external editor (Photoshop, Affinity, …): render the image's current
// adjustments to an interchange file next to the original, open it in the
// configured editor, and watch it — each time the editor saves, the frontend
// is told so the library refreshes and the result shows beside the original.

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalEditLaunch {
    pub output_path: String,
    pub editor: String,
}

static EXTERNAL_EDIT_WATCHES: Mutex<std::collections::BTreeSet<String>> =
    Mutex::new(std::collections::BTreeSet::new());

/// Open `file` in the configured editor, or the OS default handler when none
/// is set. Returns a short label describing what was launched.
fn launch_external_editor(editor: &str, file: &str) -> Result<String, String> {
    use std::process::Command;
    if editor.is_empty() {
        #[cfg(target_os = "windows")]
        Command::new("cmd")
            .args(["/C", "start", "", file])
            .spawn()
            .map_err(|e| format!("Couldn't open the file with the default app: {}", e))?;
        #[cfg(target_os = "macos")]
        Command::new("open")
            .arg(file)
            .spawn()
            .map_err(|e| format!("Couldn't open the file with the default app: {}", e))?;
        #[cfg(all(unix, not(target_os = "macos")))]
        Command::new("xdg-open")
            .arg(file)
            .spawn()
            .map_err(|e| format!("Couldn't open the file with the default app: {}", e))?;
        return Ok("the system default app".to_string());
    }

    // macOS: a .app bundle can't be exec'd directly — route through `open -a`.
    #[cfg(target_os = "macos")]
    if editor.ends_with(".app") {
        Command::new("open")
            .args(["-a", editor, file])
            .spawn()
            .map_err(|e| format!("Couldn't launch '{}': {}", editor, e))?;
        let name = Path::new(editor)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| editor.to_string());
        return Ok(name);
    }

    Command::new(editor)
        .arg(file)
        .spawn()
        .map_err(|e| {
            format!(
                "Couldn't launch '{}': {}. Check the editor path in Settings.",
                editor, e
            )
        })?;
    Ok(Path::new(editor)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| editor.to_string()))
}

/// Poll the rendered file for saves from the editor. Editors overwrite in
/// place or replace via temp+rename, so plain mtime polling covers both; a
/// change only counts once the mtime holds still for two polls (large TIFFs
/// take a while to write).
fn spawn_external_edit_watcher(app_handle: tauri::AppHandle, path: String) {
    {
        let mut watches = EXTERNAL_EDIT_WATCHES.lock().unwrap();
        if !watches.insert(path.clone()) {
            return; // already watching this file
        }
    }
    std::thread::spawn(move || {
        let file = PathBuf::from(&path);
        let mtime_of =
            |p: &Path| p.metadata().and_then(|m| m.modified()).ok();
        let mut last_seen = mtime_of(&file);
        let mut pending: Option<Option<std::time::SystemTime>> = None;
        let started = std::time::Instant::now();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if started.elapsed() > std::time::Duration::from_secs(8 * 3600) {
                break;
            }
            if !file.exists() {
                continue; // mid-rename; keep waiting
            }
            let current = mtime_of(&file);
            if current != last_seen {
                if pending.as_ref() == Some(&current) {
                    last_seen = current;
                    pending = None;
                    let _ = app_handle.emit(
                        "external-edit-saved",
                        serde_json::json!({ "path": path }),
                    );
                } else {
                    pending = Some(current);
                }
            } else {
                pending = None;
            }
        }
        EXTERNAL_EDIT_WATCHES.lock().unwrap().remove(&path);
    });
}

#[tauri::command]
pub async fn start_external_edit(
    path: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<ExternalEditLaunch, String> {
    let settings = load_settings(app_handle.clone()).unwrap_or_default();
    let editor = settings
        .external_editor_path
        .clone()
        .unwrap_or_default()
        .trim()
        .to_string();
    let format = settings
        .external_editor_format
        .clone()
        .filter(|f| !f.trim().is_empty())
        .unwrap_or_else(|| "tiff".to_string());

    let (source_path, _) = parse_virtual_path(&path);
    if !source_path.exists() {
        return Err(format!("Source file not found: {}", source_path.display()));
    }
    let dir = source_path
        .parent()
        .ok_or("The image has no parent folder")?;
    let stem = source_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or("Couldn't derive a name for the edit file")?;
    // "tiff8" shares the .tif container with "tiff" and only narrows the
    // sample depth — some editors (and most plugins) still choke on 16-bit.
    let (ext, tiff_bit_depth) = match format.as_str() {
        "jpeg" => ("jpg", TiffBitDepth::Sixteen),
        "png" => ("png", TiffBitDepth::Sixteen),
        "tiff8" => ("tif", TiffBitDepth::Eight),
        _ => ("tif", TiffBitDepth::Sixteen),
    };
    // Each session gets its own file: -Edit, -Edit-2, … — an existing file is
    // an earlier round-trip that must not be overwritten.
    let mut output = dir.join(format!("{}-Edit.{}", stem, ext));
    let mut n = 2;
    while output.exists() {
        output = dir.join(format!("{}-Edit-{}.{}", stem, n, ext));
        n += 1;
    }

    let export_settings = ExportSettings {
        jpeg_quality: 95,
        tiff_bit_depth,
        color_space: Default::default(),
        hdr: false,
        resize: None,
        keep_metadata: true,
        preserve_timestamps: false,
        strip_gps: false,
        filename_template: None,
        watermark: None,
        export_masks: false,
        preserve_folders: false,
        destination_type: None,
        subfolder: None,
        border: None,
        pad: None,
        dpi: None,
        export_original: None,
    };

    export_images_and_wait(
        vec![path.clone()],
        output.to_string_lossy().to_string(),
        true,
        vec![source_path.to_string_lossy().to_string()],
        export_settings,
        if ext == "tif" { "tiff".to_string() } else { format },
        ExportAdjustmentsMode::UseSidecars {
            active_path: None,
            active_adjustments: None,
        },
        state,
        app_handle.clone(),
    )
    .await
    .map_err(|e| format!("Rendering failed: {}", e))?;

    let output_str = output.to_string_lossy().to_string();

    // The -Edit file is the same photograph, so its library data must survive
    // the round-trip: copy the rating, tags (keywords and the color label) and
    // the user-editable metadata fields into a sidecar for the new file.
    // Adjustments stay empty — they are baked into the rendered pixels.
    let source_meta = crate::metadata::sidecar::load_sidecar(&parse_virtual_path(&path).1);
    if source_meta.rating != 0 || source_meta.tags.is_some() || source_meta.exif.is_some() {
        let edit_meta = crate::metadata::sidecar::ImageMetadata {
            adjustments: Value::Null,
            ..source_meta
        };
        let edit_sidecar = parse_virtual_path(&output_str).1;
        if let Ok(json) = serde_json::to_string_pretty(&edit_meta) {
            if let Err(error) = fs::write(&edit_sidecar, json) {
                log::warn!(
                    "Couldn't carry metadata over to '{}': {}",
                    edit_sidecar.display(),
                    error
                );
            }
        }
    }

    let editor_label = launch_external_editor(&editor, &output_str)?;
    spawn_external_edit_watcher(app_handle, output_str.clone());

    Ok(ExternalEditLaunch {
        output_path: output_str,
        editor: editor_label,
    })
}
