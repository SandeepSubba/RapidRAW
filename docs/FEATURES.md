# RapidRAW — Fork Features (`integration/all-features`)

This fork of [`CyberTimon/RapidRAW`](https://github.com/CyberTimon/RapidRAW) bundles a
set of workflow-oriented additions on top of upstream, aimed at high-volume
photo culling and portrait/product retouching. Everything below lives on the
`integration/all-features` branch and is kept additive so it survives upstream
updates.

> Maintenance / rebase workflow and the remote layout are documented separately in
> [`FORK_NOTES.md`](../FORK_NOTES.md).

---

## Import & culling (Capture One–style)

A dedicated **image importer** for offloading and triaging a shoot, documented in
detail in [`docs/IMAGE_IMPORTER.md`](IMAGE_IMPORTER.md).

- **SD-card / source import** with a culling grid modeled on Capture One.
- **AI scoring** — face detection plus eyes / gaze / expression analysis to rank
  keepers; scoring is a separate step from grouping so it can be re-run.
- **Non-people tuning** — scoring adapts for product / landscape shots where face
  metrics don't apply.
- **0–5 grades + keep threshold**, a **compare / zoom viewer**, and a green outline
  that marks the images you've **kept** (not just the AI's best guess).
- **Learns from your culling** — selections personalize future auto-selection.
- **Grouping modes** — including **time-based (burst) grouping**.
- **Grid controls** — sort by name, capture date, or quality; **multi-select**
  with Ctrl/Shift+click.
- **Auto lens correction on import** — reads lens metadata and seeds the matching
  correction so imported RAWs open already corrected.
- **Destination defaults to the current library folder** — the importer opens ready
  to import into the folder you're viewing (albums excluded); an explicit choice is
  never overwritten.
- **Rename-proof duplicate detection** — "exclude already-imported" matches by
  filename stem *or* file content (size-gated BLAKE3 hash), so photos renamed in the
  library still register as already imported.
- **Windows SD-card eject** support from within the importer.

## Film scanner

Scan 35mm film directly into the library via SANE, documented in detail in
[`docs/FILM_SCANNER.md`](FILM_SCANNER.md). Frames land already inverted, tuned,
and dated.

- **C-41 / B&W / E-6** — colour-negative, B&W-negative, and slide film, inverted
  (or not, for slides) on scan with automatic orange-mask neutralisation.
- **Runtime capability detection** — sources, resolutions, depth, and infrared
  availability are read from the scanner via `scanimage -A`, so the UI adapts to
  each device instead of assuming one model.
- **IR dust & scratch removal** — an infrared pass detects and inpaints dust,
  hair, and surface scratches (when the scanner exposes an IR source), with a
  **sensitivity slider** to trade defect removal against fine-detail retention.
- **Film metadata** — per-roll stock / ISO / camera / lens / notes written to the
  sidecar and embedded as EXIF for cross-app cataloguing.
- **Multi-sampling** — averages N passes to cut shadow noise.
- **Auto-tone** — density-domain inversion with hue-neutral high-dpi handling;
  Exposure/Contrast are written as re-tunable editor adjustments.
- **Film-base eyedropper** — click the orange rebate on the raw negative to pin
  the mask base point for stubborn stocks; per-roll, rides the tone into the scan.
- **Auto crop** — detects the film frame (holder bars, aperture shadow, rebate)
  and writes a non-destructive crop; the preview dims what will be trimmed.
- **10 / 12 / 16-bit compressed TIFF** — deflate + predictor; 12-bit halves the
  file with no visible loss.
- **Embedded metadata** — scan date and scanner make/model as TIFF/EXIF tags for
  cross-application date sorting.
- **Background scanning** with an inactivity watchdog and graceful (SIGTERM)
  shutdown so a wedged scan can't zombie the USB device.
- Runs on **macOS** (`brew install sane-backends`) and **Linux** (`sane-utils`);
  Windows is unsupported (no SANE).

## Tethered shooting

- **Folder-watch tethering (all platforms)** — point RapidRAW at a session folder and
  anything the camera vendor's app drops there is ingested into the library live.
