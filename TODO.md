# Clauvolution

An evolution simulator where you watch life emerge, adapt, compete, and speciate in real time.

## Status

### What's Working (The Good)
- [x] Rust + Bevy ECS workspace — solid architecture, compiles clean, runs smooth
- [x] Per-organism NEAT neural networks — each creature has its own evolved brain
- [x] Organisms sense food and neighbors, brains decide actions (move/eat/reproduce)
- [x] Natural selection is running — population finds equilibrium around 800
- [x] Procedural terrain generation — oceans, deserts, grasslands, forests, rock
- [x] Biome-aware food spawning — more food in fertile areas
- [x] Terrain-dependent movement costs — fins help in water, limbs help on land
- [x] Photosynthesis — organisms with photo surfaces gain energy from light
- [x] Body segment genes — torso, limbs, fins, eyes, mouth, photosynthetic surfaces
- [x] Smooth camera controls — scroll zoom, mouse drag pan, keyboard zoom/pan
- [x] Basic stats overlay — organism count, food, births, deaths

### What's Not Great (The Bad)
- [ ] **Hard to read what's happening** — dots buzzing around, can't tell what they're doing or why
- [ ] **No species colouring** — can't distinguish populations or groups visually
- [ ] **Body part rendering is underwhelming** — detailed view exists but hard to see the difference at normal zoom
- [ ] **No way to inspect an organism** — can't click on one to see its genome, brain, energy, lineage
- [ ] **No population graphs or history** — just a live count, no sense of trends
- [ ] **No temporal controls** — can't pause, speed up, or slow down
- [ ] **Food is just green dots** — no visual difference by biome
- [ ] **No indication of what organisms are "doing"** — eating, reproducing, photosynthesizing all look the same

### Quick Wins (The Easy)
- [ ] Species colouring — group by genetic similarity, colour by species
- [ ] Movement trails or speed indicators — see who's moving vs sitting still
- [ ] Visual distinction for photosynthesizers (glow green?) vs foragers
- [ ] Pause/play and speed controls (keyboard shortcuts)
- [ ] Click-to-inspect an organism — show a tooltip or panel with stats
- [ ] Biome-coloured food (algae in water, berries on land)

### Hard Problems (The Ugly)
- [ ] Niche construction feedback loop — organisms modifying biome tiles meaningfully
- [ ] Emergent speciation that's actually visible — need species tracking + phylogenetic tree
- [ ] Predation — organisms eating each other, not just food. Needs combat, size advantage, defence
- [ ] Social behaviour — cooperation requires organisms to sense relatedness and evolve signalling
- [ ] Sentience spectrum — recurrent memory in brains, learning-like behaviour
- [ ] LOD that feels good — smooth transition from body parts to dots to heatmaps as you zoom

---

## Core Vision

- [x] **Visual simulation** — watch evolution happen in real time *(basic — needs readability work)*
- [ ] **Emergent speciation** — species diverge from common ancestors, no predefined species
- [x] **Competition** — creatures compete for resources *(food only — no predation yet)*
- [x] **No hard categories** — strategies emerge from evolution itself *(photosynthesis vs foraging is evolving)*
- [x] **Biomes** — oceans, deserts, jungles with different selection pressures *(terrain exists, biome effects work)*
- [ ] **Niche construction** — species reshape their biomes, biomes reshape species back
- [x] **Phenotype rendering** — creatures visually express evolved traits *(body parts exist, need polish)*
- [x] **Spatial and temporal zoom** — pan/zoom works *(no temporal controls yet)*

## Brains

- [x] **Per-organism neural network** — NEAT brains, inherited with mutation
- [x] **Emergent behaviour** — movement and feeding strategies emerge from selection
- [ ] **Cognitive speciation** — separated populations diverge cognitively
- [ ] **Sentience spectrum** — learning, memory, communication, deception, play

## Emergent Dynamics

- [ ] **Phylogenetic tree** — a living family tree alongside the simulation
- [ ] **Arms races** — co-evolution between predators and prey
- [ ] **Mass extinction events** — catastrophes that wipe out dominant species
- [ ] **Convergent evolution** — independent lineages evolving similar solutions
- [ ] **Social behaviour** — pack hunting, herding, hive structures
- [ ] **Symbiosis** — mutualism, parasitism, commensalism
