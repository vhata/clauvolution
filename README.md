# Clauvolution

An evolution simulator where life emerges, adapts, competes, and speciates — all without programmer-designed species or behaviours.

Watch organisms evolve in real time. Each creature has its own neural network brain, inherited from its parents with mutations. Natural selection does the rest. Predators evolve claws. Prey evolves armor. Plants terraform the landscape. And sometimes an asteroid wipes the slate clean.

## What's happening

- **Per-organism neural networks** — every creature has a tiny NEAT brain (5-50 nodes) that senses its environment and decides what to do. Brains have recurrent memory slots, enabling learning-like behaviour. No behaviours are designed; they emerge from selection pressure.
- **Predation** — organisms can attack and eat each other. Combat depends on claw power vs armor, with size advantage. Arms races emerge naturally.
- **Emergent speciation** — populations diverge into 100+ distinct species, driven by geographic isolation and different selection pressures. Each species gets a unique colour.
- **No hard categories** — there are no predefined "types." Whether something photosynthesizes, hunts, or grows armor is entirely evolved. An organism's strategy emerges from its genome.
- **Niche construction** — photosynthesizers increase vegetation and moisture on their tiles. All organisms add nutrients. The environment and its inhabitants co-evolve.
- **Biomes** — procedurally generated terrain with oceans, deserts, grasslands, forests, and rock. Different environments exert different selection pressures. Fins help in water, limbs help on land.
- **Visual phenotypes** — creatures have modular body parts (torso, limbs, fins, eyes, mouth, claws, armor plates, photosynthetic surfaces) that visually express their evolved traits.
- **Mass extinction events** — trigger asteroid impacts, ice ages, or volcanic eruptions and watch life recover and diversify.

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
| **Shift + left-click drag** | Pan camera |
| **Left click** | Select organism (inspect panel) |
| **Space** | Pause / unpause |
| **[** / **]** | Slow down / speed up (0.125x to 16x) |
| **X** | Asteroid impact (kills 70%) |
| **I** | Ice age (halves temperature) |
| **V** | Volcanic eruption (local kill zone + nutrient boost) |

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
- A **brain** — a small evolved neural network with 19 sensory inputs (energy, nearest food/organism direction and distance, terrain type, health, species recognition, memory) and 9 outputs (movement, eat, reproduce, attack, signal, memory)
- A **body plan** — torso plus evolved appendages: limbs, fins, eyes, mouth, claws, armor plates, photosynthetic surfaces. Each affects gameplay (fins reduce water movement cost, eyes boost sensing range, claws increase attack power, etc.)

Every tick:
1. Each organism senses its environment
2. Its brain produces action decisions
3. Actions execute (move, eat food, attack, reproduce)
4. Photosynthesizers gain energy from light
5. Organisms modify their tiles (niche construction)
6. Metabolism drains energy (bigger bodies, faster speed, more body parts = higher cost)
7. Organisms with no energy die
8. Reproduction creates mutated offspring
9. Species are reclassified by genome compatibility

## Architecture

Rust + [Bevy](https://bevyengine.org/) ECS engine. Cargo workspace with one crate per domain:

| Crate | Purpose |
|-------|---------|
| `clauvolution_core` | Shared types, config, simulation speed, species colours |
| `clauvolution_genome` | Genome representation, body segments, mutation, crossover, NEAT genes |
| `clauvolution_brain` | NEAT neural network compilation and evaluation |
| `clauvolution_body` | Phenotype decoding: genome to renderable body plan |
| `clauvolution_world` | Tile-based terrain, biomes, spatial hashing, food spawning |
| `clauvolution_sim` | Simulation tick: sensing, actions, predation, metabolism, reproduction, speciation |
| `clauvolution_render` | Rendering, camera, LOD, organism inspection, terrain display |
| `clauvolution_phylogeny` | Ancestry tracking, species tree *(planned)* |
| `clauvolution_ui` | Data visualisation panels *(planned)* |

## Roadmap

See [TODO.md](TODO.md) for the full vision.

**Implemented:**
- Per-organism NEAT brains with recurrent memory
- Procedural terrain with biome-dependent mechanics
- Body segment evolution (limbs, fins, eyes, claws, armor, photosynthetic surfaces)
- Predation and combat
- Niche construction
- Mass extinction events
- Species classification and colouring
- Click-to-inspect organisms
- Pause/speed controls

**Next:**
- Phylogenetic tree visualisation
- Population graphs and history
- Sexual reproduction
- Social behaviour (cooperation, signalling)
- Convergent evolution detection
- GPU compute for neural net batching
- Save/load simulation state
- WASM+WebGPU browser build

## License

MIT
