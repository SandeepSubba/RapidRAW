# SD-Card Importer with AI Culling

A dedicated, full-screen import workflow for RapidRAW that brings a **Capture One–style
culling experience** to the moment you plug in an SD card: detect the card, group similar
shots, score them with AI (sharpness + faces + eyes/gaze/expression), keep the best of each
burst, and import only the keepers — with ratings, labels, and metadata travelling along.

It is built as a **mostly self-contained module** that reuses RapidRAW's existing engines
(similarity culling, the import pipeline, the thumbnail system, the bundled ONNX models)
and touches the main app in only a handful of small, well-isolated places.

---

## Table of contents

1. [Workflow overview](#workflow-overview)
2. [Source selection](#1-source-selection)
3. [Scanning](#2-scanning)
4. [Grouping similar shots](#3-grouping-similar-shots)
5. [AI scoring](#4-ai-scoring)
6. [The 0–5 quality grade](#5-the-05-quality-grade)
7. [Auto-select best](#6-auto-select-best)
8. [Manual review: grid, filters, ratings](#7-manual-review-grid-filters-ratings)
9. [The viewer (single + compare + zoom)](#8-the-viewer)
10. [Exclude already-imported](#9-exclude-already-imported)
11. [Import settings & running the import](#10-import-settings--running-the-import)
12. [Keyboard shortcuts](#keyboard-shortcuts)
13. [Preferences that persist](#preferences-that-persist)
14. [Models used](#models-used)
15. [Architecture & file map](#architecture--file-map)
16. [Backend commands](#backend-commands)

---

## Workflow overview

```
Source ──▶ Scan ──▶ Review ──▶ Import
 (drive    (recursive  (grid / viewer,    (copy + rename +
  or         image      group, score,      organize, eject)
  folder)    walk)      select keepers)
```

Open it from the library ("Import from SD card"). The importer runs as its own top-level
view with its own store, so it never entangles with the normal editing/library UI.

---

## 1. Source selection

- **Auto-detected drives** — removable volumes are listed first (via `sysinfo`). USB card
  readers that report as fixed disks are still shown, since many cameras/readers do.
- **Browse for a folder** — pick any folder (e.g. a card already copied to disk) as the
  source.
- System/pseudo mounts (`/dev`, `/System/Volumes/…`, VM stores) are hidden.

## 2. Scanning

A **recursive** walk collects every supported image (cards nest photos under
`DCIM/100XXXXX/`). macOS AppleDouble stubs (`._NAME.JPG`) are skipped. Results are sorted
and shown immediately as a flat grid — **nothing is selected by default**; selection is
always explicit.

## 3. Grouping similar shots

"Group similar" is an opt-in toggle (Capture One's "Group Overview"). When on, photos are
grouped into **bursts of visually similar frames** using perceptual hashing (DoubleGradient
16×16) + Hamming-distance BFS.

- A **Similarity %** slider controls how strict grouping is. It re-groups **instantly** —
  the analysis is cached on the backend, so moving the slider never re-decodes images.
- Grouping (fast, hash-only) is deliberately **separate from scoring** (slower) so the
  "Group similar" toggle stays snappy.

## 4. AI scoring

Scoring runs as its own step (triggered by **Auto-select best**), in two passes:

**Pass 1 — technical metrics** (all photos, in parallel)
A weighted quality from a downscaled thumbnail:
`0.40·sharpness + 0.35·center-focus + 0.25·exposure`
(sharpness = Laplacian variance; exposure = histogram-clipping penalty).

**Pass 2 — face-aware "people quality"** (grouped photos only, serial)
Because the per-group "best" pick is what matters, the expensive face pass runs only on
photos that belong to a similar group (singles are auto-kept regardless):

1. **Face detection** — UltraFace (version-RFB-320) finds every face.
2. **Per-face grading** — each face is cropped from a high-resolution (1920px) image and
   graded by **one CLIP zero-shot softmax** over five states, each mapped to a desirability
   weight:

   | Per-face state                                   | Weight |
   |--------------------------------------------------|:------:|
   | Smiling, looking at the camera                   | 1.0    |
   | Neutral expression, looking at the camera        | 0.7    |
   | Eyes closed / blinking                            | 0.0    |
   | Turned away from the camera                       | 0.0    |
   | Unhappy / frowning                                | 0.3    |

   This folds **eyes, gaze, and expression** into a single inference per face.
3. **Per-frame face score** combines faces as `0.35·mean + 0.65·worst`, so one blinker /
   looker-away / bad expression **demotes the whole frame** below an attentive sibling.
4. **Face sharpness** — the Laplacian variance of each face crop (the most relevant focus
   cue for a people burst, far better than whole-image sharpness).

**Final score (people shots):**
`0.25·technical + 0.50·face(eyes/gaze/expression) + 0.25·face-sharpness`

Photos with **no faces** (landscapes, objects) keep the pure technical score. If the face
or CLIP model can't be loaded (offline / not downloaded), scoring **degrades gracefully**
to technical-only.

> **Note on gaze:** distinguishing two near-identical frames purely on *subtle gaze* is at
> the edge of what whole-face CLIP can do. The weighting and high-res crops help, but it is
> not perfect — the viewer's zoom is there so you can verify and correct in one click.

## 5. The 0–5 quality grade

Raw scores are abstract 0–1 numbers, so each scored photo is shown as a **whole-number 0–5
grade** ("Q 4"), min–max normalized across the card (**5 = best on this card**, 0 = worst).
The "Q" prefix distinguishes it from the manual 1–5 star rating.

## 6. Auto-select best

One click runs the whole pipeline (group → score) if needed, then selects keepers:

- **Exactly one photo per similar group** — the highest-scoring representative — **only if
  it grades ≥ 3**. A burst whose best frame still grades 0–2 is dropped entirely.
- **All ungrouped singles** (unique shots) are kept; blurry shots and already-imported
  photos are excluded.

You can always override afterward (keep extras, drop keepers).

## 7. Manual review: grid, filters, ratings

- **Grid** with similar-group stacks, "Other images" (singles), and "Blurry (excluded)".
- **Keyboard navigation** across the responsive grid (arrow keys move the focused cell;
  Space keeps/skips; Enter opens the viewer).
- **Ratings & color labels** (1–5 stars, color labels) per photo — written to the source
  `.rrdata` sidecars so they travel with the import.
- **Filters** by rating, color label, and file type (e.g. hide JPEGs when shooting
  RAW+JPEG). The file-type filter also governs what gets imported.
- **Select all / Select none / Auto-select best.**

## 8. The viewer

Toggle **Grid ⇄ Viewer**. The viewer has two modes:

- **Single** (default) — a full-resolution loupe with **Fit / 100%** and drag-to-pan, plus
  a film-strip of all groups and a strip of the current group's frames.
- **Compare** — all frames of a group side by side, at **full resolution** (not
  thumbnails), so you can judge sharpness / eyes / gaze directly. Features:
  - **Mouse-wheel zoom** (up to 6×), **shared across all panes** so you zoom into the same
    spot on every frame at once.
  - **Drag to pan** when zoomed; a `1.5× · reset` control snaps back.

## 9. Exclude already-imported

The "Exclude already-imported (skip duplicates)" toggle compares the card against the
**chosen destination folder**, matching by **filename stem** (so a RAW already imported also
matches its JPEG), recursively. Matches are **hidden from the grid entirely** and excluded
from import. Requires a destination to be set (that's what it compares against).

> This is distinct from "Group similar": grouping handles *visually similar bursts* (kept
> visible so you choose); exclude-already-imported handles *photos already in your library*.

## 10. Import settings & running the import

The **Import N** button is enabled as soon as photos are selected; if no destination is
chosen yet, it opens the folder picker first, then proceeds. Settings (gear icon):

- **Filename template** (default `{original_filename}`)
- **Organize into date folders** (with a configurable date format)
- **Delete from source after import**
- **Exclude already-imported (skip duplicates)**
- **Eject card after import** — unmounts the source volume once the import finishes.

The import reuses RapidRAW's existing `import_files` engine (copy + rename + organize +
optional delete) and refreshes the destination in the library on completion.

## Keyboard shortcuts

| Key                | Action                                  |
|--------------------|-----------------------------------------|
| `1`–`5`            | Rate the focused photo (0 clears)       |
| `Shift`+`1`–`5`    | Color label (red/yellow/green/blue/purple); `Shift`+`0` clears |
| `P` / `X`          | Keep / skip the focused photo           |
| `←` `→`            | Grid: move focus · Viewer: prev/next group |
| `↑` `↓`            | Grid: move focus · Viewer: within group |
| `Space`            | Keep/skip the focused photo             |
| `Enter`            | Open the viewer on the focused photo    |
| `Esc`              | Viewer: back to Compare                 |

Both the number row and the numeric keypad are accepted for ratings.

## Preferences that persist

Saved across sessions (never the transient scan/selection): import settings, the
exclude-imported and eject-after-import toggles, the similarity %, and the file-type filter.

## Models used

| Model        | Purpose                              | Source                                   |
|--------------|--------------------------------------|------------------------------------------|
| **UltraFace** (version-RFB-320, ~1.2 MB) | Face detection for the per-face check | `onnx/models` (ONNX Model Zoo) |
| **CLIP** (combined vision+text) | Zero-shot eyes/gaze/expression grading | bundled RapidRAW model |

Both download on first use and are cached; the face pass is best-effort and degrades to
technical-only scoring if a model is unavailable.

## Architecture & file map

The module owns its store, components, and actions hook, and depends on the main app only
through the shared `Invokes` enum, read-only thumbnails, one UI boolean, one `App.tsx`
branch, and one entry button.

**Frontend (`src/`)**
- `store/useImportStore.ts` — isolated zustand store (with persisted preferences).
- `hooks/useSdImportActions.ts` — all actions (detect, scan, group, score, select, import).
- `hooks/useImportKeyboard.ts` — rating/label/keep/skip shortcuts.
- `components/views/import/`
  - `ImportView.tsx` — stage orchestrator (source → scan → review → import).
  - `SourcePicker.tsx` — drive list + browse.
  - `CullGroupsGrid.tsx` — the review grid, toolbar, grouping, grades, navigation.
  - `ImportViewer.tsx` — single + compare viewer with zoom/pan.
  - `ImportReviewBar.tsx` — destination, import settings, the Import button.
  - `ImportFilterBar.tsx`, `RatingColor.tsx`, `LazyThumb.tsx`, `importFilters.ts`.

**Backend (`src-tauri/src/`)**
- `sd_import.rs` — drive list, recursive scan, group/score commands, preview, eject,
  already-imported detection.
- `culling.rs` — perceptual hashing, grouping, technical metrics; `score_image`,
  `load_face_image`, `normalized_sharpness_of`.
- `tagging.rs` — `score_faces` (per-face CLIP grading + face sharpness), `clip_prompt_probs`.
- `ai_processing.rs` — model registry + `run_face_detection` (UltraFace) and
  `get_or_init_face_model`.

## Backend commands

| Command                       | Purpose                                            |
|-------------------------------|----------------------------------------------------|
| `list_source_drives`          | Detect removable/all volumes                        |
| `scan_source_images`          | Recursive supported-image walk                      |
| `analyze_for_import`          | Decode + perceptual hash (grouping only, fast)      |
| `group_for_import`            | Re-group cached analysis at a similarity threshold  |
| `score_for_import`            | Technical + face-aware scoring (the "AI score")     |
| `find_existing_in_destination`| Filename-stem match against the destination         |
| `get_import_preview`          | Full-resolution preview (embedded JPEG for raws)    |
| `eject_drive`                 | Unmount the source volume                           |

The actual copy reuses the existing `import_files` command unchanged.
