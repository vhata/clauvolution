# Architecture

Clauvolution is a Rust + Bevy 0.15 ECS application organised as a Cargo workspace. The core principle is **separation of simulation from presentation** — the simulation crates never import from rendering or UI, so headless operation is possible with no structural change.

## Crate layout

```
clauvolution_core       ← Shared types, ECS components, resources, events
clauvolution_genome     ← Genome representation, NEAT genes, mutation, crossover
clauvolution_brain      ← Compile genome into a runnable neural network
clauvolution_body       ← Decode genome into a positioned BodyPlan for rendering
clauvolution_world      ← Terrain generation, tile map, spatial hash, food spawning
clauvolution_sim        ← Every simulation system (the "tick")
clauvolution_phylogeny  ← Species ancestry tracking, world chronicle, naming
clauvolution_render     ← World rendering, camera, LOD, minimap, gizmo overlays
clauvolution_ui         ← bevy_egui panels (header + tabbed right panel)
clauvolution_app        ← Binary crate that wires it all together
```

Dependencies flow in one direction: `app → render/ui → sim → world/brain/body → genome → core`. None of the sim crates depend on render or ui, which is how headless mode stays tractable.

## The simulation tick

Systems run in Bevy's standard schedules:

**PreUpdate** (once per frame, before everything)
- `update_input_capture_system` (ui) — reads egui's pointer/keyboard capture state into `UiInputState` so world-view systems can gate themselves

**Update** (once per frame — real-time input, UI, etc.)
- `sim_speed_system` — apply `SimSpeed` to `Time<Virtual>`: pause/unpause, and set the relative speed to the multiplier (both the GUI speed keys and headless `--speed` write `SimSpeed`)
- `keyboard_to_events_system` — translate hotkeys into `WorldEventRequest` events
- `mass_extinction_input_system` — consume `WorldEventRequest` to trigger asteroid/ice/volcano/blooms
- `save_system` — consume `WorldEventRequest::Save` to serialise the world; logs and chronicles the outcome and records it in `SaveReport` for the headless runner
- `export_organism_system` — consume `WorldEventRequest::ExportOrganism(entity)` to write one creature file; outcome goes to the log, the chronicle and `OrganismExportReport` for the Inspect panel
- Rendering-adjacent Update systems: click-select, speed control, toggle minimap/trails, screenshot, LOD change, minimap click
- UI systems (`header_bar_system`, `right_panel_system`) — draw the egui overlays

**FixedUpdate** (strictly chained, always 30Hz of virtual time — this is the simulation tick; speed scales the virtual clock, not the timestep)

```
tick_counter_system           ← advance tick counter, advance season
tile_dynamics_system          ← vegetation grows toward each tile's carrying capacity (defined in world, scheduled here)
food_regeneration_system      ← spawn food toward the seasonal density ceiling (defined in world, scheduled here; consumes SimRng)
update_spatial_hash           ← rebuild the spatial hash from every organism Position; food is not indexed (defined in world, scheduled here)
update_food_snapshot          ← collect (entity, position, energy) of all food into FoodSnapshot
sensing_and_brain_system      ← copy organisms and food into per-tick cell grids, then for each organism: gather inputs, evaluate brain, write outputs (par_iter_mut)
action_system                 ← execute brain outputs (move, eat food items, signal, update memory)
grazing_system                ← `eat` bites the nearest living plant in reach (skipping anyone fed on a food item this tick); one bite per plant per tick
predation_system              ← attack intents → size and damage gates → kills of plants or animals (energy pyramid: 10%, digested by tissue); victim gets `Killed(Predation)`; every attacker with a living organism in reach pays the strike cost
photosynthesis_system         ← sun energy for plants, factoring plant density competition (second pass par_iter_mut)
niche_construction_system     ← organisms modify the tiles they occupy
disease_transmission_system   ← background infections + proximity spread
disease_effects_system        ← per-tick drain, direct mortality chance, timer countdown
symbiosis_tracking_system     ← nearest-neighbour streak per organism (par_iter_mut)
symbiosis_transfer_system     ← energy exchange between mutual pairs past the link threshold
metabolism_system             ← energy costs (quadratic in body/armor/claws/speed), aging (par_iter_mut)
death_marker_expiry_system    ← count each `DeathMarker` down one timestep, despawn expired ones (in the sim so headless runs expire them too; render only animates)
death_system                  ← energy ≤ 0, health ≤ 0, or `Killed` → cause from the marker, else old age/disease/starvation → despawn; folds the dying organism's `EnergyFlows` and remaining energy into the ledger
reproduction_system           ← eligible parents → crossover + mutate → spawn child
ledger_system                 ← close the energy books: sum live energy and per-organism `EnergyFlows`, compare against the tick's recorded flows, record the residual (debug_assert / rate-limited chronicle warning past tolerance)
species_classification_system ← NEAT compatibility distance with hysteresis (every 5s)
record_population_history     ← 1Hz snapshot into PopulationHistory ring buffer
record_trail_history          ← organism position samples (when trails enabled)
```

