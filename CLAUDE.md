## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).

## Working context (fork: SandeepSubba/RapidRAW)

- Active branch: `integration/all-features` — the daily-driver integration of all
  feature branches. Upstream PRs are cut from clean single-feature branches,
  never from this branch. Work happens on two machines (home + office); the
  repo is the only shared state, so keep commit messages self-explanatory and
  update docs/CLAUDE.md when semantics change.
- The user scans 35mm film (C-41 / B&W / E-6) on a Plustek OpticFilm 7600i via
  the in-app SANE scanner (Import window). `docs/FILM_SCANNER.md` is the
  canonical, up-to-date description of the whole film stack — read it before
  touching scanning or negative conversion.
- Film/negative conventions that are easy to get wrong:
  - `adjustments.negativeConversion` is **sidecar-owned**. The save guard in
    file_management.rs re-inserts the sidecar's copy over whatever the frontend
    sends — conversion params must go through the dedicated commands
    (`update_negative_conversion`, `set_negative_film_base`), never
    `setAdjustments`. This keeps them out of undo, presets, and copy/paste.
  - Conversion runs on CPU at decode time (image_loader.rs), before the GPU
    stack; caches are keyed on the whole negativeConversion blob. Any param
    change requires clearing decode caches + re-decoding (the commands do it).
  - Film-stock profiles (`AppSettings.negativeProfiles`) store params only —
    bounds are per-frame and never belong in a profile.
  - Scan-time clip percentiles: auto-tone solves against default-clip bounds
    while renders use user-clip bounds; solving on user bounds would cancel the
    sliders (auto-tone anchors the median).
  - Rust `AppSettings` (app_settings.rs) is a strict serde struct — any new
    TS-side settings field needs its one-line Rust field or it silently drops
    on the next save round-trip.
- The AI assistant panel has a scan-preview mode: when the scanner pane is open
  with a preview, it drives scanner-store controls (see AssistantPanel's
  `scannerContext`/`applyScannerPatch`) instead of editor adjustments.
- The `claudecode` assistant provider shells out to the `claude` CLI (absolute
  path in settings); prompts go via stdin — never argv (E2BIG with mask-laden
  adjustments).
