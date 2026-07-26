# Film scanner

Scan 35mm film — C-41 colour negatives, black & white negatives, and E-6 slides —
directly inside RapidRAW. Frames land in the current library folder already
inverted, tone-mapped, dust-cleaned, cropped, and dated, ready to edit. The
feature is built around SANE's `scanimage`, so it works with a wide range of
film scanners on macOS and Linux (see [Requirements](#requirements)).

> Developed and tested against a **Plustek OpticFilm 7600i**. Other SANE film
> scanners are supported through runtime capability detection (below), but have
> not been hardware-tested — see [Compatibility](#compatibility).

---

## Where it lives

Open the **Import** window → **Choose a source**. A connected film scanner
appears as its own source card beneath the drives ("PLUSTEK OpticFilm 7600i",
etc.). Click it to enter the scan pane: a large preview surface on the left and
the scan controls on the right.

## Workflow

1. Load a frame in the holder, pick your film type and settings.
2. **Preview** does a fast low-resolution pass so you can check framing and
   exposure. The Exposure / Contrast / Orientation / Auto-crop controls re-render
   the preview instantly **without rescanning**.
3. **Scan frame** does the full-resolution scan. The frame is inverted, tuned,
   optionally dust-cleaned and cropped, compressed, and saved to the library.
4. Advance the holder to the next frame and scan again — the filename counter
   increments automatically (`roll-001.tif`, `roll-002.tif`, …).

Scans run in the background: you can leave the scan pane and the frame still
finishes and lands in the library, with a toast when it's done. The **Cancel**
button stops an in-progress scan.

---

## Controls

### Roll name
The filename prefix for this roll. Frames are numbered `<roll>-NNN.tif`; a
name collision is suffixed rather than overwritten.

### Film type
- **Color negative (C-41)** — orange-mask neutralised, inverted to positive.
- **B&W negative** — scanned in grayscale, inverted.
- **Slide (E-6)** — positive film, no inversion.

### Resolution
The resolution buttons are **populated from the scanner itself** — only the
values your hardware actually supports are shown. For the 7600i that's
900 / 1800 / 3600 / 7200 dpi.

> **3600 dpi is the sweet spot** for the 7600i. Its optics resolve roughly
> 3600 dpi of real detail, so 7200 costs ~3.5× the scan time for extra noise,
> not extra sharpness — and the SANE genesys backend's colour calibration is off
> at 7200, which the app compensates for but can't fully fix. Use 7200 only when
> you specifically need the larger pixel dimensions.

### Sampling (multi-sample)
Scans the frame **N times and averages the passes** (1× / 2× / 4×). Because the
scanner has no exposure control, this reduces shadow noise (~√N) rather than
extending dynamic range. Costs N× the scan time.

### Bit depth
How many significant bits each scan keeps, stored in a compressed TIFF:

| Depth | Size (3600 dpi) | Character |
| ----- | --------------- | --------- |
| **16** | ~85 MB | mathematically lossless |
| **12** (default) | ~55 MB | discards only bits ~100× below the scanner's noise floor |
| **10** | ~40 MB | smallest; still below the noise floor |

All three are standard deflate-compressed TIFFs readable by any application. The
lower depths quantise away bits that hold only sensor noise (film grain dithers
any banding), so 12-bit halves the file with no visible loss. See
[Bit depth & compression](#bit-depth--compression).

### Dust removal (IR)
An **infrared cleaning pass** (Digital ICE–style). Shown only when the scanner
exposes an infrared source. An extra IR scan sees dust, hair, and surface
scratches as shadows (film dye is transparent to infrared, defects are not); the
app builds a defect mask and fills it from surrounding clean pixels.

- Not available for silver **B&W** film (the silver image blocks infrared
  everywhere, so there's no clean signal).
- At 7200 dpi the IR pass is noisier; detection runs on a downsampled frame and
  fills at full resolution so fine dust is still caught.
- **Limitation:** it can only remove defects that block infrared. Deep
  emulsion-side scratches that removed dye are transparent to IR and cannot be
  detected this way — use the editor's heal/clone tool for those. See
  [Limitations](#limitations).

### Exposure & Contrast
These are convenience offsets on top of the automatic tone. They are written as
**real editor adjustments** (the editor's Exposure and Contrast sliders), not
baked into the conversion — so every scan opens in the editor fully re-tunable,
and dragging Exposure to 0 shows the untouched automatic conversion.

### Orientation
Rotate the frame in 90° steps. Saved as a non-destructive orientation adjustment.

### Auto crop
Detects the film-frame edges and trims the holder bars, the aperture-shadow
border, and the clear film rebate. When on, the preview **dims the area that
will be trimmed** so you can see the result. The crop is written as a **normal,
non-destructive crop adjustment** — the editor's Crop tool can refine or clear
it, and no pixels are discarded from the TIFF. Frames where no confident frame
edge is found are left uncropped rather than guessed.

---

## How a scan is processed

Each **Scan frame** runs this pipeline, all before the file first appears in the
library (so its very first thumbnail is already correct):

1. **Scan** the visible pass at the chosen resolution/depth (N passes if
   sampling, averaged).
2. **IR pass + clean** (if enabled and available) — infrared scan, defect mask,
   inpaint fill.
3. **Auto-tone** — analyse the film frame, invert the negative, and anchor the
   brightness. Colour negatives get orange-mask neutralisation and a per-channel
   density stretch; the result targets a natural mid-tone. The Exposure/Contrast
   offsets and orientation are added as editor adjustments.
4. **Auto-crop** (if enabled) — detect the frame rect, written as a crop
   adjustment in the correct post-orientation space.
5. **Compress** — re-encode as a deflate + horizontal-predictor TIFF at the
   chosen bit depth.
6. **Metadata** — embed the scan date and scanner make/model as TIFF/EXIF tags,
   and write the RapidRAW sidecar.
7. **Save** atomically into the library and refresh it.

### Auto-tone and the 7200 dpi colour cast
The negative conversion inverts in the density domain with automatic per-channel
bounds, so colour negatives come out neutral without manual white balance. At
7200 dpi the genesys backend underexposes with a per-channel response skew; a
naive tone curve turns that skew into a purple/yellow cast. The app detects the
high-dpi case and reaches the target brightness with a **hue-neutral exposure
shift** (on the editor's Exposure key) instead of a curve move, keeping colour
neutral. That value is visible and adjustable in the editor.

### Bit depth & compression
The scanner delivers 16-bit data, but its real noise floor sits far above the
16th bit. Storing 12 significant bits discards only sub-noise bits — the
quantisation step is ~100× smaller than the measured grain+sensor noise, so
there is no visible loss and grain acts as natural dithering. The quantised
planes then compress well: a 3600 dpi frame goes from ~109 MB (raw 16-bit) to
~55 MB. The output is always a standard TIFF; 16-bit mode is bit-exact lossless.

### Metadata
`scanimage` output carries no capture date, so the library has nothing to sort
on. Every scan embeds real **TIFF/EXIF tags** — `DateTime`, `DateTimeOriginal`
(the scan time), `Make`/`Model` (the detected scanner), and `Software` — readable
by any application (Lightroom, exiftool, Finder, digiKam). The RapidRAW sidecar
carries the same date for the library's own date sort. Tags are written at encode
time (when the app authors the TIFF), so no risky in-place tag rewriting occurs.

---

## Capability detection

At detection time the app runs `scanimage -A` on the specific device and drives
the entire UI from what the scanner reports, rather than assuming a particular
model:

- **Source names** — the real `--source` strings (e.g. `Transparency Adapter`,
  Epson's `Transparency Unit`) are used for scanning, so scans don't fail with an
  "invalid argument" from a hard-coded name.
- **Resolutions** — only the values the device supports are offered (an
  enumerated list, or sensible film steps within a `min..max` range).
- **Infrared** — the Dust removal (IR) toggle appears only if the device exposes
  an infrared source.
- **Depth** — 16-bit where available, otherwise the device maximum.
- **Non-film scanners** — a plain document scanner (no transparency unit) is
  detected as such and can't be entered for film scanning.

The source list re-probes the scanner every time it opens, so an unplugged
scanner doesn't linger as a stale "ready".

## Robustness

- **Background scans** — a scan keeps running and lands in the library after you
  navigate away; returning to the pane never shows a stale progress bar.
- **Inactivity watchdog** — if `scanimage` goes silent for 120 s at any point
  (a wedged USB device), it's terminated and the error surfaced, instead of
  leaving a zombie holding the scanner.
- **Graceful shutdown** — Cancel and the watchdog send `SIGTERM` first so
  `scanimage` releases the USB interface cleanly, escalating to `SIGKILL` only if
  it doesn't exit within 2 s. (A hard kill mid-transfer can wedge the genesys
  device until a physical re-plug.)

---

## Requirements

SANE's `scanimage` command must be installed and able to see the scanner:

- **macOS:** `brew install sane-backends`
- **Linux:** `apt install sane-utils` / `dnf install sane-backends` (or your
  distro's package). Linux is SANE's native platform.
- **Windows:** not supported — SANE/`scanimage` is not available. The scanner
  card shows an install hint instead.

The scanner must be connected, powered on (film scanners are usually not
bus-powered — check the adapter), and detected by `scanimage -L`. For best
reliability, connect it directly rather than through a USB hub.

## Compatibility

| Scanner | Status |
| ------- | ------ |
| Plustek OpticFilm 7600i | **Tested** (primary development hardware) |
| Plustek OpticFilm 7300 / 7400 / 8100 / 8200i | Expected to work (share the genesys source names) |
| Other SANE film scanners with a transparency source | Supported via capability detection; not hardware-tested |
| Epson / flatbed scanners with a transparency unit | Scanning supported; **IR dust removal unavailable** (Epson's Digital ICE is not exposed through SANE) |
| Document scanners (no transparency unit) | Detected and blocked — cannot scan film |

## Limitations

- **Windows** is unsupported (no SANE).
- **7200 dpi** on the 7600i is slow and its driver colour calibration is
  imperfect; 3600 dpi is recommended.
- **IR dust removal** only catches defects that block infrared. Deep
  emulsion-side scratches that removed film dye are transparent to IR and are not
  detected — the editor's heal/clone tool is the fix for those.
- **No multi-exposure HDR** — the genesys backend exposes no exposure-time
  control, so extended-dynamic-range scanning is not possible; multi-sampling
  reduces noise instead.
- Installers built by the fork workflow are **unsigned**.
