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
- **Windows SD-card eject** support from within the importer.

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

## Auto-correct

- **Auto-correct** with highlight guards and face-aware metering that **learns
  from your edits** over time.

## Editing & color

- **Capture One–style keyboard shortcuts** for the core tonal & color sliders —
  shown in **Settings → Controls → Adjustments** and fully remappable.
- **Blown-highlight handling** — RAW highlights that clip are desaturated toward
  neutral white instead of going magenta/colored.

## Library & metadata

- **Batch metadata editing** + sync across selected images.
- **Export naming** — filename templates gain metadata tokens (`{title}`,
  `{author}`, `{copyright}`, `{comments}`) that mirror the Metadata panel, and the
  File Naming UI is available for **single-image** export (not just batch).
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

---

## Building & releasing

- Run locally with `npm install` then `npm start` (Tauri dev — Node LTS + Rust
  toolchain required).
- **Desktop installers** are built by a fork-specific GitHub Actions workflow
  ([`.github/workflows/release-fork.yml`](../.github/workflows/release-fork.yml)):
  push a tag like `fork-v1.5.8` (or run it from the Actions tab) and it publishes
  a GitHub Release with unsigned installers for **Windows** (`.exe`), **macOS**
  (`.dmg`, Apple Silicon + Intel), and **Linux** (`.deb` + `.AppImage`).
