# Clauvolution

An evolution simulator where you watch life emerge, adapt, compete, and speciate in real time.

## What's Done

- [x] Rust + Bevy ECS workspace
- [x] Per-organism NEAT neural networks with 20 inputs, 9 outputs, recurrent memory
- [x] Organisms sense food, neighbors, terrain, species, health, signals — brains decide everything
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
- [x] Species classification with distinct colours + hysteresis for stability
- [x] Phylogenetic tree with ancestry tracking and lineage grouping
- [x] World chronicle — automatic event log (speciation, extinction, convergence, seasons)
- [x] Convergent evolution detection (summarised, not per-pair spam)
- [x] Population sparkline graphs with per-strategy breakdown + fitness tracking
- [x] Strategy breakdown (Plants / Predators / Foragers counts)
- [x] Click-to-inspect with full stat panel
- [x] Help overlay (H key) explaining everything for newcomers
- [x] Action flash — organisms pulse when eating/attacking/reproducing
- [x] Initial diversity seeding — 30% photosynthesizers at start
- [x] Chemical signalling between organisms (brain input + output)
- [x] Seasonal cycles — 60-second year, affects light and food production
- [x] Geographic barriers — oceans/mountains isolate populations for speciation
- [x] Fitness tracking — average lifespan sparkline
- [x] Dynamic LOD — body parts re-render when zoom changes
- [x] Species stability — higher threshold, slower reclassification, hysteresis
- [x] Save/load — F5 saves to session directory, --load restores
- [x] Named sessions — cosmic three-word names, logs + screenshots per session
- [x] Manual screenshots (S key) saved to session directory
- [x] JetBrains Mono font for proper Unicode rendering
- [x] Photosynthesizers render as ground cover (behind active organisms)
- [x] Pause/play, speed control, smooth camera
- [x] Screenshot verification mode (--screenshot)
- [x] Seed-based terrain generation (same seed = same map, saved in save files)
- [x] Species naming — three-word trait-based names with taxonomy-like inheritance
- [x] Minimap with click-to-navigate — world overview showing terrain, organisms, camera viewport
- [x] Population heatmap toggle — minimap density view coloured by strategy (M key)
- [x] Social sensing — group size + average group signal brain inputs, group metabolic discount (~5%)
- [x] Performance — frustum culling, food hidden at far zoom, virtual time cap (100ms) prevents death spiral
- [x] UI panel backgrounds — semi-transparent dark backing on all text panels for readability

## What's Next (prioritised)

### 1. Proper UI panels (bevy_egui)
Current text panels are fixed-size and can't scroll. Need real UI: scrollable phylogenetic tree, resizable panels, tabs for different views.

### 2. Symbiosis
Mutualism, parasitism, commensalism. Two organisms evolving to depend on each other. Research-level — may need new mechanics.

### 3. Performance Scaling

Three complementary approaches, roughly in order of bang-for-buck:

#### Rayon parallelization for brain evaluation
The `sensing_and_brain_system` iterates organisms sequentially, but brain evaluation is pure computation with no side effects — textbook `par_iter`. Could halve simulation cost on multi-core machines. Bevy already parallelizes independent *systems*, but this would parallelize *within* the most expensive system.

```rust
// Conceptual — collect inputs first, evaluate brains in parallel, write outputs back
inputs.par_iter().map(|i| brain.evaluate(i)).collect()
```

#### GPU instanced rendering
Currently each organism gets its own `ColorMaterial`. True instanced rendering would pack per-instance data (position, scale, colour) into a single buffer and draw all organisms in one draw call. The bitmask shader trick can handle body parts: each instance carries a feature bitmask, the shader scales absent parts to zero — no entity churn for LOD changes.

#### GPU compute for neural net batching
The big one. Pad all NEAT networks to a uniform max size, flatten into GPU buffers, evaluate all 2000+ brains in a single compute shader dispatch. Requires wgpu compute pipeline. Only worth it at 10k+ organisms — the other two approaches should come first.

### 4. WASM+WebGPU browser build
Accessibility — run in a browser without installing anything.

---

## Ecosystem Balance

### Plant density competition
Photosynthesizers currently don't compete for light — 500 plants on one tile all get the same yield. This makes plant worlds a stable attractor that crowds out other strategies. Fix: photosynthesis yield should drop with local plant density (shading). More plants on a tile = less light each. Biologically honest and naturally caps plant dominance, leaving room for predators and foragers to evolve.

