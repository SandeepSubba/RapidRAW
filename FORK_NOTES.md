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

## Build / run

```bash
npm install        # Node.js LTS + Rust toolchain required
npm run typecheck  # note: upstream has pre-existing strict-tsc errors; the
                   # Vite/esbuild build does not gate on them
npm start          # tauri dev — builds the Rust backend and launches the app
```
