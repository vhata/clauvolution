# Phase 2 step 4: continents and shelves, 15k ticks

Step 4 of `plans/2026-09-25-phase2-biomes.md`. The terrain generator in `crates/clauvolution_world/src/lib.rs` changed in two ways, and the reasoning is in `docs/DECISIONS.md` ("Continents from seeded centres, shelves by distance to land"):

- **Continents.** `CONTINENTS` (4) centres are drawn on the torus at least `CONTINENT_MIN_SPACING` (0.3 of the width) apart. Each tile's elevation noise gets a continent term: `CONTINENT_WEIGHT` (0.8) times a smoothstep of the tile's distance to the nearest Voronoi border between centres, reaching full height `CONTINENT_RAMP_TILES` (48) in from the border. The distances are taken from the tile displaced by up to `CONTINENT_WARP_TILES` (48) by two seamless noise maps, so the borders wander. Sea level is still the quantile that leaves `LAND_FRACTION` (0.40) as land. The noise itself is the seamless noise from #89, unchanged.
- **Shelves.** Water within `SHELF_WIDTH` (8) 4-neighbour steps of land, with the torus wrap, is ShallowWater; all other water is DeepWater, whatever its elevation. `DEEP_WATER_BELOW` is gone.

`TERRAIN_GENERATOR_VERSION` is now 3. Moisture and the elevation noise draw from the terrain RNG first, in the same order as before, so each seed's moisture map is unchanged; the centres and warp maps are drawn after them.

Run details:

- **Commit:** b438aca on `roadmap/phase2-continents`. Later commits on the branch change only documentation.
- **Seeds:** 1, 2, 3, 7, 42, 99, 314, 1000, one run each (headless runs are deterministic), 15000 ticks, no overrides. Three at a time; wall times in the summaries are not comparable across audits.
- **How to repeat:** `cargo build --release`, then for each seed `./target/release/clauvolution --headless 15000 --seed S --dump-history seedS-run1.csv > seedS-run1.txt 2>&1`. The map counts come from `cargo test -p clauvolution_world -- --nocapture` (`regions_on_the_audit_seeds` and `generated_map_has_unit_moisture_and_mixed_biomes`). The settings sweep is `cargo test --release -p clauvolution_world sweep_continent_settings -- --ignored --nocapture` with comma-separated `CONTINENTS`, `WEIGHT`, `RAMP`, `WARP` and `SHELF`.
- **Before:** `docs/audits/2026-09-25-phase2-sea-level/` (step 3, same seeds and ticks). Definitions of regions, crossings, separation and aquatic bands are in the step 1 baseline note, `docs/audits/2026-09-25-phase2-baseline/`.
- **Reading cycles:** read from the CSVs at 120-tick resolution (every fourth 30-tick row), because 1500-tick samples alias with the 1,800-tick season. Every number marked "before" below was recomputed from the step 3 CSVs the same way, so a few differ slightly from the step 3 note (its biome shares, for example, round differently).

## Choosing the settings

Swept on the eight seeds' generated maps (`sweep_continent_settings`), counting the seeds with at least two landmasses of 10,000 tiles or more, and the seeds where a shelf joins two major regions (a component of land and ShallowWater tiles that holds more than one major land region):

- **Without warping** the coasts follow the ramp contour and run nearly straight along the cell borders. Warping by 48 tiles bends them without changing how many landmasses there are.
- **Weight** (4 continents, ramp 48, warp 48, shelf 8). At 0.4, 5 of 8 seeds have two large landmasses (seed 3 stays one, seed 42 is one of 92,855 tiles and islands) and shelves bridge regions on 3 seeds. At 0.6 all eight seeds have two large landmasses, but shelves bridge on 2 seeds, and with ramp 64 only 5 of 8 seeds have two large landmasses. At 0.8 and 1.0 every seed has two or more large landmasses and no shelf bridges. At 1.0 the continents are nearly the cells' interiors and the noise does little to their shape. 0.8 was taken, the lowest weight with no bridges.
- **Continent count** (weight 0.8). 3, 4 and 5 all give two or more large landmasses on every seed with no shelf bridge. 3 gives three landmasses per seed; 5 gives five or six on some, with the smallest under 5,000 tiles (seed 3: 4,432 and 2,351). 4 was taken, as the middle.
- **Shelf width.** At weight 0.8, ramp 48 and warp 48, no shelf joins two regions on any seed up to a width of 12. The first bridge appears at 14, on seeds 3 and 99; at 16 five seeds are bridged and at 20 six. **8 was taken**, well under the first bridge, so the narrowest strait on the eight seeds still has deep water across it for at least a few tiles beyond two shelf widths. The cost of the choice is that shelves are narrow: 12% to 15% of the water is shallow, against 25% to 78% before.

