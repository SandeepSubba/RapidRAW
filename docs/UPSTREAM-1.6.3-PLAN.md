# Upstream v1.6.3 — retirement analysis & merge plan

Analysis of CyberTimon's v1.6.3 (86 commits since our last sync at `9776d550`)
against this fork's feature set, done **before** merging, to decide what fork
code can be retired. Three parallel code-level comparisons (perspective /
retouch-masks / export-platform); file:line evidence in the session that
produced this doc.

**Headline: the fork's core identity is untouched.** Upstream changed nothing
in `sd_import`, `scanning` (film scanner), `tethering`, `negative_conversion`,
`assistant`, or `culling`. The importer, film stack, negative pipeline, and the
whole AI assistant have no retirement question this round.

---

## 1. RETIRE (fork code deleted in favor of upstream)

| Fork code | Replaced by | Condition |
|---|---|---|
| TS keystone solver (`src/utils/keystone.ts` solve path), `GuidedKeystoneOverlay.tsx`, `keystoneLines`/`guidedKeystoneActive` store fields | Upstream `guided_perspective.rs` (639-line Rust homography: roll correction, 1v+1h solve, degeneracy rejection) + **persisted, re-editable guides** in the sidecar (`guidedPerspective` adjustment) | **Must port the fork's auto-crop** (see §4) — upstream computes `compute_max_inscribed_crop` and never uses it: black wedges after every correction |
| Export in-run duplicate-name claim set (`export_processing.rs` `used_output_paths` block) | Upstream 27f27c3b — same idea, plus `.exists()` check → never clobbers files already on disk (`_1`, `_2`…) | **Open decision** (see §5): upstream refuses to overwrite prior exports; the fork deliberately preserves overwrite-on-re-export |
| Fork's removal of the forced `_{sequence}` suffix in `ExportPanel.tsx` | Upstream did the identical removal — take their file verbatim | none |
| `webkit2gtk-nvidia-quirk` dep + 3 call sites in `lib.rs` (import, `apply_workaround_with_options` block, `needs_workaround` logging) | Upstream `tauri-plugin-wayland-nvidia-quirk` (same crate family, repackaged as a plugin) | take the `unsafe {}`-wrapped `set_var`s as a unit |
| Five inline crop-bounds binary-search copies in `Editor.tsx` | Upstream's extracted `calculateStraightenAngle` / `isCropWithinBounds` / `calculateAutoCropForRotation` in `cropUtils.ts` | net deletion |
| `docs/FEATURES.md` claim that Clone/Heal masks are fork features | They are upstream's (`a2685974`), identical on both branches | docs fix, done in this commit |

## 2. KEEP (upstream has no equivalent, or an inferior one)

Everything untouched upstream: **SD importer + culling, film scanner, negative
conversion, AI assistant (all of it), snapshots, saved/remembered crop ratios,
import destination default + content-hash duplicates, sRGB ICC tagging,
optimized-Huffman JPEG, `{sequence:START}` per-name-group numbering (upstream's
fix is orthogonal), Capture One adjustment shortcuts, batch metadata sync.**

Plus, where upstream *did* move nearby:

- **Frequency-separation skin smoothing** (global + per-mask, Texture, Scale) —
  zero upstream counterpart; their new Retouch is a destructive bilateral
  blemish softener, a different tool.
- **AI Eyes/Mouth masks, YuNet two-pass face refine, Portrait stack** —
  upstream's `MASK_AI_TYPES` is Subject/Sky/Foreground/Depth only; their
  edge-aware guided filter refines segmentation masks, never our analytic
  ellipses.
- **Rotate by dragging outside the crop** (live preview, snap-to-0, letterbox
  layer, degree HUD) — no upstream analogue; coexists with their shift+drag.
- **Fork tethering + fork negative conversion** — standing decision from the
  1.6.2 merge; upstream didn't advance theirs this cycle.
- **GPU export perf fixes** — `processor_side()` grow-only sizing, whole-ROI
  readback, blur-pass skipping, `apply_pending_size`. Orthogonal to (and kept
  alongside) upstream's auto-heal.
