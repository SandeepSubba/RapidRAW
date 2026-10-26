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
- Assistant culling (utils/assistantCull.ts + `assistant_cull`) sends each burst
  as ONE labelled multi-image request with its own system prompt/schema — don't
  route it through the per-image batch path (the model can't compare frames
  there). In the import window (utils/importAssistant.ts) the panel works on the
  import store instead of the editor: pick/reject = keptPaths add/remove,
  ratings/labels land in the store and are written at import. Rating writes are
  guarded with "already equal" checks because `handleRate` toggles.
- Import culling learns at import time only (utils/importLearning.ts →
  `record_cull_picks` / `record_assistant_cull_feedback`): cull_model.rs keeps cue
  preferences per kind (people 8 cues, scene 6, technical 5 — order documented on
  `CueKind`) plus a keep policy (rank / grade). Changing a kind's cue list resets that
  kind's learning (vector shapes must match), so add cues deliberately and document them in
  docs/IMAGE_IMPORTER.md §4. Only the assistant's own culls are stored as `aiDecisions`;
  chat-directed changes go through `importCullTarget` and must not be recorded.
- Auto adjust (`calculate_auto_adjustments`) takes the frontend's adjustments JSON;
  with `AutoAdjustOptions.keep_look` it renders them through the preset-preview path
  (analysis.rs `render_for_analysis`) and returns deltas tagged `_relative` that the
  frontend ADDS to the sliders. `auto_results_to_json` filters by the options —
  batch auto (file_management.rs) must go through it too, absolute mode only.
  `AutoProfile` (settings.json) is learned from rendered edited thumbnails only
  (`measure_output_style` / `learn_style`); new fields need serde defaults so v1
  profiles keep loading. Auto never learns from its own output — only from saves.
- Upstream v1.6.6 replaced `sectionVisibility` with per-tool `activeTools`
  (Rust `is_tool_active`, TS `getActiveTools`/`mergeActiveTools`). Fork tools:
  `pointColor`, skin smoothing under `presence`; Auto returns an `activeTools`
  patch. New adjustment keys go in `src/utils/defaultAdjustments.json` (shared
  by TS and Rust). See FORK_NOTES.md "v1.6.6 sync notes".
- Two negative conversions exist since upstream v1.6.7: the fork's sidecar-owned
  `negativeConversion` (Film panel, the film workflow) and upstream's flat
  `negative*` keys (`negative_conversion/inline.rs`). Upstream's stands down when
  the fork's is enabled — keep that guard if either changes. Auto strength is
  `AutoAdjustOptions.strength`. See FORK_NOTES.md "v1.6.7 sync notes".
- The `claudecode` assistant provider shells out to the `claude` CLI (absolute
  path in settings); prompts go via stdin — never argv (E2BIG with mask-laden
  adjustments).
- Generative replace (Settings → Generative AI) needs the local ComfyUI +
  RapidRAW-AI-Connector stack — see `docs/AI_CONNECTOR.md` (port traps: app →
  connector:5000, connector → ComfyUI:5545). Installed on the home Mac only.