Could be as simple as: in photosynthesis_system, count plants on the same tile (or in a small radius), divide light by that count. Plants spread out to avoid competition, which creates gaps for other strategies.

---

## Cool Ideas to Try

Small-to-medium features that aren't on the critical path but would be fun. Pick one when you're in the mood.

### Organism trails
Faint fading lines showing where organisms have been recently. Makes migration patterns, territorial behaviour, and foraging routes visible.

### Death markers
Brief flash/particle where organisms die. Makes predation hotspots and starvation zones pop at a glance without needing the heatmap.

### Minimap legend
Small colour key on the heatmap so new viewers don't have to guess what green/red/white means.

### Organism family tracking
Track parent/child relationships so you can click an organism and see its lineage. "This forager's great-grandmother was a plant."

### Symbiosis starter
Organisms near each other for extended periods develop energy transfer. Parasites drain, mutualists share. Just the foundation — evolution decides the rest.

---

## Creature Portrait — Detailed Inspect Visualization

A dedicated rendering area in the inspect panel that shows a large, detailed, visually pleasing portrait of the selected organism — its body and its brain. The map sprites stay simple (circles/blobs); this is only rendered when you click to inspect.

### Body Visualization: Modular Sprite Stacking

Build on the existing `BodyPlan` system but render at much larger scale with better art.

**Approach:**
- Library of body part shapes (SVG-style or procedural meshes), each with anchor points for attachment
- Parts snap onto predefined coordinates around the torso, respecting bilateral symmetry
- Proper z-ordering: armor plates behind torso, limbs to sides, eyes and mouth in front, claws at edges
- Only 10-20 distinct part shapes needed to create thousands of unique combinations

**Part representations:**
- **Torso** — smooth organic ellipse, size reflects genome body_size
- **PhotoSurface** — leaf-like structures growing from the back, subtle green glow/particle effect
- **Claws/Fangs** — sharp triangles at the mouth area, size reflects attack_power
- **Fins** — semi-transparent veined shapes on the sides
- **Eyes** — prominent circles with pupils, count reflects eye_count
- **Armor** — layered plate shapes behind the body, opacity reflects armor_value
- **Limbs** — jointed segments radiating from torso
- **Mouth** — opening on the front, size reflects foraging ability

### Trait-Based Color Coding

Colors communicate biology at a glance:
- **Energy source**: green tint = photosynthetic, red/dark tint = predator
- **Environment**: blue/sleek = aquatic adapted, tan/rough = land/desert adapted
- **Health**: brightness reflects current energy level
- **Species**: hue from species colour, so related organisms look related

### Brain Visualization: NEAT Topology Graph

Render the actual neural network as a node graph below or beside the body portrait.

- **Nodes as circles**: inputs on the left, outputs on the right, hidden neurons in the middle
- **Connections as lines**: thickness proportional to weight, colour = positive (blue) vs negative (red)
- **Disabled connections**: shown as faint dotted lines
- **Labels**: input nodes labelled with what they sense (food dir, energy, group size, etc.), output nodes with what they do (move, eat, attack, etc.)
- Shows brain complexity at a glance — a simple forager has few connections, a sophisticated predator has a dense web

### Procedural Generation Options (Future)

For even more organic-looking creatures:
- **Metaballs**: 2D organic blobs that merge into each other. Adding a "tail" means adding a new metaball at the rear. Looks like a living, squishy organism. Requires a custom shader.
- **L-Systems**: Branching fractal structures — particularly good for photosynthesizers. Genome parameters control branching angle, depth, and leaf density.

### Visual Polish

- Consistent line weight (2px) and limited colour palette for coherent aesthetic
- Simple "squash and stretch" animation — breathing/pulsing idle animation
- Winged creatures get slight bobbing motion
- Photosynthesizers sway gently
- Scale the portrait to fit the inspect panel regardless of organism body_size

### Implementation Notes

- Render as a separate Bevy camera/layer or an egui canvas (pairs well with the bevy_egui UI overhaul)
- Only rendered for the single selected organism — no performance concern
- Portrait updates live as the organism moves, eats, takes damage, etc.
- Could eventually support "compare two organisms" side-by-side

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
- [x] **Social behaviour** — group sensing and metabolic discount in place, signalling wired up. Pack hunting, herding, flocking can emerge.
- [ ] **Symbiosis** — mutualism, parasitism, commensalism
