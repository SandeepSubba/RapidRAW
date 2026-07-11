use std::path::PathBuf;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::Emitter;

/// Direct USB tethering over libgphoto2 (feature `tether-usb`).
///
/// libgphoto2 handles are not thread-safe, so a dedicated thread owns the
/// Context + Camera for the whole connection and everything else talks to it
/// through a command channel. Captures (app-triggered or shutter presses on
/// the body) are downloaded into the watched session folder, where the
/// Tier-0 folder watcher ingests them like any other tethered shot.
#[derive(Default)]
pub struct UsbCameraState {
    control: Mutex<Option<Sender<CameraCommand>>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CameraInfo {
    pub model: String,
    pub port: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CameraConfig {
    pub key: String,
    pub label: String,
    pub current: String,
    pub choices: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedCamera {
    pub model: String,
    pub configs: Vec<CameraConfig>,
}

enum CameraCommand {
    Trigger(tokio::sync::oneshot::Sender<Result<(), String>>),
    SetConfig {
        key: String,
        value: String,
        reply: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    Disconnect,
}

// One widget per setting, probed by candidate key since drivers name them
// differently (Canon "iso"/"aperture"/"shutterspeed", Nikon "f-number", ...).
const CONFIG_CANDIDATES: &[(&str, &[&str])] = &[
    ("ISO", &["iso", "isospeed"]),
    ("Aperture", &["aperture", "f-number", "fnumber"]),
    ("Shutter", &["shutterspeed", "shutterspeed2"]),
];

#[tauri::command]
pub async fn tether_list_cameras() -> Result<Vec<CameraInfo>, String> {
    tokio::task::spawn_blocking(|| {
        let ctx = gphoto2::Context::new().map_err(|e| e.to_string())?;
        let list = ctx.list_cameras().wait().map_err(|e| e.to_string())?;
        Ok(list
            .map(|d| CameraInfo {
                model: d.model,
                port: d.port,
            })
            .collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tether_connect_camera(
    model: String,
    port: String,
    download_dir: String,
    state: tauri::State<'_, UsbCameraState>,
    app_handle: tauri::AppHandle,
) -> Result<ConnectedCamera, String> {
    // Drop any previous connection first.
    if let Some(control) = state.control.lock().unwrap().take() {
        let _ = control.send(CameraCommand::Disconnect);
    }

    let (tx, rx) = channel::<CameraCommand>();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<Result<ConnectedCamera, String>>();
    let dest = PathBuf::from(download_dir);

    std::thread::spawn(move || camera_thread(model, port, dest, ready_tx, rx, app_handle));

    let connected = ready_rx.await.map_err(|_| "camera thread died".to_string())??;
    *state.control.lock().unwrap() = Some(tx);
    Ok(connected)
}

#[tauri::command]
pub fn tether_disconnect_camera(state: tauri::State<UsbCameraState>) -> Result<(), String> {
    if let Some(control) = state.control.lock().unwrap().take() {
        let _ = control.send(CameraCommand::Disconnect);
    }
    Ok(())
}

#[tauri::command]
pub async fn tether_trigger_capture(state: tauri::State<'_, UsbCameraState>) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    {
        let guard = state.control.lock().unwrap();
        let control = guard.as_ref().ok_or("No camera connected")?;
        control
            .send(CameraCommand::Trigger(reply_tx))
            .map_err(|_| "Camera thread gone")?;
    }
    reply_rx.await.map_err(|_| "camera thread died".to_string())?
}

#[tauri::command]
pub async fn tether_set_config(
    key: String,
    value: String,
    state: tauri::State<'_, UsbCameraState>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    {
        let guard = state.control.lock().unwrap();
        let control = guard.as_ref().ok_or("No camera connected")?;
        control
            .send(CameraCommand::SetConfig {
                key,
                value,
                reply: reply_tx,
            })
            .map_err(|_| "Camera thread gone")?;
    }
    reply_rx.await.map_err(|_| "camera thread died".to_string())?
}

fn camera_thread(
    model: String,
    port: String,
    dest: PathBuf,
    ready_tx: tokio::sync::oneshot::Sender<Result<ConnectedCamera, String>>,
    rx: std::sync::mpsc::Receiver<CameraCommand>,
    app_handle: tauri::AppHandle,
) {
    let open = || -> Result<(gphoto2::Context, gphoto2::Camera), String> {
        let ctx = gphoto2::Context::new().map_err(|e| e.to_string())?;
        let camera = ctx
            .get_camera(&gphoto2::list::CameraDescriptor {
                model: model.clone(),
                port: port.clone(),
            })
            .wait()
            .map_err(|e| e.to_string())?;
        Ok((ctx, camera))
    };

    let (_ctx, camera) = match open() {
        Ok(pair) => pair,
        Err(e) => {
            let _ = ready_tx.send(Err(format!(
                "Could not open camera ({}). If macOS grabbed it, unplug/replug and close Photos/Image Capture.",
                e
            )));
            return;
        }
    };

    let configs = read_configs(&camera);
    log::info!("[tether-usb] connected: {} ({} configs)", model, configs.len());
    let _ = ready_tx.send(Ok(ConnectedCamera {
        model: model.clone(),
        configs,
    }));

    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(CameraCommand::Disconnect) | Err(RecvTimeoutError::Disconnected) => break,
            Ok(CameraCommand::Trigger(reply)) => {
                let _ = reply.send(trigger_and_download(&camera, &dest));
            }
            Ok(CameraCommand::SetConfig { key, value, reply }) => {
                let _ = reply.send(set_config(&camera, &key, &value));
            }
            Err(RecvTimeoutError::Timeout) => {}
        }

        // Drain camera events: a shutter press on the body shows up here as a
        // new file, which we pull into the watched session folder.
        match camera.wait_event(Duration::from_millis(200)).wait() {
            Ok(gphoto2::camera::CameraEvent::NewFile(path)) => {
                if let Err(e) = download_file(&camera, &path, &dest) {
                    log::warn!("[tether-usb] download failed: {}", e);
                }
            }
            Ok(_) => {}
            Err(e) => {
                log::warn!("[tether-usb] camera lost: {}", e);
                let _ = app_handle.emit("tether-camera-lost", model.clone());
                break;
            }
        }
    }
    log::info!("[tether-usb] disconnected: {}", model);
}

fn read_configs(camera: &gphoto2::Camera) -> Vec<CameraConfig> {
    CONFIG_CANDIDATES
        .iter()
        .filter_map(|(label, keys)| {
            keys.iter().find_map(|key| {
                let widget = camera
                    .config_key::<gphoto2::widget::RadioWidget>(key)
                    .wait()
                    .ok()?;
                Some(CameraConfig {
                    key: (*key).to_string(),
                    label: (*label).to_string(),
                    current: widget.choice(),
                    choices: widget.choices_iter().collect(),
                })
            })
        })
        .collect()
}

fn set_config(camera: &gphoto2::Camera, key: &str, value: &str) -> Result<(), String> {
    let widget = camera
        .config_key::<gphoto2::widget::RadioWidget>(key)
        .wait()
        .map_err(|e| e.to_string())?;
    widget.set_choice(value).map_err(|e| e.to_string())?;
    camera.set_config(&widget).wait().map_err(|e| e.to_string())
}

fn trigger_and_download(camera: &gphoto2::Camera, dest: &PathBuf) -> Result<(), String> {
    let path = camera.capture_image().wait().map_err(|e| e.to_string())?;
    download_file(camera, &path, dest)
}

fn download_file(
    camera: &gphoto2::Camera,
    path: &gphoto2::file::CameraFilePath,
    dest: &PathBuf,
) -> Result<(), String> {
    let target = dest.join(path.name().as_ref());
    camera
        .fs()
        .download_to(&path.folder(), &path.name(), &target)
        .wait()
        .map_err(|e| e.to_string())?;
    log::info!("[tether-usb] downloaded {}", target.display());
    Ok(())
}
