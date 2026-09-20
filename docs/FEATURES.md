# Implemented features

One-liner list of what Clauvolution does, grouped by area. For the design rationale behind any of these, see [DECISIONS.md](DECISIONS.md). For what's next, see [ROADMAP.md](ROADMAP.md).

## Simulation dynamics

- **Per-organism NEAT neural networks** — 22 sensory inputs, 9 outputs, 3 recurrent memory slots
- **Emergent behaviour** — no scripted actions; movement, feeding, attack, reproduction all evolve from selection
- **Genetic system** — neurons, connections, body segments, traits; mutation + crossover + structural innovations (add-neuron, add-connection). Crossover blends each scalar trait with its own factor, so a child can take one parent's speed and the other's armour
- **Body segments** — torso, limb, fin, eye, mouth, photo surface, claw, armor plate; each affects gameplay
- **Sexual reproduction** — genome crossover with nearby same-species mates; asexual fallback if no mate
- **Population ceiling, instrumented** — `SimConfig::population_ceiling` (2000) is the only birth limiter. Each engagement episode writes a chronicle entry, and the headless summary reports how often it engaged and how many births it blocked, so the cap is visible as the rule it currently is. Raising it is sequenced after the diet axis gives plants a consumer (see DECISIONS.md)
- **Predation** — attack + damage calculation (claws vs armor, size advantage); the killer is offered 10% of the prey's energy (thermodynamic energy pyramid) and keeps it times its animal digestion efficiency
- **Grazing** — an attack on a photosynthesiser is a bite, not a kill: the plant loses 10% of its current energy (`BITE_FRACTION`) and lives on, the grazer keeps the bite times its plant digestion efficiency, one bite per plant per tick, damage gate only (no size gate). Undigested energy from bites and kills is the ledger's `digestion` flow; grazes are counted in the headless predation funnel
- **Photosynthesis** — organisms with photo surfaces gain energy from sunlight; scaled by tile light and season
- **Plant density competition** — photosynthesis yield drops with local plant density (shading)
- **Disease** — proximity-transmissible infection with evolvable resistance; pulsing purple halo indicator; direct mortality + energy drain
- **Metabolism** — quadratic costs for body size, armor, claws, speed; per-segment maintenance costs
- **Organism aging** — metabolism cost rises past age 500; natural death past age 3000
- **Chemical signalling** — each organism emits and senses a signal; evolution decides meaning
- **Symbiosis** — `symbiosis_rate` genome trait; mutual-nearest neighbours within 6 units held for 10+ ticks form a link, exchanging energy per each party's rate (parasite ↔ donor). One tuning pass shipped; population-level selection on the rate is still an open question (see DECISIONS.md).
- **Diet trait** — `diet` genome trait in -1..1 (herbivore to carnivore) with plant and animal digestion efficiencies of `((1 ∓ diet) / 2)²`, so a generalist digests a quarter of each. Plant efficiency scales what a grazer keeps from a bite and what an eater keeps from a food item; animal efficiency scales the killer's pyramid share. Shown in Inspect with both efficiencies, and as the average over non-plants (plants carry the trait but never eat) in the Graphs trait plot and stats grid, the headless summary, and `avg_diet` in `--dump-history`.
- **Photosynthetic surface drag** — speed is multiplied by `1 / (1 + photo surface area × photo_drag)` beside the armour drag term, so a leafy organism is slow and a small-leaved one is not; `photo_drag` is 1.0 (`--photo-drag`). No strategy is forbidden anything; a plant with claws can still hunt, as an ambusher
- **Plant physics instruments** — each organism's movement per tick is recorded in `Velocity` and each photosynthesiser's share of full light in `LightShare`; the history averages speed over plants and over eaters, light share and leaf area over plants, and the share of each group whose energy is above its reproduction threshold (who is ready when a birth slot opens). Shown as a Graphs plot and stats rows, in the headless summary, and as six CSV columns. Instruments for `plans/2026-09-20-plant-physics.md`.
- **Strategy classification** — `classify_strategy` in `clauvolution_phylogeny` labels every organism plant (photo surface and photosynthesis rate above 0.2), else grazer / hunter / omnivore by `diet` against a threshold of ±1/3. One `Genome::is_photosynthesiser` predicate carries the plant rule for classification, rendering, density competition and niche construction; the photosynthesis yield gate stays looser at 0.01 so a lineage drifting toward plant-hood is paid for the first steps.
- **Social sensing** — group size + avg nearby signal as brain inputs; small metabolic discount for clustering
- **Niche construction** — organisms modify the tiles they occupy (vegetation, moisture, nutrients)

## Speciation & tracking