- **`fast_raw_preview_scaled` IDCT-scaled embedded-preview path** — upstream's
  thumbnail refactor still uses its old extractor; ours is why the SD review
  grid stopped OOMing and why CR2s get the full-size preview.

## 3. ADOPT (net-new upstream value, take as-is)

Retouch tool · Liquify (+ eraser/modes) · edge-aware guided filter for AI-mask
edges (`0d883727` **together with** `d2d34a70`) · mask-panel categorization
(re-insert Eyes/Mouth into `MASK_AI_TYPES`, keep Portrait entry point + icons)
· guided perspective · straighten auto-crop fix + global shift+drag ·
export-to-original-folder + subfolder mode · GPS preservation + timestamp
guards · local-EXIF-as-UTC fix (`creation_datetime_to_utc`) · GPU auto-heal +
`on_uncaptured_error` · split small/medium thumbnail resolutions (hand-merge,
see §4) · centralized `.rrcache` EXIF cache (no clash with `.rrdata`) ·
zoom-to-100%-on-click setting · crop-reset-to-free fix.

## 4. MANDATORY PORTS during the merge

1. **Auto-crop for perspective**: keep `fitScaleForParams` driving
   `transformScale` (covers manual sliders + old sidecars), or wire upstream's
   dead `compute_max_inscribed_crop` into the crop rect for the guided path.
   Without one of these, adopting upstream keystone is a visible regression.
2. **Thumbnail hand-merge**: take upstream's split-resolution plumbing,
   `Arc` geometry cache and 4-tuple return, then re-point the RAW fast path at
   `fast_raw_preview_scaled(..., Some(medium_thumbnail_resolution))`. Frontend
   fan-out: `useTauriListeners`, `useAppNavigation`, `useProcessStore`,
   `SettingsPanel` all change shape — all four are fork-diverged.
3. **Masks panel**: upstream deletes the array our Eyes/Mouth grid entries live
   in; re-add them under `MASK_AI_TYPES`, keep `MASK_ICON_MAP` entries and the
   Portrait button.
4. **GPU uniform struct** (`image_processing.rs` + `shader.wgsl`): both sides
   changed it again. FORK_NOTES rule applies — identical field order, scalar
   count before the `mat3x3` block ≡ 0 (mod 4), absorb `_pad_*` slots.
5. Restore upstream's `always_decode_raw_thumbnails` check in the RAW-thumbnail
   guard (dropped by our rewrite — fork bug, see §6).

## 5. OPEN DECISION (needed before the export hunks are merged)

**Overwrite semantics on export.** Upstream now *never* clobbers a file already
on disk (appends `_1`). The fork deliberately overwrites on re-export (fresh
export of the same photo replaces the old JPEG — matches this fork's
web-export workflow). Options: (a) take upstream's never-clobber; (b) keep
fork's overwrite; (c) upstream behavior behind an "overwrite existing" export
toggle. The merge resolves the same 60 lines either way — decide first.

## 6. FORK BUGS found by the comparison (fix regardless of merge)

- `set_timestamps_from_exif` lacks upstream's `timestamp() <= 0` guard: a RAW
  with an unparseable date stamps the export with epoch-0 file times.
- The RAW-thumbnail fast path dropped the `always_decode_raw_thumbnails`
  settings check during the `fast_raw_preview_scaled` rewrite.

## 7. Merge order (ascending conflict cost, one hotspot at a time)

1. Nvidia quirk swap (mechanical) → 2. EXIF cache (additive) → 3. zoom setting
(two adjacent-line inserts) → 4. GPU auto-heal + fork sizing (two small
conflicts) → 5. export naming + destination mode **together** (same 60 lines;
decide §5 first) → 6. `exif_processing.rs` GPS/timestamps (largest textual
conflict; fork is +745 lines there) → 7. masks panel + retouch/liquify →
8. guided perspective + straighten (port auto-crop) → 9. thumbnails **last and
alone** (worst fan-out).

Pre-merge prep done in this commit: deleted the eight tracked `" 2."`
merge-artifact duplicates (all unreferenced) that would have generated
spurious conflicts.