- **Direct USB tethering** over libgphoto2 (`tether-usb` build feature; enabled in the
  Apple Silicon macOS and Linux release builds) — camera picker, in-app shutter release,
  camera-setting sliders, and **live view** when the body supports it. Shutter presses
  on the camera body download into the same watched session folder. Windows and Intel
  macOS builds ship folder-watch tethering only (no libgphoto2 on Windows; no x86_64
  libgphoto2 on the arm64 CI runner).

## AI assistant

- **Chat assistant panel** that sees the current image and edit state and applies
  editor adjustments conversationally. Providers: **OpenAI**, **Anthropic (API key)**,
  or **Claude Code** — the last drives the `claude` CLI so an existing Claude
  subscription works with no API key.
- **OCR & metadata extraction** through cloud vision models.
- **Zoom-to-read (inspect loop)** — when small text or ruler ticks are illegible
  at the attached resolution, the assistant asks the app for a region and gets it
  back cropped from the original at native pixels (up to 3 rounds per request),
  then answers from what it can actually see. Attachments and batch OCR are also
  fed from the full-quality decode at JPEG q90 instead of the 720px thumbnail
  pipeline.
- **Cropping from chat** — ask for a square / 16:9 / subject-centred crop and the
  assistant emits a pixel rectangle that is validated, clamped to the image, and
  applied as a normal non-destructive crop (undoable, re-editable in the Crop
  panel).
- **Scan-preview mode** — while the scanner pane is open with a preview, the assistant
  drives the scanner controls (tone, crop, film settings) instead of the editor.

## Crop, rotate & perspective

- **Opt-in crop tool** (fork behaviour) — the Crop panel no longer auto-activates a
  crop; a crop icon toggles it on. The panel shows the *cropped* result until you
  activate the tool, consistent with every other module.
- **Rotate by dragging outside the crop** (Lightroom / Capture One style).
- **Inline perspective / keystone controls** in the Crop & Rotate panel, with
  live sliders that commit on drag release.
- **Guided keystone** — draw reference lines on the image to correct perspective;
  guides are editable, with automatic auto-crop to the corrected frame.
- **Batch-rotate** selected images with the `[` and `]` shortcuts.
- **Saved crop ratios** — save any custom width × height (e.g. `2048 × 2292`) as a
  named preset; saved ratios appear as buttons in the aspect-ratio grid alongside the
  built-ins, with hover-to-delete.
- **Last-used ratio remembered** — the crop panel reopens on the ratio and orientation
  you last picked instead of resetting to the image's native ratio (classically 3:2
  horizontal) on every image.

## Masks & AI retouching

- **AI Eyes / Mouth masks** — auto-select facial features.
- **Face masks** using YuNet landmarks with a two-pass refine and library-tuned
  placement.
- **One-click Portrait stack** with Add / Subtract chips and atomic undo for AI
  edits (one action = one undo step).
- **Frequency-separation skin smoothing** (global and per-mask) with a
  Picktorial-style **Texture** knob and an adjustable **Smoothing Scale** that
  preserves real skin texture and guards structural edges.

## Snapshots (versions)

- **In-editor snapshots** — checkpoint the full edit state at any point, rendered
  as preset-style cards you can rename and restore. Snapshots are kept out of the
  render payload so they don't affect preview performance.

## Negative conversion

- **One-click negative conversion**, applied **non-destructively in-library**
  instead of baking a TIFF.
- Toggle **Convert ↔ Revert** per image from the library / filmstrip right-click
  menu, with Develop-module tuning. Conversion survives navigation and batch
  revert.
- **Editor Film panel** — conversion parameters stay re-editable after the scan,
  in the develop module.
- **NegaFix-style film-stock profiles** — save conversion parameters per stock
  (params only; frame-specific bounds never pollute a profile) and apply them at
  scan time or from the Film panel.
- **Roll tools in the library** — apply a conversion across a whole roll with batch
  progress, negative badges on thumbnails, and a negative filter.

