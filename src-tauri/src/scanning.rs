use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::tethering::unique_path;

// Tone solve from the last preview: (bounds, exposure, gain). Scan frame reuses
// it once so the saved render matches the tuned preview exactly instead of
// re-solving on a fresh pass that can land slightly differently.
type ToneSolve = ([crate::negative_conversion::ChannelBounds; 3], f32, f32, f32);

// Optional shooting metadata the user enters in the scan pane. scanimage TIFFs
// carry none, so the library has nothing film-specific to catalogue on. Written
// to the sidecar (RapidRAW's metadata panel + export) and embedded as EXIF tags.
#[derive(Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilmMeta {
    #[serde(default)]
    pub film_stock: Option<String>,
    #[serde(default)]
    pub iso: Option<u32>,
    #[serde(default)]
    pub camera: Option<String>,
    #[serde(default)]
    pub lens: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

impl FilmMeta {
    fn field(o: &Option<String>) -> Option<&str> {
        o.as_deref().map(str::trim).filter(|s| !s.is_empty())
    }
}

#[derive(Default)]
pub struct ScanState {
    child: Arc<Mutex<Option<Child>>>,
    cancelled: Arc<AtomicBool>,
    preview_tone: Arc<Mutex<Option<ToneSolve>>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScannerDevice {
    pub name: String,
    pub model: String,
}

// Preview JPEG plus, when auto-crop is on, the detected frame rect normalized
// to the displayed image [x, y, w, h] so the pane can dim the area outside it.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResult {
    pub data: String,
    pub crop: Option<[f32; 4]>,
}

// What the DETECTED scanner actually supports, parsed from `scanimage -A` so the
// UI offers real options instead of the 7600i's hardcoded ones. None when the
// device couldn't be queried (found but won't open).
#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScannerCaps {
    pub source_visible: String,          // the transparency source to scan through
    pub source_infrared: Option<String>, // present only if the device exposes IR
    pub resolutions: Vec<u32>,
    pub default_resolution: u32,
    pub max_depth: u32,
    pub has_transparency: bool,          // false = document scanner, no film support
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanDetectResult {
    pub scanimage_installed: bool,
    pub device: Option<ScannerDevice>,
    pub caps: Option<ScannerCaps>,
}

// Finder-launched apps don't inherit the brew PATH; Linux keeps scanimage in
// /usr/bin (native SANE home), which a minimal desktop-launcher PATH may miss.
fn scanimage_bin() -> PathBuf {
    for p in ["/opt/homebrew/bin/scanimage", "/usr/local/bin/scanimage", "/usr/bin/scanimage"] {
        if Path::new(p).exists() {
            return PathBuf::from(p);
        }
    }
    PathBuf::from("scanimage")
}

// Stop scanimage gracefully: SIGTERM first so it releases the USB interface,
// escalating to SIGKILL only if it ignores that for ~2s. A hard SIGKILL mid-
// transfer leaves the genesys device wedged until a physical re-plug (learned
// the hard way). Falls back to a plain kill on non-Unix.
fn terminate_child(child: &mut Child) {
    #[cfg(unix)]
    {
        unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) };
        for _ in 0..20 {
            match child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
                Err(_) => break,
            }
        }
    }
    let _ = child.kill();
}

// A film source name is any transparency/film adapter (backend-specific wording).
fn is_film_source(s: &str) -> bool {
    let t = s.to_lowercase();
    ["transparency", "tpu", "film", "slide", "positive", "negative"].iter().any(|k| t.contains(k))
}

// Parse the `scanimage -A` option dump into the capabilities the UI needs.
fn parse_caps(text: &str) -> ScannerCaps {
    let mut caps = ScannerCaps { max_depth: 8, ..Default::default() };
    // Value list sits between "--<opt> " and the " [default]" marker.
    let values = |line: &str, opt: &str| -> Option<String> {
        line.trim().strip_prefix(opt)?.split(" [").next().map(|s| s.trim().to_string())
    };
    for line in text.lines() {
        if let Some(list) = values(line, "--source ") {
            let sources: Vec<&str> = list.split('|').map(|s| s.trim()).collect();
            caps.source_infrared = sources.iter().find(|s| s.to_lowercase().contains("infrared")).map(|s| s.to_string());
            let non_ir: Vec<&str> = sources.iter().copied().filter(|s| !s.to_lowercase().contains("infrared")).collect();
            // Prefer a film source; fall back to the first non-IR source so a scan
            // still attempts (a plain flatbed reports Flatbed here).
            let visible = non_ir.iter().find(|s| is_film_source(s)).or(non_ir.first());
            if let Some(v) = visible {
                caps.source_visible = v.to_string();
            }
            caps.has_transparency = caps.source_infrared.is_some() || sources.iter().any(|s| is_film_source(s));
        } else if let Some(spec) = values(line, "--resolution ") {
            let default = line.split('[').nth(1).and_then(|d| {
                d.trim_end_matches(']').trim().trim_end_matches("dpi").trim().parse::<u32>().ok()
            });
            let spec = spec.trim_end_matches("dpi");
            if spec.contains("..") {
                // Range (e.g. Epson 50..7200): offer common film steps inside it.
                let bounds: Vec<u32> = spec.split("..").filter_map(|s| {
                    s.split_whitespace().next().unwrap_or("").trim_end_matches("dpi").parse::<u32>().ok()
                }).collect();
                if let [mn, mx] = bounds[..] {
                    caps.resolutions = [300u32, 600, 1200, 1800, 2400, 3600, 4800, 6400, 7200]
                        .into_iter().filter(|r| *r >= mn && *r <= mx).collect();
                    if caps.resolutions.is_empty() {
                        caps.resolutions = vec![mn, mx];
                    }
                }
            } else {
                let mut list: Vec<u32> = spec.split('|')
                    .filter_map(|s| s.trim().trim_end_matches("dpi").trim().parse::<u32>().ok())
                    .collect();
                list.sort_unstable();
                caps.resolutions = list;
            }
            caps.default_resolution = default
                .or_else(|| caps.resolutions.first().copied())
                .unwrap_or(1800);
        } else if let Some(spec) = values(line, "--depth ") {
            if let Some(m) = spec.split('|').filter_map(|s| s.trim().parse::<u32>().ok()).filter(|d| *d <= 16).max() {
                caps.max_depth = m;
            }
        }
    }
    caps
}

// Run a scanimage subcommand, bounded so a wedged USB device can't hang the UI.
// Returns None if scanimage isn't installed; Some((success, stdout)) otherwise.
fn run_capture(args: &[&str], timeout_secs: u64) -> Option<(bool, String)> {
    let mut child = match Command::new(scanimage_bin())
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(_) => return Some((false, String::new())),
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut out = String::new();
                if let Some(mut o) = child.stdout.take() {
                    let _ = o.read_to_string(&mut out);
                }
                return Some((status.success(), out));
            }
            Ok(None) if std::time::Instant::now() > deadline => {
                terminate_child(&mut child);
                let _ = child.wait();
                return Some((false, String::new()));
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(200)),
            Err(_) => return Some((false, String::new())),
        }
    }
}

