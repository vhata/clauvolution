# Plan: phase 2, biomes as pressure and barrier, on the two-level pyramid

**Status:** planned 2026-09-25. Not started.
**Scope:** phase 2 of `docs/design/simulation-rules.md` ("Phase 2: biomes as pressure and barrier"), started on the plants-and-grazers pyramid as the design's "Pyramid-top outcome" allows. Takes `move-cost-table-by-tile`, `oceans-as-habitat` and `biome-threshold-retune` from `TODO.md`. Parks `hunter-emergence`; see "Decisions to take". Re-measures `consumer-ceiling-regulation` and `species-count-above-tuned-band` in its audits but does not work them.

## Why

The design ranks geography first: different biomes should hold different ecosystems because they cost different bodies different amounts to live in, and a barrier on the minimap should be one a lineage can be watched crossing. Phase 1 and the pyramid-top plan built the food web and left the map where phase 0 found it. Terrain still acts only on where food and light are, so the same lineages are denser where food is denser.

Three things in the code stand in the way.

- **Water is no barrier.** `action_system` picks the movement table by the tile, not the organism: on any water tile it uses `TerrainType::water_move_cost` (1.0 for both depths), and on land `land_move_cost`. The 10.0 deep-water entry in `land_move_cost` and every land entry in `water_move_cost` are never read. An organism with `aquatic_adaptation` 0 pays 1.0 per unit in deep water against 1.5 on sand, so deep water is among the cheapest terrain there is. The "Deep water 10x movement cost" entry in `docs/DECISIONS.md` describes behaviour that never ran, and no audit has shown geographic isolation.
- **Water is not habitat either.** `tile_dynamics_system` skips water tiles, so their vegetation stays at the 0.0 `Tile::from_elevation_moisture` gives them. Two corrections to what `oceans-as-habitat` says, from the code: water tiles have `light_level` 0.6 at both depths, not 50% and 30%; and `food_regeneration_system` still spawns food items on water, at probability `(vegetation + nutrients) × 0.5`, which is 0.25 on shallow and 0.15 on deep water. Plants also photosynthesise in water and their children land up to `CHILD_SPAWN_OFFSET` (5 tiles) away on any terrain, so a plant lineage can already step across water by budding without paying a movement cost.
- **The map is not built for it.** The biome thresholds in `Tile::from_elevation_moisture` (moisture 0.25 and 0.6, Rock only when dry and above elevation 0.6) predate moisture spanning 0..1, and Rock was 0 tiles on seeds 42 and 3 in the per-biome seeding counts ("Per-biome seeding" in `docs/DECISIONS.md`). Land fraction varies about twofold between seeds (171,702 land tiles on seed 42, 89,839 on seed 3), which confounds every cross-seed comparison. `generate_noise_map` interpolates a random grid without wrapping, while positions wrap with `rem_euclid`, so the world is a torus with a terrain seam at its edges. No test says how many landmasses a seed has.

The pyramid-top outcome is the other half of why now. Six steps and two hunter-bridge steps produced no hunter level, and the step 6 audit (`docs/audits/2026-09-24-pyramid-step6/`) is a stable two-level baseline: plants and grazers persist and cycle on all eight seeds, with plant floors after tick 1000 of 129 to 627. The design's sequencing wanted a pyramid for biomes to place; two levels are enough to ask whether a barrier separates them, and geography is itself a candidate for letting intermediate diets persist in refuges.

## Decisions to take

Each becomes a `docs/DECISIONS.md` entry in the pull request that implements it.

