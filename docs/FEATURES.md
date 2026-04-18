# Implemented features

One-liner list of what Clauvolution does, grouped by area. For the design rationale behind any of these, see [DECISIONS.md](DECISIONS.md). For what's next, see [ROADMAP.md](ROADMAP.md).

## Simulation dynamics

- **Per-organism NEAT neural networks** — 22 sensory inputs, 9 outputs, 3 recurrent memory slots
- **Emergent behaviour** — no scripted actions; movement, feeding, attack, reproduction all evolve from selection
- **Genetic system** — neurons, connections, body segments, traits; mutation + crossover + structural innovations (add-neuron, add-connection)
- **Body segments** — torso, limb, fin, eye, mouth, photo surface, claw, armor plate; each affects gameplay
- **Sexual reproduction** — genome crossover with nearby same-species mates; asexual fallback if no mate
- **Predation** — attack + damage calculation (claws vs armor, size advantage); 10% trophic energy transfer (thermodynamic energy pyramid)
- **Photosynthesis** — organisms with photo surfaces gain energy from sunlight; scaled by tile light and season
- **Plant density competition** — photosynthesis yield drops with local plant density (shading)
- **Disease** — proximity-transmissible infection with evolvable resistance; pulsing purple halo indicator; direct mortality + energy drain
- **Metabolism** — quadratic costs for body size, armor, claws, speed; per-segment maintenance costs
- **Organism aging** — metabolism cost rises past age 500; natural death past age 3000
- **Chemical signalling** — each organism emits and senses a signal; evolution decides meaning
- **Social sensing** — group size + avg nearby signal as brain inputs; small metabolic discount for clustering
- **Niche construction** — organisms modify the tiles they occupy (vegetation, moisture, nutrients)

## Speciation & tracking

- **Species classification** — NEAT compatibility distance with hysteresis; re-evaluated every 5 seconds
- **Phylogenetic tree** — ancestry tracking with parent/child lineage grouping
- **Species naming** — three-word trait-based names (habitat + descriptor + strategy noun); children inherit two-of-three from parent
- **Parent species tracking** — inspect panel shows organism's lineage
- **World chronicle** — automatic event log (speciation, extinction, convergence, seasons, bloom/extinction events)
- **Convergent evolution detection** — summarised, deduplicated per strategy
- **Fitness tracking** — average lifespan plotted over time

## World & environment

- **Procedural terrain** — seed-based value noise generating oceans, shallow water, sand, grassland, forest, rock
- **Biome-aware food spawning** — food density proportional to tile nutrients + vegetation
- **Terrain-dependent movement** — each biome has land and water movement costs; deep water is 10x for land organisms (creates geographic isolation)
- **Seasonal cycles** — 60-second year with sinusoidal light + food regen multipliers; winter is harsh
- **Tile dynamics** — vegetation grows toward nutrient/moisture carrying capacity; nutrients cycle

## Events — destructive and creative

- **Asteroid impact (X)** — kills 70% of organisms randomly
- **Ice age (I)** — halves global temperature, reduces moisture
- **Volcanic eruption (V)** — local kill zone + nutrient boost
- **Solar bloom (B)** — doubles light for 30 seconds; plants surge
- **Nutrient rain (N)** — massive food burst across the world
- **Cambrian spark (J)** — triples mutation rate for 30 seconds

## Visualisation

- **Dynamic LOD** — organism sprites are simple circles when zoomed out, detailed body parts when zoomed in
- **Photosynthesisers as ground cover** — render behind active organisms without outlines
- **Action flash** — organisms pulse briefly when eating, attacking, or reproducing
- **Death markers** — red flash for predation kills, amber for starvation/old age; fades over ~0.5s
- **Organism trails (T)** — gizmos linestrip behind each organism showing recent movement; frustum culled
- **Initial diversity seeding** — 30% of starting population are photosynthesisers (bootstraps food chain)

## Navigation & camera

- **Minimap** — top-right world overview with click-to-navigate and camera viewport rectangle
- **Minimap legend** — colour key below the minimap for plants / foragers / predators
- **Population heatmap (M)** — minimap toggles between organism-dots and strategy-coloured density
- **Pan / zoom / drag** — WASD, arrows, mouse wheel, right-drag, middle-drag
- **Pause / speed control** — Space to pause, `[` / `]` to change speed (0.125× to 16×)

## User interface (bevy_egui)

- **Compact header bar** — season, population, species count, speed, generation — always visible
- **Tabbed right panel** (Inspect / Phylo / Graphs / Chronicle / Events / Help):
  - **Inspect** — selected organism stats: species/strategy/parent, energy/health bars, body/brain collapsibles, infection state
  - **Phylo** — collapsible lineage tree with strategy badges, declining indicators, recently-extinct section
  - **Graphs** — `egui_plot` line charts for population by strategy, death cause breakdown, infection rate & evolved resistance, trait evolution, pop vs species, food & lifespan. Current-stats readout and average-traits grid.
  - **Chronicle** — scrollable event log with "hide seasons" filter
  - **Events** — buttons for all extinction/bloom events with cooldown feedback; save-world button; active effects readout
  - **Help** — collapsible sections explaining everything

## Performance

- **Frustum culling** — organisms and food outside camera viewport get Visibility::Hidden (skipped by GPU)
- **Food hidden at far zoom** — individual food items invisible at zoom > 2.0, so don't render
- **Shared mesh handles** — one circle/material reused across thousands of entities
- **Virtual time cap (100ms)** — prevents death spiral after lag spikes
- **Pause via virtual time** — paused sim doesn't accumulate ticks; unpause is instant
- **Incremental release builds** — enabled in Cargo.toml; much faster iteration

## Tooling

- **Save/load** — F5 saves full world state to session directory; `--load sessions/<name>` restores
- **Named sessions** — each run gets a unique cosmic three-word name; logs + screenshots + saves live in `sessions/<name>/`
- **Seed-based terrain generation** — same seed produces same terrain; saved in save files
- **Manual screenshots (S)** — saved to session directory with timestamp
- **Screenshot verification mode** — `--screenshot` CLI flag runs a scripted tour and captures images
- **JetBrains Mono font** — bundled for proper Unicode rendering

## Tuning instrumentation

- **Death cause categorisation** — every death attributed to Starvation / Predation / Old age / Disease
- **Infection stats** — count, percentage of population, spread over time
- **Trait averages over time** — disease resistance, body size, speed, attack, armor, photo — all plotted
- **Current-stats readouts in Graphs tab** — pop/food/species/lifespan/infected/per-strategy counts at a glance