## The maps

| seed | major regions (tiles), before -> after | deep / shallow tiles, before -> after | deep share of the water | Sand / Grassland / Forest / Rock, % of land, after |
|---|---|---|---|---|
| 1 | 1 (104,858) -> 4 (37,562; 29,751; 28,573; 8,972) | 86,951 / 70,335 -> 137,731 / 19,555 | 55% -> 87.6% | 27.8 / 31.2 / 24.8 / 16.2 |
| 2 | 2 (95,629; 9,160) + 69 minor -> 5 (43,221; 27,145; 17,275; 11,961; 5,256) | 70,280 / 87,006 -> 133,706 / 23,580 | 45% -> 85.0% | 27.2 / 32.2 / 24.0 / 16.7 |
| 3 | 1 (104,858) -> 4 (39,010; 30,763; 17,701; 17,384) | 88,916 / 68,370 -> 138,249 / 19,037 | 57% -> 87.9% | 19.4 / 68.8 / 4.3 / 7.5 |
| 7 | 3 (100,040; 2,484; 1,550) + 784 minor -> 4 (31,205; 25,100; 24,739; 23,814) | 34,742 / 122,544 -> 135,263 / 22,023 | 22% -> 86.0% | 12.2 / 54.2 / 25.4 / 8.2 |
| 42 | 1 (104,858) -> 4 (46,078; 34,464; 19,169; 5,097) + 50 minor | 104,021 / 53,265 -> 136,354 / 20,932 | 66% -> 86.7% | 24.2 / 27.6 / 31.5 / 16.8 |
| 99 | 1 (104,858) -> 4 (40,707; 35,245; 25,447; 3,459) | 95,214 / 62,072 -> 139,248 / 18,038 | 61% -> 88.5% | 12.4 / 23.5 / 44.1 / 20.0 |
| 314 | 1 (104,858) -> 5 (39,781; 30,516; 16,617; 15,910; 1,962) + 72 minor | 117,922 / 39,364 -> 133,998 / 23,288 | 75% -> 85.2% | 17.9 / 30.7 / 38.7 / 12.7 |
| 1000 | 1 (104,858) -> 4 (36,252; 26,911; 21,940; 19,755) | 60,942 / 96,344 -> 136,934 / 20,352 | 39% -> 87.1% | 8.3 / 43.7 / 45.5 / 2.6 |

Every seed now has four or five major regions, three or four of them over 10,000 tiles, where step 3 left five seeds as one landmass. Seed 42, the default, has two regions over 30,000 tiles, which `regions_on_the_audit_seeds` asserts (at least two of 10,000 tiles or more). No shelf joins two major regions on any seed, which the same test asserts on all eight. Minor components nearly vanished (50 and 72 tiles on two seeds): the continent term leaves few islands.

The deep share of the water went from 22%..75% to 85.0%..88.5%. That resolves `deep-water-share-varies-by-seed`: depth is now a distance from the coast, so the shallow share depends on coast length, not on how deep one trench is.

Biome shares of land moved with the continents: Rock is 2.6% to 20.0% of land (3.8% to 20.8% before), and the eight-seed means are Sand 19%, Grassland 39%, Forest 30%, Rock 13% (17, 39, 32 and 11 before). Seed 3's land is 69% Grassland and 4% Forest, because moisture is the same map as before and its continents fell on the drier part of it.

## The runs

