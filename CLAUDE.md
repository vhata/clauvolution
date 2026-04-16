# Clauvolution — Project Context

## What This Is

An evolution simulator built in Rust + Bevy 0.15. Organisms with NEAT neural network brains evolve in a procedural world. No behaviours are programmed — everything emerges from selection pressure. The user watches evolution happen in real time.

## Architecture

Cargo workspace with 9 crates:

- `clauvolution_core` — Shared types, config, resources (SimConfig, Session, Season, PopulationHistory, FitnessTracker, etc.)
- `clauvolution_genome` — Genome with NEAT neurons/connections, body segments (8 types), mutation, crossover. 20 brain inputs, 9 outputs.
- `clauvolution_brain` — Compiles genome into a neural network, evaluates per tick.
- `clauvolution_body` — Decodes genome into renderable BodyPlan with positioned parts.
- `clauvolution_world` — TileMap (6 terrain types), spatial hashing, food spawning/regen. Terrain generated from seed using value noise.
- `clauvolution_sim` — All simulation systems: sensing, actions, predation, photosynthesis, niche construction, metabolism, death, reproduction, species classification, save/load.
- `clauvolution_render` — All rendering: terrain chunks, organism sprites (circle or detailed body parts based on zoom LOD), camera, UI text panels (stats, inspect, graphs, phylo tree, chronicle, help), minimap.
- `clauvolution_phylogeny` — PhyloTree (species ancestry tracking), WorldChronicle (event log), species naming.
- `clauvolution_app` — Binary entry point, startup systems, screenshot mode.
- `clauvolution_ui` — Planned but unused.

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
- Sparkline graphs use Unicode block characters (▁▂▃▄▅▆▇█) with min-max normalization for levels, zero-anchored for rates.
- Chronicle entries written to both in-memory Vec and disk log file simultaneously.
- Population cap is 2000. Initial pop is 400 (30% seeded as photosynthesizers).

## Known Issues / Rough Edges

- **Phylo tree display**: Text-based, can't scroll. Species group by shared lineage root (10 generations back) but with high turnover many appear as roots. `bevy_egui` integration would fix this properly.
- **LOD at close zoom**: Body part meshes are created per-organism (not shared). Could be optimized with shared mesh handles per segment type.
- **Terrain not saved in save files**: Terrain regenerated from seed on load. Niche construction changes (vegetation density, moisture, nutrients modified by organisms) are lost on save/load.
- **Species naming collisions**: Similar traits + similar species ID modulo → same name. Not a bug, just a limitation of the word list approach.
- **Convergent evolution detection**: Checks every classification tick by scanning all species. Could be noisy early on.

## What's Implemented

See TODO.md "What's Done" section. Highlights: NEAT brains with memory, predation with energy pyramid, sexual reproduction, seasons, geographic barriers, phylogenetic tree, world chronicle, species naming, minimap, population graphs with per-strategy sparklines, save/load, manual screenshots.

## What's Next (from TODO.md)

1. Proper UI panels (bevy_egui) — scrollable panels
2. Minimap click navigation (done) / Population heatmap toggle
3. Symbiosis
4. GPU compute
5. WASM browser build

## Build & Run

```bash
cargo run --release                              # normal
cargo run --release -- --screenshot              # screenshot mode
cargo run --release -- --load sessions/<name>    # load saved session
```

## Controls

Space=pause, [/]=speed, WASD=pan, scroll=zoom, click=inspect, S=screenshot, F5=save, G=graphs, C=chronicle, H=help, X=asteroid, I=ice age, V=volcano
