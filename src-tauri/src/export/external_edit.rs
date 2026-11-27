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

/// What the editor gets, as in Lightroom's "Edit Photo with…" dialog.
/// Raw files can only be edited as a rendered copy.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ExternalEditMode {
    /// Render the current RapidRAW adjustments into a new -Edit file (default).
    CopyWithAdjustments,
    /// Copy the original file's pixels unchanged into a new -Edit file.
    Copy,
    /// Open the original file itself; the editor saves over it.
    Original,
}

impl ExternalEditMode {
    fn parse(mode: Option<&str>) -> Result<Self, String> {
        match mode.map(str::trim).unwrap_or("") {
            "" | "adjusted" => Ok(Self::CopyWithAdjustments),
            "copy" => Ok(Self::Copy),
            "original" => Ok(Self::Original),
            other => Err(format!("Unknown external edit mode '{}'", other)),
        }
    }
}

/// Next free `<stem>-Edit.<ext>`, `<stem>-Edit-2.<ext>`, … — an existing file
/// is an earlier round-trip that must not be overwritten.
fn next_edit_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let mut output = dir.join(format!("{}-Edit.{}", stem, ext));
    let mut n = 2;
    while output.exists() {
        output = dir.join(format!("{}-Edit-{}.{}", stem, n, ext));
        n += 1;
    }
    output
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

    // Linux: a .desktop launcher (e.g. a Wine app) isn't an executable —
    // run its Exec line ourselves. Exec'ing it directly would go through its
    // `#!/usr/bin/env xdg-open` shebang and open the file in the default app.
    #[cfg(all(unix, not(target_os = "macos")))]
    if editor.ends_with(".desktop") {
        return launch_desktop_entry(editor, file);
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

/// Single-quote `s` for `sh -c`.
#[cfg(all(unix, not(target_os = "macos")))]
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Build the shell command for a desktop entry's `Exec` value: file field
/// codes (%f %F %u %U) become the quoted file, other codes are dropped, and
/// the file is appended when the line has no file code. Exec quoting rules
/// are a subset of sh's, so the result runs under `sh -c`.
#[cfg(all(unix, not(target_os = "macos")))]
fn desktop_exec_command(exec: &str, file: &str) -> String {
    let quoted = shell_quote(file);
    let mut out = String::with_capacity(exec.len() + quoted.len());
    let mut used_file = false;
    let mut chars = exec.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('%') => out.push('%'),
            Some('f' | 'F' | 'u' | 'U') => {
                if !used_file {
                    out.push_str(&quoted);
                    used_file = true;
                }
            }
            _ => {} // %i %c %k and deprecated codes: drop
        }
    }
    if !used_file {
        out.push(' ');
        out.push_str(&quoted);
    }
    out
}

/// Launch `file` with a freedesktop `.desktop` entry (honours Exec, Path).
#[cfg(all(unix, not(target_os = "macos")))]
fn launch_desktop_entry(entry: &str, file: &str) -> Result<String, String> {
    let text = fs::read_to_string(entry)
        .map_err(|e| format!("Couldn't read '{}': {}. Check the editor path in Settings.", entry, e))?;
    let (mut exec, mut workdir, mut name) = (None, None, None);
    let mut in_main = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_main = line == "[Desktop Entry]";
            continue;
        }
        if !in_main {
            continue;
        }
        if let Some(v) = line.strip_prefix("Exec=") {
            exec = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("Path=") {
            workdir = Some(v.trim().to_string()).filter(|v| !v.is_empty());
        } else if let Some(v) = line.strip_prefix("Name=") {
            name = Some(v.trim().to_string());
        }
    }
    let exec = exec.ok_or_else(|| format!("'{}' has no Exec line.", entry))?;
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c").arg(desktop_exec_command(&exec, file));
    if let Some(dir) = workdir.filter(|d| Path::new(d).is_dir()) {
        cmd.current_dir(dir);
    }
    cmd.spawn()
        .map_err(|e| format!("Couldn't launch '{}': {}", entry, e))?;
    Ok(name.unwrap_or_else(|| {
        Path::new(entry)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| entry.to_string())
    }))
}

#[cfg(all(test, unix, not(target_os = "macos")))]
mod desktop_tests {
    use super::desktop_exec_command;

    #[test]
    fn substitutes_file_codes() {
        assert_eq!(
            desktop_exec_command(r#"env A="x; y" /bin/wine "C:/a b.exe" %F"#, "/p/it's.tif"),
            r#"env A="x; y" /bin/wine "C:/a b.exe" '/p/it'\''s.tif'"#
        );
        assert_eq!(desktop_exec_command("gimp", "/a.tif"), "gimp '/a.tif'");
        assert_eq!(desktop_exec_command("app %i %u 100%%", "/a"), "app  '/a' 100%");
    }
}

#[cfg(test)]
mod mode_tests {
    use super::{next_edit_path, ExternalEditMode};

    #[test]
    fn parses_modes() {
        assert_eq!(ExternalEditMode::parse(None), Ok(ExternalEditMode::CopyWithAdjustments));
        assert_eq!(ExternalEditMode::parse(Some("adjusted")), Ok(ExternalEditMode::CopyWithAdjustments));
        assert_eq!(ExternalEditMode::parse(Some("copy")), Ok(ExternalEditMode::Copy));
        assert_eq!(ExternalEditMode::parse(Some(" original ")), Ok(ExternalEditMode::Original));
        assert!(ExternalEditMode::parse(Some("bogus")).is_err());
    }