- **Water as habitat or barrier.** Settled as a knob by the pyramid-top plan ("Ocean vegetation becomes a config field"). Options for the default: (a) barrier, water vegetation 0 as now; (b) habitat on shallow water only; (c) habitat on all water. *Recommendation:* add `SimConfig.water_vegetation`, a multiplier on the `nutrients × moisture` capacity that `tile_dynamics_system` applies to water tiles, defaulting to 0 so the pull request that adds it is behaviour-neutral. Keep barrier as the default through the movement and generator steps, because those are what the done-when measures and one change at a time is the rule. Then cost (b) against (a) on the same seeds in its own step and pick the default with the numbers. Food items on water stay as they are unless that step finds them doing the work the knob is meant to do; the step records how much of the food-item flow lands on water first.
- **How movement cost reads `aquatic_adaptation`.** Options: (a) choose the table by organism, land table below a cut-off and water table above it; (b) interpolate per tile between the two tables by `aquatic_adaptation`, `land_cost + aquatic × (water_cost − land_cost)`, keeping the fin bonus on water tiles and the limb bonus on land; (c) keep choosing by tile and raise the water entries. *Recommendation:* (b). It is the design's "cost table by terrain and body" with no threshold for selection to sit on, every intermediate value is a real tradeoff, and it removes `AQUATIC_MOVE_COST_WEIGHT`, which would count the trait twice once both tables are read. From the existing tables, a founder at aquatic 0.5 (the top of the 0..0.5 founder draw in `Genome::new_minimal_with_diet`) would pay 5.5 in deep water and 3.0 on sand; at 0 it pays 10.0 and 1.5. (c) leaves the bug's shape in place. "Rock steep for large bodies" from the design is a separate rule and is not taken here.
- **Shallow shelves.** Options: (a) lower the ShallowWater elevation band so there is more shallow water everywhere; (b) classify water within a shelf width of land as ShallowWater, whatever its elevation; (c) reshape elevation near coasts so shelves come out of the noise. *Recommendation:* (b), applied after the elevation map is built in `TileMap::generate`. It puts shallow water where the habitat setting needs it, around land and nowhere else, and it is legible on the minimap. (a) also grows shallow water in open ocean; (c) changes the continents at the same time as the shelves. The width is a generator constant chosen in the step from per-seed tile counts. A consequence to watch: shelves narrow or bridge straits, and shallow water costs a land-adapted organism 3.0, not 10.0.
- **Biome thresholds, now that moisture spans 0..1.** Options: (a) leave them; (b) re-set fixed thresholds by hand against the world crate's per-biome counts on the eight seeds; (c) set thresholds at per-seed quantiles so every seed has the same biome shares. *Recommendation:* (b), with Rock decided by elevation alone so that ranges exist to split land, as the design's "split by rock" asks. Quantile biomes would remove the seed-to-seed variety that "not the same plant soup every run" wants. Sea level is a separate choice: set it at a fixed land fraction (an elevation quantile) so that land area stops varying twofold between seeds, while where the land lies and what biomes it holds still vary. This is a generator change and needs `generated_map_has_unit_moisture_and_mixed_biomes` extended to report the eight seeds.
- **What "barrier" is measured by.** Options: (a) the design's regime-separation number, the mean share of each species' members in its dominant biome; (b) the same share taken over regions, the connected components of land tiles; (c) crossing counts. *Recommendation:* all three, with regions as the primary measure of isolation, since a barrier separates places and a biome can occur on several landmasses. Regions are computed once in `TileMap::generate` by flood fill over non-water tiles with the torus wrap, and regenerated on load rather than saved, since terrain type never changes at runtime. Components under a minimum size pool into one "minor" region; the size is chosen in step 1 and recorded. The separation numbers are reported against a shuffled null (the same organisms with species labels permuted), as the design's done-when requires. A crossing is an organism standing on land of a region other than the last region it stood on; the design's chronicle event fires the first time a species has members on a region none of its members started on. A species counts as confined when its dominant-region share is above a cut-off set in step 1 from the baseline distribution and recorded, not assumed here.
- **How `hunter-emergence` stays parked, and what would reopen it.** Options: (a) park it in `TODO.md` with a progress line and keep reading hunters in every audit; (b) close it as won't-do; (c) keep working it alongside phase 2. *Recommendation:* (a). The entry stays in "Needs proof of concept" with a dated "Parked" line pointing here. The headless summary already prints hunters and omnivores per strategy, the per-band counters from the hunter-bridge plan, and founding-hunter ages, and every audit in this plan records them per seed at 5000 and 15000, so a change is seen at no cost. Reopen when any phase 2 audit shows hunters (diet at or above +1/3) alive at 15000 on some seed, or omnivores persisting after tick 3000 on most seeds, which would mean geography has made the bridge the hunter-bridge plan could not; or when phase 2 is done, since the design's order then asks for the top level again. No phase 2 knob is tuned toward hunters. The hunter-bridge plan's steps 3 and 4 stay unrun until then.

