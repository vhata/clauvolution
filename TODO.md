# Deferred ideas

Concrete deferred work that does not belong to a roadmap theme belongs here. A request to "grab something from the TODO" means this file only. Vision-serving work that is large enough to belong to a theme lives in the [`roadmap`](docs/ROADMAP.md), and whole-codebase review findings are scheduled separately in the [`review backlog`](review/BACKLOG.md). See the [`backlog workflow guide`](docs/TODO_GUIDE.md) before adding, claiming, moving, or resolving an entry.

## Needs triage

### P0 Critical

### P1 High

### P2 Normal

### P3 Low

- [PERSIST] `save-write-error-handling` — **Handle save write failures without panicking.** `save_system` wraps save writes in `.expect("Failed to write save file")`, so a full disk or a permissions problem takes the whole sim down.
  - Starting point: Decide whether a failed save should surface as a UI warning instead of a panic. Acceptable for a personal tool, so the first question is whether it is worth changing at all.
  - Source: docs/ROADMAP.md (Known tech debt), 2026-09-16
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
  - Related: `document-world-mutation-event-convention`
- [BRAIN] `species-name-collisions` — **Reduce species naming collisions.** Similar traits combined with a similar species ID modulo produce the same name for different species, which makes them hard to tell apart while watching.
  - Starting point: A limitation of the word-list approach rather than a bug, so first decide whether a larger word list, a different hash, or a suffix is worth it.
  - Source: CLAUDE.md (Known rough edges), 2026-09-16
- [BRAIN] `convergent-detection-noise` — **Assess convergent evolution detection cost and noise.** Detection scans every species on each classification tick, and early in a run the results may be noisy enough to be misleading.
  - Starting point: Measure how often it fires in the first few thousand ticks before deciding between a cheaper scan, a warm-up delay, or leaving it alone.
  - Source: CLAUDE.md (Known rough edges), 2026-09-16
- [TOOLING] `script-tour-virtual-time-after-speed` — **Decide whether `--script` tour timings are virtual or wall-clock seconds.** `script_runner_system` keys `at_seconds` to `Time<Virtual>`, so now that GUI speed scales virtual time every trigger after a `set_speed` action fires `multiplier` times sooner in wall time. `tours/readme.json` at 8x evolves for roughly 330 ticks before its screenshots instead of several thousand.
  - Starting point: Either retune the bundled tours or key `at_seconds` to `Time<Real>` and let tours state the speed they want.
  - Source: review/headless-gui-parity branch, 2026-09-17
- [PERSIST] `determinism-claim-recheck` — **Re-verify the "diverges after ~50 ticks" determinism claim.** Two branches each saw consecutive `--headless 1000 --seed 42` runs on `main` come out bit-identical, contradicting `docs/DECISIONS.md` and `docs/ROADMAP.md`; yet with the spatial hash rebuilt inside the fixed tick, two runs of the same command differed widely, and on the corpse-fountain branch one binary run twice at `--headless 5000 --seed 42` ended with 49 predators once and 2 the next time. Same-seed runs are not reproducible at 5000 ticks, so every single-run tuning comparison carries that spread. The 2026-09-18 attractor audit (`docs/audits/`) then ran every seed twice: seeds 7, 99, 314 and 1000 came out bit-identical, seeds 1, 2, 3 and 42 diverged, and the two simultaneous seed-42 probe runs came out bit-identical to each other at 88% plants and 6 predators, a third distinct outcome for the seed after 68% and 91%. Across the whole audit, runs that started at the same moment matched and runs that started at different moments did not: the four seeds that diverged are the four whose first run was in the first batch after launch. Whatever the source is, it is intermittent rather than constant.
  - Starting point: Establish what is deterministic today and what the fixed-tick hash rebuild changed. If same-seed runs are reproducible, the roadmap's integration tests become possible now. Update the docs either way.
  - Source: review/headless-gui-parity and review/spatial-hash-fixed-tick branches, 2026-09-17
