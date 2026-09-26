# Phase 2 step 1 baseline: geography instruments at 15k ticks

Step 1 of `plans/2026-09-25-phase2-biomes.md`: the region, water, crossing and separation instruments added with no behaviour change, read on the eight audit seeds at 15000 ticks, one run each (headless runs are deterministic). Commit and arguments are in `README.txt`. Each `seedN-run1.txt` is the headless summary, whose last block is the geography summary; each `seedN-run1.csv` is the 1 Hz history, whose trailing columns (`ticks_all_plant_aq0` onward) are the new instruments.

Seed 314 was run again after the minor-region size was set (see below). Only its region labels changed; its trajectory and every non-region number are identical to the first run.

## Definitions

- **Region**: a connected component of land tiles, 4-neighbour, with the torus wrap. Components under `MINOR_REGION_MAX_TILES` (512) pool into one minor region. Major regions are numbered by size, 0 largest.
- **Crossing**: an organism standing on a major region other than the last major region it stood on. Water and minor regions do not reset the last region. A newborn starts on the region it is born on, so a plant budding across water is not a crossing.
- **New-region event**: a species first has a member on a major region that none of its members stood on when the species was first seen (at startup, at load, or when classification creates it). Each one is a chronicle entry.
- **Separation**: the mean over species with at least 10 members (`SEPARATION_MIN_MEMBERS`) of the share of the species' members in its dominant place. Region separation counts organisms on major regions; biome separation counts every organism and treats deep and shallow water as biomes. The **null** is the same number with species labels shuffled over the same organisms, from an `StdRng` seeded from the tick, not from `SimRng`. Run figures are means over the 1 Hz samples from tick 1000.
- **Food items spawned / eaten**: the summaries' "Food items spawned" line (renamed "Food items regenerated" after these runs, same count) and the `food_spawned_*` columns count items placed by `food_regeneration_system` only, not the initial stock or nutrient-rain bursts. `food_eaten*` counts items eaten from any source by any organism that eats one; the eater's tile is taken after its move, while organism-ticks on water use the tile before the move.
- **Aquatic bands**: `aquatic_adaptation` in quarters, aq0 0..0.25 up to aq3 0.75..1. Founders draw 0..0.5.

## Baseline

| seed | land tiles | water share of tiles | major regions (tiles) | final plants / grazers | plant floor after 1000 | omnivores / hunters at 5000; 15000 | crossings plant / consumer (per second) | new-region events | region separation / null | biome separation / null | deep-water share of consumer organism-ticks, aq0 / aq1 / aq2 | share of all organism-ticks on water | regenerated food items landing on water |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 117,353 | 55% | 1 (117,353) | 3336 / 2663 | 597 | 0 / 0; 0 / 0 | 0 / 0 (0) | 0 | 1.000 / 1.000 | 0.408 / 0.350 | 31% / 38% / 36% | 47% | 55.2% |
| 2 | 82,961 | 68% | 3 (38,388; 22,473; 22,100) | 2904 / 3096 | 447 | 0 / 0; 0 / 0 | 16,069 / 101,425 (235) | 224 | **0.559 / 0.488** | 0.410 / 0.368 | 30% / 33% / 34% | 62% | 51.4% |
| 3 | 89,839 | 66% | 2 (74,952; 14,849), one 38-tile minor | 2294 / 2755 | 267 | 1 / 0; 0 / 0 | 5,554 / 59,046 (129) | 112 | 0.851 / 0.844 | 0.422 / 0.381 | 34% / 35% / 39% | 57% | 40.8% |
| 7 | 82,074 | 69% | 3 (75,331; 4,667; 2,076) | 1725 / 3443 | 351 | 0 / 0; 0 / 0 | 4,231 / 33,502 (75) | 187 | 0.932 / 0.937 | 0.411 / 0.356 | 33% / 27% / 27% | 58% | 43.1% |
| 42 | 171,702 | 35% | 1 (171,702) | 3131 / 2869 | 296 | 0 / 0; 0 / 0 | 0 / 0 (0) | 0 | 1.000 / 1.000 | 0.408 / 0.360 | 7% / 7% / 7% | 31% | 20.7% |
| 99 | 70,296 | 73% | 1 (70,296) | 2229 / 2495 | 627 | 0 / 0; 0 / 0 | 0 / 0 (0) | 0 | 1.000 / 1.000 | 0.576 / 0.563 | 57% / 59% / 59% | 68% | 59.5% |
| 314 | 152,335 | 42% | 3 (149,946; 1,399; 990) | 2718 / 2880 | 129 | 0 / 0; 0 / 0 | 2,195 / 28,704 (62) | 202 | 0.986 / 0.986 | 0.414 / 0.372 | 23% / 21% / 17% | 35% | 30.2% |
| 1000 | 104,711 | 60% | 1 (104,711) | 2231 / 3213 | 562 | 0 / 0; 0 / 0 | 0 / 0 (0) | 0 | 1.000 / 1.000 | 0.467 / 0.395 | 42% / 35% / 34% | 49% | 33.5% |

Plants' per-band figures and aq3 are in each summary; aq3 held under 0.5% of organism-ticks on every seed.

## Reading

**Most seeds have one landmass.** Four seeds (1, 42, 99, 1000) are a single land component, and seed 314 is one component of 150k tiles with two islets under 1% of the land. On those seeds region separation is 1.0, or equal to its null, by construction, and there is nothing to cross. Only seeds 2, 3 and 7 have a second region above 4,000 tiles, and only seed 2 has more than one large one. This is step 4's question (continents) arriving early: the current generator does not make several landmasses on most seeds.

