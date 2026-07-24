use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::tethering::unique_path;

#[derive(Default)]
pub struct ScanState {
    child: Arc<Mutex<Option<Child>>>,
    cancelled: Arc<AtomicBool>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScannerDevice {
    pub name: String,
    pub model: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanDetectResult {
    pub scanimage_installed: bool,
    pub device: Option<ScannerDevice>,
}

// Finder-launched apps don't inherit the brew PATH.
fn scanimage_bin() -> PathBuf {
    for p in ["/opt/homebrew/bin/scanimage", "/usr/local/bin/scanimage"] {
        if Path::new(p).exists() {
            return PathBuf::from(p);
        }
    }
    PathBuf::from("scanimage")
}

#[tauri::command]
pub async fn scan_detect_scanner() -> Result<ScanDetectResult, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut child = match Command::new(scanimage_bin())
            .args(["-f", "%d|%v %m%n"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ScanDetectResult { scanimage_installed: false, device: None });
            }
            Err(e) => return Err(e.to_string()),
        };
        // The probe blocks indefinitely on a wedged USB device — bound it.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if std::time::Instant::now() > deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Ok(ScanDetectResult { scanimage_installed: true, device: None });
                }
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(200)),
                Err(e) => return Err(e.to_string()),
            }
        }
        let mut stdout = String::new();
        if let Some(mut out) = child.stdout.take() {
            let _ = out.read_to_string(&mut stdout);
        }
        let device = stdout.lines().find_map(|l| {
            let (name, model) = l.split_once('|')?;
            Some(ScannerDevice { name: name.trim().to_string(), model: model.trim().to_string() })
        });
        Ok(ScanDetectResult { scanimage_installed: true, device })
    })
    .await
    .map_err(|e| e.to_string())?
}

enum ScanFail {
    Cancelled,
    Error(String),
}