## The plan

Each step is a branch and a pull request. Every rule change gets a `docs/DECISIONS.md` entry with before and after numbers, and the Graphs tab shows the effect before tuning starts. Audits are `scripts/attractor_audit.sh` at 15000 ticks on seeds 1, 2, 3, 7, 42, 99, 314 and 1000, one run each now that headless runs are deterministic (#31), unless a step says otherwise.

### 1. Instruments, with no behaviour change

Compute region labels in the world crate as described under "What barrier is measured by", and print the region count and sizes per seed from a unit test. Add to the sim: population by biome and by region per strategy in `PopulationHistory` (history CSV, Graphs series, headless summary); organism-ticks on deep and shallow water by `aquatic_adaptation` band, and movement energy paid on water; region crossings per second; the region and biome separation numbers with their shuffled nulls; the first-species-on-a-new-region chronicle event; and the food-item flow that lands on water tiles. Show region and biome under the cursor in Inspect beside the existing aquatic value.

Done when: history CSVs on seeds 42, 3 and 7 at 5000 ticks are byte-identical to `main` with the new columns removed, and a `docs/audits/` note records the 15k baseline on the eight seeds: regions per seed, separation against the null, crossings per run, and deep-water occupancy by aquatic band. The note also sets the minor-region size and the confinement cut-off. If the baseline already shows separation well above the null on some seeds, that is a finding the note says out loud.

### 2. Movement cost reads the organism

In `action_system`, interpolate between `land_move_cost` and `water_move_cost` by `aquatic_adaptation` on every tile, keep the fin and limb bonuses and `TERRAIN_MOVE_COST_FLOOR`, and remove `AQUATIC_MOVE_COST_WEIGHT`. Correct the "Deep water 10x movement cost" DECISIONS entry in the same pull request and resolve `move-cost-table-by-tile`.

Done when: on the eight seeds at 15k, deep-water organism-ticks by low-aquatic organisms and crossings are both recorded against step 1, the aquatic distribution of consumers at 15000 is recorded, and plants and grazers persist on every seed. If crossings do not fall, the entry says what the movement energy budget of a grazer is against the width of the channels it crosses, rather than turning the table up.

### 3. Sea level and biome thresholds

Set sea level at a fixed land fraction, make Rock an elevation band, and re-set the moisture thresholds against the per-biome counts, as recommended above. Fix the torus seam in `generate_noise_map` in the same pull request if step 1's regions show components joined or cut by it; otherwise record that it does not matter at this scale. Resolve `biome-threshold-retune`.

Done when: the world unit test prints per-biome counts for the eight seeds and asserts every land biome, Rock included, on the default seed; the audit at 15k records per-biome population and the separation numbers against step 2; and saves made before the change are documented as loading a different map (terrain regenerates from the seed).

### 4. Continents and shallow shelves

If step 3 leaves the default seed with fewer than two large land regions, tune the elevation noise toward several masses (a low-frequency term or a coarser first octave), with the design's unit test asserting at least two large passable land regions on the default seed. Then classify water within the shelf width of land as ShallowWater. The two may be one pull request if the first is not needed.

Done when: the unit test asserts the region count and prints shelf tile counts per seed; the audit at 15k records crossings and separation against step 3, and whether shelves bridged straits that deep water had closed.

### 5. Water vegetation as a knob, costed

Add `SimConfig.water_vegetation` and a CLI flag, default 0. Run the audit at 0 and at a shallow-only habitat setting (the knob applied to ShallowWater, deep water left barren), with shallow water added to `FOUNDING_BIOMES` only in the habitat run. Resolve `oceans-as-habitat` with the default chosen from the two runs.

Done when: the DECISIONS entry records, per seed, plant and grazer counts by biome, separation against the null, crossings, and plant floors for both settings, and says which is the default and why. Plant extinction on any seed in either setting is a stop condition for that setting, not a tuning target.

### 6. Heat tolerance

The pressure half of the design: a `heat_tolerance` trait in -1..1, a per-tile heat from biome and elevation (the existing `Tile.temperature` is elevation-only and read only by the ice age), and a metabolism multiplier on the distance between them. Inspect shows the trait; the speciation body term gains it, so the threshold sweep in step 7 is required. The heat per biome and the multiplier's steepness are the design's own tuning pass and are swept once here.

Done when: the audit at 15k records, per seed, mean tolerance by biome and whether the biome separation number rose against step 5; the genome save default is documented; and the steepness sweep is in the DECISIONS entry.

### 7. Threshold sweep, audit and outcome

Re-sweep the compatibility threshold at 15k on the current traits (the design's risk list asks for this after each phase; `species-count-above-tuned-band` is re-measured by it). Run the final audit, write "Phase 2 outcome" into the design doc and a dated block in the roadmap's attractor-states section, and update `consumer-ceiling-regulation` and the parked `hunter-emergence` with the numbers.

Done when: on most of the eight seeds at 15000 ticks, at least two biomes have different dominant species, region separation is well above the shuffled null, and at least one crossing event appears in the chronicle per run; or the design doc says which of the three failed and why, with the numbers.

## What done looks like for the whole plan

Deep water costs what its body says it costs; the map has several landmasses with shelves around them and every biome on it; the Graphs tab shows population by region and biome and a separation number beside its null; and the phase 2 done-when is met or its failure is written down with measurements. Whether water is habitat is decided by a run, not by argument. Hunters are read on every audit and nothing has been tuned for them.

## Not in scope

- `hunter-emergence`, the hunter-bridge plan's steps 3 and 4, and any founder-claw or output-bias lever. Parked above.
- `consumer-ceiling-regulation` as work. Its numbers are re-read at each audit; regions may change them in either direction.
- A bigger world. The design keeps 512 until separation is measured at 512.
- Movement cost by body size on rock, and any rule that forbids a tile. The first is a later tuning of the table; the second is an explicit dispersal limit, rejected in the decisions table.
- Any rule that reads a trait to decide where an organism goes, or seeds a species into a region. Programmed behaviour.
- Plant defences and climate shift (phase 3).

## Open questions

- **Plants cross water by budding.** A child lands up to 5 tiles from its parent on any terrain, and plants photosynthesise in water at light 0.6, so the movement fix does not touch plant dispersal. Whether that is a problem depends on step 2's crossing counts split by strategy; if plants make most of the crossings, the question is whether a child placed on a tile its genome would pay heavily to move on should cost the parent more, which is a new rule and a new decision.
- **Food items on water.** They spawn there today at up to a quarter of the land rate. If step 1 shows grazers living on them in water, step 5's barrier setting is not a barrier and the knob may need to cover them.
- **Shelves and straits.** Whether a shelf that bridges two landmasses is a feature (a shallow crossing that selects for moderate aquatic values) or defeats the barrier. Step 4 measures it; the width is the lever.
- **Sea level by quantile.** It makes seeds comparable, and it also makes every seed the same amount of land, which removes one kind of variety. Revisit if the audits show land fraction was doing something interesting.
- **Tick cost.** Separate regions may hold more organisms in total. The ceiling engagements and wall time per audit are recorded so `sensing-per-organism-cost` can be re-prioritised if needed.

## References

- `docs/design/simulation-rules.md`: decisions table, "Phase 0 outcome", "Pyramid-top outcome", "Phase 2".
- `TODO.md`: `move-cost-table-by-tile`, `oceans-as-habitat`, `biome-threshold-retune`, `hunter-emergence`, `consumer-ceiling-regulation`, `species-count-above-tuned-band`.
- `docs/DECISIONS.md`: "Terrain noise ranges", "Per-biome seeding", "Deep water 10x movement cost", "Emergent carrying capacity".
- `crates/clauvolution_world/src/lib.rs`: `TerrainType::land_move_cost`, `water_move_cost`, `Tile::from_elevation_moisture`, `TileMap::generate`, `generate_noise_map`, `tile_dynamics_system`, `food_regeneration_system`.
- `crates/clauvolution_sim/src/lib.rs`: `action_system` (terrain cost), `photosynthesis_system`, `reproduction_system` (`CHILD_SPAWN_OFFSET`), `spawn_initial_population` (`FOUNDING_BIOMES`).
- `plans/2026-09-21-pyramid-top.md`: the ocean-vegetation and shelf decisions. `plans/2026-09-24-hunter-bridge.md`: what is parked.
- `docs/audits/2026-09-24-pyramid-step6/`: the baseline.