- [WORLD] `biome-threshold-retune` — **Revisit the biome thresholds now that moisture spans 0..1.** With `Tile::from_elevation_moisture` still at 0.25 and 0.6, Rock is rare to absent on low-lying seeds and Grassland roughly quadrupled on seed 42. Land fraction also varies about twofold between seeds (172k land tiles on seed 42, 90k on seed 3), which confounds plant-share comparisons.
  - Starting point: Part of the tuning pass after the 8-seed 15k-tick audit is re-run on the merged fixes. The world crate's unit test prints per-biome counts.
  - Source: review/moisture-range-normalisation branch, 2026-09-17
- [UI] `header-show-species-threshold` — **Show the effective species threshold in the UI.** With `--species-threshold` now honoured in GUI mode, the only evidence that it applied is a startup log line.
  - Starting point: A small label in the header or the Phylo tab reading `SimConfig`.
  - Source: review/headless-gui-parity branch, 2026-09-17
- [PERF] `moisture-fix-tick-cost` — **Find out why the moisture fix doubled the per-tick cost.** `cargo run --release -- --headless 300 --seed 42` takes about 9 s at c8fac9d (spatial hash fix only) and about 20 s at 1ab42ab (moisture fix merged); at speed 1 it takes 21 s, so the sim is now CPU-bound below real time during the opening burst. Ticks 30 to 100 cost roughly 200 ms each before settling to about 20 ms.
  - Starting point: A wetter world grows more vegetation and so more food entities, and every food entity is indexed in the spatial hash that every neighbour query walks, so `spatial-hash-organisms-only` is the first thing to try. Measure food counts and organism counts per tick on both commits before changing anything. Also check whether the opening-burst slow phase (present before the moisture fix too, at about 8 s for the first 100 ticks) is the same cause.
  - Source: rebase of review/headless-gui-parity onto main, 2026-09-17
  - Related: `spatial-hash-organisms-only`
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

## Needs proof of concept

### P0 Critical

### P1 High

### P2 Normal

