# Deferred ideas

Concrete deferred work that does not belong to a roadmap theme belongs here. A request to "grab something from the TODO" means this file only. Vision-serving work that is large enough to belong to a theme lives in the [`roadmap`](docs/ROADMAP.md), and whole-codebase review findings are scheduled separately in the [`review backlog`](review/BACKLOG.md). See the [`backlog workflow guide`](docs/TODO_GUIDE.md) before adding, claiming, moving, or resolving an entry.

## Needs triage

### P0 Critical

### P1 High

### P2 Normal

### P3 Low

- [TOOLING] `wasm-webgpu-build` — **Build for WASM and WebGPU so the sim runs in a browser.** Removes the install step for anyone who wants to look at a running world.
  - Starting point: Only matters if the sim is ever shared, and it needs performance work first, so confirm the goal before starting.
  - Source: docs/ROADMAP.md (Backlog), 2026-09-16
  - Related: `gpu-brain-compute-shader`, `gpu-instanced-rendering`
- [TOOLING] `ci-scripted-tour-software-renderer` — **Try running the scripted tour in CI under a software renderer.** The smoke job exercises headless mode only, so a change that breaks the window, egui panels, or screenshot path is caught by nobody until a human runs a release build.
  - Starting point: Mesa's `llvmpipe` (`LIBGL_ALWAYS_SOFTWARE=1` or Vulkan `lavapipe`) on `ubuntu-latest`, running `--script tours/demo.json` and asserting the PNGs were written. Expect flakiness; `docs/QUALITY.md` lists it as out of scope for the first CI pass.
  - Source: tooling/quality-gates branch, 2026-09-18
  - Related: `screenshot-tour-egui-path` (review backlog)

### Unprioritized

- [RENDER] `render-handle-clone-clarity` — **Clarify frequent mesh and material handle cloning in the render systems.** The clones are cheap because handles are Arc-like, but the pattern reads as expensive and obscures that.
  - Starting point: Decide whether a comment, a helper, or no change at all is the right answer before touching code.
  - Source: docs/ROADMAP.md (Known tech debt), 2026-09-16
- [SIM] `system-ordering-convention` — **Decide one convention for simulation system ordering.** Some systems use explicit `.chain()` and others rely on default Bevy ordering within a tuple, so the intent behind any given ordering is unclear.
  - Starting point: Establish which form is deliberate, then apply it consistently and record the choice.
  - Source: docs/ROADMAP.md (Known tech debt), 2026-09-16
- [TOOLING] `script-tour-virtual-time-after-speed` — **Decide whether `--script` tour timings are virtual or wall-clock seconds.** `script_runner_system` keys `at_seconds` to `Time<Virtual>`, so now that GUI speed scales virtual time every trigger after a `set_speed` action fires `multiplier` times sooner in wall time. `tours/readme.json` at 8x evolves for roughly 330 ticks before its screenshots instead of several thousand.
  - Starting point: Either retune the bundled tours or key `at_seconds` to `Time<Real>` and let tours state the speed they want.
  - Source: review/headless-gui-parity branch, 2026-09-17
- [PERF] `headless-death-marker-leak` — **Despawn `DeathMarker` entities in headless mode.** `death_system` spawns a `DeathMarker` with a `Position` for every death, and only the render crate's system ticks and despawns them, so a headless run keeps every marker for the rest of the run and `update_spatial_hash` indexes all of them; every neighbour query then walks dead markers that the readers have to filter out.
  - Starting point: Either tick the marker timers in a sim-side system that runs in both modes, or skip spawning markers when the render plugin is absent. Measure the spatial hash cell sizes at 5000 ticks before and after.
  - Source: todo/determinism-claim-recheck branch, 2026-09-20
- [WORLD] `biome-threshold-retune` — **Revisit the biome thresholds now that moisture spans 0..1.** With `Tile::from_elevation_moisture` still at 0.25 and 0.6, Rock is rare to absent on low-lying seeds and Grassland roughly quadrupled on seed 42. Land fraction also varies about twofold between seeds (172k land tiles on seed 42, 90k on seed 3), which confounds plant-share comparisons.
  - Starting point: Part of the tuning pass after the 8-seed 15k-tick audit is re-run on the merged fixes. The world crate's unit test prints per-biome counts.
  - Source: review/moisture-range-normalisation branch, 2026-09-17
