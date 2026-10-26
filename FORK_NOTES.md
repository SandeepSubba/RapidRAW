# Fork notes (SandeepSubba/RapidRAW)

This fork tracks the upstream project [`CyberTimon/RapidRAW`](https://github.com/CyberTimon/RapidRAW)
and adds a set of custom changes on top. This file documents the remote layout,
the maintenance/rebase workflow, and a couple of the earliest changes in detail.

> **For the full, current catalog of what this fork adds, see
> [`docs/FEATURES.md`](docs/FEATURES.md).** This file focuses on maintenance.

## Git remotes

```
origin  -> https://github.com/CyberTimon/RapidRAW.git   (upstream / the original author)
fork    -> https://github.com/SandeepSubba/RapidRAW.git  (this fork)
```

> If `origin` points at the fork on your machine, rename it: `git remote rename
> origin fork` and add upstream as `git remote add origin https://github.com/CyberTimon/RapidRAW.git`.

## Branches

| Branch | Contents | Purpose |
| ------ | -------- | ------- |
| `integration/all-features` | Every fork feature integrated together (see [`docs/FEATURES.md`](docs/FEATURES.md)) | The full fork — **run, build, and rebase this one.** |
| feature branches (e.g. `feat/crop-opt-in-toggle`, `sd-card-importer`, `fix-exif-usercomment`) | One feature each | Each feature also lives on its own branch and is merged into `integration/all-features`. Handy for isolating or PR-ing a single change upstream. |

Each feature is developed on its own branch and then integrated into
`integration/all-features`, so the integration branch is what you build and
release from, while individual branches stay available for upstream PRs.

## Custom changes in this fork

All changes are kept **small, isolated, and additive** specifically so they
survive upstream updates with minimal merge friction.

### 1. Capture One–style adjustment shortcuts  (`feat:` commit)
Increase/decrease keyboard shortcuts for the core tonal & color sliders, shown
in **Settings → Controls → Adjustments** and fully remappable.

| File | Change | Conflict risk |
| ---- | ------ | ------------- |
| `src/utils/keyboardUtils.ts` | adds the `adjustments` section + the `ADJUSTMENT_NUDGES` table, spread into `KEYBIND_DEFINITIONS` | low (additive) |
| `src/hooks/useKeyboardShortcuts.ts` | one generated handler per nudge, after the `actions` map | low (additive) |
| `src/i18n/locales/en.json` | section label + action labels | low (additive) |

To change/extend: edit the single `ADJUSTMENT_NUDGES` array in
`keyboardUtils.ts` (combo, target adjustment key, step, clamp range). The
dispatcher and the keybind UI are both driven from it, so nothing else needs
to change.

### 2. Metadata export-naming tokens + single-image file naming  (`feat:` commit)
Export filename templates gain metadata tokens that mirror the Metadata panel's
editable fields, the File Naming section shows for single-image export (not just
batch), and unknown tokens fall back to the default template.

| File | Change | Conflict risk |
| ---- | ------ | ------------- |
| `src-tauri/src/file_management.rs` | `{title}`/`{author}`/`{copyright}`/`{comments}` substitution + `sanitize_filename_component` + `generate_export_filename` command | low (additive) |
| `src-tauri/src/lib.rs` | registers `generate_export_filename` | low (additive) |
| `src/components/ui/ExportImportProperties.tsx` | new token list entries + `DEFAULT_FILENAME_TEMPLATE` + `sanitizeFilenameTemplate` | low (additive) |
| `src/components/panel/right/ExportPanel.tsx` | show naming UI for single image; resolve single-image name via backend | low |
| `src/hooks/useExportSettings.ts` | sanitize template on preset apply | low |
| `src/components/ui/AppProperties.tsx` | `GenerateExportFilename` invoke enum entry | low (additive) |

To add a metadata token: add the substitution in `generate_filename_from_template`
and the `{token}` string to `FILENAME_VARIABLES`.

### 3. Backend hardening + refactors  (`fix:` / `refactor:` commits)
See the `code-analysis-fixes` branch / the open PR to upstream. These are
candidates to be merged upstream; if they are, drop them from the fork.

## Updating when upstream releases a new version

The branch now carries hundreds of commits including merge commits, so upstream
syncs are done with a **merge**, not a rebase (a rebase would flatten the
topology and force-push rewritten history to every machine; the v1.6.2 sync,
commit `2579cdf1`, is the template):

```bash
# 1. Get the latest upstream code
git fetch origin   # origin = upstream in this layout

# 2. Merge it into the integration branch
git checkout integration/all-features
git merge origin/main

# 3. Resolve conflicts. Two recurring rules:
#    - Where upstream grew a parallel implementation of a feature the fork
#      already ships (tethering, negative conversion), the fork's version
#      wins — the film scanner / Film panel / TetherMenu are built on it.
#    - Where the fork refactored a file upstream keeps editing
#      (SettingsPanel widgets, image_processing/analysis.rs), keep the
#      fork's structure and port upstream's new behaviour into it.

# 4. Reinstall deps, then verify
npm install
cd src-tauri && cargo check && cd ..
npm run build        # vite production bundle
npm start            # smoke-test the app

# 5. Update the fork (a merge needs no force push)
git push fork integration/all-features
```

Watch the GPU uniform struct when both sides added fields: the Rust struct in
`image_processing.rs` and its WGSL mirror in `shaders/shader.wgsl` must keep
identical field order, and the scalar count before the `mat3x3` block must stay
a multiple of 4 — absorb `_pad_*` slots rather than growing the struct.

### v1.6.3 sync notes

The v1.6.3 merge (see `docs/UPSTREAM-1.6.3-PLAN.md` for the full per-feature
analysis) retired fork code in favour of upstream equivalents:

- **Guided keystone → guided perspective.** The fork's TS solver UI
  (`GuidedKeystoneOverlay.tsx`, `keystoneLines`/`guidedKeystoneActive` store
  fields, `solveKeystone` usage in CropPanel) is gone; upstream's
  `guided_perspective.rs` Rust homography (persisted, re-editable guides,
  native `compute_max_inscribed_crop` auto-crop) replaces it.
  `src/utils/keystone.ts` stays: `fitScaleForParams` still auto-zooms the
  manual perspective sliders and old sidecars (wired into the Geometry
  accordion's V/H onChange and a CropPanel effect), and the solver tests
  document the shared homography math.
- **Export claim-set semantics kept.** Upstream 27f27c3b added an
  `output_path.exists()` check (never overwrite files already on disk).
  The fork deliberately overwrites on re-export (web-export workflow), so
  only the fork's in-run `used_output_paths` claim set survives; upstream's
  `used_paths` pass was dropped during the merge.
- **Thumbnails split small/medium** (upstream): grid uses
  `smallThumbnailResolution`, filmstrip/preview `mediumThumbnailResolution`.
  The RAW fast path still goes through the fork's
  `fast_raw_preview_scaled` (IDCT-scaled embedded preview), now honouring
  upstream's `always_decode_raw_thumbnails` setting.
- **Uncropped preview**: upstream's `geometry_cache` pipeline adopted; the
  fork's generation-counter coalescing was re-injected on top.
- **EXIF caches**: primary `.rrdata`-embedded EXIF (with the fork's
  UserComment healing) is consulted first, then upstream's new `.rrcache`;
  legacy `.rrexif` files still migrate into the primary sidecar.
- `is_tethering_supported` is implemented on the fork's `tether-usb`
  feature (upstream keys it on their `tethering` feature).

### v1.6.4 sync notes

The v1.6.4 merge again retired fork code where upstream shipped an equivalent:

- **Editor canvas color → neutral grey toggle.** The fork's
  `editorCanvasColor` preset dropdown (theme/white/greys/black, "Editor
  Canvas" in Settings → General, `EDITOR_CANVAS_COLORS` in themes.ts) is
  gone; upstream's `editorNeutralGreyBg` switch ("Neutral Grey Canvas")
  replaces it for the color-judgement use case. Old settings files carrying
  `editorCanvasColor` simply drop the key on the next save.
- **TIFF bit depth → upstream `TiffBitDepth`.** The fork's
  `ExportSettings.tiff_bit_depth: Option<u8>` (added for the external-editor
  8-bit option days before upstream's PR #1466 landed the same field) was
  replaced by upstream's `TiffBitDepth` enum + Export-panel dropdown +
  true-16-bit GPU pipeline. `start_external_edit` maps its `tiff8` format
  onto `TiffBitDepth::Eight`; 16-bit external edits now render through the
  high-precision (Rgba16Float) GPU path instead of upscaled 8-bit.
- **GPU readback hybrid.** Upstream's 16-bit path reads each tile from the
  float tile texture (the Rgba8Unorm working texture would truncate it); the
  fork's single whole-ROI readback optimization is kept for 8-bit exports
  (`run()` picks per-tile vs whole-ROI by `RenderOutputPrecision`).
- **RAW highlight desaturation → `recover_clipped_pixel`.** The fork's
  `HIGHLIGHT_DESAT_RANGE` roll-to-white was dropped for upstream's highlight
  color recovery (`40cfa3df`, `85bf424a`). The `rawHighlightCompression`
  setting is now ignored by the pipeline (upstream `_highlight_compression`),
  so its Settings slider is hidden like upstream's; the serde field stays.
- **Upstream's "external edit" is not ours.** Their `launch_request.rs` /
  `useExternalEditSession` make RapidRAW act as the editor for *other* apps
  (`rapidraw --edit <in> --output <out>`); the fork's "Edit in External
  Editor" (send a rendered TIFF *to* Photoshop/Affinity and watch for saves)
  is unrelated and both coexist.
- Settings tab layout kept the fork's structure (upstream reordered General/
  Processing); upstream's new toggles were slotted into the fork's lists.

## Build / run

```bash
npm install        # Node.js LTS + Rust toolchain required
npm run typecheck  # note: upstream has pre-existing strict-tsc errors; the
                   # Vite/esbuild build does not gate on them
npm start          # tauri dev — builds the Rust backend and launches the app
```

### v1.6.7 sync notes

Upstream v1.6.7 (+ ~100 commits: Immich library, ESAM3/BiRefNet AI masks with text
and face-part masks, global AI job tracker, GPU-accelerated AI on Windows/macOS,
split / side-by-side compare, non-destructive negative conversion and BM3D
denoise (denoise modal removed), Auto strength levels, DNG dual-illuminant
camera colour, highlight reconstruction, dehaze rewrite, decode sharing,
binary analytics packets, data-management settings). How the fork fits:

- **Negative conversion — two systems, one owner.** The fork's film stack
  (`negativeConversion` object, sidecar-owned, decode time, Film panel,
  scanner, profiles) stays the film workflow. Upstream's simpler conversion
  (flat `negative*` keys, applied after denoise) lives in
  `negative_conversion/inline.rs`, re-exported from `negative_conversion.rs`
  so upstream callers are unchanged. It stands down when the fork's
  conversion is enabled (never inverts twice), and its Details controls only
  show on images that already use it. Context menus keep the fork's
  Convert/Revert + Apply Film Settings.
- **Auto adjust.** Upstream's Subtle/Normal/Strong is `AutoAdjustOptions.strength`
  (Rust `AutoAdjustMode`, scales every correction incl. white balance), set in
  the fork's Auto popover; `calculate_auto_adjustments` / batch auto accept an
  optional one-off `mode` (upstream's library menu). Upstream dropped
  brightness from Auto; the fork keeps it (own switch) — `AutoAdjustmentResults`
  carries `brightness` again.
- **AI masks.** Upstream's ESAM3 sky/text/face and BiRefNet foreground replace
  U-2-Net/sky-seg. The fork's YuNet detector (`face_model`, eyes/mouth masks,
  import culling face cues) stays, downloaded via a `FACE_DETECTOR_ASSET`
  through upstream's asset API; `ai-eyes`/`ai-mouth` render beside `ai-face`.
  They are not in upstream's AI job types, so presets/paste keep their data.
- **Decoding.** `get_or_decode_pristine` (upstream, with `DecodeFlights`
  sharing a decode between same-file requests) gained a `_with_fallback`
  variant reporting the fork's embedded-preview fallback (never cached); the
  fork's one-decode-at-a-time `decode_permit` still gates `load_image`.
- **Export.** Upstream's memory budget + `exportWorkerThreads` replace the
  fork's RAM heuristic. The fork keeps its single whole-ROI 8-bit readback
  (now from `display_textures().working_texture`); 16-bit tiles store raw
  half-float bits and upstream's final pass converts them once.
- **Library.** Upstream's `build_image_file` listing (with `is_negative` added)
  and async/parallel album + folder-tree code were ported into
  `file_management/{albums,folder_tree}.rs`; the duplicate copies upstream
  re-added to `file_management.rs` were removed again, as were its embedded
  preview helpers (the fork uses `fast_raw_preview_scaled`).
- **Arrow keys.** Up/Down use upstream's on-screen row geometry
  (`libraryGridRows`), falling back to the fork's column step; Left/Right,
  wrap, Shift/Ctrl selection building and the first-press seed are the fork's.
- **Settings.** Fork layout kept; upstream's DataManagementSettings (storage
  usage, cache limits, clear actions, logs), Immich section, AI GPU
  acceleration, advanced processing (export threads, export memory limit) and
  the updated credits were slotted in.
- **Misc.** `save_primary_metadata` → upstream `update_sidecar`; the fork's
  processor square-sizing gave way to upstream's grow-only sizing; folder
  rename keeps the fork's full path remapping (roots, pins, icons, selection).

### v1.6.6 sync notes

Upstream v1.6.6 (+8 commits: relight, fog, AI denoise, auto straighten,
DNG-style baseline exposure, per-tool `activeTools`, shared
`defaultAdjustments.json`, zero-copy histogram/waveform, export DPI, Android
selection mode). How the fork was re-expressed on the new APIs:

- **`sectionVisibility` → `activeTools`.** Per-tool on/off replaces per-section
  eye toggles. Fork tools map as: point colors → `pointColor` (added to the
  Rust legacy `color` list so old sidecars keep working), skin smoothing /
  texture / scale → `presence` (global and per-mask), Auto's outputs →
  `basic`, `whiteBalance`, `colorPresence` (vibrance), `presence`
  (clarity/dehaze), `vignette`. The assistant's LUT action enables `lut`.
- **Shared defaults.** `src/utils/defaultAdjustments.json` is read by both TS
  and Rust (sidecars holding only defaults are deleted). The fork appended
  `pointColors`, `skinSmoothing`, `skinSmoothingScale`, `skinTexture`,
  `snapshots`; any new fork adjustment key belongs there too.
- **As-shot data.** The fork's as-shot WB cache in `white_balance.rs` moved to
  upstream's `as_shot.rs` (`AsShot { white_balance, exposure }`). Auto adjust
  measures at the effective baseline exposure (absolute mode) and reads WB
  from `loaded.as_shot`; `render_for_analysis` uses `RenderOptions`.
- **Histogram/waveform.** Upstream's zero-copy `AnalyticsFrame` code replaced
  the fork's copy inside `image_processing/analysis.rs`; the fork's auto
  analysis, auto profile and learning stay there. Upstream's duplicate block
  in `image_processing.rs` was dropped again.
- **Batch auto.** Rebuilt inside upstream's generic
  `update_sidecars_from_images` closure (git had silently merged the fork's
  auto code into the generic helper, which auto-straighten also uses).
- **Thumbnails.** Took upstream's downscale-then-warp path with relight, fog
  and lens blur — lens blur had been lost from thumbnails in the v1.6.5 merge.
- **Export XMP.** Upstream's `embed_xmp` (keywords + rating + label, merged
  with the source's XMP) now writes JPEG/PNG/WebP; the fork's keyword splicers
  for JPEG/PNG were removed. TIFF still gets the fork's XMP IFD entry (upstream
  writes nothing to TIFF), and AVIF/JXL keep the `.xmp` sidecar fallback.
  `jpeg_encoder` (fork) gained upstream's DPI density.
- **Crop.** `aspectRatio 0` is upstream's `ORIGINAL_RATIO` (resolved per image
  via `resolveAspectRatio`), which retired the fork's "keep Original tracking
  90° turns" effect. Upstream's `normalizeCropChange` replaced the fork's
  "don't persist a full-frame crop" check. The opt-in crop tool, guided
  perspective and remembered ratio (now only when `aspectRatio === 0`) stay.