| seed | region separation / null | biome separation / null | confined species, observed / null (of species counted) | consumer / plant crossings | new-region events | final plants / grazers | plant floor after t1000 | grazers after t1000 | ceiling samples (whole run; from t9000) | species at 5k / 15k | omnivores / hunters at 5k; 15k | last hunter tick |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 1.000/1.000 -> **0.525/0.409** | 0.362/0.299 -> 0.475/0.453 | 0 -> 0.7/0.0 (17.5) | 0/0 -> 57,808/26,419 | 0 -> 260 | 3087/2490 -> 4128/1869 | 756 -> 769 | 511..3059 -> 428..3162 | 74; 53 -> 114; 68 | 32/48 -> 27/34 | 0/0; 0/0 -> 0/0; 0/0 | 13711 -> 2011 |
| 2 | 0.924/0.925 -> **0.535/0.450** | 0.403/0.345 -> 0.463/0.423 | 16.1/16.7 (22.0) -> 0.7/0.0 (18.5) | 13,482/2,390 -> 63,964/17,833 | 70 -> 324 | 478/5522 -> 3899/2101 | 150 -> 748 | 785..5716 -> 402..2685 | 143; 121 -> 112; 80 | 33/41 -> 34/53 | 0/0; 0/0 -> 0/0; 0/0 | 12211 -> 8401 |
| 3 | 1.000/1.000 -> **0.462/0.405** | 0.391/0.339 -> 0.502/0.482 | 0 -> 0.2/0.0 (21.5) | 0/0 -> 91,663/33,902 | 0 -> 136 | 3564/2433 -> 4456/1542 | 1523 -> 1761 | 572..3816 -> 443..2776 | 232; 123 -> 290; 140 | 33/66 -> 16/56 | 0/0; 1/0 -> 2/0; 2/0 | 10291 -> 14281 |
| 7 | 0.966/0.969 -> **0.461/0.359** | 0.459/0.393 -> 0.500/0.478 | 13.3/14.0 (14.5) -> 0.4/0.0 (19.7) | 21,594/4,175 -> 122,923/27,930 | 190 -> 247 | 1932/2145 -> 3360/2635 | 1326 -> 1407 | 391..2877 -> 623..3510 | 35; 26 -> 215; 106 | 21/50 -> 34/43 | 0/0; 0/0 -> 0/0; 0/0 | 12061 -> 13501 |
| 42 | 1.000/1.000 -> **0.543/0.472** | 0.439/0.356 -> 0.479/0.468 | 0 -> 0.5/0.0 (21.7) | 0/0 -> 82,275/32,613 | 0 -> 244 | 3181/2123 -> 4291/1709 | 494 -> 2542 | 453..3058 -> 416..2594 | 71; 61 -> 258; 116 | 46/48 -> 27/57 | 1/0; 0/0 -> 0/0; 0/0 | 481 -> 13051 |
| 99 | 1.000/1.000 -> **0.524/0.432** | 0.382/0.319 -> 0.475/0.449 | 0 -> 0.4/0.0 (14.1) | 0/0 -> 71,311/19,751 | 0 -> 181 | 3169/2831 -> 3197/2520 | 245 -> 1141 | 626..3497 -> 445..3208 | 109; 67 -> 80; 54 | 34/59 -> 21/22 | 0/0; 0/0 -> 0/0; 2/0 | 11371 -> 7681 |
| 314 | 1.000/1.000 -> **0.492/0.417** | 0.424/0.390 -> 0.468/0.441 | 0 -> 0.3/0.0 (16.4) | 0/0 -> 83,142/21,408 | 0 -> 288 | 3436/2562 -> 4109/1889 | 1476 -> 1463 | 413..3284 -> 500..2953 | 145; 98 -> 191; 108 | 44/58 -> 35/46 | 0/0; 0/0 -> 0/0; 0/0 | 1501 -> 601 |
| 1000 | 1.000/1.000 -> **0.483/0.386** | 0.420/0.365 -> 0.482/0.436 | 0 -> 0.4/0.0 (15.4) | 0/0 -> 48,783/15,547 | 0 -> 241 | 3517/2483 -> 3218/2462 | 709 -> 1027 | 660..3507 -> 492..2889 | 231; 106 -> 45; 45 | 24/54 -> 17/39 | 0/0; 0/0 -> 0/0; 0/0 | 541 -> 631 |

Separation figures are means over the 1 Hz samples from tick 1000. Ceiling samples are 30-tick rows at 5,990 organisms or more.

Share of organisms by biome, mean of the 30-tick rows from tick 1000 (deep / shallow / Sand / Grassland / Forest / Rock, %):

| seed | before | after |
|---|---|---|
| 1 | 24.6 / 24.2 / 13.5 / 19.3 / 14.1 / 4.3 | 45.0 / 7.8 / 12.8 / 15.1 / 12.0 / 7.4 |
| 2 | 22.9 / 31.9 / 5.0 / 12.9 / 20.7 / 6.6 | 41.8 / 9.0 / 12.8 / 15.3 / 12.9 / 8.2 |
| 3 | 28.0 / 25.2 / 6.4 / 28.2 / 3.6 / 8.6 | 47.3 / 7.4 / 8.1 / 31.6 / 2.2 / 3.4 |
| 7 | 9.0 / 36.9 / 14.5 / 26.2 / 9.7 / 3.7 | 47.0 / 8.6 / 5.2 / 24.4 / 11.3 / 3.5 |
| 42 | 33.3 / 18.9 / 4.1 / 13.1 / 21.0 / 9.8 | 46.6 / 8.3 / 10.7 / 12.5 / 14.1 / 7.8 |
| 99 | 28.6 / 22.6 / 12.0 / 12.8 / 20.3 / 3.7 | 44.4 / 7.1 / 5.9 / 11.2 / 21.8 / 9.5 |
| 314 | 37.4 / 15.1 / 5.3 / 19.9 / 18.3 / 4.1 | 43.8 / 9.2 / 8.7 / 14.0 / 17.8 / 6.6 |
| 1000 | 18.6 / 34.6 / 8.4 / 19.3 / 17.2 / 1.9 | 42.6 / 8.0 / 3.8 / 20.9 / 23.5 / 1.3 |