Sensing, grazing, predation, disease transmission and symbiosis tracking read neighbours through a `CellGrid` each (sim crate), filled from the spatial hash at the start of the system with only the neighbours and fields that system uses, and walked in `SpatialHash::query_radius` order so that ties resolve as they did; see "Neighbour queries after sensing read per-system grids" in `DECISIONS.md`. Mate search in `reproduction_system` still calls `query_radius`.

The chain is the `SimTick` system set (exported by `clauvolution_sim`). Everything else in `FixedUpdate` orders itself `.after(SimTick)`: `update_body_plans` (body crate, registered by the app; inserting `BodyPlan` moves an organism to a new archetype, so it must happen at a fixed point in the tick, not once per frame) and, headless only, `headless_tick_counter`. Nothing that touches simulation state runs per frame, which is what lets a headless run be a function of the seed alone; see "Headless runs are deterministic" in `DECISIONS.md`.

**PostUpdate** (once per frame, after logic — rendering only)

```
spawn_terrain_sprites         ← one-time: generate chunked terrain meshes
sync_organism_transforms      ← position, scale, LOD, frustum cull
sync_selection_ring           ← one ring on SelectedOrganism, whatever set it
sync_food_transforms          ← position; hidden at far zoom
update_death_markers          ← fade/despawn death flash entities
draw_trails_system            ← gizmos linestrips for visible organisms (if trails on)
draw_infection_indicators_system ← pulsing purple halo around infected organisms
camera_control_system         ← pan/zoom/drag
update_minimap                ← repaint the minimap image every 0.5s
```

## Key ECS patterns

- **Component presence as state.** `Infection` is a component with severity and timer; organisms without it are healthy. Avoids a nullable field and makes `Query<..., With<Infection>>` the natural way to find the sick.
- **Unified event channel.** `WorldEventRequest` (in `core`) is fired by keyboard *and* UI buttons; one system consumes it. Avoids keyboard/UI code duplication and keeps triggering symmetrical.
- **World mutations go through an event only when the request originates outside the tick.** `WorldEventRequest` is the sim's only Bevy event, and it exists for mutations the simulation did not decide on itself: a hotkey (`keyboard_to_events_system`), a UI button (`clauvolution_ui`), or the run's driver (the headless runner sends `Save` at the end of a `--save-as` run). Its emitters and both consumers (`mass_extinction_input_system`, `save_system`) run in `Update`, so a request lands between ticks and never inside the `SimTick` chain. Everything the simulation causes on its own writes state directly with `Commands` and `ResMut` from inside `FixedUpdate`: `food_regeneration_system` spawns food, `action_system` despawns eaten food, `reproduction_system` spawns children, `death_system` despawns the dead and spawns their `DeathMarker`, `death_marker_expiry_system` despawns expired markers, `niche_construction_system` edits tiles. Startup and save-load (`spawn_initial_population`, `spawn_initial_food`, `spawn_saved_*`) also spawn directly; they run once, outside the tick, but are still not requests. Adding a new externally triggered effect (a scheduled catastrophe, a REST endpoint) means adding a `WorldEventRequest` variant and another emitter, not another consumer. Adding a new emergent dynamic means a system in the `SimTick` chain that mutates directly; do not route it through the event. The tradeoff behind the single channel is in "Unified event bus" in `DECISIONS.md`.
- **Shared mesh handles.** `SharedMeshes` resource holds one circle/food-circle/material handles reused across 2000+ organisms instead of creating unique meshes. Cloning one of these handles per sprite bumps a reference count and copies no asset data.
- **Per-organism scratch for parallel systems.** `photosynthesis_system` and `metabolism_system` run under `par_iter_mut` and cannot write the shared `EnergyLedger` resource, so each organism carries an `EnergyFlows` component that the owning iteration writes; `ledger_system` sums and zeroes those records serially. Serial systems write the resource directly. The same rule applies to any future parallel system that moves energy.
- **Spatial hash for neighbour queries.** Rebuilt once per fixed tick at the head of the `FixedUpdate` chain, used by sensing, grazing, predation, disease transmission, symbiosis tracking and mate search. The readers re-check real distance after the lookup, so entities that moved within the tick (after `action_system`) are missed rather than falsely matched.
- **try_despawn everywhere.** `commands.entity(e).try_despawn()` and `.try_despawn_recursive()` avoid B0003 errors when two systems both try to despawn the same entity in one frame.
- **Frustum culling off-screen.** Organisms and food outside the camera viewport get `Visibility::Hidden` — GPU skips them. Margin-padded to prevent pop-in at edges.
- **egui input gating.** `UiInputState` resource tracks whether egui is capturing mouse/keyboard; the camera and click-select systems skip their handlers when true, so scrolling a panel doesn't also zoom the world.