- **Film panel** is registered in `AdjustmentSection`'s component map; the
  Controls panel only gates it on `negativeConversion.enabled`. The unused
  `NegativeConversionModal.tsx` was deleted rather than receiving upstream's
  pinch-zoom.
- **Settings** keep the fork's layout; upstream's baseline-exposure,
  highlight-preservation and AI-denoise-cache items were slotted into
  Processing.
- **Library arrows** in recursive view stay inside the focused image's folder
  (upstream), combined with the fork's row-step / shift-extend navigation.
- `ai_processing.rs` keeps both the fork's `face_model` and upstream's
  `normal_model` (relight normals).

### v1.6.5 sync notes

Upstream v1.6.5 (guided-filter local contrast, whites rewrite, LMS white
balance, pick/reject flags + rating operators, RAW+JPEG grouping, Apple RAW 9
engine on macOS, cloud/Clerk auth, custom aspect ratios, keybind rework,
AI-free mode, Czech/Dutch locales). Duplicates retired in upstream's favor:

- **Blur-pass gating.** The fork's `compute_blur_needs` (skip gaussian blurs
  with no consumer) was superseded by upstream's equivalent inline gating for
  the new guided-filter shader; the fork extends it with a `need_skin` term so
  skin smoothing keeps its tonal/clarity/structure blurs alive.
