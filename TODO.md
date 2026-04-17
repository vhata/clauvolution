# Clauvolution

An evolution simulator where you watch life emerge, adapt, compete, and speciate in real time.

## What's Done

- [x] Rust + Bevy ECS workspace
- [x] Per-organism NEAT neural networks with 22 inputs, 9 outputs, recurrent memory
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
- [x] Population line graphs (egui_plot) with per-strategy breakdown + fitness tracking
- [x] Strategy breakdown (Plants / Predators / Foragers counts)
- [x] Click-to-inspect with full stat panel
- [x] Help tab explaining everything for newcomers
- [x] Action flash — organisms pulse when eating/attacking/reproducing
- [x] Initial diversity seeding — 30% photosynthesizers at start
- [x] Chemical signalling between organisms (brain input + output)
- [x] Seasonal cycles — 60-second year, affects light and food production
- [x] Geographic barriers — oceans/mountains isolate populations for speciation
- [x] Fitness tracking — average lifespan plotted over time
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
- [x] Death markers — red flash for predation kills, amber for starvation/old age
- [x] Parent species tracking — inspect panel shows organism's lineage
- [x] Bloom events — solar bloom (B), nutrient rain (N), Cambrian spark (J)
- [x] Plant density competition — photosynthesizers shade each other, caps monoculture dominance
- [x] bevy_egui UI overhaul — header bar + tabbed right panel (Inspect / Phylo / Graphs / Chronicle / Events / Help). All old text overlays replaced; egui_plot for real line charts; collapsible phylo tree; buttons for events; WorldEventRequest event channel for keyboard-or-button triggering
- [x] Organism trails — faint gizmos linestrip behind each organism, T toggle, frustum culled, zero cost when off

---

# What's Next

## Guiding motivation

Clauvolution is a personal project. The goal isn't to answer a research question, ship a product, or present publicly. It's **pure curiosity and joy in watching evolution unfold** — the slow "look at that weird creature" delight of discovery.

That reshapes priorities:
- Features that **enrich what you can see and understand while watching** are the core. (Comprehension, new dynamics, visual polish.)
- Features that **skip past the watching** (evolve-until, replay) are less urgent — the unfolding *is* the point.
- Features for **sharing, exporting, external research** (WASM, config sweeps, organism export) are deprioritised — there's no audience but you.

Themes below are ordered by how much they serve the joy-of-watching motivation.

---

## Theme 1: Comprehension — make the invisible visible

The sim shows WHAT happens (species rising and falling) but hides WHY. These items surface the underlying causes so every moment of watching is richer.

**Top pick:** Brain activation heatmap. You already watch creatures move and compete; this lets you watch them *think*.

### Brain activation heatmap
When an organism is selected, show its neural network in real time — which inputs are currently firing, which connections are active, which outputs are being driven. Probably lives in the Inspect tab alongside the creature portrait.

**Why it matters:** Right now you know a creature is "complex" (23 neurons, 14 connections) but not what it's *doing*. You'd be able to watch a predator's "attack" output light up as prey comes into range, or see a forager's memory slots cycle as it navigates. Makes evolution legible.

**Shape of implementation:**
- Expose the brain's last-tick input and output values (already computed, just not exposed)
- Add a neural-network renderer in the Inspect tab (reuses the Creature Portrait design for the brain DAG part)
- Colour nodes by activation level, connections by signal flow
- Input node labels: "energy", "food dir x", "group size", etc.

### Species range heatmap
Click a species in the Phylo tab and the minimap (or a world overlay) highlights only where that species lives.

**Why it matters:** Right now you can see "13 species alive" but not how they're geographically distributed. Are they interleaved? Partitioned by biome? Competing on one landmass while another has nobody? Answers niche-partitioning questions you currently can't ask.

**Shape of implementation:**
- SelectedSpecies resource (already exists conceptually via SelectedOrganism → species_id)
- Minimap gets a third mode: Range (shows only this species at high contrast, everything else faded)
- Or a main-world overlay: semi-transparent coloured tiles where the species lives

### Genome diff view
Pick two organisms (or two species representatives), see a side-by-side diff of their traits, body plans, and brain topology with the differences highlighted.

**Why it matters:** "How different are these two?" is currently answered by a single number (compatibility distance). This unpacks it. You can see precisely what evolved — bigger claws? New eye? Different photosynthesis rate?

**Shape of implementation:**
- Comparison tab or modal in the egui right panel
- Click "compare with..." on an organism, then click a second
- Grid layout: trait | org A | org B | diff
- Body plan shown side-by-side with shared segments greyed out
- Brain topology shown with shared neurons/connections greyed, unique ones coloured

### Extinction post-mortem
When a species dies out, capture a snapshot of its last 30 seconds — population trend, causes of death (starvation vs predation vs old age), competitors, environment. Clickable chronicle entry.

