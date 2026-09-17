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

## Needs proof of concept

### P0 Critical

### P1 High

### P2 Normal

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

- [PERF] `photosynthesis-density-cache` — **Stop allocating a plant density map every tick.** `photosynthesis_system` builds a new `HashMap<(u32,u32), u32>` 30 times a second and makes two passes over all organisms.
  - Starting point: Reuse a cached resource for the density count instead of allocating per tick. Fine at 2000 organisms, so this matters most as populations grow.
  - Source: docs/ROADMAP.md (Known tech debt), 2026-09-16
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