- **Species classification** — NEAT compatibility distance with hysteresis; re-evaluated every 5 seconds. Trait-led: the body term is normalised to 0..1 across the ten scalar traits and weighted 1.0, the three NEAT brain terms 0.5 each
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
- **Death markers** — red flash for predation kills, amber for every other cause (starvation, old age, disease, world events); fades over ~0.5s
- **Organism trails (T)** — toggle a gizmos linestrip behind the selected organism showing its last ~2 seconds of movement
- **Initial diversity seeding** — 30% of starting population are photosynthesisers (bootstraps food chain)
- **Per-biome seeding** — the 400 founders are placed on land in proportion to each biome's area (Sand, Grassland, Forest, Rock; never water), with a floor of 5% for any biome holding at least 1% of the land, and start just below their own reproduction threshold; the starting food stock (`initial_food_density`) and the regeneration ceiling (`max_food_density`) are separate settings, both 0.1; the founder counts per biome and strategy are logged at startup and printed in the headless summary

## Navigation & camera

- **Minimap** — top-left world overview with click-to-navigate and camera viewport rectangle
- **Minimap legend** — colour key below the minimap for plants / grazers / hunters / omnivores
- **Minimap selection marker** — bright yellow plus at the selected organism's position
- **Population heatmap (M)** — minimap toggles between organism-dots and strategy-coloured density
- **Pan / zoom / drag** — WASD, arrows, mouse wheel, right-drag, middle-drag
- **Pause / speed control** — Space to pause, `[` / `]` to change speed (0.125× to 16×)
- **Focus on selected (F)** — snap camera to the selected organism's position
- **Cycle species members (, / .)** — step through living members of the selected organism's species
- **Random select (R)** — pick any living organism at random

## User interface (bevy_egui)

- **Compact header bar** — always-visible summary: sim time, season, population, species, generation, speed, infection count (when > 0), active bloom effects with seconds remaining
- **Tabbed right panel** (Inspect / Phylo / Graphs / Chronicle / Events / Help):
  - **Inspect** — selected organism stats: species/strategy/parent, energy/health bars, diet and digestion efficiencies, body/brain collapsibles, infection state
  - **Phylo** — collapsible lineage tree with strategy badges, declining indicators, recently-extinct section; click a species name to select a living member
  - **Graphs** — `egui_plot` line charts for population by strategy, death cause breakdown, infection rate & evolved resistance, trait evolution, pop vs species, symbiosis, energy income and costs per second, ledger residual, food & lifespan. Current-stats readout and average-traits grid.
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

- **Headless mode** — `--headless N` runs N ticks without rendering/UI, prints end-of-run summary (plant / grazer / hunter / omnivore counts, death cause breakdown, trait averages including diet, predation funnel, energy ledger). `--speed N` multiplies virtual time (default 10×, ceiling is CPU-bound at ~85 ticks/sec). `--save-as <name>` writes a save file at end; `--load sessions/<name>` resumes from one. Combine for: evolve headless → save → reload in GUI → script a tour.
- **Seeded runs** — `--seed N` seeds all sim randomness. Deterministic for ~50 ticks (Bevy task pool parallelism causes later divergence — not yet fully reproducible).
- **Save/load** — F5 saves full world state to session directory; `--load sessions/<name>` restores
- **Named sessions** — each run gets a unique cosmic three-word name; logs + screenshots + saves live in `sessions/<name>/`
- **Seed-based terrain generation** — same seed produces same terrain; saved in save files
- **Manual screenshots (Shift+S)** — saved to session directory with timestamp. Captures camera + egui overlays via a secondary-camera + `EguiRenderToImage` + gpu readback pipeline (see DECISIONS.md).
- **Screenshot verification mode** — `--screenshot` CLI flag runs a scripted tour and captures images
- **Scripted automation** — `--script path.json` runs a timed sequence of actions (set tab, set speed, set zoom, move camera, screenshot, exit). See `tours/demo.json` for an example; useful for producing README screenshots.
- **Panel tab shortcuts (1–6)** — switch right-panel tab from the keyboard (1=Inspect, 2=Phylo, 3=Graphs, 4=Chronicle, 5=Events, 6=Help)
- **JetBrains Mono font** — bundled for proper Unicode rendering

## Tuning instrumentation

- **Death cause categorisation** — every death attributed to Starvation / Predation / Old age / Disease / Event (asteroid, volcano); the per-cause totals sum to total deaths
- **Infection stats** — count, percentage of population, spread over time
- **Trait averages over time** — disease resistance, body size, speed, attack, armor, photo — all plotted
- **Current-stats readouts in Graphs tab** — pop/food/species/lifespan/infected/per-strategy counts at a glance
- **Energy ledger** — every flow that moves organism energy (photosynthesis, food, predation, grazing, symbiosis, metabolism, movement, disease, reproduction paid and received, energy lost at death, energy destroyed at the `max_organism_energy` clamp, energy lost to digestion) is accumulated per tick and per run in `EnergyLedger`. `ledger_system` compares the change in total live energy against the net of the flows; the residual is zero to f32 rounding when nothing mints or destroys energy unrecorded. Shown as income/cost charts and a residual line on the Graphs tab, as a block in the headless summary, and as columns in `--dump-history`. Past tolerance it is a `debug_assert!` and a rate-limited chronicle warning.