// Spawns scanimage writing to tmp_out, streams `Progress: NN.N%\r` stderr lines
// as scan-progress events, and reaps the child. Shared by preview and scan.
fn run_scanimage(
    state_child: &Arc<Mutex<Option<Child>>>,
    cancelled: &Arc<AtomicBool>,
    app_handle: &AppHandle,
    mode: &str,
    dpi: u32,
    tmp_out: &Path,
    pass: u32,
    passes: u32,
) -> Result<(), ScanFail> {
    // No -d: the 7600i re-enumerates on libusb after every scanimage run, so any
    // cached address goes stale; letting scanimage pick the first scanner both
    // avoids that and saves a separate probe.
    let mut child = Command::new(scanimage_bin())
        .args([
            "--source", "Transparency Adapter",
            "--mode", mode,
            "--depth", "16",
            "--resolution", &dpi.to_string(),
            "--format=tiff",
            "--progress",
            "-o", &tmp_out.to_string_lossy(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ScanFail::Error(format!("failed to launch scanimage: {e}")))?;

    let stderr = child.stderr.take();
    cancelled.store(false, Ordering::SeqCst);
    *state_child.lock().unwrap() = Some(child);

    // With the scanner absent or wedged, scanimage can block forever without a
    // byte of output; kill it if nothing arrives before the deadline (cold-start
    // lamp warmup + calibration stay well under it).
    let got_output = Arc::new(AtomicBool::new(false));
    let timed_out = Arc::new(AtomicBool::new(false));
    {
        let child_slot = state_child.clone();
        let got_output = got_output.clone();
        let timed_out = timed_out.clone();
        std::thread::spawn(move || {
            for _ in 0..24 {
                std::thread::sleep(std::time::Duration::from_secs(5));
                if got_output.load(Ordering::SeqCst) || child_slot.lock().unwrap().is_none() {
                    return;
                }
            }
            if let Some(c) = child_slot.lock().unwrap().as_mut() {
                timed_out.store(true, Ordering::SeqCst);
                let _ = c.kill();
            }
        });
    }

    let mut last_line = String::new();
    let mut last_pct = -1i32;
    if let Some(stderr) = stderr {
        let mut buf = Vec::new();
        for byte in stderr.bytes().flatten() {
            got_output.store(true, Ordering::SeqCst);
            if byte != b'\r' && byte != b'\n' {
                buf.push(byte);
                continue;
            }
            let line = String::from_utf8_lossy(&buf).trim().to_string();
            buf.clear();
            if line.is_empty() {
                continue;
            }
            if let Some(pct) = line.strip_prefix("Progress: ").and_then(|p| p.trim_end_matches('%').parse::<f32>().ok()) {
                // Multi-sample scans report one continuous 0-100 across all passes.
                let whole = ((pass as f32 * 100.0 + pct) / passes.max(1) as f32) as i32;
                if whole != last_pct {
                    last_pct = whole;
                    let _ = app_handle.emit("scan-progress", serde_json::json!({ "percent": whole }));
                }
            } else {
                last_line = line;
            }
        }
    }

    let status = match state_child.lock().unwrap().take() {
        Some(mut c) => c.wait().map_err(|e| ScanFail::Error(e.to_string()))?,
        None => return Err(ScanFail::Error("scan child vanished".into())),
    };

    if cancelled.load(Ordering::SeqCst) {
        let _ = std::fs::remove_file(tmp_out);
        return Err(ScanFail::Cancelled);
    }
    if timed_out.load(Ordering::SeqCst) {
        let _ = std::fs::remove_file(tmp_out);
        return Err(ScanFail::Error(
            "Scanner is not responding — power-cycle it, check the USB connection, then try again".into(),
        ));
    }
    if !status.success() {
        let _ = std::fs::remove_file(tmp_out);
        let msg = if last_line.is_empty() { format!("scanimage exited with {status}") } else { last_line };
        return Err(ScanFail::Error(msg));
    }
    Ok(())
}

fn scan_mode(film_type: &str) -> &'static str {
    if film_type == "bw" { "Gray" } else { "Color" }
}

// Average N identical passes into one 16-bit TIFF — multi-sampling à la
// SilverFast: same exposure each pass (genesys exposes no exposure control),
// so this cuts shadow noise rather than extending dynamic range.
// ponytail: no sub-pixel registration — the carriage repeats well enough;
// add alignment only if a real scan shows edge doubling.
fn average_scans(passes: &[PathBuf], out: &Path) -> Result<(), String> {
    let first = image::open(&passes[0]).map_err(|e| e.to_string())?.to_rgb16();
    let (w, h) = first.dimensions();
    let mut acc: Vec<u32> = first.as_raw().iter().map(|&v| v as u32).collect();
    for p in &passes[1..] {
        let img = image::open(p).map_err(|e| e.to_string())?.to_rgb16();
        let (pw, ph) = img.dimensions();
        // Passes can drift by a row; sum the overlap.
        let (cw, ch) = (w.min(pw), h.min(ph));
        let src_raw = img.as_raw();
        for y in 0..ch {
            let src = (y * pw * 3) as usize;
            let dst = (y * w * 3) as usize;
            for k in 0..(cw * 3) as usize {
                acc[dst + k] += src_raw[src + k] as u32;
            }
        }
    }
    let n = passes.len() as u32;
    let data: Vec<u16> = acc.iter().map(|&v| (v / n) as u16).collect();
    image::ImageBuffer::<image::Rgb<u16>, Vec<u16>>::from_raw(w, h, data)
        .ok_or_else(|| "size mismatch averaging scan passes".to_string())?
        .save_with_format(out, image::ImageFormat::Tiff)
        .map_err(|e| e.to_string())
}

// The scan window is wider than the film frame: it also contains the holder's
// opaque bars (density far beyond any film) and open-lamp white, both of which
// wreck the converter's percentile stretch. Bounds must come from film only —
// find the largest contiguous span per axis that isn't holder-black, inset it,
// and crop. Falls back to the full image when detection looks implausible.
fn detect_frame_crop(img: &image::DynamicImage) -> image::DynamicImage {
    let small = img.thumbnail(400, 400).to_rgb32f();
    let (w, h) = small.dimensions();
    if w < 20 || h < 20 {
        return img.clone();
    }
    // Red channel: brightest through the C-41 orange mask, fine for B&W/E-6 too.
    let col_mean: Vec<f32> = (0..w)
        .map(|x| (0..h).map(|y| small.get_pixel(x, y)[0]).sum::<f32>() / h as f32)
        .collect();
    let row_mean: Vec<f32> = (0..h)
        .map(|y| (0..w).map(|x| small.get_pixel(x, y)[0]).sum::<f32>() / w as f32)
        .collect();

    // Holder bars sit at the scanner's noise floor; anything within a small
    // absolute margin of the darkest line is treated as separator.
    fn largest_bright_run(means: &[f32]) -> (usize, usize) {
        let floor = means.iter().cloned().fold(f32::MAX, f32::min);
        let thr = floor + 0.015;
        let (mut best, mut cur) = ((0usize, 0usize), None::<usize>);
        for (i, &m) in means.iter().enumerate() {
            match (m > thr, cur) {
                (true, None) => cur = Some(i),
                (false, Some(s)) => {
                    if i - s > best.1 - best.0 {
                        best = (s, i);
                    }
                    cur = None;
                }
                _ => {}
            }
        }
        if let Some(s) = cur {
            if means.len() - s > best.1 - best.0 {
                best = (s, means.len());
            }
        }
        best
    }

    let (cx0, cx1) = largest_bright_run(&col_mean);
    let (cy0, cy1) = largest_bright_run(&row_mean);
    if cx1 - cx0 < (w as usize) * 3 / 10 || cy1 - cy0 < (h as usize) * 3 / 10 {
        return img.clone();
    }
    // Inset to stay clear of soft bar edges and light bleed.
    let inset_x = (cx1 - cx0) / 12;
    let inset_y = (cy1 - cy0) / 12;
    let sx = img.width() as f32 / w as f32;
    let sy = img.height() as f32 / h as f32;
    let x = ((cx0 + inset_x) as f32 * sx) as u32;
    let y = ((cy0 + inset_y) as f32 * sy) as u32;
    let cw = ((cx1 - cx0 - 2 * inset_x) as f32 * sx) as u32;
    let ch = ((cy1 - cy0 - 2 * inset_y) as f32 * sy) as u32;
    img.crop_imm(x.min(img.width() - 1), y.min(img.height() - 1), cw.max(1), ch.max(1))
}

// Auto-tone: anchor the frame's median to ~0.38 display. Two knobs both darken,
// with different character:
//   - curve-center shift (`exposure`): moves the median onto the steep part of
//     the sigmoid → rich, contrasty result. But the curve re-pins its endpoints,
//     so past a center of ~0.95 it flattens out and can't darken further (this is
//     why a plain exposure solve pegged at -6.4 and still blew out a bright scan).
//   - pre-curve gain: scales density before the curve → monotonic, unlimited
//     range, but lands the median on the flat toe → milky, low-contrast.
// So use the center shift as the primary (rich) knob up to a safe limit, and only
// once it's exhausted let gain finish the darkening (no blow-out). Returns
// (exposure, gain); solved at DEFAULT_SCAN_CONTRAST so contrast still bites.
const AUTO_TARGET: f32 = 0.38;
const SAFE_X0_MAX: f32 = 0.95;

fn auto_tone_for(
    crop: &image::DynamicImage,
    bounds: [crate::negative_conversion::ChannelBounds; 3],
    contrast: f32,
) -> (f32, f32) {
    let small = crop.thumbnail(540, 540).to_rgb32f();
    let mut n_green: Vec<f32> = small
        .as_raw()
        .chunks(3)
        .map(|px| {
            let d = -px[1].clamp(1e-6, 1.0).log10();
            ((d - bounds[1].min) / (bounds[1].max - bounds[1].min)).clamp(0.0, 1.5)
        })
        .collect();
    if n_green.is_empty() {
        return (0.0, 1.0);
    }
    let mid = n_green.len() / 2;
    n_green.select_nth_unstable_by(mid, f32::total_cmp);
    let n_med = n_green[mid].max(1e-4);

    let target = AUTO_TARGET.powf(2.2);
    let k = 4.0 * contrast;
    // Normalized sigmoid with center x0 and pre-curve gain w, mirroring run_pipeline.
    let curve = |v: f32, x0: f32| -> f32 {
        let y0 = 1.0 / (1.0 + (k * x0).exp());
        let y1 = 1.0 / (1.0 + (-k * (1.0 - x0)).exp());
        let sigmoid = 1.0 / (1.0 + (-k * (v - x0)).exp());
        ((sigmoid - y0) / (y1 - y0)).clamp(0.0, 1.0)
    };

    let (mut x0, mut gain) = (0.6f32, 1.0f32);
    if curve(n_med, SAFE_X0_MAX) > target {
        // Even at the safe center limit it's still too bright — shift to the limit,
        // then bisect gain (<1) to bring the median the rest of the way down.
        x0 = SAFE_X0_MAX;
        let (mut lo, mut hi) = (0.02f32, 1.0f32);
        for _ in 0..30 {
            let w = (lo + hi) / 2.0;
            if curve(n_med * w, x0) > target { hi = w } else { lo = w }
        }
        gain = (lo + hi) / 2.0;
    } else if curve(n_med, 0.1) < target {
        // Too dark even at min shift — brighten with gain (>1) at a neutral center.
        let (mut lo, mut hi) = (1.0f32, 8.0f32);
        for _ in 0..30 {
            let w = (lo + hi) / 2.0;
            if curve(n_med * w, x0) < target { lo = w } else { hi = w }
        }
        gain = (lo + hi) / 2.0;
    } else {
        // Pure center shift hits the anchor — the rich path most frames take.
        let (mut lo, mut hi) = (0.1f32, SAFE_X0_MAX);
        for _ in 0..30 {
            let c = (lo + hi) / 2.0;
            if curve(n_med, c) > target { lo = c } else { hi = c }
        }
        x0 = (lo + hi) / 2.0;
    }
    (((0.6 - x0) / 0.25).clamp(-7.0, 3.0), gain)
}

pub const DEFAULT_SCAN_CONTRAST: f32 = 1.5;


// Sidecar for a freshly scanned file: negative conversion (bounds from the
// detected frame, auto exposure, manual gain as uniform weights) and/or
// rotation. set_negative_conversion would recompute bounds over the full scan
// window (holder bars included) and churn caches a brand-new file doesn't have.
// The user's pane tune is written as REAL editor adjustments (`brightness` is
// the editor's visible Exposure slider, `contrast` its Contrast) so it shows up
// re-adjustable in the edit view — the conversion itself stays purely auto.
fn write_scan_sidecar(
    target: &Path,
    negative: Option<([crate::negative_conversion::ChannelBounds; 3], f32, f32)>,
    rotation_steps: u32,
    brightness: f32,
    contrast: f32,
) -> Result<(), String> {
    let steps = rotation_steps % 4;
    if negative.is_none() && steps == 0 && brightness == 0.0 && contrast == 0.0 {
        return Ok(());
    }
    let sidecar = crate::exif_processing::get_primary_sidecar_path(target);
    let mut meta = crate::exif_processing::load_sidecar(&sidecar);
    if !meta.adjustments.is_object() {
        meta.adjustments = serde_json::json!({});
    }
    if let Some((bounds, exposure, weight)) = negative {
        meta.adjustments["negativeConversion"] = serde_json::json!({
            "enabled": true,
            "exposure": exposure,
            "contrast": DEFAULT_SCAN_CONTRAST,
            "redWeight": weight,
            "greenWeight": weight,
            "blueWeight": weight,
            "bounds": [
                [bounds[0].min, bounds[0].max],
                [bounds[1].min, bounds[1].max],
                [bounds[2].min, bounds[2].max],
            ],
        });
    }
    if steps > 0 {
        meta.adjustments["orientationSteps"] = serde_json::json!(steps);
    }
    if brightness != 0.0 {
        meta.adjustments["brightness"] = serde_json::json!(brightness);
    }
    if contrast != 0.0 {
        meta.adjustments["contrast"] = serde_json::json!(contrast);
    }
    let json = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    std::fs::write(&sidecar, json).map_err(|e| e.to_string())
}

fn preview_tif_path() -> PathBuf {
    std::env::temp_dir().join(format!("rapidraw-scan-preview-{}.tif", std::process::id()))
}

// Render the (cached) preview TIFF exactly like the library would: bake the
// same sidecar a real scan gets, then push it through the app's own thumbnail
// engine (load → negative conversion → GPU pipeline). Preview == library by
// construction — a hand-rolled lookalike renderer kept drifting from it.
fn render_preview(
    tif: &Path,
    film_type: &str,
    exposure_offset: f32,
    contrast: f32,
    rotation_steps: u32,
    app_handle: &AppHandle,
) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use tauri::Manager;

    // Fresh sidecar every render — a film-type switch must not inherit stale params.
    let sidecar = crate::exif_processing::get_primary_sidecar_path(tif);
    let _ = std::fs::remove_file(&sidecar);
    if film_type != "e6" {
        let img = image::open(tif).map_err(|e| e.to_string())?;
        let crop = detect_frame_crop(&img);
        let bounds = crate::negative_conversion::analyze_bounds_for(&crop);
        let (base_exposure, auto_gain) = auto_tone_for(&crop, bounds, DEFAULT_SCAN_CONTRAST);
        write_scan_sidecar(
            tif,
            Some((bounds, base_exposure, auto_gain)),
            rotation_steps,
            exposure_offset,
            contrast,
        )?;
    } else {
        write_scan_sidecar(tif, None, rotation_steps, exposure_offset, contrast)?;
    }

    let path_str = tif.to_string_lossy().to_string();
    let state = app_handle.state::<crate::AppState>();
    // Slider changes alter conversion params but not geometry; the engine's
    // geometry cache would otherwise serve the previously converted base.
    state.thumbnail_geometry_cache.lock().unwrap().remove(&path_str);
    let context = crate::gpu_processing::get_or_init_gpu_context(&state, app_handle).ok();
    let img = crate::file_management::generate_thumbnail_data(&path_str, context.as_ref(), None, app_handle)
        .map_err(|e| e.to_string())?;

    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 88)
        .encode_image(&img.to_rgb8())
        .map_err(|e| e.to_string())?;
    Ok(format!("data:image/jpeg;base64,{}", STANDARD.encode(&jpeg)))
}

