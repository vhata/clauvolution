# Phase 2 step 3: sea level, biome thresholds and the torus seam, 15k ticks

Step 3 of `plans/2026-09-25-phase2-biomes.md`. The terrain generator changed in three ways, all in `crates/clauvolution_world/src/lib.rs`, and the reasoning is in `docs/DECISIONS.md` ("Sea level at a fixed land fraction, Rock as an elevation band, seamless noise"):

- **Sea level at a fixed land fraction.** `TileMap::generate` puts sea level at the elevation quantile that leaves `LAND_FRACTION` (0.40) of the tiles as land and stretches elevation piecewise onto -1..0 below it and 0..1 above it. Deep water is below -0.3 on that scale.
- **Rock as an elevation band.** Land at or above `ROCK_ABOVE` (0.75) is Rock whatever its moisture. Below it, Sand under moisture 0.25, Forest at 0.6 and above, Grassland between (unchanged cut-offs).
- **Seamless noise.** Each octave's grid in `generate_noise_map` wraps, so the terrain runs smoothly across the torus edges.

Run details:

- **Commit:** d878820 on `roadmap/phase2-sea-level` (the seamless-noise commit), stacked on the step 2 movement branch (#87). The save-format commit after it does not touch the simulation. The runs were made at 48f86d4, the same commit before a rebase; the only difference is #87's review fix (an aquatic clamp in `terrain_move_cost`, a no-op for in-range genomes), and the reviewer's 5000-tick rerun of seed 7 at the PR head was byte-identical.
- **Seeds:** 1, 2, 3, 7, 42, 99, 314, 1000, one run each (headless runs are deterministic), 15000 ticks, no overrides. Three at a time; wall times in the summaries are not comparable across audits.
- **How to repeat:** `cargo build --release`, then for each seed `./target/release/clauvolution --headless 15000 --seed S --dump-history seedS-run1.csv > seedS-run1.txt 2>&1`. The map counts come from `cargo test -p clauvolution_world -- --nocapture` (`generated_map_has_unit_moisture_and_mixed_biomes` and `regions_on_the_audit_seeds`).
- **Before:** `docs/audits/2026-09-25-phase2-movement/` (step 2, same seeds and ticks, old generator). Definitions of regions, crossings, separation, aquatic bands and "at ceiling" are in that note and in the step 1 baseline note.
- **Reading cycles:** the plants/grazers table below samples every 1500 ticks, which can alias with the 1800-tick seasonal cycle (a sample lands 300 ticks earlier in the season each time, so several in a row can sit near the same phase). Cycling is read from the CSVs at 120-tick resolution instead.

## The maps

| seed | land tiles (share) | deep / shallow tiles | Sand / Grassland / Forest / Rock, % of land | major regions (tiles) |
|---|---|---|---|---|
| 1 | 117,353 (44.8%) -> 104,858 (40.0%) | 102,608 / 42,183 -> 86,951 / 70,335 | 48.6 / 27.1 / 3.0 / 21.3 -> 25.8 / 38.6 / 27.4 / 8.2 | 1 (117,353) -> 1 (104,858) |
| 2 | 82,961 (31.6%) -> 104,858 | 96,991 / 82,192 -> 70,280 / 87,006 | 15.1 / 64.0 / 20.9 / 0.0 -> 12.0 / 28.9 / 44.1 / 15.0 | 3 (38,388; 22,473; 22,100) -> 2 (95,629; 9,160) + 69-tile minor |
| 3 | 89,839 (34.3%) -> 104,858 | 112,225 / 60,080 -> 88,916 / 68,370 | 1.9 / 54.9 / 43.2 / 0.0 -> 14.5 / 59.5 / 7.1 / 18.9 | 2 (74,952; 14,849) -> 1 (104,858) |
| 7 | 82,074 (31.3%) -> 104,858 | 99,447 / 80,623 -> 34,742 / 122,544 | 0.3 / 37.1 / 62.6 / 0.0 -> 25.6 / 49.9 / 18.0 / 6.5 | 3 (75,331; 4,667; 2,076) -> 3 (100,040; 2,484; 1,550) + 784 minor |
| 42 | 171,702 (65.5%) -> 104,858 | 22,201 / 68,241 -> 104,021 / 53,265 | 15.7 / 46.2 / 38.1 / 0.0 -> 8.4 / 27.8 / 43.0 / 20.8 | 1 (171,702) -> 1 (104,858) |
| 99 | 70,296 (26.8%) -> 104,858 | 162,503 / 29,345 -> 95,214 / 62,072 | 33.5 / 35.6 / 21.4 / 9.4 -> 24.8 / 26.2 / 41.6 / 7.4 | 1 (70,296) -> 1 (104,858) |
| 314 | 152,335 (58.1%) -> 104,858 | 63,432 / 46,377 -> 117,922 / 39,364 | 33.5 / 52.8 / 11.6 / 2.1 -> 10.8 / 41.6 / 39.2 / 8.4 | 3 (149,946; 1,399; 990) -> 1 (104,858) |
| 1000 | 104,711 (39.9%) -> 104,858 | 122,035 / 35,398 -> 60,942 / 96,344 | 5.7 / 37.1 / 56.5 / 0.7 -> 17.7 / 40.9 / 37.6 / 3.8 | 1 (104,711) -> 1 (104,858) |

Land area went from a spread of 70,296 to 171,702 tiles to 104,858 on every seed. Rock went from absent on four seeds to 3.8% to 20.8% of land on all eight. Across the eight seeds the mean land shares are now Sand 17%, Grassland 39%, Forest 32%, Rock 11%, against 19%, 44%, 32% and 4% before; the per-seed spread is still wide (Forest 7% to 44%, Grassland 26% to 60%), which is the seed-to-seed variety the plan keeps by not using per-seed quantile biomes.

### How the thresholds were chosen

Swept on the eight seeds' generated maps at land fraction 0.40, with Rock at 0.65, 0.70 and 0.75, Sand under 0.25, 0.30 and 0.35 and Forest from 0.55, 0.60 and 0.65 (mean share over the eight seeds, with the range):

- Rock 0.65: 18% (6..30). Rock 0.70: 14% (5..25). **Rock 0.75: 11% (4..21).**
- Sand 0.25: 17% (8..26); 0.30: 23% (11..34); 0.35: 29% (16..42).
- Forest 0.55: 38% (11..50); 0.60: 32% (7..44); 0.65: 26% (3..38).

Rock at 0.75 is the highest ground on each seed without covering a fifth of the land on average (Rock carries nutrients 0.1, so it is barren). Moisture 0.25 and 0.6 kept the mean productive mix close to what it was, with Rock taken mostly out of what would have been Sand. Land fractions 0.30, 0.35, 0.40 and 0.45 were compared for regions; none gave more than two major regions on more than two seeds (the noise's lowest octave makes one mass), so 0.40 was taken, near the eight-seed mean land share of 41.5% before.

### The seam

Region counts at land fraction 0.40, the old non-wrapping noise against the new seamless noise:

| seed | seamed noise | seamless noise |
|---|---|---|
| 1 | 1 (104,858) | 1 (104,858) |
| 2 | 2 (62,367; 42,068) | 2 (95,629; 9,160) |
| 3 | 2 (83,813; 21,045) | 1 (104,858) |
| 7 | 2 (85,060; 19,798) | 3 (100,040; 2,484; 1,550) |
| 42 | 1 (104,858) | 1 (104,858) |
| 99 | 2 (103,615; 1,243) | 1 (104,858) |
| 314 | 1 (104,858) | 1 (104,858) |
| 1000 | 1 (104,858) | 1 (104,858) |

The seamed noise happens to leave three seeds with a second landmass of 19,000 tiles or more; the seamless noise leaves one second landmass of 9,160 tiles. The seam was fixed anyway, because those landmasses are cut out along straight lines at the map edge where uncorrelated values meet. A step 1 comparison against a flat fill could only show the seam joining land; against a seamless map it also cuts, since land on one side of the edge line meets water on the other, making a straight coast. Neither is geography. Every map changes in this step regardless, so fixing the seam now costs saves one generator change instead of two, and step 4 tunes the continents on the generator it will keep. Several landmasses are step 4's job: this step leaves the default seed, 42, with one region, which is step 4's trigger ("fewer than two large land regions on the default seed").

## The runs

| seed | major regions | region separation / null | biome separation / null | consumer / plant crossings | new-region events | final plants / grazers | plant floor after t1000 | grazers after t1000 | at ceiling | species at 5k / 15k | omnivores / hunters at 5k; 15k | last hunter tick |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 1 -> 1 | 1.000/1.000 -> 1.000/1.000 | 0.419/0.335 -> 0.362/0.299 | 0/0 -> 0/0 | 0 -> 0 | 3192/2472 -> 3087/2490 | 754 -> 756 | 322..3204 -> 511..3059 | 93 -> 95 | 16/51 -> 32/48 | 0/0; 0/1 -> 0/0; 0/0 | 15001 -> 13711 |
| 2 | 3 -> 2 | 0.561/0.474 -> 0.924/0.925 | 0.408/0.362 -> 0.403/0.345 | 43,273/15,509 -> 13,482/2,390 | 107 -> 70 | 3140/2858 -> **478/5522** | 924 -> **150** | 400..3593 -> 785..5716 | 125 -> 162 | 21/47 -> 33/41 | 0/0; 0/0 -> 0/0; 0/0 | 14191 -> 12211 |
| 3 | 2 -> 1 | 0.852/0.849 -> 1.000/1.000 | 0.413/0.368 -> 0.391/0.339 | 33,304/6,728 -> 0/0 | 65 -> 0 | 3192/2586 -> 3564/2433 | 376 -> 1523 | 503..3373 -> 572..3816 | 94 -> 245 | 20/52 -> 33/66 | 0/0; 0/0 -> 0/0; 1/0 | 14971 -> 10291 |
| 7 | 3 -> 3 | 0.919/0.926 -> 0.966/0.969 | 0.436/0.354 -> 0.459/0.393 | 16,259/6,415 -> 21,594/4,175 | 165 -> 190 | 2813/2690 -> 1932/2145 | 592 -> 1326 | 489..3250 -> 391..2877 | 70 -> 52 | 12/44 -> 21/50 | 0/0; 0/0 -> 0/0; 0/0 | 8251 -> 12061 |
| 42 | 1 -> 1 | 1.000/1.000 -> 1.000/1.000 | 0.413/0.363 -> 0.439/0.356 | 0/0 -> 0/0 | 0 -> 0 | 2950/3050 -> 3181/2123 | 944 -> 494 | 1044..4203 -> 453..3058 | 267 -> 95 | 49/89 -> 46/48 | 0/0; 0/0 -> 1/0; 0/0 | 14641 -> 481 |
| 99 | 1 -> 1 | 1.000/1.000 -> 1.000/1.000 | 0.541/0.522 -> 0.382/0.319 | 0/0 -> 0/0 | 0 -> 0 | 1524/1475 -> 3169/2831 | 1058 -> 245 | 292..2417 -> 626..3497 | 0 -> 125 | 17/31 -> 34/59 | 0/0; 0/0 -> 0/0; 0/0 | 7381 -> 11371 |
| 314 | 3 -> 1 | 0.986/0.986 -> 1.000/1.000 | 0.387/0.360 -> 0.424/0.390 | 19,931/5,435 -> 0/0 | 124 -> 0 | 3666/2331 -> 3436/2562 | 395 -> 1476 | 579..3657 -> 413..3284 | 124 -> 167 | 16/26 -> 44/58 | 0/0; 0/0 -> 0/0; 0/0 | 421 -> 1501 |
| 1000 | 1 -> 1 | 1.000/1.000 -> 1.000/1.000 | 0.464/0.399 -> 0.420/0.365 | 0/0 -> 0/0 | 0 -> 0 | 4062/1935 -> 3517/2483 | 661 -> 709 | 406..3144 -> 660..3507 | 237 -> 247 | 29/76 -> 24/54 | 0/0; 0/0 -> 0/0; 0/0 | 481 -> 541 |

Share of organisms by biome, mean of the 1 Hz samples from tick 1000 (deep / shallow / Sand / Grassland / Forest / Rock, %):

| seed | before | after |
|---|---|---|
| 1 | 30.0 / 14.7 / 26.0 / 15.2 / 1.9 / 12.2 | 24.6 / 24.2 / 13.4 / 19.3 / 14.2 / 4.2 |
| 2 | 31.2 / 30.7 / 6.0 / 23.9 / 8.2 / 0.0 | 23.4 / 32.2 / 5.0 / 12.6 / 20.3 / 6.5 |
| 3 | 34.4 / 22.0 / 0.6 / 23.1 / 19.8 / 0.0 | 28.2 / 25.3 / 6.3 / 28.1 / 3.6 / 8.5 |
| 7 | 29.9 / 28.3 / 0.1 / 14.4 / 27.3 / 0.0 | 9.0 / 36.7 / 14.5 / 26.3 / 9.8 / 3.7 |
| 42 | 7.5 / 24.1 / 10.6 / 31.9 / 25.9 / 0.0 | 33.3 / 18.8 / 4.0 / 13.2 / 20.9 / 9.8 |
| 99 | 52.4 / 11.6 / 12.0 / 12.7 / 8.0 / 3.3 | 28.7 / 22.7 / 12.0 / 12.8 / 20.2 / 3.6 |
| 314 | 19.3 / 16.2 / 20.8 / 34.4 / 7.9 / 1.4 | 37.5 / 15.2 / 5.3 / 19.8 / 18.2 / 4.1 |
| 1000 | 38.2 / 12.6 / 2.6 / 18.5 / 27.7 / 0.3 | 18.8 / 34.7 / 8.3 / 19.1 / 17.2 / 1.8 |

Plants / grazers by biome at tick 15000 (deep, shallow, Sand, Grassland, Forest, Rock):

| seed | plants | grazers |
|---|---|---|
| 1 | 635, 674, 522, 693, 389, 174 | 630, 665, 201, 561, 375, 58 |
| 2 | 130, 139, 24, 50, 83, 52 | 1438, 1827, 243, 599, 1047, 368 |
| 3 | 1232, 836, 220, 848, 107, 321 | 668, 644, 120, 765, 103, 133 |
| 7 | 191, 498, 447, 514, 164, 118 | 190, 865, 205, 590, 254, 41 |
| 42 | 1412, 598, 142, 316, 354, 359 | 435, 432, 82, 350, 662, 162 |
| 99 | 989, 588, 532, 414, 478, 168 | 679, 694, 251, 400, 733, 74 |
| 314 | 1385, 532, 224, 636, 523, 136 | 836, 351, 114, 618, 560, 83 |
| 1000 | 680, 1096, 342, 769, 551, 79 | 488, 951, 182, 422, 411, 29 |

Deep-water share of consumer organism-ticks by aquatic band, aq0 / aq1 / aq2 / aq3 (bands in brackets held under 100,000 consumer organism-ticks over the run):

| seed | before | after |
|---|---|---|
| 1 | 26% / 36% / (41%) / (-) | 25% / 27% / 27% / 30% |
| 2 | 24% / 31% / (39%) / (-) | 22% / 21% / 24% / 26% |
| 3 | 29% / 30% / (30%) / (-) | 24% / 29% / 31% / (61%) |
| 7 | 20% / 27% / (29%) / (-) | 9% / 11% / (12%) / (-) |
| 42 | 7% / 7% / 8% / (14%) | 22% / 38% / (55%) / (65%) |
| 99 | 50% / 57% / 57% / 59% | 21% / 26% / 37% / (43%) |
| 314 | 15% / 18% / 24% / (-) | 20% / 30% / 40% / (67%) |
| 1000 | 28% / 30% / 40% / 45% | 10% / 19% / 21% / (-) |

Plants / grazers every 1500 ticks from tick 1501, after the change:

- **Seed 1:** 1432/1329, 3211/2485, 2679/2354, 2663/1937, 2104/940, 2603/705, 2871/1495, 3021/2783, 3265/2735, 3087/2490.
- **Seed 2:** 871/1889, 659/3048, 1293/3913, 1496/3487, 1096/1769, 663/1729, 588/5412, 615/5384, 546/5452, 478/5522.
- **Seed 3:** 2522/1277, 3279/2719, 2491/3505, 3445/2554, 2988/1609, 2365/1079, 2646/3013, 3292/2708, 3646/2350, 3564/2433.
- **Seed 7:** 1683/1076, 2373/1989, 2063/2198, 3198/1823, 2957/769, 2722/596, 2884/1436, 2387/2284, 1780/2594, 1932/2145.
- **Seed 42:** 2546/1496, 505/1750, 3227/2702, 2396/2044, 1818/853, 2489/623, 3214/1334, 3404/2594, 3433/2567, 3181/2123.
- **Seed 99:** 383/1266, 1942/2322, 2198/2807, 3247/2657, 2883/1332, 2385/773, 2467/1693, 2504/3328, 2865/3134, 3169/2831.
- **Seed 314:** 2064/956, 2863/2237, 2675/2692, 3584/2416, 3052/1234, 2857/909, 2832/1842, 3271/2728, 3203/2797, 3436/2562.
- **Seed 1000:** 1324/1520, 2973/3027, 3540/2460, 3659/2339, 3154/1531, 2358/1007, 2369/2224, 2708/3292, 3151/2847, 3517/2483.

These samples alias with the seasonal cycle; seed 2's last four (grazers 5,384 to 5,522) all land near a grazer peak. At 120-tick resolution from tick 9001, every seed's grazers boom and crash with a period of about 1,800 ticks, the seasonal year, with 7 grazer peaks after tick 1000 on each seed. On seven seeds the grazer troughs fall on the same ticks (9061, 10861, 12661, 14461); seed 2's lag about 120 ticks. Plants cycle on the same period, in antiphase. Ranges from tick 9001 (plants; grazers; 30-tick samples at 5,990 organisms or more):

| seed | plants | grazers | ceiling samples |
|---|---|---|---|
| 1 | 2,298..3,531 | 662..3,044 | 53 |
| 2 | 158..1,342 | 1,637..5,706 | 121 |
| 3 | 2,301..3,939 | 1,079..3,236 | 123 |
| 7 | 1,601..3,799 | 577..2,872 | 26 |
| 42 | 2,456..4,042 | 586..2,714 | 61 |
| 99 | 2,300..3,356 | 773..3,470 | 67 |
| 314 | 2,437..3,742 | 859..3,255 | 98 |
| 1000 | 2,313..3,636 | 981..3,471 | 106 |

Seed 2's grazers after tick 9000: 5,422 (t10531), 1,973 (t10891), 5,623 (t12451), 2,368 (t12811), 5,716 (t14371), 2,701 (t14611), 5,505 (t14971). Its peaks reach the 6,000 ceiling (engaged 5 times over the run, 8.6M births blocked, 143 samples at 5,990 or more).

## Reading

**Land area is now equal across seeds, and every seed has every biome.** The twofold spread is gone by construction. Rock appears on all eight seeds, and organisms use it (1.8% to 9.8% of organisms over the run, and plants on Rock at 15000 on every seed).

**Fewer regions, so less to separate.** Five seeds are now a single landmass (1, 3, 42, 99, 314) against four before, and seed 2's three comparable landmasses became one large and one small (91% and 9% of the land). Region separation is at its null on every seed; seed 2's gap above the null (0.561 against 0.474 in step 2) is gone because 91% of its land is one region. Crossings exist only on seeds 2 and 7 (13,482 and 21,594 consumer crossings), and are zero on seeds 3 and 314, which had crossings before. This step does not produce the plan's barrier; it hands step 4 a generator with equal land and every biome, and step 4's condition (fewer than two large land regions on the default seed) is met.

**Biome separation stays a little above its null everywhere** (0.362 to 0.459 against 0.299 to 0.393). The gap is 0.03 to 0.08 on each seed, much as before (0.02 to 0.08). Seed 99's high figures from step 2 (0.541 against 0.522, when 73% of its tiles were water) came down with its water share.

**Plants and grazers persist and cycle on every seed; seed 2's grazers live on food items.** Plant floors after tick 1000 rose on five seeds (to 709 to 1523) and fell on three (seed 42 to 494, seed 99 to 245, seed 2 to 150). All eight seeds boom and bust on the seasonal year, about 1,800 ticks (see the 120-tick ranges above). Seed 2 cycles on the same period at a different level: from tick 9000 its grazers swing between 1,637 and 5,706, reaching the 6,000 ceiling at each peak, over 158 to 1,342 plants. (An earlier draft read the four 1500-tick samples from tick 10501, which all land near a grazer peak, as a plateau; that was aliasing.) Over ticks 14521 to 15001 its grazers took 98.6% of their income from food items, paid 0.204 energy per tick against 0.810 in step 2, had a mean age of 737 against 247, and 59.1% of them stood on water at tick 15000. Mean body size at 15000 was 0.57 against 1.02 in step 2, and eaters moved at 1.12 against 2.21 per tick. That is a slow, small, food-item grazer that barely needs plants. It is one trajectory on one seed, but it is the food-item supply doing the work the plan expects of vegetation, which step 5 is set up to test (39% to 50% of regenerated food items landed on water in these runs).

**Ceiling samples** summed over the eight seeds went from 1010 to 1188: up on seeds 2, 3, 99 (0 to 125, since it now has 40% land instead of 27%) and 314, down on 7 and 42 (267 to 95, with 39% less land).

**Species at 15k** rose on four seeds (3, 7, 99, 314) and fell on four (1, 2, 42, 1000); seed 42 went from 89 to 48 on 39% less land. Species at 5k rose on six seeds.

**Aquatic bands move with the map.** Consumer organism-ticks by band over the run (aq0/aq1/aq2/aq3, %) went from 88/12/0/0 to 2/27/70/1 on seed 1 and from 78/22/0/0 to 0/18/77/5 on seed 2; the other way, from 7/91/2/0 to 93/6/0/0 on seed 42 (water from 35% to 60% of tiles, but mostly deep) and from 1/3/13/82 to 20/74/6/0 on seed 99 (water from 73% to 60%). Seeds 3, 7, 314 and 1000 changed less. On six seeds the deep-water share rises with the band (for example 22%, 38%, 55% on seed 42), as the movement rule intends; on seeds 1 and 2 it is nearly flat.

**Hunters.** No seed has a hunter alive at 15000 (step 2 had one on seed 1). Hunters were alive at scattered samples after tick 5000 on seeds 1, 2, 3, 7 and 99, never more than three at a time (three on seed 7, whose last hunter was at tick 12061). One omnivore was alive at 15000 on seed 3 and one at 5000 on seed 42; omnivores were alive at 5 to 122 samples after tick 3000, and persisted on no seed. The `hunter-emergence` reopen condition is not met by this audit.

## Not measured here

- A second run per seed. Headless runs are deterministic, so one run per seed is one trajectory; seed 2's food-item-fed grazers may or may not recur on a perturbed run.
- The late aquatic band mix per seed; the whole-run band totals are in each summary's geography block.
