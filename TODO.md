# Clauvolution

An evolution simulator where you watch life emerge, adapt, compete, and speciate in real time.

## What's Done

- [x] Rust + Bevy ECS workspace
- [x] Per-organism NEAT neural networks with 19 inputs, 9 outputs, recurrent memory
- [x] Organisms sense food, neighbors, terrain, species, health — brains decide everything
- [x] Procedural terrain — oceans, deserts, grasslands, forests, rock
- [x] Biome-aware food spawning + terrain-dependent movement costs
- [x] Photosynthesis — organisms with photo surfaces gain energy from light
- [x] Body segments — torso, limbs, fins, eyes, mouth, photo surfaces, claws, armor plates
- [x] Predation — attack and eat other organisms (claws vs armor, size advantage)
- [x] Energy pyramid — 10% trophic efficiency (thermodynamics, not tuning)
- [x] Sexual reproduction with genome crossover
- [x] Niche construction — organisms terraform tiles
- [x] Mass extinction events — asteroid (X), ice age (I), volcanic eruption (V)
- [x] Brain memory — 3 recurrent slots
- [x] Organism aging and natural death
- [x] Generation tracking
- [x] Species classification with distinct colours
- [x] Phylogenetic tree with ancestry tracking
- [x] World chronicle — automatic event log (speciation, extinction, convergence)
- [x] Convergent evolution detection
- [x] Population sparkline graphs with per-strategy breakdown
- [x] Strategy breakdown (Plants / Predators / Foragers counts)
- [x] Click-to-inspect with full stat panel
- [x] Help overlay (H key) explaining everything for newcomers
- [x] Action flash — organisms pulse when eating/attacking/reproducing
- [x] Initial diversity seeding — 30% photosynthesizers at start
- [x] Chemical signalling between organisms (brain input + output)
- [x] Seasonal cycles — 60-second year, affects light and food production
- [x] Geographic barriers — oceans/mountains isolate populations for speciation
- [x] Fitness tracking — average lifespan sparkline proves evolution is happening
- [x] Photosynthesizers render as ground cover (behind active organisms)
- [x] Better LOD — body parts visible at medium zoom
- [x] Pause/play, speed control, smooth camera
- [x] Screenshot verification mode (--screenshot)

## What's Next (prioritised)

### 1. Symbiosis
Mutualism, parasitism, commensalism. Two organisms evolving to depend on each other.

### 2. Save/load simulation state
Quality of life — pause a run, come back to it later.

### 3. GPU compute for neural net batching
Performance scaling — batch neural net forward passes on GPU for 100k+ organisms.

### 4. WASM+WebGPU browser build
Accessibility — run in a browser without installing anything.

---

## Core Vision

- [x] **Visual simulation** — watch evolution happen in real time
- [x] **Emergent speciation** — species diverge from common ancestors
- [x] **Competition** — creatures compete for resources, territory, and each other
- [x] **No hard categories** — strategies emerge from evolution
- [x] **Biomes** — different environments with different selection pressures
- [x] **Niche construction** — species reshape biomes, biomes reshape species back
- [x] **Phenotype rendering** — creatures visually express evolved traits
- [x] **Spatial and temporal zoom** — pan/zoom + pause/speed controls

## Brains

- [x] **Per-organism neural network** — NEAT brains, inherited with mutation
- [x] **Emergent behaviour** — movement, feeding, attack, reproduction strategies emerge
- [x] **Recurrent memory** — 3 memory slots for learning-like behaviour
- [x] **Chemical signalling** — organisms emit and sense signals, evolution decides meaning
- [ ] **Cognitive speciation** — separated populations diverge cognitively
- [ ] **Sentience spectrum** — communication, deception, play

## Emergent Dynamics

- [x] **Phylogenetic tree** — a living family tree alongside the simulation
- [x] **Arms races** — predator/prey co-evolution (claws vs armor)
- [x] **Mass extinction events** — asteroid, ice age, volcanic eruption
- [x] **Convergent evolution** — detection of independent lineages evolving similar solutions
- [x] **Seasonal pressure** — changing environment forces ongoing adaptation
- [x] **Geographic isolation** — ocean/mountain barriers drive allopatric speciation
- [ ] **Social behaviour** — pack hunting, herding, hive structures (signalling foundation in place)
- [ ] **Symbiosis** — mutualism, parasitism, commensalism
