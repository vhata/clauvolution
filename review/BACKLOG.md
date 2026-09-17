# Review backlog

Work promoted from whole-codebase reviews because it merits its own branch belongs here. A request to “grab something from the review backlog” means this file; a request to “grab something from the TODO” or “grab something from the roadmap” does not. Every entry is ready for separate work and is prioritized directly rather than grouped by workflow stage.

See the [`code review guide`](../docs/CODE_REVIEW_GUIDE.md) for how findings enter and leave this backlog and the [`backlog workflow guide`](../docs/TODO_GUIDE.md) before claiming or resolving an entry.

## P0 Critical

## P1 High

- [WORLD] `moisture-range-normalisation` — **Normalise the moisture map to 0..1 so biomes and vegetation capacity get the range they assume.** The noise map is rescaled to −1..1, so half the land has negative moisture, a negative vegetation carrying capacity, and no regrowth; Forest is confined to the top fifth of the range. A candidate cause of the green-world attractor.
  - Starting point: Rescale moisture (not elevation) to 0..1 in `TileMap::generate` in `clauvolution_world`, then re-run the 8-seed 15k-tick headless audit from `docs/ROADMAP.md` and compare plant share.
  - Source: review/2026-09-17-0756-full.md, 2026-09-17
  - Findings: `moisture-range-mismatch`

## P2 Normal

- [SIM] `headless-gui-parity` — **Make the GUI speed control and the species-threshold flag behave the same in both modes.** The GUI rescales the fixed timestep, which shrinks every virtual-time timer's delta, while headless scales virtual time; and `--species-threshold` is silently dropped in GUI mode.
  - Starting point: In `sim_speed_system`, set `Time<Virtual>` relative speed instead of `set_timestep_hz`, matching `set_headless_speed`; apply `apply_species_threshold` in both startup chains in `clauvolution_app`.
  - Source: review/2026-09-17-0756-full.md, 2026-09-17
  - Findings: `gui-speed-rescales-timers`, `species-threshold-flag-gui-ignored`
- [SIM] `sim-accounting-fixes` — **Fix the five small stat and energy accounting defects in the simulation.** The header Pop readout is frozen at the initial population, species count includes empty representatives, small parents mint energy on reproduction, one victim can pay several attackers, and event deaths never reach the per-cause totals.
  - Starting point: All in `clauvolution_sim/src/lib.rs` (`predation_system`, `reproduction_system`, `species_classification_system`, `mass_extinction_input_system`) plus the `total_organisms` writes in `clauvolution_app`. Validate with `--headless 1000 --seed 42`: "Total organisms (final)" must equal the strategy sum, and kills must not exceed predation deaths.
  - Source: review/2026-09-17-0756-full.md, 2026-09-17
  - Findings: `frozen-total-organisms-stat`, `reproduction-child-energy-mint`, `multi-attacker-kill-duplication`, `event-deaths-bypass-cause-totals`, `species-count-includes-empty-reps`
- [BRAIN] `convergence-chronicle-dedupe` — **Stop re-logging the same convergent-evolution entry every classification pass.** The dedupe substring omits the word "independent" that the logged text contains, so it never matches.
  - Starting point: Track the highest lineage count logged per strategy in a small map instead of scanning chronicle text. Validate by grepping `chronicle.log` after a headless run with `--save-as`: no repeated convergence lines.
  - Source: review/2026-09-17-0756-full.md, 2026-09-17
  - Findings: `convergence-chronicle-spam`
- [WORLD] `volcano-full-world-range` — **Let the volcano strike anywhere in the world.** Its centre is rolled in 0..256 on a 512×512 map.
  - Starting point: Use `config.world_width`/`world_height` in `mass_extinction_input_system`, as nutrient rain already does. Validate in the GUI: press V several times with the minimap open and confirm kill zones appear in all quadrants.
  - Source: review/2026-09-17-0756-full.md, 2026-09-17
  - Findings: `volcano-lower-left-quadrant`
- [TOOLING] `screenshot-tour-egui-path` — **Make `--screenshot` use the egui-aware capture path and wait for its last image.** The legacy tour exits before the sixth capture is written and its images have no header, panel, or minimap, so the baseline check in the code review guide proves only that a window opens.
  - Starting point: Drive `--screenshot` through `clauvolution_render::begin_screenshot` (or load a bundled tour JSON) and gate `AppExit` on `ScreenshotState.pending` being clear. Validate: `cargo run --release -- --screenshot` writes six PNGs and each shows the side panel.
  - Source: review/2026-09-17-0756-full.md, 2026-09-17
  - Findings: `screenshot-tour-drops-last-capture`

## P3 Low

- [RENDER] `render-input-viewport-fixes` — **Fix the render-side input and viewport defects together.** Culling and the minimap rectangle assume a 1920×1080 window, the selection ring only appears on mouse click and outlives its organism, detailed-LOD bodies are scaled by body size twice, some hotkeys ignore egui keyboard focus, and Shift checks are inconsistent.
  - Starting point: All in `clauvolution_render/src/lib.rs` (`sync_organism_transforms`, `click_select_system`, `speed_control_system`, `toggle_minimap_mode_system`, `camera_control_system`, `update_minimap`) plus `keyboard_to_events_system` in `clauvolution_sim`. Validate in the GUI at a non-default window size: no edge pop-in, ring follows R and `,`/`.` selections, close-zoom body sizes match the circle LOD.
  - Source: review/2026-09-17-0756-full.md, 2026-09-17
  - Findings: `hardcoded-half-viewport`, `selection-ring-only-on-click`, `detailed-lod-double-scale`, `hotkeys-ignore-egui-keyboard-focus`, `shift-modifier-checks-inconsistent`

## Unprioritized