    #[test]
    fn edit_paths_never_overwrite() {
        let dir = std::env::temp_dir().join(format!("rr-edit-path-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(next_edit_path(&dir, "a", "tif"), dir.join("a-Edit.tif"));
        std::fs::write(dir.join("a-Edit.tif"), b"x").unwrap();
        std::fs::write(dir.join("a-Edit-2.tif"), b"x").unwrap();
        assert_eq!(next_edit_path(&dir, "a", "tif"), dir.join("a-Edit-3.tif"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// Poll the rendered file for saves from the editor. Editors overwrite in
/// place or replace via temp+rename, so plain mtime polling covers both; a
/// change only counts once the mtime holds still for two polls (large TIFFs
/// take a while to write).
/// `original` is true when the watched file is the library original itself
/// (Edit Original), so the frontend reloads it if it's open in the editor.
fn spawn_external_edit_watcher(app_handle: tauri::AppHandle, path: String, original: bool) {
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
                        serde_json::json!({ "path": path, "original": original }),
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

/// The -Edit file is the same photograph, so its library data must survive
/// the round-trip: copy the rating, tags (keywords and the color label) and
/// the user-editable metadata fields into a sidecar for the new file.
/// Adjustments stay empty — they are baked into the pixels (or, for a plain
/// copy, deliberately left off).
fn carry_library_metadata(source_virtual_path: &str, output: &str) {
    let source_meta =
        crate::metadata::sidecar::load_sidecar(&parse_virtual_path(source_virtual_path).1);
    if source_meta.rating == 0 && source_meta.tags.is_none() && source_meta.exif.is_none() {
        return;
    }
    let edit_meta = crate::metadata::sidecar::ImageMetadata {
        adjustments: Value::Null,
        ..source_meta
    };
    let edit_sidecar = parse_virtual_path(output).1;
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

#[tauri::command]
pub async fn start_external_edit(
    path: String,
    mode: Option<String>,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<ExternalEditLaunch, String> {
    let mode = ExternalEditMode::parse(mode.as_deref())?;
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

    if mode != ExternalEditMode::CopyWithAdjustments
        && crate::decode::formats::is_raw_file(&source_path)
    {
        return Err(
            "Raw files can only be edited as a copy with RapidRAW adjustments.".to_string(),
        );
    }

    if mode == ExternalEditMode::Original {
        // Hand the original straight to the editor; it saves over it. Its
        // RapidRAW adjustments stay in the sidecar and apply on top, as in
        // Lightroom.
        let source_str = source_path.to_string_lossy().to_string();
        let editor_label = launch_external_editor(&editor, &source_str)?;
        crate::render::gpu::context::release_gpu_memory(&state, "external editor");
        spawn_external_edit_watcher(app_handle, source_str.clone(), true);
        return Ok(ExternalEditLaunch {
            output_path: source_str,
            editor: editor_label,
        });
    }

    if mode == ExternalEditMode::Copy {
        // Byte-for-byte copy of the original, without RapidRAW adjustments.
        let ext = source_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_else(|| "tif".to_string());
        let output = next_edit_path(dir, &stem, &ext);
        fs::copy(&source_path, &output)
            .map_err(|e| format!("Couldn't copy '{}': {}", source_path.display(), e))?;
        let output_str = output.to_string_lossy().to_string();
        carry_library_metadata(&path, &output_str);
        let editor_label = launch_external_editor(&editor, &output_str)?;
        crate::render::gpu::context::release_gpu_memory(&state, "external editor");
        spawn_external_edit_watcher(app_handle, output_str.clone(), false);
        return Ok(ExternalEditLaunch {
            output_path: output_str,
            editor: editor_label,
        });
    }

    // "tiff8" shares the .tif container with "tiff" and only narrows the
    // sample depth — some editors (and most plugins) still choke on 16-bit.
    let (ext, tiff_bit_depth) = match format.as_str() {
        "jpeg" => ("jpg", TiffBitDepth::Sixteen),
        "png" => ("png", TiffBitDepth::Sixteen),
        "tiff8" => ("tif", TiffBitDepth::Eight),
        _ => ("tif", TiffBitDepth::Sixteen),
    };
    // Each session gets its own file: -Edit, -Edit-2, …
    let output = next_edit_path(dir, &stem, ext);

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

    // Free RapidRAW's own preview textures first so the full-resolution
    // render has room even while the external editor holds VRAM.
    crate::render::gpu::context::release_gpu_memory(&state, "before external edit render");

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
    carry_library_metadata(&path, &output_str);

    let editor_label = launch_external_editor(&editor, &output_str)?;
    // Hand the VRAM to the external editor (e.g. Affinity under Wine).
    {
        use tauri::Manager;
        let app_state = app_handle.state::<AppState>();
        crate::render::gpu::context::release_gpu_memory(&app_state, "external editor");
    }
    spawn_external_edit_watcher(app_handle, output_str.clone(), false);

    Ok(ExternalEditLaunch {
        output_path: output_str,
        editor: editor_label,
    })
}
