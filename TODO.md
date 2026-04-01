# Clauvolution

An evolution simulator where you watch life emerge, adapt, compete, and speciate in real time.

## Status

### What's Working (The Good)
- [x] Rust + Bevy ECS workspace — solid architecture, compiles clean, runs smooth
- [x] Per-organism NEAT neural networks with 19 inputs, 9 outputs, recurrent memory
- [x] Organisms sense food, neighbors, terrain, species, health — brains decide everything
- [x] Natural selection running — population finds equilibrium with healthy turnover
- [x] Procedural terrain generation — oceans, deserts, grasslands, forests, rock
- [x] Biome-aware food spawning — more food in fertile areas
- [x] Terrain-dependent movement costs — fins help in water, limbs help on land
- [x] Photosynthesis — organisms with photo surfaces gain energy from light
- [x] Body segment genes — torso, limbs, fins, eyes, mouth, photosynthetic surfaces, claws, armor plates
- [x] Smooth camera controls — scroll zoom, mouse drag pan, keyboard zoom/pan
- [x] Species classification (NEAT compatibility distance) with distinct colours
- [x] Predation — organisms attack and eat each other (claws vs armor, size advantage)
- [x] Sexual reproduction with genome crossover when mates are nearby
- [x] Niche construction — photosynthesizers terraform tiles, all organisms add nutrients
- [x] Mass extinction events — asteroid (X), ice age (I), volcanic eruption (V)
- [x] Brain memory — 3 recurrent slots enabling learning-like behaviour
- [x] Organism aging — increasing metabolism, natural death from old age
- [x] Generation tracking — lineage depth visible per organism and globally
- [x] Click-to-inspect — full stat panel: energy, health, body parts, brain size, strategy label
- [x] Pause/play (Space), speed control ([ / ]) from 0.125x to 16x
- [x] Dark outlines on organisms for terrain contrast
- [x] Predators tinted red, photosynthesizers tinted green
- [x] Population cap to prevent runaway growth
- [x] Screenshot verification mode (--screenshot)

### What Could Be Better (The Bad)
- [ ] **Body part rendering at close zoom** — detailed view exists but LOD threshold is aggressive
- [ ] **No population graphs or history** — just live counts, no sense of trends over time
- [ ] **No phylogenetic tree** — can't see species ancestry or divergence history
- [ ] **Food is just green dots** — no visual difference by biome
- [ ] **No indication of active behaviour** — eating, attacking, reproducing all look the same visually

### What's Next (The Ugly)
- [ ] Phylogenetic tree visualisation — sidebar showing species branching
- [ ] Population graphs — organism count, species count, births/deaths over time
- [ ] Convergent evolution detection — notice when unrelated species evolve similar strategies
- [ ] Social behaviour — cooperation, pack hunting, alarm calls
- [ ] Symbiosis — mutualism, parasitism
- [ ] Better LOD — smooth transition from body parts to dots to heatmaps
- [ ] Save/load simulation state
- [ ] GPU compute for neural net batching at scale
- [ ] WASM+WebGPU browser build

---

## Core Vision

- [x] **Visual simulation** — watch evolution happen in real time
- [x] **Emergent speciation** — 100+ species diverge from common ancestors
- [x] **Competition** — creatures compete for resources, territory, and each other
- [x] **No hard categories** — photosynthesizers, predators, foragers all emerge from evolution
- [x] **Biomes** — different environments with different selection pressures
- [x] **Niche construction** — species reshape biomes, biomes reshape species back
- [x] **Phenotype rendering** — creatures visually express evolved traits
- [x] **Spatial and temporal zoom** — pan/zoom + pause/speed controls

## Brains

- [x] **Per-organism neural network** — NEAT brains, inherited with mutation
- [x] **Emergent behaviour** — movement, feeding, attack, reproduction strategies emerge
- [x] **Recurrent memory** — 3 memory slots for learning-like behaviour
- [ ] **Cognitive speciation** — separated populations diverge cognitively
- [ ] **Sentience spectrum** — communication, deception, play

## Emergent Dynamics

- [ ] **Phylogenetic tree** — a living family tree alongside the simulation
- [x] **Arms races** — predator/prey co-evolution (claws vs armor)
- [x] **Mass extinction events** — asteroid, ice age, volcanic eruption
- [ ] **Convergent evolution** — independent lineages evolving similar solutions
- [ ] **Social behaviour** — pack hunting, herding, hive structures
- [ ] **Symbiosis** — mutualism, parasitism, commensalism
