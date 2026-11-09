# Review backlog

Work promoted from whole-codebase reviews because it merits its own branch belongs here. A request to “grab something from the review backlog” means this file; a request to “grab something from the TODO” or “grab something from the roadmap” does not. Every entry is ready for separate work and is prioritized directly rather than grouped by workflow stage.

See [`docket/CODE_REVIEW_GUIDE.md`](../docket/CODE_REVIEW_GUIDE.md) for how findings enter and leave this backlog and [`docket/TODO_GUIDE.md`](../docket/TODO_GUIDE.md) before claiming or resolving an entry.

## P0 Critical

## P1 High

## P2 Normal

- [SIM] `neighbour-range-rules` — **Make neighbour readers honour their stated ranges and decide whether neighbourhoods wrap across the torus.** Mate search and symbiosis accept anything in a 5×5 block of hash cells, and no neighbour query sees across the map edges that movement and terrain both wrap, so who meets whom is not what the constants and docs say.
  - Starting point: add `dist <= mate_range` in `reproduction_system` and a `SYMBIOSIS_RANGE` test in `symbiosis_tracking_system`; for the torus, wrap `SpatialHash` cell keys and `CellGrid` indices and use a torus delta in distance checks, or record the flat neighbourhood in `docs/DECISIONS.md` as accepted. Ask the user about the torus half first. Both change dynamics: re-run the 8-seed audit and compare.
  - Source: review/2026-10-09-0906-full.md, 2026-10-09
  - Findings: `neighbour-reach-not-enforced`, `neighbour-queries-ignore-torus-wrap`
- [PERSIST] `load-restores-run-state` — **Restore the run state a loaded world reports, or reset it explicitly.** After a load the per-cause deaths no longer sum to the total, the first history sample spikes by the whole saved run, the diet-band summary divides by absolute ticks, infections and symbiosis links vanish, and early extinctions go unchronicled.
  - Starting point: add the remaining `SimStats` counters and infection state to the save with `#[serde(default)]`; seed `PopulationHistory`'s `prev_*` from the loaded stats; set phylogeny `current_population` at load; start the diet-band and geography timelines from the first snapshot's tick. Add one test that round-trips a populated world through `save_world` and `load_world`.
  - Source: review/2026-10-09-0906-full.md, 2026-10-09
  - Findings: `load-restores-partial-stats`, `diet-band-summary-assumes-tick-zero`, `restored-phylo-misses-first-extinctions`, `save-drops-infection-and-symbiosis`, `save-round-trip-tests-partial`
  - Related: `infection-state-not-saved` (TODO), `headless-save-ticks-past-summary` (TODO)
- [TOOLING] `screenshot-tour-egui-path` — **Make `--screenshot` use the egui-aware capture path and wait for its last image.** The legacy tour exits before the sixth capture is written and its images have no header, panel, or minimap, so the baseline check in the code review guide proves only that a window opens.
  - Starting point: Drive `--screenshot` through `clauvolution_render::begin_screenshot` (or load a bundled tour JSON) and gate `AppExit` on `ScreenshotState.pending` being clear. Validate: `cargo run --release -- --screenshot` writes six PNGs and each shows the side panel.
  - Source: review/2026-09-17-0756-full.md, 2026-09-17
  - Findings: `screenshot-tour-drops-last-capture`

## P3 Low

- [TOOLING] `headless-cli-edges` — **Bound `--speed` and script values, fail on a failed history dump, and reject flags the chosen mode ignores.** `--speed 1e30` panics, a bad tour value panics the GUI, a failed `--dump-history` exits 0, and `--save-as` in the GUI or `--script` headless silently do nothing.
  - Starting point: `cli.rs` ranges and `check`, `script.rs` `load_script`, `headless_tick_counter` in `main.rs`.
  - Source: review/2026-10-09-0906-full.md, 2026-10-09
  - Findings: `speed-and-script-values-unbounded`, `dump-history-failure-exits-zero`, `mode-only-flags-silently-ignored`
  - Related: `headless-save-ticks-past-summary` (TODO), `script-tour-virtual-time-after-speed` (TODO)
