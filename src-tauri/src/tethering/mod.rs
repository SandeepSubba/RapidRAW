#[cfg(feature = "tether-usb")]
pub mod usb;

// Same command surface without libgphoto2: list returns empty (UI hides the
// camera section), actions error. Keeps generate_handler! unconditional.
#[cfg(not(feature = "tether-usb"))]
pub mod usb {
    use serde::Serialize;

    #[derive(Default)]
    pub struct UsbCameraState;

    #[derive(Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct CameraInfo {
        pub model: String,
        pub port: String,
    }

    const OFF: &str = "USB tethering is not included in this build";

    #[tauri::command]
    pub fn tether_list_cameras() -> Result<Vec<CameraInfo>, String> {
        Ok(Vec::new())
    }

    #[tauri::command]
    pub fn tether_connect_camera(
        _model: String,
        _port: String,
        _download_dir: String,
    ) -> Result<serde_json::Value, String> {
        Err(OFF.into())
    }

    #[tauri::command]
    pub fn tether_disconnect_camera() -> Result<(), String> {
        Ok(())
    }

    #[tauri::command]
    pub fn tether_trigger_capture() -> Result<(), String> {
        Err(OFF.into())
    }

    #[tauri::command]
    pub fn tether_set_config(_key: String, _value: String) -> Result<(), String> {
        Err(OFF.into())
    }

    #[tauri::command]
    pub fn tether_set_live_view(_on: bool) -> Result<(), String> {
        Err(OFF.into())
    }
}

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::Mutex;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use tauri::Emitter;

use crate::formats::is_supported_image_file;
use crate::image_processing::ImageMetadata;

/// Watched-folder tethering: any vendor utility (X Acquire, EOS Utility, NX
/// Tether...) drops shots into a folder; we ingest them as they land.
#[derive(Default)]
pub struct TetherState {
    session: Mutex<Option<ActiveSession>>,
}

struct ActiveSession {
    // Held only to keep the watcher alive; dropping it disconnects the event
    // channel, which ends the ingest thread.
    _watcher: RecommendedWatcher,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TetherShotPayload {
    path: String,
    file_name: String,
    shot_count: u32,
}

const POLL_INTERVAL: Duration = Duration::from_millis(500);
// A file is ingested once its size is non-zero and unchanged for two
// consecutive polls — vendor utilities stream RAWs progressively, so
// reacting to the create event alone would import half-written files.
const STABLE_POLLS_REQUIRED: u8 = 2;

#[tauri::command]
pub fn start_tether_session(
    folder: String,
    preset_adjustments: Option<serde_json::Value>,
    state: tauri::State<TetherState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let folder_path = PathBuf::from(&folder);
    if !folder_path.is_dir() {
        return Err(format!("Not a folder: {}", folder));
    }

    let (tx, rx) = channel::<PathBuf>();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            for path in event.paths {
                // Any event kind is fine: correctness comes from the
                // size-stability polling in the ingest thread.
                if is_supported_image_file(&path) {
                    let _ = tx.send(path);
                }
            }
        }
    })
    .map_err(|e| e.to_string())?;
    watcher
        .watch(&folder_path, RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;

    std::thread::spawn(move || {
        ingest_loop(rx, preset_adjustments, app_handle);
    });

    // Replacing an existing session drops its watcher, ending the old thread.
    *state.session.lock().unwrap() = Some(ActiveSession { _watcher: watcher });
    log::info!("[tether] session started on {}", folder);
    Ok(())
}

#[tauri::command]
pub fn stop_tether_session(state: tauri::State<TetherState>) -> Result<(), String> {
    if state.session.lock().unwrap().take().is_some() {
        log::info!("[tether] session stopped");
    }
    Ok(())
}

fn ingest_loop(
    rx: std::sync::mpsc::Receiver<PathBuf>,
    preset_adjustments: Option<serde_json::Value>,
    app_handle: tauri::AppHandle,
) {
    let mut pending: HashMap<PathBuf, (u64, u8)> = HashMap::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut shot_count: u32 = 0;

    loop {
        match rx.recv_timeout(POLL_INTERVAL) {
            Ok(path) => {
                if !seen.contains(&path) {
                    pending.entry(path).or_insert((0, 0));
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        pending.retain(|path, (last_size, stable_polls)| {
            let Ok(meta) = std::fs::metadata(path) else {
                return false; // vanished (vendor temp file renamed away)
            };
            let size = meta.len();
            if size > 0 && size == *last_size {
                *stable_polls += 1;
            } else {
                *last_size = size;
                *stable_polls = 0;
            }
            if *stable_polls < STABLE_POLLS_REQUIRED {
                return true;
            }

            seen.insert(path.clone());
            shot_count += 1;
            if let Some(adjustments) = &preset_adjustments {
                write_preset_sidecar(path, adjustments);
            }
            let _ = app_handle.emit(
                "tether-shot",
                TetherShotPayload {
                    path: path.to_string_lossy().into_owned(),
                    file_name: path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    shot_count,
                },
            );
            log::info!("[tether] shot {} ingested: {}", shot_count, path.display());
            false
        });
    }
    log::info!("[tether] ingest loop ended after {} shots", shot_count);
}

fn write_preset_sidecar(image_path: &PathBuf, adjustments: &serde_json::Value) {
    let Some(file_name) = image_path.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    let sidecar = image_path.with_file_name(format!("{}.rrdata", file_name));
    if sidecar.exists() {
        return; // never clobber existing edits
    }
    let metadata = ImageMetadata {
        adjustments: adjustments.clone(),
        ..Default::default()
    };
    match serde_json::to_string_pretty(&metadata) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&sidecar, json) {
                log::warn!("[tether] sidecar write failed for {}: {}", sidecar.display(), e);
            }
        }
        Err(e) => log::warn!("[tether] sidecar serialize failed: {}", e),
    }
}