**Seed 2 already separates by region above the null.** Its three landmasses are of similar size, and the region separation from tick 1000 is 0.559 against a null of 0.488. The gap grows over the run: at the thousand-tick samples from 2000 to 8000 it is near the null (0.48 to 0.55 against 0.45 to 0.55), and from 9000 on it is 0.55 to 0.66 against 0.43 to 0.52. A few species at a time are confined (0.8 per sample against 0.0 under the null). Seed 3 is at its null within noise (0.851 against 0.844) and seeds 7 and 314 are at it. The separation on seed 2 happens with water costing a land organism less than sand, so it is not the barrier the plan intends; it is what 235 crossings per second across comparable landmasses leave.

**Water is habitat, not a barrier, and aquatic adaptation makes no difference to where organisms go.** Organisms spend 31% to 68% of their organism-ticks on water, 0.82 to 0.93 of water's share of the map. The deep-water share of consumer organism-ticks is flat across aquatic bands on every seed (for example 30%, 33% and 34% for aq0, aq1 and aq2 on seed 2; 7% for each band on seed 42): low-aquatic consumers are in deep water as often as higher-aquatic ones. That is the movement bug the plan describes, measured: step 2's done-when compares against the aq0 column.

**Food items on water are doing the work the vegetation knob is meant to do.** 21% to 60% of regenerated food items land on water, and the share of food items eaten by eaters standing on water is within half a point of that on every seed. The two shares are not on the same base: the spawn count is regeneration only (`food_regeneration_system`; the initial stock and nutrient-rain bursts are not counted), while the eaten count covers items from every source. Over 15000 ticks regeneration is almost all of the supply in a headless run (nutrient rain is only triggered by hand; items eaten exceed items regenerated by 1% to 2% on each seed, which is the initial stock), so the match still says consumers are feeding in water, on food items, at vegetation 0; it is not an exact ledger of where each item went. This answers the plan's open question "Food items on water": in step 5, a barrier setting that leaves food-item spawning on water as it is will not be a barrier, and the knob, or a separate one, needs to cover food items.

**Biome separation is a little above the null everywhere** (0.408 to 0.576 against 0.350 to 0.563), with no species confined to a biome at the 0.9 cut-off. Seed 99's higher figures, observed and null alike, follow from its map: 73% of its tiles are water and 57% to 59% of consumer organism-ticks are on deep water, so one biome holds most of every species.

**Crossings are dominated by consumers** (86% to 93% of crossings on the four seeds with crossings). Plants cross by moving, not only by budding: 2,195 to 16,069 plant crossings per run.

**Hunters:** none alive at 5000 or 15000 on any seed; one omnivore on seed 3 at tick 5000 and none at 15000. The `hunter-emergence` reopen condition is not met.

**Plant floors** after tick 1000 (129 to 627) match the pyramid step 6 baseline, as expected with no behaviour change.

## Cut-offs set from this baseline

- **Minor-region size: 512 tiles** (`MINOR_REGION_MAX_TILES`). Across the eight seeds the land components are either 38 tiles (one, on seed 3, holding a median of 0 organisms and at most 3) or 990 tiles and larger. The two islets on seed 314 (1,399 and 990 tiles) held medians of 20 and 14 organisms from tick 1000 (10th to 90th percentile 8 to 36 and 5 to 29), close to the 2,076-tile region on seed 7 (median 29, 13 to 47), so they are regions in their own right. The first seed 314 run used 2,048 and pooled them; it was run again at 512.
- **Confinement cut-off: 0.9** (`CONFINEMENT_CUTOFF`), applied to regions and biomes. On seed 2, the one seed whose largest region holds under 80% of the land, the shuffled null put 2 of 7,495 species-samples at or above 0.9 (the null's dominant shares there run 0.30 to 0.85). Where one region holds most of the land (seeds 3, 7, 314) the null itself sits mostly above 0.9, so a confined count only means something beside its `confined_null` column, which the instruments report next to it.

## The torus seam

The plan's step 3 depends on whether the torus seam joins landmasses. `regions_on_the_audit_seeds` also runs the flood fill without the wrap (same 512-tile minor cut-off); run it with `--nocapture` to read the numbers. Major regions with the wrap against without it:

| seed | with the wrap | without the wrap |
|---|---|---|
| 1 | 1 (117,353) | 2 (102,689; 14,664) |
| 2 | 3 (38,388; 22,473; 22,100) | 5 (38,388; 20,299; 11,289; 10,811; 2,174) |
| 3 | 2 (74,952; 14,849) + 38-tile minor | 5 (38,944; 20,657; 14,849; 12,180; 3,171) + 38-tile minor |
| 7 | 3 (75,331; 4,667; 2,076) | 4 (73,894; 4,595; 2,076; 1,217) + 2 minor (292 tiles) |
| 42 | 1 (171,702) | 1 (171,702) |
| 99 | 1 (70,296) | 2 (47,908; 22,388) |
| 314 | 3 (149,946; 1,399; 990) | 3 (149,946; 1,399; 990) |
| 1000 | 1 (104,711) | 2 (58,223; 46,488) |

The seam joins landmasses on six of the eight seeds and cuts nothing on any (a wrapping fill can only merge components). On seeds 1, 99 and 1000 it turns what would be two landmasses into one, and on seeds 2 and 3 it merges pieces into the large regions. Only seeds 42 and 314 are unaffected. So the seam is part of why most seeds have a single landmass, which is the condition step 3 was waiting on.

## Not measured here

- Wall time: runs overlapped other agents' work (three at a time), so per-run times in the summaries are not comparable with earlier audits.