- [PERF] `moisture-fix-tick-cost` — **Find out why the moisture fix doubled the per-tick cost.** `cargo run --release -- --headless 300 --seed 42` takes about 9 s at c8fac9d (spatial hash fix only) and about 20 s at 1ab42ab (moisture fix merged); at speed 1 it takes 21 s, so the sim is now CPU-bound below real time during the opening burst. Ticks 30 to 100 cost roughly 200 ms each before settling to about 20 ms.
  - Starting point: A wetter world grows more vegetation and so more food entities. Every food entity used to be indexed in the spatial hash that every neighbour query walks; that is fixed (the hash indexes organisms only), so remeasure on the merged commit before looking further. Measure food counts and organism counts per tick on both commits before changing anything. Also check whether the opening-burst slow phase (present before the moisture fix too, at about 8 s for the first 100 ticks) is the same cause.
  - Source: rebase of review/headless-gui-parity onto main, 2026-09-17
- [SIM] `energy-clamp-waste` — **Decide what happens to income above `max_organism_energy`.** The energy ledger shows the 120-energy clamp destroying roughly as much energy as foragers eat: 7.40M destroyed against 7.62M eaten over 5000 ticks on seed 42, 6.15M against 2.86M on seed 1, where photosynthesis is the main income. A forager near the cap that eats a 25-energy item keeps almost none of it.
  - Starting point: Options are a higher or body-scaled cap, letting excess raise reproduction readiness instead of vanishing, or accepting the loss as satiety and saying so in DECISIONS. Belongs with the phase 1 tuning pass in `docs/design/simulation-rules.md`, where food items shrink to a supplement; the ledger's clamp flow is the measurement.
  - Source: roadmap/energy-ledger branch, 2026-09-18
- [SIM] `founding-boom-food-regen` — **Tame the founding boom at its actual driver, food regeneration.** Per-biome seeding showed that founder energy and the tick-0 food stock only move the opening predator boom; `food_regeneration_system` refills toward `max_food_density` in proportion to the deficit while the population is still flat, and the boom then runs on regenerated food.
  - Starting point: Deferred to phase 1 of `docs/design/simulation-rules.md`, where terrain food items become a supplement and `max_food_density` is what gets turned. The 12-run early-tick table is in the "Per-biome seeding" DECISIONS entry.
  - Source: roadmap/per-biome-seeding branch, 2026-09-18
- [SIM] `move-cost-table-by-tile` — **Deep water is the cheapest terrain to cross, not a barrier.** `TerrainType` has two movement tables, one for land-adapted organisms (deep water 10.0) and one for water-adapted ones (deep water 1.0), but `action_system` picks the table by the tile's type rather than the organism's `aquatic_adaptation`, so everyone standing in deep water pays the aquatic base cost of 1.0, less than sand at 1.5. The "Deep water 10x movement cost" entry in `docs/DECISIONS.md` describes behaviour that never ran, and no audit run has shown geographic isolation.
  - Starting point: Interpolate between the two tables by `aquatic_adaptation`, or select by organism. This is the terrain-aware movement piece of phase 2 in `docs/design/simulation-rules.md`; making oceans a real barrier changes every seed, so it needs the phase 2 tuning pass and the DECISIONS entry corrected at the same time.
  - Source: per-biome seeding review, 2026-09-18
  - Related: `oceans-as-habitat`
- [WORLD] `oceans-as-habitat` — **Decide whether water is a habitat or only a barrier.** Water tiles carry nutrients (0.5 shallow, 0.3 deep) and light (50% and 30%) but zero vegetation, so no food ever grows there and only photosynthesisers can live in water at all; per-biome seeding therefore founds no life in water. Life on Earth began in the oceans, and an ocean rich enough to found life in, with the colonisation of land as something to watch, is at least as interesting as water as a barrier that aquatic specialists later unlock.
  - Starting point: A phase 2 design question in `docs/design/simulation-rules.md`. If water becomes habitat: vegetation or a plankton analogue on water tiles, shallow water among the founding biomes, and the aquatic axis doing real work in metabolism and movement. Decide before the phase 2 terrain work, since it changes what the generator should produce.
  - Source: per-biome seeding review, 2026-09-18
  - Related: `move-cost-table-by-tile`