## Auto-correct

- **Auto-correct** with highlight guards and face-aware metering that **learns
  from your edits** over time.

## Editing & color

- **Capture One–style keyboard shortcuts** for the core tonal & color sliders —
  shown in **Settings → Controls → Adjustments** and fully remappable.
- **Blown-highlight handling** — RAW highlights that clip are desaturated toward
  neutral white instead of going magenta/colored.
- **Keyboard filmstrip selection** — build a multi-image selection from the keyboard
  in the develop module, without reaching for the mouse.

## Export

- **sRGB ICC profile embedded in JPEG exports** — files are colour-tagged instead of
  leaving every browser, editor, and phone gallery to guess. The pipeline already
  renders sRGB, so this states what is true of the pixels; nothing is converted.
- **Smaller JPEGs at identical quality** — per-image optimized Huffman tables
  (typically 5–20% smaller on real photos; the decoded image is bit-identical).
  Full 4:4:4 chroma is kept — no subsampling, colour edges stay sharp.
- **Keyword tags carried into exports** — library keywords are embedded in the
  exported file's metadata.
- **Export naming** — filename templates gain metadata tokens (`{title}`,
  `{author}`, `{copyright}`, `{comments}`) that mirror the Metadata panel, and the
  File Naming UI is available for **single-image** export (not just batch).
- **`{sequence:START}`** — sequence numbering with a chosen start and padding
  (`{sequence:002}` → `002, 003, …`), counted **per rendered name**: with
  `{title}_{sequence:2}`, images sharing a title number `_2, _3, …` and the
  counter resets for the next title. Bare `{sequence}` still numbers the whole
  batch.
- **Templates are used verbatim** — no silent `_{sequence}` suffix is appended.
  If two images in a batch actually render the same name, the later ones get
  `_2`, `_3`, … so nothing is overwritten; templates that already produce unique
  names are left exactly as written.

## Library & metadata

- **Batch metadata editing** + sync across selected images.
- Selection stays on the nearest visible image when a rating filter would hide the
  current one.

## Settings

- **Wrap Image Navigation** toggle (Settings → General) — turn off the arrow-key
  wrap-around so navigation stops at the first / last image instead of looping.

## RAW loading fixes & robustness

- **Correct RAW orientation** is detected and applied on decode (and on the
  embedded-preview fallback).
- **Fast, oriented RAW thumbnails**; "open with file" works while the app is
  running.
- **Embedded-preview fallback** for RAWs that can't be fully decoded, so the file
  stays viewable/editable.
- **Cancelled loads no longer stick on a low-res preview** — a load superseded by
  fast navigation is not cached as a tiny fallback, so the full image renders.
- **Auto-crop never persists geometry against a fallback preview**, preventing
  corrupt sidecar crops.
- **EXIF UserComment** is decoded properly instead of showing a `0000000…` hex
  dump (e.g. Canon 5D Mark III).
- Backend hardening / correctness fixes and module refactors
  (`file_management`, `image_processing`, settings widgets).

## Performance & platform

- **Thin-LTO release profile** — release builds compile ~3–5× faster than upstream's
  fat-LTO/single-codegen-unit profile, for a runtime cost the GPU-bound pipeline
  doesn't feel.
- **OS-native TLS on Windows and macOS** — assistant/network requests use schannel /
  Secure Transport, matching the system browser on networks that drop rustls's
  ClientHello (Linux keeps rustls to avoid an OpenSSL dependency).

---

## Building & releasing

- Run locally with `npm install` then `npm start` (Tauri dev — Node LTS + Rust
  toolchain required).
- **Desktop installers** are built by a fork-specific GitHub Actions workflow
  ([`.github/workflows/release-fork.yml`](../.github/workflows/release-fork.yml)):
  push a tag like `fork-v1.6.1` (or run it from the Actions tab) and it publishes
  a GitHub Release with unsigned installers for **Windows** (`.exe`), **macOS**
  (`.dmg`, Apple Silicon + Intel), and **Linux** (`.deb` + `.AppImage`).
