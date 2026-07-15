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

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct SliderRange {
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub default: f64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CameraConfig {
    pub key: String,
    pub label: String,
    pub current: String,
    pub choices: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<SliderRange>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedCamera {
    pub model: String,
    pub configs: Vec<CameraConfig>,
    pub live_view_supported: bool,
}

// Bodies whose driver claims preview support but whose tether firmware
// refuses every stream-open path (hardware-verified on X-T3 fw5.11 from a
// virgin boot; X-T2 shares that tether generation). Their live view only
// speaks Fuji's licensed SDK protocol.
const LIVE_VIEW_REFUSERS: &[&str] = &["X-T3", "X-T2"];

enum CameraCommand {
    Trigger(tokio::sync::oneshot::Sender<Result<(), String>>),
    SetConfig {
        key: String,
        value: String,
        reply: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    SetLiveView {
        on: bool,
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

// Numeric properties rendered as sliders; per label the first key the body
// exposes wins. Key names cover Canon/Fuji/Sony/Olympus/Panasonic drivers;
// bodies without one (Fuji has no tint over PTP) just don't get the row.
const SLIDER_CANDIDATES: &[(&str, &str, SliderRange)] = &[
    ("Color Temp", "colortemperature", SliderRange { min: 2500.0, max: 10000.0, step: 100.0, default: 5500.0 }),
    ("Tint", "whitebalanceadjustb", SliderRange { min: -9.0, max: 9.0, step: 1.0, default: 0.0 }),
    ("Tint", "whitebalanceadjustgm", SliderRange { min: -9.0, max: 9.0, step: 1.0, default: 0.0 }),
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
pub async fn tether_set_live_view(
    on: bool,
    state: tauri::State<'_, UsbCameraState>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    {
        let guard = state.control.lock().unwrap();
        let control = guard.as_ref().ok_or("No camera connected")?;
        control
            .send(CameraCommand::SetLiveView { on, reply: reply_tx })
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

    let (ctx, camera) = match open() {
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
    let live_view_supported = camera
        .abilities()
        .camera_operations()
        .capture_preview()
        && !LIVE_VIEW_REFUSERS.iter().any(|m| model.contains(m));
    log::info!(
        "[tether-usb] connected: {} ({} configs, live view {})",
        model,
        configs.len(),
        live_view_supported
    );
    let _ = ready_tx.send(Ok(ConnectedCamera {
        model: model.clone(),
        configs,
        live_view_supported,
    }));

    let mut live_view = false;
    loop {
        // Stay responsive while pumping preview frames.
        let cmd_wait = Duration::from_millis(if live_view { 5 } else { 50 });
        match rx.recv_timeout(cmd_wait) {
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
            Ok(CameraCommand::SetLiveView { on, reply }) => {
                // Probe one frame before confirming: bodies without preview
                // support error here and the UI switch reverts.
                let result = if on { grab_preview_frame(&camera, &ctx).map(|_| ()) } else { Ok(()) };
                live_view = on && result.is_ok();
                log::info!("[tether-usb] live view {} ({:?})", on, result.as_ref().err());
                let _ = reply.send(result);
            }
            Err(RecvTimeoutError::Timeout) => {}
        }

        if live_view {
            match grab_preview_frame(&camera, &ctx) {
                Ok(frame_b64) => {
                    let _ = app_handle.emit("tether-preview-frame", frame_b64);
                }
                Err(e) => {
                    log::warn!("[tether-usb] live view stopped: {}", e);
                    live_view = false;
                    let _ = app_handle.emit("tether-live-view-stopped", e);
                }
            }
        }

        // Drain camera events: a shutter press on the body shows up here as a
        // new file, which we pull into the watched session folder.
        let event_wait = Duration::from_millis(if live_view { 5 } else { 200 });
        match camera.wait_event(event_wait).wait() {
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
    let mut configs: Vec<CameraConfig> = CONFIG_CANDIDATES
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
                    range: None,
                })
            })
        })
        .collect();

    // Driver-unmapped enum entries read as "Unknown value 8020" — meaningless
    // to pick; keep one only when it's the camera's current state.
    for config in configs.iter_mut() {
        let current = config.current.clone();
        config
            .choices
            .retain(|choice| !choice.starts_with("Unknown value") || *choice == current);
    }

    // Numeric properties as sliders. Bodies expose these either as an
    // enumerated radio (Fuji/Canon list every Kelvin stop they accept) or as
    // a bare INT text widget; radio choices are kept so the UI snaps to
    // values the body actually takes.
    for (label, key, range) in SLIDER_CANDIDATES {
        if configs.iter().any(|c| c.label == *label) {
            continue;
        }
        if let Ok(widget) = camera.config_key::<gphoto2::widget::RadioWidget>(key).wait() {
            let choices: Vec<String> = widget.choices_iter().collect();
            if choices.len() >= 2 && choices.iter().all(|c| c.trim().parse::<f64>().is_ok()) {
                configs.push(CameraConfig {
                    key: (*key).to_string(),
                    label: (*label).to_string(),
                    current: widget.choice(),
                    choices,
                    range: Some(*range),
                });
            }
        } else if let Ok(widget) = camera.config_key::<gphoto2::widget::TextWidget>(key).wait() {
            configs.push(CameraConfig {
                key: (*key).to_string(),
                label: (*label).to_string(),
                current: widget.value(),
                choices: Vec::new(),
                range: Some(*range),
            });
        }
    }

    // Bodies advertise the full mechanical f-stop scale regardless of the
    // mounted lens; clamp to the widest stop parsed from the lens name.
    // ponytail: stale after a mid-session lens swap until reconnect.
    if let Some(max_aperture) = lens_max_aperture(camera) {
        if let Some(aperture) = configs.iter_mut().find(|c| c.label == "Aperture") {
            aperture.choices.retain(|choice| {
                parse_f_number(choice).is_none_or(|f| f >= max_aperture - 0.01)
            });
        }
    }

    // Bodies enumerate every 1/3-stop plus electronic-shutter extremes —
    // far too long a list. Keep the classic full-stop dial values (plus
    // bulb/time and whatever is currently set), slowest → fastest.
    // ponytail: third-stops still settable on the body; re-add if missed.
    if let Some(shutter) = configs.iter_mut().find(|c| c.label == "Shutter") {
        let current = shutter.current.clone();
        shutter.choices.retain(|choice| {
            *choice == current
                || parse_shutter_secs(choice).is_none_or(is_standard_shutter)
        });
        shutter.choices.sort_by(|a, b| {
            let secs = |s: &str| parse_shutter_secs(s).unwrap_or(f64::INFINITY);
            secs(b).total_cmp(&secs(a))
        });
    }

    configs
}

fn is_standard_shutter(secs: f64) -> bool {
    const DIAL: [f64; 19] = [
        30.0, 15.0, 8.0, 4.0, 2.0, 1.0, 0.5, 0.25, 0.125, 1.0 / 15.0, 1.0 / 30.0,
        1.0 / 60.0, 1.0 / 125.0, 1.0 / 250.0, 1.0 / 500.0, 1.0 / 1000.0,
        1.0 / 2000.0, 1.0 / 4000.0, 1.0 / 8000.0,
    ];
    DIAL.iter().any(|d| (secs - d).abs() / d < 0.05)
}

fn parse_shutter_secs(choice: &str) -> Option<f64> {
    let s = choice.trim().trim_end_matches(['s', '"']);
    match s.split_once('/') {
        Some((num, den)) => {
            let (n, d) = (num.trim().parse::<f64>().ok()?, den.trim().parse::<f64>().ok()?);
            (d != 0.0).then(|| n / d)
        }
        None => s.parse().ok(),
    }
}

/// Widest aperture from the lens name the body reports — every major mount
/// embeds it ("XF16-55mmF2.8 R LM WR", "EF24-70mm f/2.8L II USM"). Focal
/// lengths also match the pattern ("XF16..." → F16), so keep only plausible
/// maximum apertures and prefer the first.
fn lens_max_aperture(camera: &gphoto2::Camera) -> Option<f32> {
    let widget = camera
        .config_key::<gphoto2::widget::TextWidget>("lensname")
        .wait()
        .ok()?;
    let name = widget.value();
    let max = max_aperture_from_name(&name)?;
    log::info!("[tether-usb] lens '{}' → apertures clamped to ≥ f/{}", name, max);
    Some(max)
}

fn max_aperture_from_name(name: &str) -> Option<f32> {
    let re = regex::Regex::new(r"[Ff]/?\s?(\d+(?:\.\d+)?)").ok()?;
    re.captures_iter(name)
        .filter_map(|c| c[1].parse::<f32>().ok())
        .find(|f| (0.7..=8.0).contains(f))
}

fn parse_f_number(choice: &str) -> Option<f32> {
    choice.trim().trim_start_matches(['f', 'F']).trim_start_matches('/').trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{max_aperture_from_name, parse_f_number};

    #[test]
    fn aperture_from_lens_names() {
        // "XF16" must not be read as F16 — focal lengths are out of the plausible range.
        assert_eq!(max_aperture_from_name("XF16-55mmF2.8 R LM WR"), Some(2.8));
        assert_eq!(max_aperture_from_name("XF35mmF1.4 R"), Some(1.4));
        assert_eq!(max_aperture_from_name("XC15-45mmF3.5-5.6 OIS PZ"), Some(3.5));
        assert_eq!(max_aperture_from_name("EF24-70mm f/2.8L II USM"), Some(2.8));
        assert_eq!(max_aperture_from_name("AF-S NIKKOR 50mm f/1.8G"), Some(1.8));
        assert_eq!(max_aperture_from_name("XF100-400mmF4.5-5.6 R LM OIS WR"), Some(4.5));
        assert_eq!(max_aperture_from_name("Mystery Lens"), None);
    }

    #[test]
    fn f_number_choices() {
        assert_eq!(parse_f_number("f/2.8"), Some(2.8));
        assert_eq!(parse_f_number("F4"), Some(4.0));
        assert_eq!(parse_f_number("22"), Some(22.0));
        assert_eq!(parse_f_number("auto"), None);
    }

    #[test]
    fn shutter_seconds() {
        use super::parse_shutter_secs;
        assert_eq!(parse_shutter_secs("1/8000"), Some(1.0 / 8000.0));
        assert_eq!(parse_shutter_secs("0.5"), Some(0.5));
        assert_eq!(parse_shutter_secs("30"), Some(30.0));
        assert_eq!(parse_shutter_secs("bulb"), None);
    }

    #[test]
    fn standard_shutter_filter() {
        use super::is_standard_shutter;
        assert!(is_standard_shutter(1.0 / 125.0));
        assert!(is_standard_shutter(30.0));
        assert!(!is_standard_shutter(1.0 / 160.0)); // third-stop
        assert!(!is_standard_shutter(1.0 / 32000.0)); // electronic-only
        assert!(!is_standard_shutter(900.0)); // long-exposure mode
    }
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
    // Kelvin only takes effect in the temperature WB mode; flip it first
    // (every driver labels that choice with "temperature"). Best-effort —
    // the WB widget itself is no longer surfaced in the UI.
    if key == "colortemperature" {
        if let Ok(wb) = camera
            .config_key::<gphoto2::widget::RadioWidget>("whitebalance")
            .wait()
        {
            if let Some(target) = wb
                .choices_iter()
                .find(|c| c.to_lowercase().contains("temperature"))
            {
                if wb.choice() != target && wb.set_choice(&target).is_ok() {
                    let _ = camera.set_config(&wb).wait();
                }
            }
        }
    }
    if let Ok(widget) = camera
        .config_key::<gphoto2::widget::RadioWidget>(key)
        .wait()
    {
        widget.set_choice(value).map_err(|e| e.to_string())?;
        return camera.set_config(&widget).wait().map_err(|e| e.to_string());
    }
    // INT-style entries (e.g. colortemperature) surface as text widgets.
    let widget = camera
        .config_key::<gphoto2::widget::TextWidget>(key)
        .wait()
        .map_err(|e| e.to_string())?;
    widget.set_value(value).map_err(|e| e.to_string())?;
    camera.set_config(&widget).wait().map_err(|e| e.to_string())
}

fn grab_preview_frame(camera: &gphoto2::Camera, ctx: &gphoto2::Context) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let file = camera.capture_preview().wait().map_err(|e| e.to_string())?;
    let data = file.get_data(ctx).wait().map_err(|e| e.to_string())?;
    Ok(STANDARD.encode(&data))
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