## Where to find X

| Looking for… | Look in… |
|---|---|
| Organism behaviour / brain | `clauvolution_sim::sensing_and_brain_system`, `clauvolution_brain` |
| Genetic system (mutation, crossover, speciation) | `clauvolution_genome`, `clauvolution_sim::species_classification_system` |
| Strategy labels (plant / grazer / hunter / omnivore) | `clauvolution_phylogeny::classify_strategy`, `Genome::is_photosynthesiser` |
| Body plan decoding / rendering | `clauvolution_body`, `clauvolution_render::segment_mesh` |
| A specific simulation dynamic | `clauvolution_sim::<name>_system` |
| Terrain / biomes / food | `clauvolution_world` |
| Species naming, ancestry, chronicle | `clauvolution_phylogeny` |
| UI panels | `clauvolution_ui::<tab>_tab` functions |
| Camera, minimap, gizmos | `clauvolution_render` |
| Save/load | `clauvolution_sim::save` module |
| Creature export / `--seed-with` import | `CreatureFile` in `clauvolution_sim::save`; founders spawned by `spawn_initial_population` |
| Energy accounting | `EnergyLedger` and `EnergyFlows` in `clauvolution_core`; `clauvolution_sim::ledger_system` |
| Trophic instruments (feeding counts, gate outcomes, diet-band income and deaths) | `PredationStats`, `FeedingCounts`, `GateOutcomes` and `DietBandStats` in `clauvolution_core`; bands from `clauvolution_sim::diet_band` |
| Species distance instruments and innovation re-keying | `SpeciesPassCounts` in `clauvolution_core`, filled by `clauvolution_sim::species_classification_system`; `rekey_genome` / `InnovationTable` and `CompatibilityTerms` in `clauvolution_genome`; offline report `crates/clauvolution_sim/examples/species_keying_report.rs` |
| Command-line flags (known list, `--help`, rejection of unknown flags) | `FLAGS` in `clauvolution_app/src/cli.rs`; values are read in `main` |

## Bevy schedule essentials

- **Time::\<Fixed\>** at 30Hz of virtual time drives the simulation rate in both GUI and headless mode. It is never rescaled; one tick is always 1/30 s of sim time.
- **Time::\<Virtual\>** with a 100ms `max_delta` cap. `SimSpeed::multiplier` sets its relative speed (GUI speed keys, headless `--speed`). When paused we pause virtual time directly (so the accumulator doesn't build up and cause a death spiral on unpause).
- **Incremental release builds** are enabled in `Cargo.toml` (trades slightly larger binary for much faster rebuilds during development).

## Data flow summary

```
Genome (genetic code)
  ↓ Brain::from_genome
Brain (runnable neural net)
  ↓ sensing_and_brain_system
BrainOutput (per-tick decisions)
  ↓ action_system + grazing_system + predation_system + ...
World state updates (position, energy, health)
  ↓ render systems
Pixels on screen
```

Every Bevy component above is the visible boundary between stages; add/remove/modify them and you affect the corresponding stage of the tick.