Plants / grazers by biome at tick 15000 (deep, shallow, Sand, Grassland, Forest, Rock), after:

| seed | plants | grazers |
|---|---|---|
| 1 | 1778, 274, 697, 575, 399, 405 | 862, 152, 160, 320, 300, 75 |
| 2 | 1621, 324, 571, 576, 356, 451 | 885, 219, 175, 367, 375, 80 |
| 3 | 2206, 329, 366, 1317, 92, 146 | 664, 120, 88, 597, 50, 23 |
| 7 | 1698, 260, 268, 769, 208, 157 | 1102, 220, 103, 704, 442, 64 |
| 42 | 2027, 371, 528, 476, 517, 372 | 762, 161, 127, 243, 334, 82 |
| 99 | 1298, 268, 260, 346, 574, 451 | 1059, 186, 94, 302, 745, 134 |
| 314 | 1874, 345, 429, 521, 573, 367 | 782, 178, 114, 315, 429, 71 |
| 1000 | 1522, 245, 168, 685, 546, 52 | 764, 205, 51, 602, 830, 10 |

Deep-water share of consumer organism-ticks by aquatic band, aq0 / aq1 / aq2 / aq3 (bands in brackets held under 100,000 consumer organism-ticks over the run):

| seed | before | after |
|---|---|---|
| 1 | 25% / 27% / 27% / 30% | 39% / 43% / 46% / (54%) |
| 2 | 22% / 21% / 24% / 26% | 28% / 39% / 44% / (48%) |
| 3 | 24% / 29% / 31% / (61%) | 44% / 48% / 43% / (-) |
| 7 | 9% / 11% / (12%) / (-) | 42% / 48% / 49% / (62%) |
| 42 | 22% / 38% / (55%) / (65%) | 43% / 45% / 49% / (56%) |
| 99 | 21% / 26% / 37% / (43%) | 41% / 41% / 44% / 46% |
| 314 | 20% / 30% / 40% / (45%) | 38% / 41% / 43% / (31%) |
| 1000 | 10% / 19% / 21% / (-) | 32% / 43% / (50%) / (-) |

Cycling at 120-tick resolution from tick 9000 (plants; grazers), and the grazer troughs after tick 1000:

| seed | plants, before -> after | grazers, before -> after | grazer peaks after t1000 | grazer troughs |
|---|---|---|---|---|
| 1 | 2,298..3,531 -> 2,245..4,128 | 662..3,044 -> 623..2,837 | 9 | 1861, 3661, ... 14461 |
| 2 | 158..1,342 -> 2,428..3,990 | 1,637..5,706 -> 609..2,654 | 9 | same |
| 3 | 2,301..3,939 -> 2,825..4,539 | 1,079..3,236 -> 785..2,668 | 8 | same |
| 7 | 1,601..3,799 -> 2,491..4,073 | 577..2,872 -> 962..3,509 | 9 | same |
| 42 | 2,456..4,042 -> 3,270..4,485 | 586..2,714 -> 739..2,579 | 9 | same |
| 99 | 2,300..3,356 -> 2,024..3,507 | 773..3,470 -> 660..3,208 | 9 | same |
| 314 | 2,437..3,742 -> 2,995..4,342 | 859..3,255 -> 688..2,530 | 9 | same |
| 1000 | 2,313..3,636 -> 1,841..3,556 | 981..3,471 -> 534..2,867 | 9 | same |

A peak is a 120-tick sample that is the highest within 600 ticks either side; the count includes the first sample after tick 1000 and the last at 15000 where they are local highs. On all eight seeds the grazer troughs fall on the same ticks, 1861 and every 1,800 ticks after it to 14461, which is the seasonal year; plants peak eight to ten times on the same period.

## Reading