- [SIM] `hunter-emergence` — **Find what lets a hunter level exist.** With canopy light sharing and grazing, plants and grazers persist and cycle on every seed, but hunters (diet at or above +1/3) are gone by tick 600 to 1000 on every seed and every configuration tried: kill share 0.1, 0.5 and 1.0, food items at 0.1 and 0.02. Payoff is not what binds (a share of 1.0 hands over the whole prey); a founding hunter digests 11% or less of plant tissue, so it has no food-item bridge while its random brain learns to chase, and it starves in a few hundred ticks. The interdependence test in `plans/2026-09-19-diet-axis.md` cannot pass until a top level exists.
  - Starting point: three candidates, cheapest first. A `nearest eater` brain input (direction, distance, size of the nearest non-photosynthesiser), the analogue of the existing nearest-food and photo-hint inputs, so a hunter can steer at prey rather than at whatever organism is nearest, which with plants at half the population is usually a plant. Second, the size gate in `predation_system` (attacker above 0.6 of the prey's size) applied to consumer prey, which may exclude small early hunters. Third, time: carnivory in nature emerges from omnivory, so a 15k-tick audit may show late hunters that 5000-tick runs cannot; check the audit before building anything. Whatever is tried, measure hunters per seed at 5000 and 15000 ticks and record it under "Diet axis tuning pass" in `docs/DECISIONS.md`.
  - Source: plant-physics step 4 (resumed diet pass), 2026-09-20
  - Related: `founding-boom-food-regen`, `predation-target-order`
- [SIM] `predation-target-order` — **Attackers strike the first passing neighbour, not the nearest, so grazers kill bystanders.** In the 2026-09-20 phase 1 audit there were no hunters, yet predation deaths ran at 24k to 73k per 15k-tick run, equal to the kill count and about as many as starvation deaths. A grazer whose attack output fires bites the first target in the spatial hash's neighbour list that passes the gates; that list is in bucket order, not distance order, so when a consumer happens to come first the grazer kills it (keeping almost nothing at its animal efficiency, the rest going to digestion) instead of biting the plant it was heading for. One output drives both grazing and killing, and the choice between them is an artefact of hash order.
  - Starting point: sort the passing candidates by distance and strike the nearest, which is the physical reading (you bite what is in front of you) and adds one sort of a short list per attacker. Measure predation deaths and grazes per run before and after on three seeds. Whether grazing and attacking should be separate brain outputs is a larger question for the design doc; do not decide it here.
  - Source: phase 1 audit, 2026-09-20
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

- [SIM] `name-sim-tuning-literals` — **Name the remaining inline simulation literals.** Disease severity clamps are still inline while the rest of the sim tuning constants have been promoted to named consts.
  - Starting point: Follow the existing named-const block at the top of `clauvolution_sim/src/lib.rs`. Low priority because these are not frequently tuned.
  - Source: docs/ROADMAP.md (Known tech debt), 2026-09-16
  - Related: `name-neat-mutation-literals`, `name-view-literals`
- [BRAIN] `name-neat-mutation-literals` — **Name the inline NEAT innovation and mutation thresholds in the genome crate.** They read as magic numbers today.
  - Starting point: Low priority because they are not frequently tuned.
  - Source: docs/ROADMAP.md (Known tech debt), 2026-09-16
  - Related: `name-sim-tuning-literals`
- [RENDER] `name-view-literals` — **Name the inline click radius and frustum margin.** Both are view-side magic numbers left over from the naming pass.
  - Starting point: Low priority because they are not frequently tuned.
  - Source: docs/ROADMAP.md (Known tech debt), 2026-09-16
  - Related: `name-sim-tuning-literals`
- [PERF] `rayon-remaining-systems` — **Parallelise the remaining O(n) simulation systems.** `sensing_and_brain_system`, `metabolism_system`, and the photosynthesis second pass already use `par_iter_mut`, but predation, disease effects, and niche construction still run serially.
  - Starting point: `disease_effects_system` takes `SimRng` and `Commands`, and `niche_construction_system` mutates the shared `TileMap` (several organisms can hit one tile), so each needs restructuring before `par_iter_mut` is safe. Benchmark with `--headless N --speed 10`, which runs FixedUpdate as fast as the CPU allows (about 85 ticks/sec on an M4 Max at 2000 organisms), so wall time does track per-tick cost. The compute pool is capped at 6 workers by default and overridable via `CLAU_WORKERS`.
  - Source: docs/ROADMAP.md (Backlog), 2026-09-16
  - Related: `gpu-brain-compute-shader`, `split-sensing-and-brain-system`
- [PERF] `batch-spatial-hash-queries` — **Cache or batch the per-tick spatial hash queries.** Roughly 2000 radius queries run every tick, each one independent of the others.
  - Starting point: Look for queries that can share a single pass or reuse the previous tick's result.
  - Source: docs/ROADMAP.md (Backlog), 2026-09-16
- [PERSIST] `organism-export-import` — **Export and import a single organism.** Lets an interesting creature be saved and dropped into another sim as a seed population.
  - Starting point: JSON export of one organism's genome plus a `--seed-with creature.json` CLI flag.
  - Source: docs/ROADMAP.md (Backlog), 2026-09-16

### Unprioritized

- [PERF] `reproduction-genome-clone` — **Avoid repeated genome clones when assembling mate candidates.** `reproduction_system` calls `genome.clone()` several times, and genomes are large because they carry neurons, connections, and body segments.
  - Starting point: Pass references through a lookup table instead of cloning.
  - Source: docs/ROADMAP.md (Known tech debt), 2026-09-16
  - Related: `split-reproduction-system`
- [SIM] `sim-config-resource` — **Promote the tuning constants to a `SimConfig` resource.** Constants held in a resource can be changed at runtime, which serves the tuning loop far more directly than recompiling.
  - Starting point: Start from the existing named consts at the top of `clauvolution_sim/src/lib.rs`.
  - Source: docs/ROADMAP.md (Known tech debt), 2026-09-16
  - Related: `sim-config-live-editor`, `name-sim-tuning-literals`
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
  - Related: `reproduction-genome-clone`
- [DOCS] `document-world-mutation-event-convention` — **Document when a world mutation goes through an event.** `mass_extinction_input_system` raises a `WorldEventRequest` while `action_system` spawns food entities directly, and nothing records which is intended.
  - Starting point: The current rule is that only user-triggered mutations become events, which is a reasonable convention. It just needs writing down in `docs/ARCHITECTURE.md`.
  - Source: docs/ROADMAP.md (Known tech debt), 2026-09-16
  - Related: `system-ordering-convention`
- [PERSIST] `genome-serde-default-consistency` — **Default every genome field for save compatibility.** Only `disease_resistance` carries `#[serde(default)]`, so an older save missing a newer field fails to load.
  - Starting point: Apply `#[serde(default)]` across the genome fields and record the rule so new fields get it too.
  - Source: docs/ROADMAP.md (Known tech debt), 2026-09-16
- [UI] `clickable-chronicle-entries` — **Make chronicle entries clickable.** Turns each chronicle line into a way to reach what it describes instead of a dead label.
  - Starting point: A species entry switches to the Phylo tab and highlights that species; a location entry focuses the camera there. The extinction post-mortem item in roadmap Theme 1 wants the same click target, so check that shape before building.
  - Source: docs/ROADMAP.md (Cool ideas to try), 2026-09-16
- [RENDER] `creature-portrait-v2-polish` — **Polish the creature portrait.** V1 reads the anatomy correctly but looks rough, and the portrait is one of the main places the sim is looked at closely.
  - Starting point: Curved or jointed limbs instead of single line segments, layered fin art with veins or gradients, a subtly shaded torso, an idle breathing animation synced to Age, visibly stacking armor plates for multiple ArmorPlate genes, and proper bilateral-pair alignment along a centre axis rather than jittering on attachment slot offsets. Metaballs and L-systems remain optional future work. See `docs/design/creature-portrait.md`.
  - Source: docs/ROADMAP.md (Cool ideas to try), 2026-09-16
- [RENDER] `shared-segment-mesh-handles` — **Share body part meshes across organisms.** Meshes are built per organism today, which is the main cost behind the LOD roughness at close zoom.
  - Starting point: One shared mesh handle per segment type.
  - Source: CLAUDE.md (Known rough edges), 2026-09-16
  - Related: `gpu-instanced-rendering`
- [PERSIST] `persist-terrain-state` — **Save terrain state instead of regenerating it from the seed.** Niche construction changes to vegetation density, moisture, and nutrients are silently lost on save and load, so a loaded world is not the world that was saved.
  - Starting point: Decide whether to persist the full tilemap or only the fields organisms modify.
  - Source: CLAUDE.md (Known rough edges), 2026-09-16
- [PERF] `spatial-hash-organisms-only` — **Index only organisms in the spatial hash.** `update_spatial_hash` inserts every `Position`, food included, so every neighbour query wades through food entities and discards them through failed `get()` lookups.
  - Starting point: Add a `With<Organism>` filter to the rebuild query after confirming none of the five readers (sensing, predation, disease, symbiosis, reproduction) relies on food being present; none do today.
  - Source: review/spatial-hash-fixed-tick branch, 2026-09-17
- [PERF] `reproduction-linear-scans` — **Replace the linear scans in `reproduction_system`.** `already_mated.contains` and `mate_candidates.iter().find` scan vectors per organism, so mate search is quadratic in population; at 6000 organisms the carrying-capacity experiment measured 2.5x to 3.4x the tick cost of 2000.
  - Starting point: A `HashSet` for `already_mated` and an index for candidates. This is the first performance cost the population-ceiling raise in phase 1 of `docs/design/simulation-rules.md` will hit.
  - Source: roadmap/emergent-carrying-capacity branch, 2026-09-18
  - Related: `split-reproduction-system`, `reproduction-genome-clone`, `rayon-remaining-systems`
