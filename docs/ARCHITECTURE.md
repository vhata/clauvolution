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
- `update_spatial_hash` (world) — rebuilds the spatial hash for this frame's neighbour queries

**Update** (once per frame — real-time input, UI, etc.)
- `sim_speed_system` — pause/unpause virtual time, scale fixed timestep by speed multiplier
- `keyboard_to_events_system` — translate hotkeys into `WorldEventRequest` events
- `mass_extinction_input_system` — consume `WorldEventRequest` to trigger asteroid/ice/volcano/blooms
- `save_system` — consume `WorldEventRequest::Save` to serialise the world
- Rendering-adjacent Update systems: click-select, speed control, toggle minimap/trails, screenshot, LOD change, minimap click
- UI systems (`header_bar_system`, `right_panel_system`) — draw the egui overlays

**FixedUpdate** (strictly chained, 30Hz × speed multiplier — this is the simulation tick)

```
tick_counter_system           ← advance tick counter, advance season
sensing_and_brain_system      ← for each organism: gather inputs, evaluate brain, write outputs
action_system                 ← execute brain outputs (move, eat, signal, update memory)
predation_system              ← attack intents → damage → kills (energy pyramid: 10%)
photosynthesis_system         ← sun energy for plants, factoring plant density competition
niche_construction_system     ← organisms modify the tiles they occupy
disease_transmission_system   ← background infections + proximity spread
disease_effects_system        ← per-tick drain, direct mortality chance, timer countdown
metabolism_system             ← energy costs (quadratic in body/armor/claws/speed), aging
death_system                  ← energy ≤ 0 → categorise cause → despawn
reproduction_system           ← eligible parents → crossover + mutate → spawn child
species_classification_system ← NEAT compatibility distance with hysteresis (every 5s)
record_population_history     ← 1Hz snapshot into PopulationHistory ring buffer
record_trail_history          ← organism position samples (when trails enabled)
```

Plus in FixedUpdate separately: `food_regeneration_system` and `tile_dynamics_system` (vegetation growth).

**PostUpdate** (once per frame, after logic — rendering only)

```
spawn_terrain_sprites         ← one-time: generate chunked terrain meshes
sync_organism_transforms      ← position, scale, LOD, frustum cull
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
- **Shared mesh handles.** `SharedMeshes` resource holds one circle/food-circle/material handles reused across 2000+ organisms instead of creating unique meshes.
- **Spatial hash for neighbour queries.** Rebuilt once per frame in `PreUpdate`, used by sensing and disease transmission.
- **try_despawn everywhere.** `commands.entity(e).try_despawn()` and `.try_despawn_recursive()` avoid B0003 errors when two systems both try to despawn the same entity in one frame.
- **Frustum culling off-screen.** Organisms and food outside the camera viewport get `Visibility::Hidden` — GPU skips them. Margin-padded to prevent pop-in at edges.
- **egui input gating.** `UiInputState` resource tracks whether egui is capturing mouse/keyboard; the camera and click-select systems skip their handlers when true, so scrolling a panel doesn't also zoom the world.

## Where to find X

| Looking for… | Look in… |
|---|---|
| Organism behaviour / brain | `clauvolution_sim::sensing_and_brain_system`, `clauvolution_brain` |
| Genetic system (mutation, crossover, speciation) | `clauvolution_genome`, `clauvolution_sim::species_classification_system` |
| Body plan decoding / rendering | `clauvolution_body`, `clauvolution_render::segment_mesh` |
| A specific simulation dynamic | `clauvolution_sim::<name>_system` |
| Terrain / biomes / food | `clauvolution_world` |
| Species naming, ancestry, chronicle | `clauvolution_phylogeny` |
| UI panels | `clauvolution_ui::<tab>_tab` functions |
| Camera, minimap, gizmos | `clauvolution_render` |
| Save/load | `clauvolution_sim::save` module |

## Bevy schedule essentials

- **Time::\<Fixed\>** at 30Hz drives simulation rate. Scaled by `SimSpeed::multiplier`.
- **Time::\<Virtual\>** with a 100ms `max_delta` cap. When paused we pause virtual time directly (so the accumulator doesn't build up and cause a death spiral on unpause).
- **Incremental release builds** are enabled in `Cargo.toml` (trades slightly larger binary for much faster rebuilds during development).

## Data flow summary

```
Genome (genetic code)
  ↓ Brain::from_genome
Brain (runnable neural net)
  ↓ sensing_and_brain_system
BrainOutput (per-tick decisions)
  ↓ action_system + predation_system + ...
World state updates (position, energy, health)
  ↓ render systems
Pixels on screen
```

Every Bevy component above is the visible boundary between stages; add/remove/modify them and you affect the corresponding stage of the tick.