**Barriers now separate by region above the null on every seed.** Region separation from tick 1000 is 0.461 to 0.543 against nulls of 0.359 to 0.472, a gap of 0.057 (seed 3) to 0.116 (seed 1); step 3 had it at its null on every seed, and at exactly 1.0 on the five single-landmass seeds. The gap tends to widen over the run: at tick 15001 it is 0.155 on seed 1, 0.201 on seed 7, 0.132 on seed 314 and 0.099 on seed 3. About half a species per sample is confined to one region at the 0.9 cut-off (0.2 to 0.7), against 0.0 under the null on every seed. The effect is real and still modest: a typical species has about half its members on its dominant region where a random split would give about 40%.

**The barrier is weak because the ocean is inhabited, not because shelves bridge it.** No shelf joins two regions on any seed (asserted by the unit test), so every crossing goes over deep water, and there are many: 48,783 to 122,923 consumer crossings per run and 15,547 to 33,902 by plants, against 0 on five seeds before. Crossings per 1000-tick window fall over the run (seed 1: 8,256 consumer crossings in the window to tick 5011, 2,661 in the window to 15001; seed 7: 11,971 to 4,009). Organisms spend 42% to 47% of their time on deep water, and the deep share rises only a little with the aquatic band (aq0 28% to 44%). Deep water is now 52% of the map's tiles and it holds food: 28.5% to 38.1% of regenerated food items land on deep water, and the share eaten by eaters on deep water matches within half a point on every seed. Plants photosynthesise there too: at tick 15000, 1,298 to 2,206 plants per seed stand on deep water, the largest single biome for plants on every seed. A grazer crossing an ocean with plants and food items in it is being paid to be there, and the 10x movement cost for aq0 does not stop it. This is the question step 5 exists to answer (water vegetation, and whether food items on water need the same knob), and the plan's open question "Food items on water" now has a sharper answer: with continents, food items on deep water are most of what joins them.

**Shelves are narrow.** Shallow water is 7.1% to 9.2% of organisms' time, against 15.1% to 36.9% before, because the shelf is 12% to 15% of the water. For step 5's shallow-only habitat setting, the habitat is a ring 8 tiles wide around each continent. If that is too little, the width can go up to about 12 before the first strait is bridged (14 bridges seeds 3 and 99).

**Plants and grazers persist and cycle on every seed.** Plant floors after tick 1000 rose on seven seeds (314 is flat, 1,476 to 1,463) and are 748 to 2,542. Seed 2's step 3 state, a small food-item grazer at the ceiling over 150 to 1,342 plants, is gone: its plants from tick 9000 run 2,428 to 3,990 and grazers 609 to 2,654, like the other seeds. Plants now outnumber grazers at 15000 on every seed (1.3 to 2.9 to 1).

**Ceiling samples** summed over the eight seeds went from 1,040 to 1,305 over the whole run, and from 655 to 717 from tick 9000. Up on seeds 1, 3, 7 (35 to 215), 42 (71 to 258) and 314; down on 2, 99 and 1000 (231 to 45).

**Species at 15k** fell on six seeds and rose on two (2 and 42). Seed 99 went from 59 to 22 and seed 1 from 48 to 34. With four landmasses one might expect more species, not fewer; one run per seed cannot say whether this is the map or the trajectory, and the threshold sweep in step 7 re-reads species counts on the final traits.

**Biome separation stays a little above its null**, 0.463 to 0.502 against 0.423 to 0.482, gaps of 0.011 to 0.046 against 0.034 to 0.083 before. Both numbers rose because deep water now holds over 40% of every species' members, so the dominant biome is often deep water.

**Hunters.** No hunter is alive at 15000 on any seed. Hunters were alive at scattered samples after tick 5000 on seeds 2, 3, 7, 42 and 99, most on seed 3 (44 samples, at most 3 at a time, last at tick 14281). Two omnivores are alive at 15000 on seeds 3 and 99; omnivores were alive at 206 and 107 samples after tick 3000 on those seeds and at 11 to 68 on the rest, so they persist on no seed in the sense of the reopen condition. The `hunter-emergence` reopen condition is not met.

**Wall time** was 131 to 172 seconds per run, against 279 to 440 in step 3, with the same three-at-a-time scheduling. That is noted, not measured: the step 3 runs overlapped other agents' work.

## Not measured here

- A second run per seed. Each row above is one trajectory; the species-count falls in particular need a second run or step 7's sweep before being read as the map's effect.
- Crossings split by aquatic band, and whether the falling crossing rate is consumers becoming land-bound or more of them staying in the ocean. The whole-run aquatic band totals are in each summary's geography block.