**Why it matters:** Currently the chronicle says "Highland Crusted Wanderer went extinct" and that's it. You can't answer "did a predator wipe them out? did they starve? did a cousin species outcompete them?" This would tell you.

**Shape of implementation:**
- When species classification detects extinction, grab last N population snapshots, last N deaths and their causes, last tile occupation heatmap
- Store per-species post-mortem in PhyloTree node (small — just a few stats)
- Chronicle entries become clickable; open a modal with the post-mortem

---

## Theme 2: New dynamics — richer ecosystem

More kinds of evolution to watch unfold. Each item adds a qualitatively new pressure or possibility — meaning more variety when you watch.

**Top pick:** Disease. Solves plant-dominance AND creates a whole new arms race to observe.

### Disease / pathogens
Abstract infection that spreads between organisms in proximity. Kills or weakens hosts. Evolution of resistance vs virulence. Natural population regulator that can target any strategy (not just predation).

**Why it matters:** Plant dominance is a stable attractor because plants have no real predator apart from foragers. Disease adds a density-dependent mortality pressure that hits plants harder when they monoculture (high density = high transmission). Also pairs beautifully with social sensing: group bonuses vs disease risk becomes a real tradeoff.

**Shape of implementation:**
- `Disease` resource or per-organism `Infection` component
- Transmission: chance per tick scales with nearby infected count
- Virulence: small energy drain + small chance of death per tick
- Resistance: new genome trait, evolvable
- Periodically spawn new strains (pathogens evolve)

### Symbiosis (original plan item)
Two organisms spending extended time in close proximity develop an "energy link". A genome trait determines whether they drain (parasite), donate (altruist), or both (mutualist). Evolution decides whether symbiotic pairs outcompete solo organisms.

**Why it matters:** Real symbiosis is one of evolution's biggest innovations (mitochondria, chloroplasts, coral, lichens). Enables qualitatively new strategies.

**Shape of implementation:**
- Proximity tracker per organism (nearest-stable-neighbour over N ticks)
- New genome trait: `symbiosis_rate` (-1.0 drain to +1.0 donate)
- When link forms, energy transfer happens per tick based on both parties' rates
- Selection sorts out viable strategies

### Long-term climate shift
Very slow sinusoidal temperature drift over many seasons (e.g. a 30-minute cycle vs the 60-second year). Creates multi-generational selection pressure, distinct from seasons which snap back.

**Why it matters:** Adds real geological-timescale change. Species adapted to warm climate get squeezed when cold comes; species with broader thermal tolerance win. Multi-generation pressure, not within-lifetime.

**Shape of implementation:**
- Slow cosine drift on global temperature multiplier
- Optional: random climate events ("volcanic winter" lasting many seasons)
- Gets interesting in combination with seed-based terrain (deserts expand/contract, forests migrate)

### Larger body-plan mutations
Currently body parts are variations on torso+attachments. Rare mutation events could reshuffle the entire plan — swap torso type, rearrange attachment slots, introduce novel asymmetries. Big jumps, low frequency (maybe 0.1% chance per reproduction).

**Why it matters:** Current evolution is gradual tweaks to a fixed archetype. Big macro-mutations occasionally give genuinely new forms (Cambrian-explosion style), which recombine with existing variations.

**Shape of implementation:**
- In the mutation function, rare rolls for structural changes
- New BodyPlan variants: radial symmetry, stacked torsos, asymmetric plans
- Pair with the Cambrian-spark event to trigger a burst of these

---

## Theme 3: Time mastery — scrub, don't skip

Real evolution takes many generations. When watching *is* the point, skipping ahead conflicts with the motivation — but scrubbing backward to re-savour something you noticed is valuable. Cherry-pick carefully here.

### Replay / timeline scrubbing
Record a run as periodic snapshots. Scrub backward through time with a slider. Population graphs become scrubbable — click a point in history to see the world at that moment.

**Why it matters (given joy-of-watching):** "I noticed a spike in species count around tick 4000 — what happened?" This lets you rewind and re-experience the moment properly.

**Shape of implementation:**
- Periodic (every N seconds) snapshots of organism state + tilemap + phylo state
- Snapshots are lightweight (no brain eval, just state) and ring-buffered
- Timeline scrubber in the header or as a new mode
- Scrubbing pauses live sim; resume returns to live

### Interesting-moment auto-screenshot
Automatically capture screenshots when notable things happen: new species, mass extinctions, convergent evolution detected, population peaks/crashes, arms-race milestones.

**Why it matters:** Session highlights reel. Open sessions/<name>/highlights/ and flick through evolutionary history like a photo album.

**Shape of implementation:**
- Already have the detection logic in place (chronicle triggers)
- Add a screenshot request when those events fire
- Saved to sessions/<name>/highlights/ with descriptive filenames

