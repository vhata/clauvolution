# Clauvolution

An evolution simulator where life emerges, adapts, competes, and speciates — all without programmer-designed species or behaviours.

Watch organisms evolve in real time. Each creature has its own neural network brain, inherited from its parents with mutations. Natural selection does the rest. Predators evolve claws. Prey evolves armor. Plants terraform the landscape. And sometimes an asteroid wipes the slate clean.

## What's happening

- **Per-organism neural networks** — every creature has a tiny NEAT brain (5-50 nodes) that senses its environment and decides what to do. Brains have recurrent memory slots, enabling learning-like behaviour. No behaviours are designed; they emerge from selection pressure.
- **Energy pyramid** — predators only gain 10% of their prey's energy (thermodynamics). This naturally limits predator populations, just like real food chains.
- **Predation** — organisms can attack and eat each other. Combat depends on claw power vs armor, with size advantage. Arms races emerge naturally.
- **Sexual reproduction** — nearby same-species organisms crossover their genomes. Falls back to asexual if no mate is available.
- **Emergent speciation** — populations diverge into distinct species, driven by geographic isolation and different selection pressures. Each species gets a unique colour.
- **Convergent evolution** — the simulation detects when unrelated species independently evolve the same strategy, and logs it.
- **Seasonal cycles** — 60-second year with spring, summer, autumn, winter. Light and food production rise and fall sinusoidally. Summer is abundant; winter is harsh. Organisms must adapt to changing conditions.
- **Chemical signalling** — organisms emit a signal (-1 to 1) that nearby organisms can sense. Evolution decides what it means — could become alarm calls, mating signals, territorial markers, or nothing.
- **Social sensing** — organisms sense how many same-species neighbours are nearby and their average signal. A small metabolic discount (~5%) for grouping creates selection pressure for social behaviour — flocking, herding, or pack hunting can emerge naturally.
- **Geographic isolation** — deep oceans and mountains are nearly impassable barriers. Populations on different landmasses evolve independently, driving real allopatric speciation.
- **Fitness tracking** — average lifespan graphed over time. If organisms are getting better at surviving, this line trends upward — visible proof of evolution.
- **No hard categories** — there are no predefined "types." Whether something photosynthesizes, hunts, or grows armor is entirely evolved.
- **Niche construction** — photosynthesizers increase vegetation and moisture on their tiles. All organisms add nutrients. The environment and its inhabitants co-evolve.
- **Biomes** — procedurally generated terrain with oceans, deserts, grasslands, forests, and rock. Different environments exert different selection pressures.
- **Visual phenotypes** — creatures have modular body parts (torso, limbs, fins, eyes, mouth, claws, armor plates, photosynthetic surfaces) that visually express their evolved traits.
- **Mass extinction events** — trigger asteroid impacts, ice ages, or volcanic eruptions and watch life recover and diversify.
- **Phylogenetic tree** — living family tree showing species ancestry, population, and extinction history.
- **Species naming** — three-word trait-based names (habitat + body descriptor + strategy noun, e.g. "Swamp Dwarf Moss"). Child species inherit their parent's habitat and noun, like real taxonomy.
- **World chronicle** — automatic log of evolutionary events: speciation, extinction, mass extinction, convergent evolution, season changes.
- **Minimap** — world overview in the top-right corner showing terrain, organism positions (colour-coded by strategy), and camera viewport. Click anywhere on it to teleport the camera.

## Running

```bash
cargo run --release
```

### Controls

| Key | Action |
|-----|--------|
| **WASD / Arrows** | Pan camera |
| **Q / E** or **- / +** | Zoom out / in |
| **Scroll wheel** | Zoom |
| **Right-click drag** | Pan camera |
| **Left click** | Select organism (inspect panel) |
| **Space** | Pause / unpause |
| **[** / **]** | Slow down / speed up (0.125x to 16x) |
| **M** | Toggle minimap heatmap (density by strategy) |
| **T** | Toggle organism trails (off by default) |
| **X** | Asteroid impact (kills 70%) |
| **I** | Ice age (halves temperature) |
| **V** | Volcanic eruption (local kill zone + nutrient boost) |
| **B** | Solar bloom (double light for 30s) |
| **N** | Nutrient rain (massive food burst) |
| **J** | Cambrian spark (triple mutation for 30s) |
| **S** | Take screenshot (saved to session directory) |
| **F5** | Save world state |

Graphs, chronicle, phylogenetic tree, events, and help all live in tabs of the right-side panel — click the tab names to switch between views.

### Save/load

Each run gets a unique cosmic name (e.g. "pale-fading-shard"). Session data — chronicle log, screenshots, save files — lives in `sessions/<name>/`.

```bash
# Save: press F5 during gameplay
# Load a saved session:
cargo run --release -- --load sessions/pale-fading-shard
```

### Screenshot mode

Capture verification screenshots automatically:

```bash
cargo run --release -- --screenshot
```

### Requirements

- Rust (latest stable)
- macOS, Linux, or Windows (Metal / Vulkan / DX12)

## How it works

