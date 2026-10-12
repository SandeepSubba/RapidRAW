# Image Importer with AI Culling

A dedicated, full-screen import workflow for RapidRAW that brings a **Capture One–style
culling experience** to importing from **any source** — an SD card, a camera, a USB drive,
or any folder on disk: detect/pick the source, group similar shots, score them with AI
(sharpness + faces + eyes/gaze/expression, plus subject focus + composition for non-people
shots), keep the best of each burst, and import only the keepers — with ratings, labels, and
metadata travelling along.

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

**Your culling work is saved.** Which photos are kept, plus ratings and colour labels, are
written to local storage as you go (`savedSelection` in `useImportStore`). If the importer
is closed or the app restarts before you import, scanning the **same source path** again
restores them (limited to photos still on the card) and shows a toast. Scanning a different
source with nothing picked leaves the saved work alone; "Select none" on the same source
clears it; a successful import discards it. Grouping and scores are not saved and are
recomputed. Matching is by path only, so a different shoot on a card that reuses the same
file names would inherit the old picks.

## 3. Grouping similar shots

"Group" is an opt-in toggle (Capture One's "Group Overview") with two modes:

**Similar look** (default) — a **hybrid of look and capture time** that groups photos
into **scenes** (`culling::group_by_scene`). Each frame is compared only with the next 3
frames in capture order. Two frames link when either their **colour signature** (a 4×4×4
RGB histogram of a 16×16 thumbnail) or their **perceptual hash** (DoubleGradient 16×16,
144 bits) is within the threshold; links chain, so a long sequence in one setup becomes
one group. Two sliders:

- **Similarity %** — strictness of the look match (higher = stricter). Maps to hash bits
  as `(100 − %) × 144 / 100`; the colour signature gets the same fraction of its range.
- **Max gap** (1–60 s, default 20, stored separately from the By-time gap) — frames
  further apart than this are never linked directly.

Time also weights the match: frames shot back-to-back get a threshold 1.5× looser, fading
linearly to no bonus at 10 s apart, because consecutive frames are far likelier to be the
same scene.

Why not compare every photo with every other one (the library culling dialog still does,
via `group_analyses`): to keep unrelated shots apart that comparison must be strict, and
then it misses most of a burst. On a 1232-shot event card at 80% it left 414 photos
ungrouped; the hybrid at its defaults leaves 71. Known limit: consecutive poses in one
setup share a palette and chain into one group — raise Similarity or lower Max gap to
split them.

**By time** — true **burst detection** by capture time. Photos are sorted by their EXIF
timestamp and split into bursts whenever the gap between consecutive frames exceeds the
**Max gap** slider (1–60 s). This catches *event bursts* that visual hashing misses — e.g.
five different poses shot seconds apart group together even though the frames don't look
near-identical. Timestamps are read once during analysis and cached.

Both modes:
- re-group **instantly** — the analysis is cached on the backend, so changing mode / slider
  never re-decodes images;
- are deliberately **separate from scoring** (slower) so the toggle stays snappy;
- show groups and ungrouped photos in **one timeline** in the chosen sort order: a group
  sits where its first member falls, and ungrouped photos appear in place between the
  groups (sort by date and the grid reads chronologically straight through).

## 4. AI scoring

Scoring runs as its own step (triggered by **Auto-select best**), in two passes:

**Pass 1 — technical metrics** (all photos, in parallel)
A weighted quality from a 720px thumbnail:
`0.40·sharpness + 0.35·centre-focus + 0.10·highlights + 0.08·shadows + 0.07·balance`
(sharpness = Laplacian variance. **Lighting** is three cues: *highlights* and *shadows*
penalize the share of clipped / crushed pixels separately, and *balance* penalizes a frame
whose median brightness is clearly too dark or too bright even when little is clipped —
an under-exposed bracket used to score as well as the good one.)

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
3. **Focus where it matters, on the full-size pixels.** Faces are found on the 1920px copy,
   but focus is measured on the same faces in the **full-size image** (for RAWs, the
   camera's embedded preview), downscaled to a fixed size so faces compare:
   - **face sharpness** — the face box;
   - **eye sharpness** — a patch around each eye from YuNet's landmarks, taking the
     **sharper eye** (you focus on the near eye). At 1920px a frame focused on the ear or
     hair still looks sharp; at full size it doesn't.
4. **Light on the faces** — the exposure of the middle of each face: a face sunk in shadow
   (backlit, under-exposed) or washed out / blown scores low. The band is deliberately wide
   so darker and lighter skin both read as well lit; within a burst the faces are the same
   people, so the comparison is about the light.

**Final score (people shots)** — the cue weights (learnable, see §5b), worst-face terms
make one blinker / looker-away demote the whole frame:

| Cue | Default weight |
|---|:-:|
| technical quality (pass 1) | 0.15 |
| face sharpness | 0.10 |
| eye sharpness | 0.15 |
| looking at camera (mean / worst face) | 0.12 / 0.20 |
| expression (smiling) | 0.08 |
| eyes open (worst face) | 0.10 |
| light on the faces | 0.10 |

**Non-people shots** (landscape, product, food, still life — no faces detected) get a
dedicated score instead of the centre-biased technical metric:

`0.25·overall-sharpness + 0.30·subject-region-sharpness + 0.10·highlights + 0.08·shadows + 0.07·balance + 0.20·aesthetic`

- **Subject-region sharpness** = the sharpest cell of a 3×3 grid **of the full-size image**,
  so the in-focus subject is rewarded *wherever it sits* in the frame (not just the centre) —
  the key fix for off-centre subjects and landscapes — and a slightly missed focus plane
  isn't hidden by downscaling.
- **Aesthetic** = a CLIP zero-shot "well-composed, well-lit, appealing" vs "blurry, poorly
  composed, dull" cue (reuses the bundled CLIP model, no extra download).

For near-duplicate bursts the ranking is driven mostly by subject sharpness + exposure (what
you want for product/food/still life); the aesthetic term mainly breaks ties on composition.

If the face or CLIP model can't be loaded (offline / not downloaded), scoring **degrades
gracefully** to the technical metric (which still has the lighting cues).

> **Note on gaze:** distinguishing two near-identical frames purely on *subtle gaze* is at
> the edge of what whole-face CLIP can do. The weighting and high-res crops help, but it is
> not perfect — the viewer's zoom is there so you can verify and correct in one click.

## 5. The 0–5 quality grade

Raw scores are abstract 0–1 numbers, so each scored photo is shown as a **whole-number 0–5
grade** ("Q 4"), min–max normalized across the card (**5 = best on this card**, 0 = worst).
The "Q" prefix distinguishes it from the manual 1–5 star rating.

## 5b. Learning from your culling (personalized selection)

The culling **learns your taste** from what you actually import. Two things are learned,
per similar group, into an on-device model (`import_cull_model.json` in the app data dir):

- **Which cues you prefer.** Every scored photo carries its cue vector — the people cues
  above, the non-people cues, or the technical cues when the AI models are unavailable —
  and each kind learns separately (before, only photos with detected faces learned
  anything). Within each burst, the frames you **kept** vs the group-mates you **skipped**
  shift the weights toward the cues where your keepers win (a nearest-centroid / Rocchio
  update), blended with the defaults so a few picks nudge gently and sustained feedback
  personalizes more. Frames of different kinds in one burst aren't compared.
  Example: keep the well-exposed frame over a slightly sharper dark one a few times and
  *balance* gains weight.
- **Your keep policy.** How often you keep the app's best / 2nd / 3rd / later frame of a
  burst, and how good a burst's best frame must be (its 0–5 grade) before you keep the burst
  at all. Priors reproduce the old behaviour (one frame, grade ≥ 3) until your imports
  outweigh them — e.g. keep the top two of three bursts once and auto-select starts keeping
  two per burst.

The **assistant** learns too. Its cull decisions in the import window are remembered
until import; the ones you override — unmarking its pick, keeping a frame it rejected,
changing its stars — are recorded (`assistant_cull_feedback.json`, newest 30), together
with your keep habits above, and quoted in every later cull request ("You rejected a3.jpg
(soft focus) — they kept it"), so the model adapts instead of repeating the misjudgement.
Requests you give in chat ("mark these keepers") are your decisions and aren't recorded.

Learning happens only from imports (scored bursts of ≥ 2 importable frames — hidden file
types and already-imported photos don't count). It is per machine: the model lives in the
app data dir, not the repo.

Controls live in the import settings panel:
- **Personalize selection (learn from my picks)** — on by default; off uses fixed defaults
  and stops recording. A line underneath says what has been learned ("Learned from 12
  bursts over 4 imports · you keep ~2 per burst · auto-select keeps the top 2").
- **Reset learning** — wipes all of it (cue weights, keep policy, assistant corrections).

## 6. Auto-select best

One click runs the whole pipeline (group → score) if needed, then selects keepers:

- Per similar group, the frames your **keep policy** keeps (§5b) — by default **exactly one**,
  the highest-scoring, and **only if it grades ≥ 3** (a burst whose best frame grades 0–2 is
  dropped entirely). Once learned, e.g. the top two, or weaker bursts too.
- **All ungrouped singles** (unique shots) are kept; blurry shots and already-imported
  photos are excluded.

You can always override afterward (keep extras, drop keepers).

## 7. Manual review: grid, filters, ratings

- **Grid** with similar-group stacks and ungrouped photos interleaved in sort order, then
  "Blurry (excluded)".
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
**chosen destination folder** (recursively), matching on two independent signals:

- **Filename stem** — so a RAW already imported also matches its JPEG counterpart.
- **File content** — a BLAKE3 hash comparison, so photos you have **renamed in the
  library** after importing still register as duplicates. Hashing only runs when the
  destination holds a file of exactly the same byte size, so a card full of new photos
  costs metadata reads alone.

Matches are **hidden from the grid entirely** and excluded from import. Requires a
destination to be set (that's what it compares against).

> This is distinct from "Group similar": grouping handles *visually similar bursts* (kept
> visible so you choose); exclude-already-imported handles *photos already in your library*.

## 10. Import settings & running the import

The **destination defaults to the folder the library is currently showing** (albums are
virtual, so they never pre-fill), and an explicit choice you make is never overwritten —
importing into the folder you are looking at needs no picking at all.

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
