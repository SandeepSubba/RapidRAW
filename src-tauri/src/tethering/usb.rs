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
    ("Format", &["imageformat", "imagequality"]), // RAW / JPEG / RAW+JPEG
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
        // macOS's PTP daemons hold an exclusive claim on any PTP camera. They
        // respawn instantly when killed, but can't reclaim once we hold the
        // interface — so killing them right before opening wins the race.
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("/usr/bin/pkill")
            .args(["-9", "-f", "ptpcamerad|mscamerad-xpc"])
            .status();

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
                // Fire-and-forget; the shot arrives as a NewFile event below,
                // same download path as a physical shutter press.
                let result = if model.starts_with("Fuji") {
                    fuji_shoot(&camera)
                } else {
                    camera.trigger_capture().wait().map_err(|e| e.to_string())
                };
                if let Err(e) = &result {
                    log::warn!("[tether-usb] trigger failed: {}", e);
                }
                let _ = reply.send(result);
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

// Fuji bodies ignore plain PTP InitiateCapture. Their remote release is
// two-step (libgphoto2 camlibs/ptp2: fuji_action table): arm 0xD208 with
// "Shoot" (0x0304, full press), then send InitiateCapture to execute the
// armed action — here via the raw 'opcode' widget, which stays on the bulk
// endpoint like the config writes that already work.
fn fuji_shoot(camera: &gphoto2::Camera) -> Result<(), String> {
    let mut last_err = String::new();
    for attempt in 1..=3 {
        match fuji_shoot_once(camera) {
            Ok(()) => return Ok(()),
            Err(e) => {
                log::warn!("[tether-usb] fuji shoot attempt {}: {}", attempt, e);
                last_err = e;
                std::thread::sleep(Duration::from_millis(150));
            }
        }
    }
    // Recent Fuji firmware refuses USB remote release even to stock gphoto2
    // (verified against this exact sequence; see gphoto/libgphoto2#1241).
    if last_err.contains("in progress") || last_err.contains("Busy") {
        return Err(
            "This Fuji body refuses USB remote release (known libgphoto2 limitation). \
             Use the camera's shutter button — shots still import instantly."
                .to_string(),
        );
    }
    Err(last_err)
}

// Faithful port of libgphoto2's camera_fuji_capture handshake: the body
// answers DeviceBusy to InitiateCapture unless PriorityMode=USB is written
// fresh in-session and the AF phase (S1) ran before the shoot phase (S2).
fn fuji_shoot_once(camera: &gphoto2::Camera) -> Result<(), String> {
    // Best-effort, like the driver's LOG_ON_PTP_E: some bodies reject the
    // write while already granting control.
    if let Err(e) = set_config(camera, "prioritymode", "USB") {
        log::info!("[tether-usb] priority handshake declined ({}), continuing", e);
    }

    set_config(camera, "cameraaction", "AF").map_err(|e| format!("arm AF: {}", e))?;
    initiate_capture(camera).map_err(|e| format!("execute AF: {}", e))?;
    std::thread::sleep(Duration::from_millis(500)); // AF settle (instant in MF)

    set_config(camera, "cameraaction", "Shoot").map_err(|e| format!("arm Shoot: {}", e))?;
    initiate_capture(camera).map_err(|e| format!("execute Shoot: {}", e))
}

fn initiate_capture(camera: &gphoto2::Camera) -> Result<(), String> {
    let widget = camera
        .config_key::<gphoto2::widget::TextWidget>("opcode")
        .wait()
        .map_err(|e| e.to_string())?;
    widget.set_value("0x100e,0x0,0x0").map_err(|e| e.to_string())?;
    camera.set_config(&widget).wait().map_err(|e| e.to_string())
}

fn set_config(camera: &gphoto2::Camera, key: &str, value: &str) -> Result<(), String> {
    let widget = camera
        .config_key::<gphoto2::widget::RadioWidget>(key)
        .wait()
        .map_err(|e| e.to_string())?;
    widget.set_choice(value).map_err(|e| e.to_string())?;
    camera.set_config(&widget).wait().map_err(|e| e.to_string())
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