Each organism has:
- A **genome** encoding body segments, metabolic traits (photosynthesis, aquatic adaptation, armor, attack power), and NEAT neural network topology
- A **brain** — a small evolved neural network with 22 sensory inputs (energy, nearest food/organism direction and distance, terrain type, health, species recognition, nearby organism's signal, group size, average group signal, memory) and 9 outputs (movement, eat, reproduce, attack, signal, memory)
- A **body plan** — torso plus evolved appendages: limbs, fins, eyes, mouth, claws, armor plates, photosynthetic surfaces. Each affects gameplay (fins reduce water movement cost, eyes boost sensing range, claws increase attack power, etc.)

Every tick:
1. Each organism senses its environment
2. Its brain produces action decisions
3. Actions execute (move, eat food, attack, reproduce)
4. Photosynthesizers gain energy from light
5. Organisms modify their tiles (niche construction)
6. Metabolism drains energy (quadratic body size cost, armor/claw maintenance)
7. Aging increases metabolism; old organisms eventually die
8. Organisms with no energy die
9. Reproduction creates mutated offspring (sexual if mate nearby, asexual otherwise)
10. Species are reclassified by genome compatibility
11. Convergent evolution detected across unrelated lineages

## Architecture

Rust + [Bevy](https://bevyengine.org/) ECS engine. Cargo workspace with one crate per domain:

| Crate | Purpose |
|-------|---------|
| `clauvolution_core` | Shared types, config, population history, species colours |
| `clauvolution_genome` | Genome representation, body segments, mutation, crossover, NEAT genes |
| `clauvolution_brain` | NEAT neural network compilation and evaluation |
| `clauvolution_body` | Phenotype decoding: genome to renderable body plan |
| `clauvolution_world` | Tile-based terrain, biomes, spatial hashing, food spawning |
| `clauvolution_sim` | Simulation tick: sensing, actions, predation, metabolism, reproduction, speciation |
| `clauvolution_render` | World rendering: terrain, organism sprites, food, death markers, camera, LOD, minimap |
| `clauvolution_phylogeny` | Phylogenetic tree, species ancestry tracking, species naming, world chronicle |
| `clauvolution_ui` | bevy_egui panels: header bar, tabbed right panel (Inspect / Phylo / Graphs / Chronicle / Events / Help) |

## Roadmap

See [TODO.md](TODO.md) for the prioritised backlog.

**Implemented:**
- Per-organism NEAT brains with recurrent memory
- Energy pyramid (10% trophic efficiency)
- Procedural terrain with biome-dependent mechanics
- Body segment evolution (limbs, fins, eyes, claws, armor, photosynthetic surfaces)
- Predation and combat (claws vs armor, size advantage)
- Sexual reproduction with genome crossover
- Organism aging and natural death
- Generation tracking
- Niche construction (organisms terraform tiles)
- Mass extinction events (asteroid, ice age, volcano)
- Species classification and colouring
- Phylogenetic tree with ancestry tracking
- World chronicle (automatic event log)
- Convergent evolution detection
- Chemical signalling between organisms (brain input + output)
- Seasonal cycles (60-second year, affects light and food)
- Geographic barriers (oceans/mountains isolate populations)
- Fitness tracking (average lifespan sparkline)
- Population line graphs via egui_plot (per-strategy breakdown, rates, food, lifespan)
- Click-to-inspect organisms with full stat panel
- Help tab explaining everything
- Action flash (organisms pulse when eating/attacking/reproducing)
- Initial diversity seeding (30% photosynthesizers)
- Dynamic LOD — body parts re-render when zoom level changes
- Species stability — hysteresis prevents flip-flopping between species
- Save/load (F5 to save, --load to restore)
- Named sessions with cosmic names, per-session logs and screenshots
- JetBrains Mono font for Unicode rendering
- Pause/speed controls
- Screenshot verification mode + manual screenshots (S key)
- Seed-based terrain generation (same seed = same map, saved in save files)
- Species naming — three-word trait-based names with taxonomy-like inheritance
- Minimap with click-to-navigate and population heatmap toggle (M key)
- Social sensing (group size + average group signal brain inputs, group metabolic discount)
- Performance: frustum culling, food hidden at far zoom, virtual time cap prevents death spiral at high speeds
- Death markers (red for predation, amber for starvation)
- Parent species tracking (inspect shows lineage)
- Bloom events (solar bloom B, nutrient rain N, Cambrian spark J)
- Plant density competition (photosynthesizers shade each other — caps monoculture)
- bevy_egui UI overhaul: header bar + tabbed right panel (Inspect / Phylo / Graphs / Chronicle / Events / Help) with egui_plot line charts, collapsible phylo tree, event buttons
- Organism trails (T to toggle) — fading lines showing recent movement, frustum culled

**Next:**
- Symbiosis (mutualism, parasitism)
- Performance scaling (Rayon, GPU instancing, GPU compute)
- WASM+WebGPU browser build

## Built with

This project was built collaboratively by [Jonathan Hitchcock](https://github.com/vhata) and [Claude Code](https://claude.ai/claude-code) (Anthropic's Claude Opus 4.6).

## License

MIT