- [TOOLING] `skip-test-hook-for-docs-only-pushes` — **Skip the pre-push test gate when a push touches no code.** Pushing a plan or a docs change runs the whole workspace test suite, which took 54 seconds for a single markdown file on 2026-09-22.
  - Starting point: `scripts/test.sh` is the pre-push job in `lefthook.yml`. Decide first whether skipping is compatible with the gate policy in `docs/QUALITY.md`; if so, have the script diff the pushed range against the remote and exit early when only `*.md` under `docs/`, `plans/` and `review/` (plus `TODO.md` and `README.md`) changed. CI still runs the full suite either way.
  - Source: pyramid-top plan push, 2026-09-22

## Needs proof of concept

### P0 Critical

### P1 High

### P2 Normal

- [BRAIN] `species-distance-unrelated-ceiling` — **Unrelated brains score at most 1.0 apart, so a drifting organism almost always fits some other species.** Every connection gets a fresh innovation number, so two genomes of separate descent share no genes, and the NEAT terms give them `0.5 × (a + b) / max(a, b)`: exactly 1.0 for equal connection counts and less for unequal ones, against a join threshold of 1.0. On seeds 42, 3 and 7 at 5000 ticks, 93 to 100% of organisms sat within 1.0 of another species' representative on every pass (99 to 100% on average), and of the organisms found past 1.3 from their own representative (657, 1481 and 485 across the passes) only 0, 58 and 1 were more than 1.0 from every other one, so a new species needs the rare member that is past both at once. Seed 42 still forms only 3 species after its founders with the stay-threshold fix.
  - Starting point: Normalising the structural terms by `a + b` instead of `max(a, b)` makes unrelated genomes score 1.0 plus the body term whatever their sizes; tried on 2026-09-23 it founded 353 to 355 species from the 400 founders at tick 151 on seeds 42, 3 and 7, and the runs were several times slower, so it needs the threshold sweep repeated and probably a founding population that shares innovation numbers, or innovation numbers keyed by (from, to) as in NEAT proper. The measurements and the discarded representative variants are in the "Species classification" DECISIONS entry.
  - Source: todo/speciation-stalls-after-founding branch, 2026-09-23
  - Remaining from: `speciation-stalls-after-founding`