- **Saved crop ratios.** The fork's named crop presets
  (`AppSettings.cropPresets`, name + WxH) gave way to upstream's
  `customAspectRatios` (unnamed WxH chips with a Save button beside the custom
  fields). Old `cropPresets` entries drop from settings on next save.
- **Auto-adjustment duplicates.** Upstream's unchanged copies of
  histogram/waveform/auto-analysis were dropped in favor of the fork's
  `image_processing/analysis.rs` (which carries the auto-profile and
  scan-clip extensions); upstream's new white-balance area sampler was kept.
- **Albums/folder-tree.** Upstream's copies were dropped (the fork had moved
  them into `file_management/{albums,folder_tree}.rs`); upstream's `flag`
  field was ported into the fork modules.
- **Export ICC.** JPEG keeps the fork's hand-built profile + mozjpeg-class
  encoder (guard-tested); PNG adopts upstream's new tagging with their
  736-byte CC0 profile.
- **Adjustment visibility.** Upstream's `adjustmentLayout`
  (hidden/reordered/collapsed tools + section focus mode) replaces the fork's
  visibility menu in the UI; Point Color registered as a `pointColor` tool in
  that system. The fork's Rust-side `adjustment_visibility` parse gating
  remains but now stays at its all-visible default.
- **Kept fork-side:** per-mask sharpness loop and skin smoothing in the
  shader (rebuilt around upstream's guided-filter main), point colors,
  negative-conversion stack, panorama projections, assistant stack,
  external-editor round trip, boot error boundary, preview-fallback flag,
  tether/scanning modules, reqwest native-TLS split, thin-LTO release
  profile, fork thumbnail fast path (`fast_raw_preview_scaled`).