#[tauri::command]
pub async fn scan_detect_scanner() -> Result<ScanDetectResult, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let Some((_, stdout)) = run_capture(&["-f", "%d|%v %m%n"], 20) else {
            return Ok(ScanDetectResult { scanimage_installed: false, device: None, caps: None });
        };
        let device = stdout.lines().find_map(|l| {
            let (name, model) = l.split_once('|')?;
            Some(ScannerDevice { name: name.trim().to_string(), model: model.trim().to_string() })
        });
        // Query the specific device's real capabilities (also an open test — if
        // -A fails the device is present but wedged, so caps stays None).
        let caps = device.as_ref().and_then(|d| {
            let (ok, out) = run_capture(&["-A", "-d", &d.name], 25)?;
            (ok && !out.is_empty()).then(|| parse_caps(&out))
        });
        Ok(ScanDetectResult { scanimage_installed: true, device, caps })
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
    source: &str,
    mode: &str,
    dpi: u32,
    depth: u32,
    tmp_out: &Path,
    pass: u32,
    passes: u32,
) -> Result<(), ScanFail> {
    // No -d: the 7600i re-enumerates on libusb after every scanimage run, so any
    // cached address goes stale; letting scanimage pick the first scanner both
    // avoids that and saves a separate probe.
    let mut child = Command::new(scanimage_bin())
        .args([
            "--source", source,
            "--mode", mode,
            "--depth", &depth.to_string(),
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

    // Rolling inactivity watchdog: scanimage can wedge with the USB device held
    // both at startup (never a byte) AND mid-scan (genesys stalls after some
    // progress). A one-shot first-output check misses the latter and leaves a
    // zombie holding the scanner, so kill whenever output goes quiet for 120s
    // (cold-start warmup/calibration and inter-pass gaps stay well under it).
    let activity = Arc::new(AtomicU64::new(0));
    let timed_out = Arc::new(AtomicBool::new(false));
    {
        let child_slot = state_child.clone();
        let activity = activity.clone();
        let timed_out = timed_out.clone();
        std::thread::spawn(move || {
            let (mut last_seen, mut stale) = (0u64, 0u32);
            loop {
                std::thread::sleep(std::time::Duration::from_secs(5));
                if child_slot.lock().unwrap().is_none() {
                    return;
                }
                let now = activity.load(Ordering::SeqCst);
                stale = if now == last_seen { stale + 1 } else { 0 };
                last_seen = now;
                if stale >= 24 {
                    if let Some(c) = child_slot.lock().unwrap().as_mut() {
                        timed_out.store(true, Ordering::SeqCst);
                        terminate_child(c);
                    }
                    return;
                }
            }
        });
    }

    let mut last_line = String::new();
    let mut last_pct = -1i32;
    if let Some(stderr) = stderr {
        let mut buf = Vec::new();
        for byte in stderr.bytes().flatten() {
            if byte != b'\r' && byte != b'\n' {
                buf.push(byte);
                continue;
            }
            let line = String::from_utf8_lossy(&buf).trim().to_string();
            buf.clear();
            if line.is_empty() {
                continue;
            }
            activity.fetch_add(1, Ordering::SeqCst);
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

const SRC_VISIBLE: &str = "Transparency Adapter";
const SRC_INFRARED: &str = "Transparency Adapter Infrared";

// image::open guesses the format from the extension (scan temps like .partial
// have none it recognizes) and its default decode limit rejects 7200dpi TIFFs
// (~437MB > 512MB cap with overhead) — sniff the bytes and lift the limit.
fn open_image(path: &Path) -> Result<image::DynamicImage, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    reader.no_limits();
    reader.decode().map_err(|e| e.to_string())
}

// --- IR dust/scratch removal (Digital-ICE style) -------------------------
// Film dyes pass infrared; dust, scratches and fibers block it. An IR pass of
// the same frame shows defects as dark spots on a nearly blank field, so:
// mask = pixels well below the local IR background, fill = nearest clean
// pixels in four directions, inverse-distance weighted. Registration between
// passes measured dead-on (dx=dy=0) on real hardware, so no alignment step;
// the 2px dilation absorbs sub-pixel drift.

fn box_blur(src: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
    let mut tmp = vec![0.0f32; src.len()];
    let mut out = vec![0.0f32; src.len()];
    for y in 0..h {
        let row = &src[y * w..(y + 1) * w];
        let mut sum: f32 = row[..=(r.min(w - 1))].iter().sum();
        let mut cnt = r.min(w - 1) + 1;
        for x in 0..w {
            tmp[y * w + x] = sum / cnt as f32;
            if x + r + 1 < w {
                sum += row[x + r + 1];
                cnt += 1;
            }
            if x >= r {
                sum -= row[x - r];
                cnt -= 1;
            }
        }
    }
    for x in 0..w {
        let mut sum = 0.0f32;
        let mut cnt = 0usize;
        for y in 0..=r.min(h - 1) {
            sum += tmp[y * w + x];
            cnt += 1;
        }
        for y in 0..h {
            out[y * w + x] = sum / cnt as f32;
            if y + r + 1 < h {
                sum += tmp[(y + r + 1) * w + x];
                cnt += 1;
            }
            if y >= r {
                sum -= tmp[(y - r) * w + x];
                cnt -= 1;
            }
        }
    }
    out
}

fn dilate(mask: &mut [bool], w: usize, h: usize, r: usize) {
    let src = mask.to_vec();
    for y in 0..h {
        for x in 0..w {
            if src[y * w + x] {
                for dy in y.saturating_sub(r)..=(y + r).min(h - 1) {
                    for dx in x.saturating_sub(r)..=(x + r).min(w - 1) {
                        mask[dy * w + dx] = true;
                    }
                }
            }
        }
    }
}

// Defect mask from the IR gray frame; vis_luma gates out the holder/border,
// which blocks IR too but isn't film.
// `thr_mult` scales the MAD threshold: lower catches more defects (aggressive,
// risks softening detail), higher catches only strong ones (conservative). 6.0
// is the neutral default.
fn ir_defect_mask(ir_gray: &[f32], vis_luma: &[f32], w: usize, h: usize, dpi: u32, thr_mult: f32) -> Vec<bool> {
    let r = ((12 * dpi) / 1800).clamp(4, 48) as usize;
    let bg = box_blur(ir_gray, w, h, r);
    let resid: Vec<f32> = bg.iter().zip(ir_gray).map(|(b, v)| b - v).collect();
    let mut sample: Vec<f32> = resid.iter().step_by(97).copied().collect();
    sample.sort_by(f32::total_cmp);
    let med = sample[sample.len() / 2];
    let mut dev: Vec<f32> = sample.iter().map(|v| (v - med).abs()).collect();
    dev.sort_by(f32::total_cmp);
    let mad = dev[dev.len() / 2];
    let thr = (thr_mult * 1.4826 * mad).max(8.0 / 255.0);
    let mut mask: Vec<bool> = resid
        .iter()
        .zip(vis_luma)
        .map(|(rv, lv)| *rv > thr && *lv > 0.015)
        .collect();
    dilate(&mut mask, w, h, 2);
    mask
}

// Fill masked pixels from the nearest unmasked pixel in each of the four
// directions, weighted by inverse distance. Linear passes keep it O(n).
fn fill_masked(rgb: &mut [f32], mask: &[bool], w: usize, h: usize) {
    let mut acc = vec![0.0f32; rgb.len()];
    let mut wsum = vec![0.0f32; w * h];
    let mut pass = |iter: &mut dyn Iterator<Item = usize>, stride_reset: usize| {
        let mut last: Option<(usize, usize)> = None; // (index, distance-counter start)
        let mut steps = 0usize;
        for (n, i) in iter.enumerate() {
            if n % stride_reset == 0 {
                last = None;
            }
            if !mask[i] {
                last = Some((i, 0));
                steps = 0;
            } else if let Some((j, _)) = last {
                steps += 1;
                let wt = 1.0 / steps as f32;
                acc[i * 3] += rgb[j * 3] * wt;
                acc[i * 3 + 1] += rgb[j * 3 + 1] * wt;
                acc[i * 3 + 2] += rgb[j * 3 + 2] * wt;
                wsum[i] += wt;
            }
        }
    };
    // left→right and right→left per row
    pass(&mut (0..h).flat_map(|y| (0..w).map(move |x| y * w + x)), w);
    pass(&mut (0..h).flat_map(|y| (0..w).rev().map(move |x| y * w + x)), w);
    // top→bottom and bottom→top per column
    pass(&mut (0..w).flat_map(|x| (0..h).map(move |y| y * w + x)), h);
    pass(&mut (0..w).flat_map(|x| (0..h).rev().map(move |y| y * w + x)), h);
    for i in 0..w * h {
        if mask[i] && wsum[i] > 0.0 {
            for c in 0..3 {
                rgb[i * 3 + c] = acc[i * 3 + c] / wsum[i];
            }
        }
    }
}

// 2x2 average, dropping any odd trailing row/column.
fn downsample2(src: &[f32], w: usize, h: usize) -> Vec<f32> {
    let (dw, dh) = (w / 2, h / 2);
    let mut out = vec![0.0f32; dw * dh];
    for y in 0..dh {
        for x in 0..dw {
            out[y * dw + x] = (src[2 * y * w + 2 * x]
                + src[2 * y * w + 2 * x + 1]
                + src[(2 * y + 1) * w + 2 * x]
                + src[(2 * y + 1) * w + 2 * x + 1])
                / 4.0;
        }
    }
    out
}

// Run the IR clean on a finished visible scan: returns defect pixel count.
// `sensitivity` (0..100, 50 = default) tunes how aggressively defects are caught:
// higher removes more (lower MAD multiplier), lower is more conservative.
fn ir_clean_scan(vis_path: &Path, ir_path: &Path, dpi: u32, sensitivity: f32) -> Result<usize, String> {
    // 50 -> 6.0 (neutral); 0 -> 9.5 (conservative); 100 -> 2.5 (aggressive).
    let thr_mult = (9.5 - 0.07 * sensitivity.clamp(0.0, 100.0)).clamp(2.5, 9.5);
    let vis = open_image(vis_path)?.to_rgb32f();
    let ir = open_image(ir_path)?.to_rgb32f();
    let (w, h) = vis.dimensions();
    if ir.dimensions() != (w, h) {
        return Err("IR pass dimensions differ from visible scan".into());
    }
    let (w, h) = (w as usize, h as usize);
    let ir_gray: Vec<f32> = ir.as_raw().chunks_exact(3).map(|c| (c[0] + c[1] + c[2]) / 3.0).collect();
    let vis_luma: Vec<f32> = vis.as_raw().chunks_exact(3).map(|c| (c[0] + c[1] + c[2]) / 3.0).collect();
    // The 7200dpi IR pass is so noisy that a MAD threshold on the full-res
    // residual only catches the biggest chunks (measured: thr 0.165 vs the
    // 0.031 floor, 0.1% masked vs 0.6% on 3600 scans of the same film).
    // Detect on a 2x2-averaged frame instead — 3600-equivalent noise — and
    // upscale the mask; the fill still runs at full resolution.
    let mask = if dpi >= 7200 {
        let (dw, dh) = (w / 2, h / 2);
        let m = ir_defect_mask(
            &downsample2(&ir_gray, w, h),
            &downsample2(&vis_luma, w, h),
            dw,
            dh,
            dpi / 2,
            thr_mult,
        );
        let mut full = vec![false; w * h];
        for y in 0..dh {
            for x in 0..dw {
                if m[y * dw + x] {
                    full[2 * y * w + 2 * x] = true;
                    full[2 * y * w + 2 * x + 1] = true;
                    full[(2 * y + 1) * w + 2 * x] = true;
                    full[(2 * y + 1) * w + 2 * x + 1] = true;
                }
            }
        }
        // cover the upsampling edge at full res
        dilate(&mut full, w, h, 2);
        full
    } else {
        ir_defect_mask(&ir_gray, &vis_luma, w, h, dpi, thr_mult)
    };
    let count = mask.iter().filter(|m| **m).count();
    if count == 0 {
        return Ok(0);
    }
    let mut rgb = vis.into_raw();
    fill_masked(&mut rgb, &mask, w, h);
    let data: Vec<u16> = rgb.iter().map(|v| (v.clamp(0.0, 1.0) * 65535.0) as u16).collect();
    image::ImageBuffer::<image::Rgb<u16>, Vec<u16>>::from_raw(w as u32, h as u32, data)
        .ok_or_else(|| "buffer mismatch in IR clean".to_string())?
        .save_with_format(vis_path, image::ImageFormat::Tiff)
        .map_err(|e| e.to_string())?;
    Ok(count)
}

// Store scans in a deflate+predictor TIFF, keeping `keep_bits` significant
// bits (16 = lossless; at 12 the discarded bits sit ~100x below the scanner's
// measured noise floor and the file lands at about half the raw size —
// 109MB -> 55MB at 3600dpi). The scan date and scanner make/model are embedded
// as real TIFF tags so any application can read them, not just the sidecar.
// Encodes to memory and renames over the original, so a failure never costs
// the scan.
fn compress_scan(path: &Path, keep_bits: u8, scanner_model: &str, meta: &FilmMeta) -> Result<u64, String> {
    use tiff::encoder::{colortype::ColorType, compression::DeflateLevel, Compression, TiffEncoder};
    use tiff::tags::Tag;

    let img = open_image(path)?;
    let mask = !(((1u32 << (16 - keep_bits.clamp(10, 16) as u32)) - 1) as u16);
    let date = chrono::Local::now().format("%Y:%m:%d %H:%M:%S").to_string();

    fn write<C: ColorType<Inner = u16>>(
        buf: &mut std::io::Cursor<Vec<u8>>,
        (w, h): (u32, u32),
        data: &[u16],
        date: &str,
        scanner_model: &str,
        meta: &FilmMeta,
    ) -> Result<(), String> {
        let mut enc = TiffEncoder::new(buf)
            .map_err(|e| e.to_string())?
            .with_compression(Compression::Deflate(DeflateLevel::default()))
            .with_predictor(tiff::tags::Predictor::Horizontal);
        let mut image = enc.new_image::<C>(w, h).map_err(|e| e.to_string())?;
        let d = image.encoder();
        let _ = d.write_tag(Tag::DateTime, date);
        // EXIF DateTimeOriginal; in IFD0 rather than an Exif sub-IFD, which
        // exiftool and most readers accept.
        let _ = d.write_tag(Tag::Unknown(36867), date);
        let _ = d.write_tag(Tag::Software, "RapidRAW film scanner");
        // Make/Model = the camera the frame was shot on when given (what readers
        // expect); otherwise the scanner. The scanner stays named in Software.
        let device = FilmMeta::field(&meta.camera).unwrap_or(scanner_model);
        if let Some((make, model)) = device.split_once(' ') {
            let _ = d.write_tag(Tag::Make, make);
            let _ = d.write_tag(Tag::Model, model);
        } else if !device.is_empty() {
            let _ = d.write_tag(Tag::Model, device);
        }
        if let Some(iso) = meta.iso {
            let _ = d.write_tag(Tag::Unknown(34855), iso as u16); // ISOSpeedRatings
        }
        if let Some(lens) = FilmMeta::field(&meta.lens) {
            let _ = d.write_tag(Tag::Unknown(42036), lens); // LensModel
        }
        if let Some(stock) = FilmMeta::field(&meta.film_stock) {
            let _ = d.write_tag(Tag::ImageDescription, stock);
        }
        image.write_data(data).map_err(|e| e.to_string())
    }

    let mut buf = std::io::Cursor::new(Vec::new());
    match img {
        image::DynamicImage::ImageLuma16(g) => {
            let (w, h) = g.dimensions();
            let mut data = g.into_raw();
            for v in &mut data {
                *v &= mask;
            }
            write::<tiff::encoder::colortype::Gray16>(&mut buf, (w, h), &data, &date, scanner_model, meta)?;
        }
        other => {
            let rgb = other.to_rgb16();
            let (w, h) = rgb.dimensions();
            let mut data = rgb.into_raw();
            for v in &mut data {
                *v &= mask;
            }
            write::<tiff::encoder::colortype::RGB16>(&mut buf, (w, h), &data, &date, scanner_model, meta)?;
        }
    }
    let bytes = buf.into_inner();
    let size = bytes.len() as u64;
    let tmp = path.with_file_name(".rapidraw-scan.compress");
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(size)
}

// Average N identical passes into one 16-bit TIFF — multi-sampling à la
// SilverFast: same exposure each pass (genesys exposes no exposure control),
// so this cuts shadow noise rather than extending dynamic range.
// ponytail: no sub-pixel registration — the carriage repeats well enough;
// add alignment only if a real scan shows edge doubling.
fn average_scans(passes: &[PathBuf], out: &Path) -> Result<(), String> {
    let first = open_image(&passes[0])?.to_rgb16();
    let (w, h) = first.dimensions();
    let mut acc: Vec<u32> = first.as_raw().iter().map(|&v| v as u32).collect();
    for p in &passes[1..] {
        let img = open_image(p)?.to_rgb16();
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
    // Deep inset (span/12 per side): bounds/tone sampling must stay well clear
    // of bar soft-edges and light bleed, cropping into the image is fine there.
    match detect_frame_rect(img, 12) {
        Some((x, y, w, h)) => img.crop_imm(x, y, w, h),
        None => img.clone(),
    }
}

// Frame rect (x, y, w, h) in the full image's pixel space, or None when
// detection looks implausible (caller falls back to the whole frame).
// inset_div: each side is inset by span/inset_div — use a large divisor for a
// user-facing crop that should hug the frame edges.
//
// Detection stops at the holder bars (dark, at the scanner's noise floor) and
// deliberately does NOT try to trim the clear film rebate: on a negative the
// rebate is bright on the red channel, but so is any dark scene area (dark
// subjects expose the film little → clear → bright red), so a brightness-based
// rebate trim can't tell them apart and was eating real content. A thin leftover
// rebate is left for the user to drag off rather than risk cropping the picture.
fn detect_frame_rect(img: &image::DynamicImage, inset_div: usize) -> Option<(u32, u32, u32, u32)> {
    let small = img.thumbnail(400, 400).to_rgb32f();
    let (w, h) = small.dimensions();
    if w < 20 || h < 20 {
        return None;
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
        return None;
    }
    let inset_x = (cx1 - cx0) / inset_div;
    let inset_y = (cy1 - cy0) / inset_div;
    let sx = img.width() as f32 / w as f32;
    let sy = img.height() as f32 / h as f32;
    let x = (((cx0 + inset_x) as f32 * sx) as u32).min(img.width() - 1);
    let y = (((cy0 + inset_y) as f32 * sy) as u32).min(img.height() - 1);
    let cw = (((cx1 - cx0 - 2 * inset_x) as f32 * sx) as u32).max(1).min(img.width() - x);
    let ch = (((cy1 - cy0 - 2 * inset_y) as f32 * sy) as u32).max(1).min(img.height() - y);
    Some((x, y, cw, ch))
}

// Map a rect through orientationSteps quarter-turns (image crate rotate90 = CW),
// so an auto-crop detected on the raw scan lands in the post-orientation space
// the editor's crop is applied in. Returns the rect and the rotated dimensions.
fn rotate_rect(r: (u32, u32, u32, u32), w: u32, h: u32, steps: u32) -> ((u32, u32, u32, u32), (u32, u32)) {
    let (x, y, rw, rh) = r;
    match steps % 4 {
        1 => ((h - (y + rh), x, rh, rw), (h, w)),
        2 => ((w - (x + rw), h - (y + rh), rw, rh), (w, h)),
        3 => ((y, w - (x + rw), rh, rw), (h, w)),
        _ => ((x, y, rw, rh), (w, h)),
    }
}

// Auto-tone: anchor the frame's median to ~0.38 display. Two knobs both darken,
// with different character:
//   - curve-center shift (`exposure`): moves the median onto the steep part of
//     the sigmoid → rich, contrasty result. But the curve re-pins its endpoints,
//     so past a center of ~0.95 it flattens out and can't darken further (this is
//     why a plain exposure solve pegged at -6.4 and still blew out a bright scan).
//   - pre-curve gain: scales density before the curve → monotonic, unlimited
//     range, but lands the median on the flat toe → milky, low-contrast.
// So use the center shift as the primary (rich) knob up to a safe limit. Once
// it's exhausted (very thin scans — genesys underexposes badly at 7200dpi),
// finish the darkening AFTER the conversion via the editor's Exposure key
// instead of pre-curve gain: gain squashes the channels through different parts
// of the sigmoid and the divergent 7200 channel response comes out as a purple/
// yellow crossover, while a post-conversion EV cut is hue-neutral (and the user
// can re-tune it in the edit view). Returns (exposure, gain, brightness_ev);
// solved at DEFAULT_SCAN_CONTRAST so contrast still bites.
const AUTO_TARGET: f32 = 0.38;
const SAFE_X0_MAX: f32 = 0.95;

fn auto_tone_for(
    crop: &image::DynamicImage,
    bounds: [crate::negative_conversion::ChannelBounds; 3],
    contrast: f32,
    hue_safe: bool,
) -> (f32, f32, f32) {
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
        return (0.0, 1.0, 0.0);
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

    // genesys underexposes 7200dpi scans with a per-channel response skew;
    // any curve-center shift or pre-curve gain turns that skew into a purple/
    // yellow crossover. Keep the curve at its neutral center and reach the
    // brightness anchor entirely with a hue-neutral post-conversion EV shift
    // (the editor's Exposure key, re-tunable in the edit view).
    if hue_safe {
        let ev = (target / curve(n_med, 0.6).max(1e-4)).log2().clamp(-5.0, 5.0);
        return (0.0, 1.0, ev);
    }

    let (mut x0, mut gain, ev) = (0.6f32, 1.0f32, 0.0f32);
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
    (((0.6 - x0) / 0.25).clamp(-7.0, 3.0), gain, ev)
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
    crop: Option<(u32, u32, u32, u32)>,
    film_meta: &FilmMeta,
) -> Result<(), String> {
    let steps = rotation_steps % 4;
    let sidecar = crate::exif_processing::get_primary_sidecar_path(target);
    let mut meta = crate::exif_processing::load_sidecar(&sidecar);
    if !meta.adjustments.is_object() {
        meta.adjustments = serde_json::json!({});
    }
    // Scan timestamp as the sidecar's EXIF date — scanimage TIFFs carry no
    // capture date, which leaves the library nothing to sort on.
    if meta.exif.is_none() {
        let now = chrono::Local::now().format("%Y:%m:%d %H:%M:%S").to_string();
        meta.exif = Some(std::collections::HashMap::from([
            ("DateTimeOriginal".to_string(), now.clone()),
            ("CreateDate".to_string(), now),
        ]));
    }
    // Film metadata into the sidecar's EXIF map — shown in the Metadata panel and
    // carried on export. Same tag choices as the embedded TIFF: camera → Make/
    // Model, stock → ImageDescription, notes → UserComment.
    if let Some(exif) = meta.exif.as_mut() {
        if let Some(cam) = FilmMeta::field(&film_meta.camera) {
            match cam.split_once(' ') {
                Some((make, model)) => {
                    exif.insert("Make".into(), make.into());
                    exif.insert("Model".into(), model.into());
                }
                None => {
                    exif.insert("Model".into(), cam.into());
                }
            }
        }
        if let Some(iso) = film_meta.iso {
            exif.insert("ISOSpeedRatings".into(), iso.to_string());
        }
        if let Some(lens) = FilmMeta::field(&film_meta.lens) {
            exif.insert("LensModel".into(), lens.into());
        }
        if let Some(stock) = FilmMeta::field(&film_meta.film_stock) {
            exif.insert("ImageDescription".into(), stock.into());
        }
        if let Some(notes) = FilmMeta::field(&film_meta.notes) {
            exif.insert("UserComment".into(), notes.into());
        }
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
    if let Some((x, y, w, h)) = crop {
        // Post-orientation pixel rect; the editor's crop tool can refine it.
        meta.adjustments["crop"] = serde_json::json!({ "x": x, "y": y, "width": w, "height": h, "unit": "px" });
    }
    let json = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    std::fs::write(&sidecar, json).map_err(|e| e.to_string())
}

fn preview_tif_path() -> PathBuf {
    std::env::temp_dir().join(format!("rapidraw-scan-preview-{}.tif", std::process::id()))
}

// Film-base eyedropper: average a small window around a normalized point (in the
// displayed, oriented preview) and return its per-channel density (-log10),
// matching analyze_bounds' domain. Pins bounds[c].min to the clicked rebate so a
// stubborn orange mask neutralises exactly instead of by percentile estimate.
// The eyedropper's aim view: the raw negative with a display gamma only (no
// inversion), so the orange rebate is visible to click. The scan is near-linear
// and dark (median ~0.07); generate_thumbnail_data would show it almost black
// because the negative conversion is what normally supplies the gamma.
fn raw_preview_data(tif: &Path, rotation_steps: u32) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let img = open_image(tif)?;
    let img = match rotation_steps % 4 {
        1 => img.rotate90(),
        2 => img.rotate180(),
        3 => img.rotate270(),
        _ => img,
    };
    let mut rgb = img.thumbnail(1200, 1200).to_rgb32f();
    for p in rgb.pixels_mut() {
        for c in 0..3 {
            p[c] = p[c].clamp(0.0, 1.0).powf(1.0 / 2.2);
        }
    }
    let rgb8 = image::DynamicImage::ImageRgb32F(rgb).to_rgb8();
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 85)
        .encode_image(&rgb8)
        .map_err(|e| e.to_string())?;
    Ok(format!("data:image/jpeg;base64,{}", STANDARD.encode(&jpeg)))
}

fn sample_base_density(img: &image::DynamicImage, nx: f32, ny: f32, rotation_steps: u32) -> [f32; 3] {
    let oriented = match rotation_steps % 4 {
        1 => img.rotate90(),
        2 => img.rotate180(),
        3 => img.rotate270(),
        _ => img.clone(),
    };
    let rgb = oriented.to_rgb32f();
    let (w, h) = rgb.dimensions();
    if w == 0 || h == 0 {
        return [0.0; 3];
    }
    let cx = (nx.clamp(0.0, 1.0) * (w - 1) as f32) as u32;
    let cy = (ny.clamp(0.0, 1.0) * (h - 1) as f32) as u32;
    let rad = ((w.min(h) as f32 * 0.01) as u32).max(2);
    let (mut sr, mut sg, mut sb, mut n) = (0.0f32, 0.0f32, 0.0f32, 0u32);
    for y in cy.saturating_sub(rad)..=(cy + rad).min(h - 1) {
        for x in cx.saturating_sub(rad)..=(cx + rad).min(w - 1) {
            let p = rgb.get_pixel(x, y).0;
            sr += p[0];
            sg += p[1];
            sb += p[2];
            n += 1;
        }
    }
    let n = n.max(1) as f32;
    let dens = |s: f32| -(s / n).clamp(1e-6, 1.0).log10();
    [dens(sr), dens(sg), dens(sb)]
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
    auto_crop: bool,
    raw: bool,
    base_point: Option<(f32, f32)>,
    app_handle: &AppHandle,
    tone_slot: &Arc<Mutex<Option<ToneSolve>>>,
) -> Result<PreviewResult, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use tauri::Manager;

    // Fresh sidecar every render — a film-type switch must not inherit stale params.
    let sidecar = crate::exif_processing::get_primary_sidecar_path(tif);
    let _ = std::fs::remove_file(&sidecar);
    // `raw` shows the un-inverted negative (gamma-lifted) so the eyedropper can
    // target the orange rebate; returned directly since it needs no conversion.
    if raw {
        *tone_slot.lock().unwrap() = None;
        return Ok(PreviewResult { data: raw_preview_data(tif, rotation_steps)?, crop: None });
    }
    // Loaded for tone (converting negatives) and/or the auto-crop overlay rect.
    let convert = film_type != "e6";
    let src = if convert || auto_crop { Some(open_image(tif)?) } else { None };
    if convert {
        let img = src.as_ref().unwrap();
        let crop = detect_frame_crop(img);
        let mut bounds = crate::negative_conversion::analyze_bounds_for(&crop);
        if let Some((bx, by)) = base_point {
            // Pin each channel's base (bounds min) to the sampled rebate; keep the
            // auto white point but guard the divisor stays positive.
            let base = sample_base_density(img, bx, by, rotation_steps);
            for c in 0..3 {
                bounds[c].min = base[c];
                if bounds[c].max <= bounds[c].min + 0.05 {
                    bounds[c].max = bounds[c].min + 0.5;
                }
            }
        }
        let (base_exposure, auto_gain, auto_ev) = auto_tone_for(&crop, bounds, DEFAULT_SCAN_CONTRAST, false);
        *tone_slot.lock().unwrap() = Some((bounds, base_exposure, auto_gain, auto_ev));
        write_scan_sidecar(
            tif,
            Some((bounds, base_exposure, auto_gain)),
            rotation_steps,
            exposure_offset + auto_ev,
            contrast,
            None,
            &FilmMeta::default(),
        )?;
    } else {
        // Slide (E-6) — positive film, no inversion.
        *tone_slot.lock().unwrap() = None;
        write_scan_sidecar(tif, None, rotation_steps, exposure_offset, contrast, None, &FilmMeta::default())?;
    }

    // Overlay rect only — the preview image itself stays uncropped so the dimmed
    // area is visible; the crop is baked into the sidecar at scan time, not here.
    let crop = if auto_crop {
        src.as_ref().and_then(|im| {
            let (w, h) = (im.width(), im.height());
            // A rect that spans ~the whole window means no holder bars were in
            // view — nothing confident to trim, so don't dim or write a crop.
            detect_frame_rect(im, 100)
                .filter(|r| (r.2 as u64) * (r.3 as u64) * 100 < (w as u64) * (h as u64) * 95)
                .map(|r| {
                let ((cx, cy, cw, ch), (rw, rh)) = rotate_rect(r, w, h, rotation_steps);
                [cx as f32 / rw as f32, cy as f32 / rh as f32, cw as f32 / rw as f32, ch as f32 / rh as f32]
            })
        })
    } else {
        None
    };

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
    Ok(PreviewResult { data: format!("data:image/jpeg;base64,{}", STANDARD.encode(&jpeg)), crop })
}

#[tauri::command]
pub async fn scan_preview(
    film_type: String,
    exposure_offset: f32,
    contrast: f32,
    rotation_steps: u32,
    auto_crop: bool,
    source_visible: String,
    preview_dpi: u32,
    scan_depth: u32,
    raw: bool,
    base_point: Option<(f32, f32)>,
    state: tauri::State<'_, ScanState>,
    app_handle: AppHandle,
) -> Result<PreviewResult, String> {
    if state.child.lock().unwrap().is_some() {
        return Err("A scan is already running".into());
    }
    let source = if source_visible.is_empty() { SRC_VISIBLE.to_string() } else { source_visible };
    let child_slot = state.child.clone();
    let cancelled = state.cancelled.clone();
    let tone_slot = state.preview_tone.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Kept on disk so the exposure/contrast sliders can re-render without rescanning.
        let tmp = preview_tif_path();
        // Frontend picks a low-but-not-lowest resolution: the true minimum often
        // carries periodic decimation noise the negative auto-stretch amplifies.
        run_scanimage(&child_slot, &cancelled, &app_handle, &source, scan_mode(&film_type), preview_dpi, scan_depth, &tmp, 0, 1).map_err(
            |e| match e {
                ScanFail::Cancelled => "cancelled".to_string(),
                ScanFail::Error(m) => m,
            },
        )?;
        render_preview(&tmp, &film_type, exposure_offset, contrast, rotation_steps, auto_crop, raw, base_point, &app_handle, &tone_slot)
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
    auto_crop: bool,
    raw: bool,
    base_point: Option<(f32, f32)>,
    state: tauri::State<'_, ScanState>,
    app_handle: AppHandle,
) -> Result<PreviewResult, String> {
    if state.child.lock().unwrap().is_some() {
        return Err("A scan is already running".into());
    }
    let tif = preview_tif_path();
    if !tif.exists() {
        return Err("No preview scanned yet".into());
    }
    let tone_slot = state.preview_tone.clone();
    tauri::async_runtime::spawn_blocking(move || {
        render_preview(&tif, &film_type, exposure_offset, contrast, rotation_steps, auto_crop, raw, base_point, &app_handle, &tone_slot)
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
    ir_clean: bool,
    ir_sensitivity: f32,
    auto_crop: bool,
    crop_override: Option<(f32, f32, f32, f32)>,
    bit_depth: u32,
    scan_depth: u32,
    source_visible: String,
    source_infrared: Option<String>,
    scanner_model: String,
    dest_folder: String,
    file_name: String,
    film_meta: FilmMeta,
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
    // Consume-once: the very next Scan frame gets the previewed tone exactly;
    // later frames (holder advanced, no fresh preview) re-solve on their own scan.
    let preview_tone = state.preview_tone.lock().unwrap().take();
    log::info!("[scan] starting {file_name} @ {dpi}dpi into {dest_folder}");

    std::thread::spawn(move || {
        // Unsupported extension keeps the partial file out of library listings;
        // same dir keeps the final rename atomic.
        let tmp = dest.join(".rapidraw-scan.partial");
        let samples = samples.clamp(1, 4);
        // Use the device's real source names (from -A); fall back to the 7600i
        // constants when caps were unavailable.
        let source = if source_visible.is_empty() { SRC_VISIBLE.to_string() } else { source_visible };
        // IR needs dye-based film AND a device that exposes an infrared source;
        // silver B&W blocks infrared everywhere.
        let do_ir = ir_clean && film_type != "bw" && source_infrared.is_some();
        let total_passes = samples + do_ir as u32;
        let mut scan_result = if samples == 1 {
            run_scanimage(&child_slot, &cancelled, &app_handle, &source, scan_mode(&film_type), dpi, scan_depth, &tmp, 0, total_passes)
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
                    &child_slot, &cancelled, &app_handle, &source, scan_mode(&film_type), dpi, scan_depth, pass_file,
                    i as u32, total_passes,
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
        if scan_result.is_ok() && do_ir && !cancelled.load(Ordering::SeqCst) {
            let ir_tmp = dest.join(".rapidraw-scan.ir");
            let ir_source = source_infrared.as_deref().unwrap_or(SRC_INFRARED);
            match run_scanimage(
                &child_slot, &cancelled, &app_handle, ir_source, scan_mode(&film_type), dpi, scan_depth,
                &ir_tmp, samples, total_passes,
            ) {
                Ok(()) => match ir_clean_scan(&tmp, &ir_tmp, dpi, ir_sensitivity) {
                    Ok(n) => log::info!("[scan] IR clean filled {n} defect px"),
                    // A failed clean never loses the scan itself.
                    Err(e) => log::warn!("[scan] IR clean skipped: {e}"),
                },
                Err(ScanFail::Cancelled) => scan_result = Err(ScanFail::Cancelled),
                Err(ScanFail::Error(m)) => log::warn!("[scan] IR pass failed, scan kept unclean: {m}"),
            }
            let _ = std::fs::remove_file(&ir_tmp);
        }
        if scan_result.is_ok() && !cancelled.load(Ordering::SeqCst) {
            let keep_bits = if matches!(bit_depth, 10 | 12 | 16) { bit_depth as u8 } else { 12 };
            match compress_scan(&tmp, keep_bits, &scanner_model, &film_meta) {
                Ok(size) => log::info!("[scan] compressed to {} MB ({keep_bits}-bit)", size / 1_000_000),
                Err(e) => log::warn!("[scan] compression skipped, keeping raw TIFF: {e}"),
            }
        }
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
                // Auto-crop frame rect in post-orientation space (opt-in, so the
                // extra decode only happens when asked). ponytail: separate decode
                // from the tone one below; fold together only if it ever matters.
                // A crop the user dragged in the preview (normalized, post-orientation
                // space) wins — the scan matches exactly what they framed. Otherwise
                // auto-detect the frame when auto-crop is on.
                let crop_rect = if let Some((cx, cy, cw, ch)) = crop_override {
                    open_image(&target).ok().map(|img| {
                        let (ow, oh) = if rotation_steps % 2 == 1 {
                            (img.height(), img.width())
                        } else {
                            (img.width(), img.height())
                        };
                        (
                            (cx.clamp(0.0, 1.0) * ow as f32) as u32,
                            (cy.clamp(0.0, 1.0) * oh as f32) as u32,
                            (cw.clamp(0.0, 1.0) * ow as f32).max(1.0) as u32,
                            (ch.clamp(0.0, 1.0) * oh as f32).max(1.0) as u32,
                        )
                    })
                } else if auto_crop {
                    open_image(&target).ok().and_then(|img| {
                        let (w, h) = (img.width(), img.height());
                        detect_frame_rect(&img, 100)
                            .filter(|r| (r.2 as u64) * (r.3 as u64) * 100 < (w as u64) * (h as u64) * 95)
                            .map(|r| rotate_rect(r, w, h, rotation_steps).0)
                    })
                } else {
                    None
                };
                // Sidecar is written before scan-complete, so the library's first
                // sight of the file (and its first thumbnail) is already converted.
                let result = if film_type != "e6" {
                    // The 1800dpi preview's cached solve can't represent the skewed
                    // 7200 response — at 7200 always re-solve on the actual scan.
                    let tone = match preview_tone {
                        Some(t) if dpi != 7200 => Ok(t),
                        _ => open_image(&target).map(|img| {
                            let crop = detect_frame_crop(&img);
                            let bounds = crate::negative_conversion::analyze_bounds_for(&crop);
                            let (exposure, gain, ev) =
                                auto_tone_for(&crop, bounds, DEFAULT_SCAN_CONTRAST, dpi == 7200);
                            (bounds, exposure, gain, ev)
                        }),
                    };
                    tone.and_then(|(bounds, exposure, gain, ev)| {
                        write_scan_sidecar(
                            &target,
                            Some((bounds, exposure, gain)),
                            rotation_steps,
                            exposure_offset + ev,
                            contrast,
                            crop_rect,
                            &film_meta,
                        )
                    })
                } else {
                    write_scan_sidecar(&target, None, rotation_steps, exposure_offset, contrast, crop_rect, &film_meta)
                };
                if let Err(e) = result {
                    log::warn!("[scan] sidecar write failed for {path}: {e}");
                }
                log::info!("[scan] saved {path}");
                let file_name = target.file_name().unwrap_or_default().to_string_lossy().to_string();
                let _ = app_handle.emit("scan-complete", serde_json::json!({ "path": path, "fileName": file_name }));
            }
            Err(ScanFail::Cancelled) => {
                let _ = std::fs::remove_file(&tmp);
                let _ = app_handle.emit("scan-cancelled", ());
            }
            Err(ScanFail::Error(m)) => {
                let _ = std::fs::remove_file(&tmp);
                let _ = app_handle.emit("scan-error", serde_json::json!({ "message": m }));
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{average_scans, detect_frame_crop, fill_masked, ir_defect_mask};

    #[test]
    fn ir_mask_and_fill_remove_synthetic_speck() {
        let (w, h) = (120usize, 100usize);
        // IR frame: bright film field with one dark dust speck
        let mut ir = vec![0.8f32; w * h];
        for y in 40..44 {
            for x in 60..65 {
                ir[y * w + x] = 0.2;
            }
        }
        let vis_luma = vec![0.5f32; w * h];
        let mask = ir_defect_mask(&ir, &vis_luma, w, h, 1800, 6.0);
        assert!(mask[42 * w + 62], "speck not masked");
        assert!(!mask[10 * w + 10], "clean area masked");

        // Visible frame: flat gray with the defect burned in dark
        let mut rgb = vec![0.5f32; w * h * 3];
        for y in 40..44 {
            for x in 60..65 {
                for c in 0..3 {
                    rgb[(y * w + x) * 3 + c] = 0.05;
                }
            }
        }
        fill_masked(&mut rgb, &mask, w, h);
        let v = rgb[(42 * w + 62) * 3];
        assert!((v - 0.5).abs() < 0.01, "fill missed: {v}");
    }

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
        // Extension-less names like production's temp files — image::open used to
        // reject these, silently skipping the average/IR-clean steps.
        let passes = vec![mk(".rapidraw-scan.pass0", 1000), mk(".rapidraw-scan.pass1", 3000)];
        let out = dir.join(".rapidraw-scan.partial");
        average_scans(&passes, &out).unwrap();
        let avg = super::open_image(&out).unwrap().to_rgb16();
        assert_eq!(avg.dimensions(), (8, 6));
        assert_eq!(avg.get_pixel(4, 3)[1], 2000);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn compress_scan_quantizes_shrinks_and_round_trips() {
        let dir = std::env::temp_dir().join(format!("rr-compress-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(".rapidraw-scan.partial");
        // Gradient plus deterministic noise, so deflate has realistic work.
        let mut rng: u32 = 12345;
        let mut img = image::ImageBuffer::<image::Rgb<u16>, Vec<u16>>::new(128, 128);
        for (x, y, px) in img.enumerate_pixels_mut() {
            rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
            let noise = (rng >> 20) as u16; // 0..4095
            *px = image::Rgb([(x * 500) as u16 ^ noise, (y * 500) as u16 ^ noise, noise]);
        }
        let orig = img.clone();
        img.save_with_format(&p, image::ImageFormat::Tiff).unwrap();
        let raw_size = std::fs::metadata(&p).unwrap().len();
        super::compress_scan(&p, 12, "PLUSTEK OpticFilm 7600i (v1)", &super::FilmMeta::default()).unwrap();
        assert!(std::fs::metadata(&p).unwrap().len() < raw_size, "no size reduction");
        // Round-trip through the app's own decoder: proves deflate+predictor
        // 16-bit TIFFs stay readable, and only sub-noise bits changed.
        let back = super::open_image(&p).unwrap().to_rgb16();
        assert_eq!(back.dimensions(), (128, 128));
        for (a, b) in orig.as_raw().iter().zip(back.as_raw()) {
            assert_eq!(a & 0xFFF0, *b, "quantized value mismatch");
        }
        // Embedded tags must be readable by a standard EXIF reader.
        let file = std::fs::File::open(&p).unwrap();
        let exif = exif::Reader::new().read_from_container(&mut std::io::BufReader::new(file)).unwrap();
        assert!(exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY).is_some(), "DateTime missing");
        let model = exif.get_field(exif::Tag::Model, exif::In::PRIMARY).expect("Model missing");
        assert!(model.display_value().to_string().contains("OpticFilm"));

        // 16-bit mode: bit-exact lossless.
        img.save_with_format(&p, image::ImageFormat::Tiff).unwrap();
        super::compress_scan(&p, 16, "", &super::FilmMeta::default()).unwrap();
        let back = super::open_image(&p).unwrap().to_rgb16();
        assert_eq!(orig.as_raw(), back.as_raw(), "16-bit mode must be lossless");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn compress_embeds_film_metadata() {
        let dir = std::env::temp_dir().join(format!("rr-meta-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("frame.tif");
        image::ImageBuffer::<image::Rgb<u16>, Vec<u16>>::from_pixel(16, 16, image::Rgb([20000u16, 20000, 20000]))
            .save_with_format(&p, image::ImageFormat::Tiff)
            .unwrap();
        let meta = super::FilmMeta {
            film_stock: Some("Kodak Portra 400".into()),
            iso: Some(400),
            camera: Some("Nikon FM2".into()),
            lens: Some("50mm f/1.8".into()),
            notes: None,
        };
        super::compress_scan(&p, 12, "PLUSTEK OpticFilm 7600i (v1)", &meta).unwrap();
        let file = std::fs::File::open(&p).unwrap();
        let exif = exif::Reader::new().read_from_container(&mut std::io::BufReader::new(file)).unwrap();
        let get = |t| exif.get_field(t, exif::In::PRIMARY).map(|f| f.display_value().to_string());
        // Camera takes Make/Model over the scanner; stock lands in ImageDescription.
        assert!(get(exif::Tag::Make).unwrap().contains("Nikon"), "camera make not embedded");
        assert!(!get(exif::Tag::Make).unwrap().contains("PLUSTEK"), "scanner should not win Make");
        assert!(get(exif::Tag::Model).unwrap().contains("FM2"), "camera model not embedded");
        assert!(get(exif::Tag::ImageDescription).unwrap().contains("Portra"), "stock not embedded");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_caps_reads_sources_resolutions_depth() {
        // Real 7600i -A output.
        let plustek = "\
    --mode Color|Gray [Gray]
    --source Transparency Adapter|Transparency Adapter Infrared [Transparency Adapter]
    --depth 16 [16]
    --resolution 7200|3600|1800|900dpi [900]";
        let c = super::parse_caps(plustek);
        assert_eq!(c.source_visible, "Transparency Adapter");
        assert_eq!(c.source_infrared.as_deref(), Some("Transparency Adapter Infrared"));
        assert_eq!(c.resolutions, vec![900, 1800, 3600, 7200]);
        assert_eq!(c.default_resolution, 900);
        assert_eq!(c.max_depth, 16);
        assert!(c.has_transparency);

        // Epson-style flatbed: range resolution, 8|16 depth, no IR, has a TPU.
        let epson = "\
    --mode Color|Gray|Lineart [Color]
    --source Flatbed|Transparency Unit [Flatbed]
    --depth 8|16 [8]
    --resolution 50..6400dpi [50]";
        let e = super::parse_caps(epson);
        assert_eq!(e.source_visible, "Transparency Unit"); // film source preferred over Flatbed
        assert_eq!(e.source_infrared, None);
        assert!(e.has_transparency);
        assert_eq!(e.max_depth, 16);
        assert!(e.resolutions.iter().all(|r| *r >= 50 && *r <= 6400) && e.resolutions.contains(&3600));

        // Plain document scanner: no film support.
        let doc = "    --source Flatbed|ADF [Flatbed]\n    --resolution 75|150|300|600dpi [75]\n    --depth 8 [8]";
        let d = super::parse_caps(doc);
        assert!(!d.has_transparency);
        assert_eq!(d.max_depth, 8);
    }

    #[test]
    fn rotate_rect_matches_rotate90_cw() {
        // 100x60 image, rect near top-left. rotate90() is clockwise.
        let (w, h) = (100u32, 60u32);
        let r = (10, 5, 20, 8); // x,y,w,h
        // 90 CW: top-left corner moves to top-right; dims swap to 60x100.
        assert_eq!(super::rotate_rect(r, w, h, 1), ((60 - (5 + 8), 10, 8, 20), (60, 100)));
        // 180: mirror both axes, dims unchanged.
        assert_eq!(super::rotate_rect(r, w, h, 2), ((100 - (10 + 20), 60 - (5 + 8), 20, 8), (100, 60)));
        // 270 CW: inverse of 90.
        assert_eq!(super::rotate_rect(r, w, h, 3), ((5, 100 - (10 + 20), 8, 20), (60, 100)));
        // 0: identity.
        assert_eq!(super::rotate_rect(r, w, h, 0), (r, (100, 60)));
    }

    #[test]
    fn base_sample_reads_density_and_follows_orientation() {
        use image::{DynamicImage, Rgb, Rgb32FImage};
        // Left third is a bright "rebate" (v=0.5 -> density ~0.301); rest is dark.
        let (w, h) = (90u32, 30u32);
        let mut img = Rgb32FImage::from_pixel(w, h, Rgb([0.05f32, 0.05, 0.05]));
        for y in 0..h {
            for x in 0..(w / 3) {
                img.put_pixel(x, y, Rgb([0.5f32, 0.5, 0.5]));
            }
        }
        let dyn_img = DynamicImage::ImageRgb32F(img);
        let expect = -0.5f32.log10(); // ~0.301
        // Rotation 0: the left band sits at x~0.15.
        let d = super::sample_base_density(&dyn_img, 0.15, 0.5, 0);
        assert!((d[0] - expect).abs() < 0.02, "density {d:?} != {expect}");
        // 90 CW moves the left band to the top; the same value now reads at (0.5, 0.1).
        let d_top = super::sample_base_density(&dyn_img, 0.5, 0.1, 1);
        assert!((d_top[0] - expect).abs() < 0.05, "rotated band density {d_top:?}");
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
        terminate_child(child);
    }
    Ok(())
}
