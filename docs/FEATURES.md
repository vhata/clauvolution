# Implemented features

One-liner list of what Clauvolution does, grouped by area. For the design rationale behind any of these, see [DECISIONS.md](DECISIONS.md). For what's next, see [ROADMAP.md](ROADMAP.md).

## Simulation dynamics

- **Per-organism NEAT neural networks** — 26 sensory inputs, 9 outputs, 3 recurrent memory slots
- **Nearest-eater sensing** — four brain inputs give the direction, closeness and size ratio of the nearest non-photosynthesiser in sense range, encoded as the nearest-organism inputs are, so a brain can steer at an animal when the nearest organism is a plant. Labelled `eater dx` / `eater dy` / `eater near` / `eater size` in the Inspect brain view. Genomes from older saves and creature files gain these inputs unconnected on load and behave as before
- **Emergent behaviour** — no scripted actions; movement, feeding, attack, reproduction all evolve from selection
- **Genetic system** — neurons, connections, body segments, traits; mutation + crossover + structural innovations (add-neuron, add-connection). Crossover blends each scalar trait with its own factor, so a child can take one parent's speed and the other's armour
- **Body segments** — torso, limb, fin, eye, mouth, photo surface, claw, armor plate; each affects gameplay
- **Sexual reproduction** — genome crossover with nearby same-species mates; asexual fallback if no mate
- **Population ceiling, instrumented** — `SimConfig::population_ceiling` (6000 since 2026-09-20; carrying capacity now comes from canopy light sharing and grazing, and the ceiling is a safety net that some seeds still meet) is the only birth limiter. Each engagement episode writes a chronicle entry, and the headless summary reports how often it engaged and how many births it blocked, so the cap is visible as the rule it currently is. Raising it is sequenced after the diet axis gives plants a consumer (see DECISIONS.md)
- **Predation** — the `attack` output is a kill attempt on the nearest neighbour in reach, plant or animal, past a size gate (attacker above 0.6 of the target's size) and a damage gate (claws vs armor); the killer is offered a share of the prey's energy set by the prey's tissue, 10% for animal and plant victims alike by default (thermodynamic energy pyramid; `kill_transfer_animal` / `kill_transfer_plant`, `--kill-transfer-animal` / `--kill-transfer-plant`, `--kill-transfer` for both) and keeps it times its digestion efficiency for that tissue: plant efficiency for a plant, animal efficiency for an animal. A strike (attack firing with any living organism in reach, whether it lands or not) costs the attacker 1.0 energy per unit of strike force, claw power × body size (`strike_cost`, `--strike-cost`); firing at nothing is free
- **Grazing** — the `eat` output bites the nearest living photosynthesiser within 1 × body size (`bite_reach`, `--bite-reach`) when no food item was eaten that tick (`grazing_system`): the plant loses 30% of its current energy (`bite_fraction`, `--bite-fraction`) times the eater's mouth bonus (1.0 with a mouth segment, 0.3 without: `mouthless_bite_bonus`, `--mouthless-bite`) and lives on, and the eater keeps the bite times its plant digestion efficiency. One bite per plant per tick, first eater wins; no claw or size gate. Undigested energy from bites and kills is the ledger's `digestion` flow; grazes are counted in the headless predation funnel
- **Photosynthesis** — organisms with photo surfaces gain energy from sunlight; scaled by tile light and season
- **Canopy light sharing** — each tile can fully light `leaf_capacity_per_tile` (0.02, `--leaf-capacity`) of leaf area; a photosynthesiser's light share is what the 9 by 9 tile window around it can light divided by the leaf area in that window, capped at 1, from a summed-area table rebuilt each tick. Where leaves exceed the ground, everyone there is shaded in proportion; this is what bounds plant growth. Replaces per-tile density competition
- **Disease** — proximity-transmissible infection with evolvable resistance; pulsing purple halo indicator; direct mortality + energy drain
- **Metabolism** — quadratic costs for body size, armor, claws, speed; per-segment maintenance costs
- **Organism aging** — metabolism cost rises past age 500; natural death past age 3000
- **Chemical signalling** — each organism emits and senses a signal; evolution decides meaning
- **Symbiosis** — `symbiosis_rate` genome trait; mutual-nearest neighbours within 6 units held for 10+ ticks form a link, exchanging energy per each party's rate (parasite ↔ donor). One tuning pass shipped; population-level selection on the rate is still an open question (see DECISIONS.md).
- **Diet trait** — `diet` genome trait in -1..1 (herbivore to carnivore); founders draw it uniformly across the whole axis (`founder_diet_spread` 1.0, `--founder-diet-spread`) so grazers and hunters exist from tick 0 with plant and animal digestion efficiencies of `((1 ∓ diet) / 2)^x`, so a generalist digests a quarter of each at the default exponent x = 2 (`diet_efficiency_exponent`, `--diet-exponent`, at least 1; swept at 1.5 and 1.0 in `docs/audits/2026-09-24-hunter-bridge-exponent/`). Plant efficiency scales what a grazer keeps from a bite, what an eater keeps from a food item and what a killer keeps from a plant it killed; animal efficiency scales the killer's pyramid share of an animal. Shown in Inspect with both efficiencies, and as the average over non-plants (plants carry the trait but never eat) in the Graphs trait plot and stats grid, the headless summary, and `avg_diet` in `--dump-history`.
- **Photosynthetic surface drag** — speed is multiplied by `1 / (1 + photo surface area × photo_drag)` beside the armour drag term, so a leafy organism is slow and a small-leaved one is not; `photo_drag` is 1.0 (`--photo-drag`). No strategy is forbidden anything; a plant with claws can still hunt, as an ambusher
- **Plant physics instruments** — each organism's movement per tick is recorded in `Velocity` and each photosynthesiser's share of full light in `LightShare`; the history averages speed over plants and over eaters, light share and leaf area over plants, and the share of each group whose energy is above its reproduction threshold (who is ready when a birth slot opens). Shown as a Graphs plot and stats rows, in the headless summary, and as six CSV columns. Instruments for `plans/2026-09-20-plant-physics.md`.
- **Feeding instruments** — `PredationStats::feeding` (`FeedingCounts`) counts grazes through `eat` (and how many of those were taken by plants) and through `attack` (0 since grazing moved to `eat`), kills of consumers and of plants, plant kills by non-photosynthesisers and the energy they kept, kills by killers with `diet < 0`, and attack intents with no living plant in reach. Per-second values are in the history CSV (`grazes_eat` … `plant_kill_energy_consumer`), the Graphs tab and the headless summary. Counting only; added for `plans/2026-09-21-pyramid-top.md`, baseline in `docs/audits/2026-09-23-pyramid-baseline/`.
- **Geography instruments** — `TileMap::regions` labels the connected components of land (flood fill with the torus wrap, computed at generation, not saved; components under `MINOR_REGION_MAX_TILES` pool into one minor region). `GeographyStats` counts organism-ticks on deep and shallow water by kind (plant or consumer) and `aquatic_adaptation` band, the movement energy paid there, region crossings, species first reaching a new major region (also a chronicle entry), and food items spawned and eaten on water. Each 1 Hz snapshot adds population by biome and by region per strategy label, and the region and biome separation numbers (mean dominant share per species) against a shuffled-label null. History CSV trailing columns (`ticks_all_plant_aq0` ... `biome_confined_null`), a geography block in the headless summary, Graphs plots of population by region, separation against its null and crossings per second, and the Inspect tab's Region row. Counting only; added for `plans/2026-09-25-phase2-biomes.md`, baseline in `docs/audits/2026-09-25-phase2-baseline/`.
- **Diet-band instruments** — `DietBandStats` counts, for consumers in six diet bands (each strategy label split in half at -2/3, 0 and +2/3), organism-ticks, energy kept from food items, eat bites, consumer kills and plant kills, plant tissue taken before digestion, metabolism, movement and strike costs, births, and deaths by cause with age at death; a parent-to-child strategy-label matrix counts band crossings. `FeedingCounts::omnivore_gates` adds an omnivore band to the consumer-prey gate outcomes. Per-second values are the history CSV's trailing columns (`omnivore_intents` ... `cross_hunter_omnivore`), the headless summary prints a per-band block every 500 ticks and run totals, and the Graphs tab plots omnivore income by source against their costs. Counting only; added for `plans/2026-09-24-hunter-bridge.md`, baseline in `docs/audits/2026-09-24-hunter-bridge-baseline/`.
- **Strategy classification** — `classify_strategy` in `clauvolution_phylogeny` labels every organism plant (photo surface and photosynthesis rate above 0.2), else grazer / hunter / omnivore by `diet` against a threshold of ±1/3. One `Genome::is_photosynthesiser` predicate carries the plant rule for classification, rendering and niche construction (shading uses the looser yield gate: anything that earns sun also casts shade); the photosynthesis yield gate stays looser at 0.01 so a lineage drifting toward plant-hood is paid for the first steps.
- **Social sensing** — group size + avg nearby signal as brain inputs; small metabolic discount for clustering
- **Niche construction** — organisms modify the tiles they occupy (vegetation, moisture, nutrients)

## Speciation & tracking

- **Species classification** — NEAT compatibility distance with hysteresis; re-evaluated every 5 seconds. Trait-led: the body term is normalised to 0..1 across the ten scalar traits and weighted 1.0, the three NEAT brain terms 0.5 each. Each pass counts organisms within the join threshold of another species, those drifting past the stay threshold, and those isolated enough to found a species (`SpeciesPassCounts`: Graphs tab, history CSV, headless summary)
- **Phylogenetic tree** — ancestry tracking with parent/child lineage grouping
- **Species naming** — three-word trait-based names (habitat + descriptor + strategy noun); children inherit two-of-three from parent; no two living species share a name (the descriptor is walked past taken names, a number is the last resort)
- **Parent species tracking** — inspect panel shows organism's lineage; species ids are issued once, so a new species never takes over an extinct one's tree node
- **World chronicle** — automatic event log (speciation, extinction, convergence, seasons, bloom/extinction events)
- **Convergent evolution detection** — one chronicle line per strategy each time the number of independent times it evolved (a species switching strategy from its parent, founders excluded) reaches a new high
- **Fitness tracking** — average lifespan plotted over time

## World & environment

- **Procedural terrain** — seed-based value noise generating oceans, shallow water, sand, grassland, forest, rock
- **Biome-aware food spawning** — food density proportional to tile nutrients + vegetation
- **Terrain-dependent movement** — each biome has a land-adapted and a water-adapted movement cost, and an organism pays the interpolation between them by its `aquatic_adaptation` on every tile: deep water costs 10.0 at aquatic 0 and 1.0 at aquatic 1, sand 1.5 and 5.0. Fins cut the cost on water, limbs on land
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

- **Compact header bar** — always-visible summary: sim time, season, population, species (with the effective compatibility threshold, so a `--species-threshold` override is visible), generation, speed, infection count (when > 0), active bloom effects with seconds remaining
- **Tabbed right panel** (Inspect / Phylo / Graphs / Chronicle / Events / Help):
  - **Inspect** — selected organism stats: species/strategy/parent, energy/health bars, diet and digestion efficiencies, body/brain collapsibles, infection state
  - **Phylo** — collapsible lineage tree with strategy badges, declining indicators, recently-extinct section; click a species name to select a living member; the last species reached by click (here or from the Chronicle tab) stays highlighted
  - **Graphs** — `egui_plot` line charts for population by strategy, death cause breakdown (with grazer kills of consumers as its own line), grazes per second by output (eat / attack), infection rate & evolved resistance, trait evolution, pop vs species, species passes (near another species %, drifting, isolated), symbiosis, energy income and costs per second, ledger residual, food & lifespan. Current-stats readout and average-traits grid.
  - **Chronicle** — scrollable event log with "hide seasons" filter; speciation and extinction entries are links that switch to the Phylo tab with that species highlighted (and a living member selected, if any), and the volcano entry is a link that centres the camera on the eruption. Entries carry their target through save and load
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

- **Headless mode** — `--headless N` runs N ticks without rendering/UI, prints end-of-run summary (plant / grazer / hunter / omnivore counts, death cause breakdown with predation split into kills of consumers / plants, kills by grazers (diet < 0) and plants killed by consumers and by hunters with the energy they kept, how many founding hunters died and their mean and oldest age, trait averages including diet, a grazer timeline every 500 ticks (grazer size, armour and eat grazes), a diet-band timeline every 500 ticks with run totals (income by source, costs, births, deaths by cause, band crossings, omnivore attack outcomes), predation funnel with strikes and the energy they cost, grazes by output and attacks with no plant in reach, energy ledger). `--speed N` sets the number of ticks per headless frame (default 10); the run goes as fast as the CPU allows at every speed. `--save-as <name>` writes a save file at end; `--load sessions/<name>` resumes from one. Combine for: evolve headless → save → reload in GUI → script a tour.
- **Seeded runs** — `--seed N` seeds all sim randomness. Same-seed headless runs are bit-identical (summary and history CSV) at any compute pool size; GUI runs are wall-clock paced and are not.
- **Save/load** — F5 saves full world state to session directory; `--load sessions/<name>` restores. The save is written to a temporary file and renamed into place, so a failed write (full disk, permissions) never clobbers the previous save. Success and failure both land in the chronicle ("World saved to ..." / "Save failed: ..."); the sim keeps running either way. A save written before a field existed still loads, with that field at a neutral value; only a file missing the tick, the terrain seed, the organisms, or an organism's position, energy or genome is refused. Terrain state (vegetation, moisture, nutrients, temperature) is saved and restored on top of the terrain regenerated from the seed, so niche construction and event effects survive a reload; a save without it loads with the regenerated terrain and a warning. In headless mode the outcome is printed to stderr and a failed `--save-as` exits non-zero.
- **Creature export and import** — the Inspect tab's "Export creature" button writes the selected organism's genome plus provenance (species name, strategy, generation, tick, origin seed and session) to `sessions/<name>/<species>-t<tick>.json`; `--seed-with <file>...` (repeatable) adds those genomes to the founding population of a fresh world through the normal per-biome founder path, placed from `SimRng` so seeded runs stay reproducible. Success and failure land in the log, the chronicle and the Inspect panel; a broken or missing file stops the run at startup. Only `genome` is required in the file, so one can be written by hand.
- **Named sessions** — each run gets a unique cosmic three-word name; logs + screenshots + saves live in `sessions/<name>/`
- **Seed-based terrain generation** — same seed produces same terrain; the seed is saved in save files and the terrain type, elevation and light are regenerated from it on load
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