- [SIM] `hunter-emergence` — **Find what lets a hunter level exist.** With canopy light sharing and grazing, plants and grazers persist and cycle on every seed, but hunters (diet at or above +1/3) are gone by tick 600 to 1000 on every seed and every configuration tried: kill share 0.1, 0.5 and 1.0, food items at 0.1 and 0.02. Payoff is not what binds (a share of 1.0 hands over the whole prey); a founding hunter digests 11% or less of plant tissue, so it has no food-item bridge while its random brain learns to chase, and it starves in a few hundred ticks. The interdependence test in `plans/2026-09-19-diet-axis.md` cannot pass until a top level exists.
  - Starting point: three candidates, cheapest first. A `nearest eater` brain input (direction, distance, size of the nearest non-photosynthesiser), the analogue of the existing nearest-food and photo-hint inputs, so a hunter can steer at prey rather than at whatever organism is nearest, which with plants at half the population is usually a plant. Second, the size gate in `predation_system` (attacker above 0.6 of the prey's size) applied to consumer prey, which may exclude small early hunters. Third, time: carnivory in nature emerges from omnivory, so a 15k-tick audit may show late hunters that 5000-tick runs cannot; check the audit before building anything. Whatever is tried, measure hunters per seed at 5000 and 15000 ticks and record it under "Diet axis tuning pass" in `docs/DECISIONS.md`.
  - Source: plant-physics step 4 (resumed diet pass), 2026-09-20
  - Related: `founding-boom-food-regen`, `graze-attack-output-split`
- [SIM] `graze-attack-output-split` — **One brain output drives both grazing and killing, so grazers kill each other.** In the 2026-09-20 phase 1 audit there were no hunters, yet predation deaths equalled kills at 24k to 73k per 15k-tick run, about as many as starvation deaths. An attacker whose `attack` output fires strikes a passing neighbour; when that neighbour is a consumer the grazer kills it and keeps almost nothing at its animal efficiency. The first hypothesis, that hash-bucket order picked the wrong target, was tested and disproved in #32: striking the nearest passing neighbour instead did not lower predation deaths and raised starvation, because in a grazer-dense world the nearest passing target is usually another grazer. The kills are not an ordering artefact; they follow from the attack output being the only way to bite a plant.
  - Starting point: a design question for `docs/design/simulation-rules.md`, not a constant. Candidates: a separate `graze` brain output (or letting `eat` near a living plant bite it, with the mouth bonus, so a grazer never needs claws or attack to feed), with `attack` reserved for what it says; or a diet-weighted target preference, which is a programmed behaviour and therefore last. Decide with the `hunter-emergence` question, since a hunter level needs attack to mean attack. Measure predation deaths per run before and after.
  - Source: phase 1 audit and #32, 2026-09-20
  - Step 2 of `plans/2026-09-21-pyramid-top.md` (2026-09-23) did the split: `eat` grazes, `attack` only kills. The feeding reason for the kills is gone, but the kills are not: grazer kills of consumers rose 14% to 24% on seeds 42, 3 and 7 at 5000 ticks. Attack intents fell by four fifths on two seeds, consumer armour relaxed (1.48 to 0.48 on seed 42), and kills per intent rose eightfold. Firing attack is free and a kill removes a competitor for food items, so nothing selects it away. What remains is a question about whether an attack attempt should cost the attacker anything, not about outputs; see "Grazing through eat" in `docs/DECISIONS.md` and `docs/audits/2026-09-23-pyramid-graze-eat/`.
  - Related: `hunter-emergence`

### P3 Low

- [RENDER] `gpu-instanced-rendering` — **Draw all organisms in one instanced draw call.** Each organism currently gets its own `ColorMaterial`, so the draw call count scales with population.
  - Starting point: Pack per-instance data into a single buffer. A feature bitmask per instance lets the shader scale absent parts to zero, which removes entity churn on LOD changes. Prove the bitmask approach on a subset before converting the renderer.
  - Source: docs/ROADMAP.md (Backlog), 2026-09-16
  - Related: `shared-segment-mesh-handles`
- [PERF] `gpu-brain-compute-shader` — **Evaluate every brain in a single compute shader dispatch.** The largest available throughput win, but only worth it at 10k or more organisms.
  - Starting point: Pad all NEAT networks to a uniform maximum size and flatten them into GPU buffers. Confirm the padding cost does not erase the win before committing.
  - Source: docs/ROADMAP.md (Backlog), 2026-09-16
  - Related: `rayon-remaining-systems`

### Unprioritized

## Ready for separate work

### P0 Critical

### P1 High

### P2 Normal

### P3 Low

- [PERF] `rayon-remaining-systems` — **Parallelise the remaining O(n) simulation systems.** `sensing_and_brain_system`, `metabolism_system`, and the photosynthesis second pass already use `par_iter_mut`, but predation, disease effects, and niche construction still run serially.
  - Starting point: `disease_effects_system` takes `SimRng` and `Commands`, and `niche_construction_system` mutates the shared `TileMap` (several organisms can hit one tile), so each needs restructuring before `par_iter_mut` is safe. Benchmark with `--headless N --speed 10`, which runs FixedUpdate as fast as the CPU allows (about 85 ticks/sec on an M4 Max at 2000 organisms), so wall time does track per-tick cost. The compute pool is capped at 6 workers by default and overridable via `CLAU_WORKERS`.
  - Source: docs/ROADMAP.md (Backlog), 2026-09-16
  - Related: `gpu-brain-compute-shader`, `split-sensing-and-brain-system`
- [PERF] `batch-spatial-hash-queries` — **Cache or batch the per-tick spatial hash queries.** Roughly 2000 radius queries run every tick, each one independent of the others.
  - Starting point: Look for queries that can share a single pass or reuse the previous tick's result.
  - Source: docs/ROADMAP.md (Backlog), 2026-09-16

### Unprioritized

- [SIM] `sim-config-resource` — **Promote the tuning constants to a `SimConfig` resource.** Constants held in a resource can be changed at runtime, which serves the tuning loop far more directly than recompiling.
  - Starting point: Start from the existing named consts at the top of `clauvolution_sim/src/lib.rs`.
  - Source: docs/ROADMAP.md (Known tech debt), 2026-09-16
  - Related: `sim-config-live-editor`
- [UI] `sim-config-live-editor` — **Edit simulation tuning parameters live from the UI.** Removes the recompile step from every tuning iteration.
  - Starting point: Needs `sim-config-resource` first. A panel in the right-hand tabs that writes to the resource is enough.
  - Source: docs/ROADMAP.md (Known tech debt), 2026-09-16
  - Related: `sim-config-resource`
- [SIM] `split-sensing-and-brain-system` — **Split the sensing pass from the brain evaluation pass.** `sensing_and_brain_system` runs about 114 lines covering spatial querying, input assembly, social sensing, and brain evaluation in one loop.
  - Starting point: Splitting along those concerns also opens up further Rayon parallelism.
  - Source: docs/ROADMAP.md (Known tech debt), 2026-09-16
  - Related: `rayon-remaining-systems`
- [SIM] `split-reproduction-system` — **Split reproduction into its three concerns.** `reproduction_system` runs about 114 lines mixing mate finding, genome crossover and mutation, and child spawning.
  - Starting point: The three concerns are a natural split boundary.
  - Source: docs/ROADMAP.md (Known tech debt), 2026-09-16
- [RENDER] `creature-portrait-v2-polish` — **Polish the creature portrait.** V1 reads the anatomy correctly but looks rough, and the portrait is one of the main places the sim is looked at closely.
  - Starting point: Curved or jointed limbs instead of single line segments, layered fin art with veins or gradients, a subtly shaded torso, an idle breathing animation synced to Age, visibly stacking armor plates for multiple ArmorPlate genes, and proper bilateral-pair alignment along a centre axis rather than jittering on attachment slot offsets. Metaballs and L-systems remain optional future work. See `docs/design/creature-portrait.md`.
  - Source: docs/ROADMAP.md (Cool ideas to try), 2026-09-16
- [RENDER] `shared-segment-mesh-handles` — **Share body part meshes across organisms.** Meshes are built per organism today, which is the main cost behind the LOD roughness at close zoom.
  - Starting point: One shared mesh handle per segment type.
  - Source: CLAUDE.md (Known rough edges), 2026-09-16
  - Related: `gpu-instanced-rendering`
- [TOOLING] `unknown-flag-launches-gui` — **Refuse unknown CLI flags and answer `--help` instead of opening a window.** The argument parser in `clauvolution_app` looks up the flags it knows and ignores everything else, so a typo or `--help` launches the GUI as if no flags were given.
  - Starting point: Collect the known flag names in one place, print usage and exit non-zero for anything unrecognised, and treat `--help` as usage. The README's flag list is the source for the usage text; keep them from drifting (a test that every README flag is known would do). Found when a headless agent ran `--help` to check usage and got a window.
  - Source: todo/reproduction-linear-scans branch, 2026-09-22
- [SIM] `name-action-predation-reproduction-literals` — **Name the inline literals in `action_system`, `predation_system` and `reproduction_system`.** #39 named the disease and niche literals and deliberately left these three systems alone because grazing and attack were about to be redesigned; they still carry armour drag 0.3, speed 2.0, fin 0.3, limb 0.15, aquatic 0.5, mouth bonus 0.3, eat range 3.0, attack gate 0.5, attack range 4.0, defence 0.5, size gate 0.6, damage gate 0.1, reproduce gate 0.5, mate range 8.0, spawn offset 5.0 and the flash timers.
  - Starting point: Do it as part of step 2 of `plans/2026-09-21-pyramid-top.md`, which touches the mouth bonus, eat range and the predation gates anyway, or immediately after it lands so the names match the new rules. Values unchanged; prove it with identical same-seed CSVs as #39 did.
  - Source: #39 report, 2026-09-22
  - Related: `sim-config-resource`