### Evolve-until mode *(deprioritised)*
Run at unlimited speed until a triggering event (new species, extinction, etc.), then pause. Classic "skip to the punchline" feature — useful in a research framing but works against the joy-of-watching motivation. Leaving it documented but not chasing.

---

# Backlog

Items that don't directly serve the joy-of-watching motivation. Here for completeness — pick up if we ever want to scale, share, or do science.

## Theme: Performance scaling

Three complementary approaches, roughly in order of bang-for-buck:

### Rayon parallelization for brain evaluation
The `sensing_and_brain_system` iterates organisms sequentially, but brain evaluation is pure computation with no side effects — textbook `par_iter`. Could halve simulation cost on multi-core machines. Bevy already parallelizes independent *systems*, but this would parallelize *within* the most expensive system.

```rust
// Conceptual — collect inputs first, evaluate brains in parallel, write outputs back
inputs.par_iter().map(|i| brain.evaluate(i)).collect()
```

### GPU instanced rendering
Currently each organism gets its own `ColorMaterial`. True instanced rendering would pack per-instance data (position, scale, colour) into a single buffer and draw all organisms in one draw call. The bitmask shader trick can handle body parts: each instance carries a feature bitmask, the shader scales absent parts to zero — no entity churn for LOD changes.

### GPU compute for neural net batching
The big one. Pad all NEAT networks to a uniform max size, flatten into GPU buffers, evaluate all 2000+ brains in a single compute shader dispatch. Requires wgpu compute pipeline. Only worth it at 10k+ organisms — the other two approaches should come first.

---

## Theme: Accessibility — for sharing *(deprioritised)*

### WASM+WebGPU browser build
Run in a browser without installing anything. Only matters if you ever want to share the sim. Needs perf work first.

---

## Theme: Meta / experimentation *(deprioritised)*

No research question to answer, so these are lower priority. Here for reference.

### Config sweep mode
Launch N simulations with different parameters, summarise outcomes at the end. Answers questions like "does mutation_rate=0.3 produce more species than 0.1?"

**Shape:** Headless batch mode, N parallel instances, collect end-of-run stats, output a CSV or comparison view.

### Organism export / import
Save an interesting creature to a file. Load it into another sim as seed population.

**Shape:** JSON export of a single organism's genome. `--seed-with creature.json` CLI flag that spawns N copies at simulation start.

---

## Cool Ideas to Try

Small-to-medium features that aren't on the critical path but would be fun. Pick one when you're in the mood.

### Minimap legend
Small colour key on the heatmap so new viewers don't have to guess what green/red/white means.

### Symbiosis starter
Lighter version of full symbiosis — just energy transfer between nearby stable pairs, no genome trait yet. See whether the dynamic works at all before investing in evolvability.

### Trails for selected organism only
Turn trails from "all 2000 organisms = visual noise" into "this organism's last 2 seconds = useful inspection tool". Maybe also its species (last member of each living individual trails shown).

### Clickable chronicle entries
Click a chronicle entry → if it's about a species, switch to Phylo tab and highlight that species. If it's about a location, focus camera there.

---

## Creature Portrait — Detailed Inspect Visualization

A dedicated rendering area in the inspect panel that shows a large, detailed, visually pleasing portrait of the selected organism — its body and its brain. The map sprites stay simple (circles/blobs); this is only rendered when you click to inspect.

Now pairs naturally with **Brain activation heatmap** — the portrait shows the creature, the brain DAG shows its thinking, both animated live.

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
- **Live activation**: nodes brighten when firing, connections pulse when transmitting (this is the Brain Activation Heatmap item merging with the portrait)
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
- Could eventually support "compare two organisms" side-by-side — which dovetails into Genome Diff View

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
- [x] **Social sensing** — group size + signal sensing enables flocking, herding, pack dynamics

*Aspirational (no clear implementation shape):*
- **Cognitive speciation** — separated populations diverge cognitively
- **Sentience spectrum** — communication, deception, play

## Emergent Dynamics

- [x] **Phylogenetic tree** — a living family tree alongside the simulation
- [x] **Arms races** — predator/prey co-evolution (claws vs armor)
- [x] **Mass extinction events** — asteroid, ice age, volcanic eruption
- [x] **Convergent evolution** — detection of independent lineages evolving similar solutions
- [x] **Seasonal pressure** — changing environment forces ongoing adaptation
- [x] **Geographic isolation** — ocean/mountain barriers drive allopatric speciation
- [x] **Social behaviour** — group sensing and metabolic discount in place, signalling wired up
- [x] **Bloom events** — positive disturbance counterparts (solar / nutrient / mutation burst)
- [x] **Density competition** — plants shade each other, capping monocultures
- [ ] **Symbiosis** — mutualism, parasitism, commensalism
- [ ] **Disease** — density-dependent mortality, host/pathogen coevolution
- [ ] **Climate shift** — multi-generational drift distinct from seasons
