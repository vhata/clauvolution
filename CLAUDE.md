# Clauvolution — Project Context

## What This Is

An evolution simulator built in Rust + Bevy 0.15. Organisms with NEAT neural network brains evolve in a procedural world. No behaviours are programmed — everything emerges from selection pressure. The user watches evolution happen in real time.

## Architecture

Cargo workspace with 10 crates:

- `clauvolution_core` — Shared types, config, resources (SimConfig, Session, Season, PopulationHistory, FitnessTracker, etc.)
- `clauvolution_genome` — Genome with NEAT neurons/connections, body segments (8 types), mutation, crossover. 22 brain inputs, 9 outputs.
- `clauvolution_brain` — Compiles genome into a neural network, evaluates per tick.
- `clauvolution_body` — Decodes genome into renderable BodyPlan with positioned parts.
- `clauvolution_world` — TileMap (6 terrain types), spatial hashing, food spawning/regen. Terrain generated from seed using value noise.
- `clauvolution_sim` — All simulation systems: sensing, actions, predation, photosynthesis, niche construction, metabolism, death, reproduction, species classification, save/load.
- `clauvolution_render` — World rendering: terrain chunks, organism sprites (circle or detailed body parts based on zoom LOD), food sprites, death markers, camera, minimap.
- `clauvolution_phylogeny` — PhyloTree (species ancestry tracking), WorldChronicle (event log), species naming.
- `clauvolution_app` — Binary entry point, startup systems, screenshot mode.
- `clauvolution_ui` — All UI panels. bevy_egui header bar + tabbed right panel (Inspect / Phylo / Graphs / Chronicle / Events / Help). Uses egui_plot for line charts.

## Key Design Decisions

- **Energy pyramid**: Predators get 10% of prey's stored energy (thermodynamics). Prevents predator meta.
- **Quadratic costs**: Body size, armor, claws, speed all cost quadratically in metabolism. Prevents "stack everything" meta.
- **Species classification**: NEAT compatibility distance with threshold 2.0, hysteresis 1.3x to stay in current species. Runs every 5 seconds.
- **Species naming**: Three-word names from traits (habitat + body descriptor + strategy noun). Children inherit parent's habitat and noun, only change the descriptor.
- **Seasons**: 60-second year cycle. Light and food regen vary sinusoidally. Winter is harsh (40% light, 20% food regen).
- **Geographic barriers**: Deep water 10x movement cost for land organisms. Creates real isolation.
- **Photosynthesizer rendering**: Fully opaque bright yellow-green, z=0.3 (behind active organisms), no outline. Active organisms at z=1.0 with outlines.
- **Dynamic LOD**: When zoom crosses 0.6 threshold, all organism sprites are stripped and re-created at new detail level.
- **Sessions**: Each run gets a cosmic three-word name (e.g. "pale-fading-shard"). Logs, screenshots, saves go in sessions/<name>/.
- **Font**: JetBrains Mono bundled at crates/clauvolution_app/assets/fonts/ for Unicode rendering.

## Common Patterns

- All despawns use `try_despawn()` / `try_despawn_recursive()` to avoid B0003 errors from multi-system despawn races.
- Population graphs use `egui_plot` line charts (4 stacked plots, each independently zoomable).
- Chronicle entries written to both in-memory Vec and disk log file simultaneously.
- Population cap is 2000. Initial pop is 400 (30% seeded as photosynthesizers).
- World events (extinctions, blooms, save) funnel through `WorldEventRequest` — keyboard hotkeys and UI buttons both emit the same event.

## Known Issues / Rough Edges

- **LOD at close zoom**: Body part meshes are created per-organism (not shared). Could be optimized with shared mesh handles per segment type.
- **Terrain not saved in save files**: Terrain regenerated from seed on load. Niche construction changes (vegetation density, moisture, nutrients modified by organisms) are lost on save/load.
- **Species naming collisions**: Similar traits + similar species ID modulo → same name. Not a bug, just a limitation of the word list approach.
- **Convergent evolution detection**: Checks every classification tick by scanning all species. Could be noisy early on.

## What's Implemented

See TODO.md "What's Done" section. Highlights: NEAT brains with memory and social sensing (22 inputs), predation with energy pyramid, sexual reproduction, seasons, geographic barriers, phylogenetic tree, world chronicle, species naming, minimap with click-to-navigate and heatmap toggle, egui UI with tabbed right panel (Inspect / Phylo / Graphs / Chronicle / Events / Help), egui_plot line charts, plant density competition, bloom events, death markers, parent species tracking, save/load.

## What's Next (from TODO.md)

1. Symbiosis
2. Performance scaling (Rayon, GPU instancing, GPU compute)
3. WASM browser build

## Build & Run

```bash
cargo run --release                              # normal
cargo run --release -- --screenshot              # screenshot mode
cargo run --release -- --load sessions/<name>    # load saved session
```

## Controls

Space=pause, [/]=speed, WASD=pan, scroll=zoom, click=inspect, S=screenshot, F5=save, M=heatmap, X=asteroid, I=ice age, V=volcano, B=solar bloom, N=nutrient rain, J=Cambrian spark. Graphs/chronicle/phylo/help are in egui tabs in the right panel.
