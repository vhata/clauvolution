# Clauvolution

An evolution simulator where life emerges, adapts, competes, and speciates — all without programmer-designed species or behaviours.

Watch organisms evolve in real time. Each creature has its own neural network brain, inherited from its parents with mutations. Natural selection does the rest.

## What's happening

- **Per-organism neural networks** — every creature has a tiny NEAT brain that senses its environment and decides what to do. No behaviours are designed; they emerge from selection pressure.
- **Emergent speciation** — populations diverge into distinct species over time, driven by geographic isolation and different selection pressures.
- **No hard categories** — there are no predefined "types." Whether something photosynthesizes, hunts, or parasitizes is entirely evolved.
- **Niche construction** — organisms reshape their environment, which reshapes selection pressures in return.
- **Visual phenotypes** — creatures visually express their evolved traits. You see the evolution, not just stats about it.

## Running

```bash
cargo run --release
```

### Controls

- **WASD / Arrow keys** — pan camera
- **Scroll wheel** — zoom in/out

### Requirements

- Rust (latest stable)
- macOS, Linux, or Windows (Metal / Vulkan / DX12)

## Architecture

Rust + [Bevy](https://bevyengine.org/) ECS engine. The project is a Cargo workspace with one crate per domain:

| Crate | Purpose |
|-------|---------|
| `clauvolution_core` | Shared types and configuration |
| `clauvolution_genome` | Genome representation, mutation, crossover |
| `clauvolution_brain` | NEAT neural network implementation |
| `clauvolution_world` | Biome tiles, resources, spatial hashing |
| `clauvolution_sim` | Simulation tick: sensing, actions, metabolism |
| `clauvolution_render` | Rendering, camera, LOD |
| `clauvolution_body` | Phenotype decoding (Phase 2) |
| `clauvolution_phylogeny` | Ancestry tracking, species tree (Phase 3) |
| `clauvolution_ui` | Data visualisation panels (Phase 4) |

## Roadmap

See [TODO.md](TODO.md) for the full vision. Development is phased:

1. **Minimum Viable Evolution** — organisms move, eat, reproduce, evolve *(current)*
2. **Bodies and Biomes** — visible phenotypes, varied terrain, photosynthesis
3. **Predation and Competition** — arms races, niche construction, phylogenetic tree
4. **Social Behaviour** — cooperation, mass extinctions, full UI
5. **Scale and Accessibility** — GPU compute, browser build, save/load

## License

MIT