#[tauri::command]
pub async fn scan_preview(
    film_type: String,
    exposure_offset: f32,
    contrast: f32,
    rotation_steps: u32,
    state: tauri::State<'_, ScanState>,
    app_handle: AppHandle,
) -> Result<String, String> {
    if state.child.lock().unwrap().is_some() {
        return Err("A scan is already running".into());
    }
    let child_slot = state.child.clone();
    let cancelled = state.cancelled.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Kept on disk so the exposure/contrast sliders can re-render without rescanning.
        let tmp = preview_tif_path();
        // 1800 dpi: the 900 dpi mode carries periodic decimation noise that the
        // negative auto-stretch amplifies into a visible grid.
        run_scanimage(&child_slot, &cancelled, &app_handle, scan_mode(&film_type), 1800, &tmp, 0, 1).map_err(
            |e| match e {
                ScanFail::Cancelled => "cancelled".to_string(),
                ScanFail::Error(m) => m,
            },
        )?;
        render_preview(&tmp, &film_type, exposure_offset, contrast, rotation_steps, &app_handle)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn scan_rerender_preview(
    film_type: String,
    exposure_offset: f32,
    contrast: f32,
    rotation_steps: u32,
    state: tauri::State<'_, ScanState>,
    app_handle: AppHandle,
) -> Result<String, String> {
    if state.child.lock().unwrap().is_some() {
        return Err("A scan is already running".into());
    }
    let tif = preview_tif_path();
    if !tif.exists() {
        return Err("No preview scanned yet".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        render_preview(&tif, &film_type, exposure_offset, contrast, rotation_steps, &app_handle)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn scan_start(
    dpi: u32,
    film_type: String,
    exposure_offset: f32,
    contrast: f32,
    rotation_steps: u32,
    samples: u32,
    dest_folder: String,
    file_name: String,
    state: tauri::State<'_, ScanState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    if state.child.lock().unwrap().is_some() {
        return Err("A scan is already running".into());
    }
    let dest = PathBuf::from(&dest_folder);
    if !dest.is_dir() {
        return Err(format!("Destination folder does not exist: {dest_folder}"));
    }
    let child_slot = state.child.clone();
    let cancelled = state.cancelled.clone();
    log::info!("[scan] starting {file_name} @ {dpi}dpi into {dest_folder}");

    std::thread::spawn(move || {
        // Unsupported extension keeps the partial file out of library listings;
        // same dir keeps the final rename atomic.
        let tmp = dest.join(".rapidraw-scan.partial");
        let samples = samples.clamp(1, 4);
        let scan_result = if samples == 1 {
            run_scanimage(&child_slot, &cancelled, &app_handle, scan_mode(&film_type), dpi, &tmp, 0, 1)
        } else {
            let pass_files: Vec<PathBuf> =
                (0..samples).map(|i| dest.join(format!(".rapidraw-scan.pass{i}"))).collect();
            let mut result = Ok(());
            for (i, pass_file) in pass_files.iter().enumerate() {
                if cancelled.load(Ordering::SeqCst) {
                    result = Err(ScanFail::Cancelled);
                    break;
                }
                if let Err(e) = run_scanimage(
                    &child_slot, &cancelled, &app_handle, scan_mode(&film_type), dpi, pass_file,
                    i as u32, samples,
                ) {
                    result = Err(e);
                    break;
                }
            }
            if result.is_ok() {
                log::info!("[scan] averaging {samples} passes");
                result = average_scans(&pass_files, &tmp).map_err(ScanFail::Error);
            }
            for pass_file in &pass_files {
                let _ = std::fs::remove_file(pass_file);
            }
            result
        };
        match scan_result {
            Ok(()) => {
                let target = unique_path(&dest, &file_name);
                if let Err(e) = std::fs::rename(&tmp, &target) {
                    let _ = std::fs::remove_file(&tmp);
                    log::error!("[scan] rename to {} failed: {e}", target.display());
                    let _ = app_handle.emit("scan-error", serde_json::json!({ "message": e.to_string() }));
                    return;
                }
                let path = target.to_string_lossy().to_string();
                // Sidecar is written before scan-complete, so the library's first
                // sight of the file (and its first thumbnail) is already converted.
                let result = if film_type != "e6" {
                    image::open(&target).map_err(|e| e.to_string()).and_then(|img| {
                        let crop = detect_frame_crop(&img);
                        let bounds = crate::negative_conversion::analyze_bounds_for(&crop);
                        let (base_exposure, auto_gain) = auto_tone_for(&crop, bounds, DEFAULT_SCAN_CONTRAST);
                        write_scan_sidecar(
                            &target,
                            Some((bounds, base_exposure, auto_gain)),
                            rotation_steps,
                            exposure_offset,
                            contrast,
                        )
                    })
                } else {
                    write_scan_sidecar(&target, None, rotation_steps, exposure_offset, contrast)
                };
                if let Err(e) = result {
                    log::warn!("[scan] sidecar write failed for {path}: {e}");
                }
                log::info!("[scan] saved {path}");
                let file_name = target.file_name().unwrap_or_default().to_string_lossy().to_string();
                let _ = app_handle.emit("scan-complete", serde_json::json!({ "path": path, "fileName": file_name }));
            }
            Err(ScanFail::Cancelled) => {
                let _ = app_handle.emit("scan-cancelled", ());
            }
            Err(ScanFail::Error(m)) => {
                let _ = app_handle.emit("scan-error", serde_json::json!({ "message": m }));
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{average_scans, detect_frame_crop};

    #[test]
    fn average_scans_midpoints_two_passes() {
        let dir = std::env::temp_dir().join(format!("rr-avg-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mk = |name: &str, v: u16| {
            let p = dir.join(name);
            image::ImageBuffer::<image::Rgb<u16>, Vec<u16>>::from_pixel(8, 6, image::Rgb([v, v, v]))
                .save_with_format(&p, image::ImageFormat::Tiff)
                .unwrap();
            p
        };
        let passes = vec![mk("a.tif", 1000), mk("b.tif", 3000)];
        let out = dir.join("out.tif");
        average_scans(&passes, &out).unwrap();
        let avg = image::open(&out).unwrap().to_rgb16();
        assert_eq!(avg.dimensions(), (8, 6));
        assert_eq!(avg.get_pixel(4, 3)[1], 2000);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn frame_crop_trims_holder_bars_and_lamp_area() {
        // Synthetic scan window: black holder bars framing the film, open-lamp
        // white to the right of the second bar — the layout that broke bounds.
        let mut img = image::Rgb32FImage::from_pixel(400, 200, image::Rgb([0.3f32, 0.2, 0.1]));
        for y in 0..200 {
            for x in 0..400 {
                let holder_bar = x < 8 || (280..300).contains(&x) || y < 6 || y > 192;
                let lamp = x >= 300;
                if holder_bar {
                    img.put_pixel(x, y, image::Rgb([0.004, 0.004, 0.004]));
                } else if lamp {
                    img.put_pixel(x, y, image::Rgb([0.98, 0.97, 0.96]));
                }
            }
        }
        let crop = detect_frame_crop(&image::DynamicImage::ImageRgb32F(img));
        // Crop must live strictly inside the film area (8..280 x 6..192), clear
        // of both bars and the lamp strip.
        assert!(crop.width() > 180 && crop.width() < 272, "width {}", crop.width());
        assert!(crop.height() > 120 && crop.height() < 186, "height {}", crop.height());
    }
}

#[tauri::command]
pub fn scan_cancel(state: tauri::State<'_, ScanState>) -> Result<(), String> {
    state.cancelled.store(true, Ordering::SeqCst);
    if let Some(child) = state.child.lock().unwrap().as_mut() {
        let _ = child.kill();
    }
    Ok(())
}
