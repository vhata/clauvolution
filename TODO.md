# Clauvolution

An evolution simulator where you watch life emerge, adapt, compete, and speciate in real time.

## What's Done

- [x] Rust + Bevy ECS workspace — solid architecture, compiles clean
- [x] Per-organism NEAT neural networks with 19 inputs, 9 outputs, recurrent memory
- [x] Organisms sense food, neighbors, terrain, species, health — brains decide everything
- [x] Procedural terrain — oceans, deserts, grasslands, forests, rock
- [x] Biome-aware food spawning + terrain-dependent movement costs
- [x] Photosynthesis — organisms with photo surfaces gain energy from light
- [x] Body segments — torso, limbs, fins, eyes, mouth, photo surfaces, claws, armor plates
- [x] Predation — attack and eat other organisms (claws vs armor, size advantage)
- [x] Sexual reproduction with genome crossover
- [x] Niche construction — organisms terraform tiles
- [x] Mass extinction events — asteroid (X), ice age (I), volcanic eruption (V)
- [x] Brain memory — 3 recurrent slots
- [x] Organism aging and natural death
- [x] Generation tracking
- [x] Species classification with distinct colours
- [x] Click-to-inspect with full stat panel
- [x] Pause/play, speed control, smooth camera
- [x] Population sparkline graphs
- [x] Strategy breakdown (Plants / Predators / Foragers counts)
- [x] Photosynthesizers render as ground cover (behind active organisms)
- [x] Screenshot verification mode (--screenshot)

## What's Next (prioritised)

### 1. Energy pyramid / trophic efficiency (structural balance fix)
Predators should only get ~10% of the energy their prey had. This is how real ecosystems self-balance — it's thermodynamics, not tuning. Without this, predator meta always dominates because eating things is free energy. One structural change that replaces endless number tweaking.

### 2. Phylogenetic tree
The feature that turns dots-on-a-map into a story. A living family tree showing when species branched, which ones went extinct, and how everything alive today is related. Without this, speciation is just a number going up.

### 3. World narrative / chronicle
Automatic log of evolutionary history: new species emerging, extinctions, mass extinction events and recovery, population milestones, dominant species shifts. Timestamped with generation/tick count, optionally paired with auto-screenshots. The fossil record, written in real time.

### 4. Seed initial diversity
Start some organisms as dedicated photosynthesizers (high photo rate + photo surfaces) so the strategy exists from day one. Currently photosynthesis has to evolve from nearly zero, which may not happen if foraging is easier. We're not rigging the game — just providing initial diversity for evolution to work with.

### 5. Better LOD / close-zoom rendering
The detailed body-part rendering exists but the LOD threshold is too aggressive. When zoomed in, you should see torsos, limbs, fins, claws — not just circles. Smooth transition between detail levels.

### 6. Active behaviour indicators
Eating, attacking, reproducing all look the same visually. Some indication of what organisms are doing — a flash, a direction indicator, something.

### 7. Population history graphs (improvement)
Current sparklines work but could be richer. Per-species population over time. Area chart showing strategy breakdown changing. Historical events marked on the timeline.

### 8. Convergent evolution detection
Notice when unrelated species independently evolve similar strategies (e.g. fins in two separate ocean populations). This is one of the most fascinating things in real evolution.

### 9. Social behaviour
Cooperation, pack hunting, alarm calls, herding. Requires organisms to sense relatedness and evolve signalling. Late-game but spectacular when it emerges.

### 10. Symbiosis
Mutualism, parasitism, commensalism. Two organisms evolving to depend on each other.

### 11. Save/load simulation state
Quality of life — pause a run, come back to it later.

### 12. GPU compute for neural net batching
Performance scaling — batch neural net forward passes on GPU for 100k+ organisms.

### 13. WASM+WebGPU browser build
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
- [ ] **Cognitive speciation** — separated populations diverge cognitively
- [ ] **Sentience spectrum** — communication, deception, play

## Emergent Dynamics

- [ ] **Phylogenetic tree** — a living family tree alongside the simulation
- [x] **Arms races** — predator/prey co-evolution (claws vs armor)
- [x] **Mass extinction events** — asteroid, ice age, volcanic eruption
- [ ] **Convergent evolution** — independent lineages evolving similar solutions
- [ ] **Social behaviour** — pack hunting, herding, hive structures
- [ ] **Symbiosis** — mutualism, parasitism, commensalism