- [PERSIST] `chronicle-bounds-and-order` — **Make chronicle and phylogeny output deterministic and bound their per-frame cost.** Extinction lines and saved nodes follow std `HashMap` order, so same-seed runs differ in `chronicle.log`, and the Chronicle and Phylo tabs re-lay out every entry ever recorded each frame.
  - Starting point: sort `previously_living` by species id in `species_classification_system` and the saved nodes in `save_world` (convergence results already have stable strategy order); egui `show_rows` in `chronicle_tab`; a top-ten selection in `phylo_tab`.
  - Source: review/2026-10-09-0906-full.md, 2026-10-09
  - Findings: `phylo-hashmap-order-reaches-output`, `chronicle-entries-unbounded`, `phylo-tab-scans-all-nodes-per-frame`
- [DOCS] `code-comment-drift-sweep` — **Correct the stale code comments, the brain I/O legend, the eat threshold, and the `--help` field names.** About twenty comments and help strings describe values or behaviour that changed, which misleads the next reader of the tuning code.
  - Starting point: the site list in `code-comments-stale`; add an `EAT_INTENT_THRESHOLD` constant beside the attack and reproduce thresholds. Comment and string changes only, except the constant.
  - Source: review/2026-10-09-0906-full.md, 2026-10-09
  - Findings: `code-comments-stale`, `genome-io-legend-drift`, `eat-threshold-comment-mismatch`, `cli-help-field-names`
- [TOOLING] `dead-code-sweep` — **Remove unused items, fields, dependencies, and leftovers across the crates.** Dead phylogeny helpers, empty plugins, an unread UI field, orphaned derives, an empty loop, and unused crate dependencies make the code look larger and more connected than it is.
  - Starting point: the site lists in the findings. No behaviour change; `scripts/check.sh` and a same-seed headless summary diff are the validation.
  - Source: review/2026-10-09-0906-full.md, 2026-10-09
  - Findings: `dead-code-across-crates`, `dangling-derive-component`, `classification-dead-loop`, `spatial-hash-lazy-cell-size`, `unused-crate-dependencies`

## Unprioritized

- [BRAIN] `reconcile-total-species-extinction` — **Reconcile species populations when no organisms remain.** The empty-population return leaves the last species count and phylogeny populations alive forever and omits final extinction entries.
  - Starting point: `species_classification_system` in `crates/clauvolution_sim/src/lib.rs:3122`; let the empty-count path zero populations and emit final extinctions. Validate with a previously living species of population 20, no organisms and 151 actual headless ticks: currently `species_count=1`, population 20 and no extinction tick; expect zero counts and an extinction tick/log entry.
  - Source: review/2026-10-24-0257-full.md, 2026-10-24
  - Findings: `empty-world-skips-species-extinction`
- [PERSIST] `unique-creature-export-paths` — **Give each exported organism a path that cannot overwrite another creature at the same tick.** Two named members of one species currently share the species-and-tick filename, so exporting both while paused silently replaces the first genome.
  - Starting point: `creature_file_name`, `export_creature` and `write_atomically` in `crates/clauvolution_sim/src/save.rs`; include organism identity or allocate a non-colliding path. Validate by pausing, exporting two distinct same-species organisms, then importing both files and confirming both genomes survived. Preserve clear success/error reporting in Inspect.
  - Source: review/2026-10-24-0257-full.md, 2026-10-24
  - Findings: `creature-export-same-tick-collision`
- [SIM] `account-event-lifetime-statistics` — **Include asteroid and volcano victims in lifetime and diet-band death statistics.** Direct event despawns already book global causes and energy, but bypass the counters and lifespan tracking in `death_system`.
  - Starting point: share the non-energy accounting between `mass_extinction_input_system` and `death_system` without charging deaths twice. Validate a volcano kill of an age-40 founder hunter: currently global/Event deaths are 1 but diet-band and founder-hunter deaths are 0 with no lifespan sample; expect each relevant counter once and an age-40 sample.
  - Source: review/2026-10-24-0257-full.md, 2026-10-24
  - Findings: `event-deaths-skip-lifetime-instruments`
- [UI] `reveal-collapsed-phylo-lineage` — **Open a collapsed living lineage when a Chronicle species link targets it.** The one-shot reveal flag is consumed even when the collapsed header suppresses the highlighted row and its scroll request.
  - Starting point: `phylo_tab` in `crates/clauvolution_ui/src/lib.rs:568`; use the reveal flag to force open the lineage containing the target, as the recently-extinct section does. Validate in a release build: collapse a living lineage, click its species link in Chronicle, and confirm Phylo opens that lineage and reveals the highlighted row.
  - Source: review/2026-10-24-0257-full.md, 2026-10-24
  - Findings: `chronicle-link-does-not-open-lineage`
