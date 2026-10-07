# Phase 2 step 5: water vegetation as a knob, 15k ticks

Step 5 of `plans/2026-09-25-phase2-biomes.md`. `SimConfig.water_vegetation` (`--water-vegetation M`, default 0) is a multiplier on the `nutrients × moisture` carrying capacity of ShallowWater tiles in `tile_dynamics_system`. At 0, water stays out of tile dynamics as it always has. Above 0, shallow water relaxes toward `M × nutrients × moisture` at the land rate (0.001 per tick) and ShallowWater is added to the founding biomes, after the four land biomes (`founding_biomes` in `crates/clauvolution_sim/src/lib.rs`). Deep water stays out of tile dynamics at every value. The reasoning and the choice of default are in `docs/DECISIONS.md` ("Water vegetation as a knob").

Two settings, the two the plan asks for:

- **`barrier/`**: `water_vegetation` 0, the default.
- **`shallow-habitat/`**: `--water-vegetation 1`. One value, not swept. 1 is the value at which shallow water follows the same carrying-capacity rule as land, so its capacity is `0.5 × moisture` (nutrients 0.5): between Sand (0.15 × moisture) and Grassland (0.6 × moisture). A larger value would make shallow water richer than the land it surrounds, which needs its own argument; a smaller one is a weaker version of the same test.

Run details:

- **Commit:** 52e5297 on `todo/oceans-as-habitat` (c8d2c30 after the branch was rebased onto d1a1ae9, a plan-only commit), the binary copied aside so later builds could not change it mid-audit. The `barrier/` summaries were rerun at 086784f, which adds mean nutrients to the summary's vegetation line; their history CSVs are byte-identical to the 52e5297 runs on all eight seeds. Other later commits change only documentation and a CLI test.
- **Seeds:** 1, 2, 3, 7, 42, 99, 314, 1000, one run each (headless runs are deterministic), 15000 ticks, four at a time on a loaded machine. Wall times (412 to 752 s) are not comparable with other audits.
- **How to repeat:** `cargo build --release`, then for each seed `./target/release/clauvolution --headless 15000 --seed S --dump-history seedS-run1.csv > seedS-run1.txt 2>&1`, adding `--water-vegetation 1` for the habitat setting.
- **Before:** `docs/audits/2026-09-26-phase2-continents/` (step 4, same seeds and ticks). Definitions of regions, crossings, separation and aquatic bands are in `docs/audits/2026-09-25-phase2-baseline/`.

## The barrier setting is behaviour-neutral

On all eight seeds the `barrier/` history CSVs are byte-identical to step 4's, and the summaries are identical apart from the new "Vegetation / nutrients at the end" line and the wall-time and path lines. So the barrier columns below are step 4's numbers, re-read, and the step 4 note's cycling and per-band tables apply to it unchanged.

## How much of the food-item flow lands on water

The plan asks for this before choosing. With the barrier setting, 28.5% to 38.1% of regenerated food items land on deep water and 6.5% to 9.8% on shallow, 35.2% to 46.2% on water in all, and eaters on water take the same shares of what is eaten to within half a point (step 4 note). Water is 60% of the map's tiles, and food lands there at about half the land rate per tile.

The new line in the summary shows why water is fed at all under the barrier setting: **water is not barren.** At tick 15000 every water tile on every seed has vegetation above 0, at a mean of 0.119 to 0.194 on deep water and 0.132 to 0.221 on shallow. This is niche construction: a photosynthesiser deposits 0.001 of vegetation per tick on its tile (`NICHE_VEGETATION_DEPOSIT`), plants live on water (deep water holds the most plants of any biome on every seed), and nothing takes water vegetation back down, because tile dynamics, the only decay, skips water. Nutrients accrue undecayed too, on every tile: `NICHE_NUTRIENT_DEPOSIT` (0.0001 per organism per tick) and a meteor's +0.5, with nothing taking them back. At tick 15000 deep water's mean nutrients are 0.318 to 0.327, from the 0.3 it starts at, and shallow water's 0.523 to 0.531, from 0.5 (`barrier/` summaries, rerun at 086784f with the nutrient column; the history CSVs are byte-identical to the 52e5297 runs). A food item's spawn chance is `(vegetation + nutrients) × 0.5`, so at the end of the run deep water's chance is 0.22 to 0.26 against the 0.15 it starts at, and 27% to 37% of it is owed to deposited vegetation. That share is for the end of the run only. Water vegetation starts at 0 and builds up (the review's 3000-tick seed 42 run had deep water at a mean of 0.029), so its share of the run's total food on deep water is smaller, and was not measured; the history CSV has no vegetation column.

| seed | vegetation deep / shallow, barrier | vegetation deep / shallow, habitat | food items on deep / shallow, barrier | habitat |
|---|---|---|---|---|
| 1 | 0.127 / 0.149 | 0.140 / 0.262 | 36.5% / 8.2% | 36.2% / 10.0% |
| 2 | 0.139 / 0.155 | 0.153 / 0.280 | 35.5% / 9.8% | 34.8% / 12.2% |
| 3 | 0.185 / 0.204 | 0.117 / 0.221 | 38.1% / 8.1% | 35.9% / 9.6% |
| 7 | 0.150 / 0.170 | 0.120 / 0.281 | 32.4% / 8.2% | 30.8% / 10.4% |
| 42 | 0.194 / 0.221 | 0.129 / 0.266 | 35.8% / 8.4% | 32.7% / 10.4% |
| 99 | 0.120 / 0.138 | 0.131 / 0.334 | 31.8% / 6.5% | 31.3% / 8.8% |
| 314 | 0.160 / 0.179 | 0.111 / 0.287 | 31.8% / 8.6% | 29.7% / 11.1% |
| 1000 | 0.119 / 0.132 | 0.109 / 0.326 | 28.5% / 6.7% | 27.9% / 9.2% |

Vegetation is the end state at tick 15000; food shares are over the whole run.

The habitat setting raises shallow vegetation to 0.221 to 0.334 and the shallow share of food items by 1.5 to 2.5 points. Deep water, which the knob does not touch, keeps a slightly lower share on every seed (0.3 to 3.1 points less), since the food ceiling is shared.

## The runs

Barrier -> shallow habitat. Separation figures are means over the 1 Hz samples from tick 1000; the gap is observed minus null, mean ± SD over the 467 samples.

| seed | region separation / null | region gap | biome separation / null | confined, observed (of species counted) | consumer / plant crossings | new-region events | final plants / grazers | plant floor after t1000 | grazers after t1000 | ceiling samples (whole run; from t9000) |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 0.525/0.409 -> 0.504/0.407 | 0.116 ± 0.048 -> 0.097 ± 0.048 | 0.475/0.453 -> 0.472/0.461 | 0.7 (17.5) -> 0.3 (13.9) | 57,808/26,419 -> 115,826/24,311 | 260 -> 221 | 4128/1869 -> 4019/1981 | 769 -> 673 | 428..3162 -> 501..2786 | 114; 68 -> 166; 85 |
| 2 | 0.535/0.450 -> 0.498/0.448 | 0.085 ± 0.046 -> 0.050 ± 0.037 | 0.463/0.423 -> 0.465/0.458 | 0.7 (18.5) -> 0.2 (14.8) | 63,964/17,833 -> 94,869/33,775 | 324 -> 338 | 3899/2101 -> 3908/2092 | 748 -> 867 | 402..2685 -> 362..2652 | 112; 80 -> 163; 96 |
| 3 | 0.462/0.405 -> 0.503/0.401 | 0.057 ± 0.036 -> 0.101 ± 0.047 | 0.502/0.482 -> 0.510/0.468 | 0.2 (21.5) -> 0.5 (13.2) | 91,663/33,902 -> 38,240/16,779 | 136 -> 176 | 4456/1542 -> 3448/2552 | 1761 -> 859 | 443..2776 -> 435..3203 | 290; 140 -> 64; 64 |
| 7 | 0.461/0.359 -> 0.474/0.357 | 0.102 ± 0.045 -> 0.117 ± 0.045 | 0.500/0.478 -> 0.487/0.459 | 0.4 (19.7) -> 0.3 (17.4) | 122,923/27,930 -> 95,044/18,677 | 247 -> 285 | 3360/2635 -> 2939/3053 | 1407 -> 786 | 623..3510 -> 620..3850 | 215; 106 -> 154; 72 |
| 42 | 0.543/0.472 -> 0.587/0.460 | 0.070 ± 0.041 -> 0.127 ± 0.055 | 0.479/0.468 -> 0.441/0.433 | 0.5 (21.7) -> 0.6 (11.8) | 82,275/32,613 -> 52,473/18,858 | 244 -> 152 | 4291/1709 -> 4575/1425 | 2542 -> 183 | 416..2594 -> 241..2690 | 258; 116 -> 105; 104 |
| 99 | 0.524/0.432 -> 0.506/0.430 | 0.092 ± 0.043 -> 0.077 ± 0.034 | 0.475/0.449 -> 0.483/0.460 | 0.4 (14.1) -> 0.3 (18.0) | 71,311/19,751 -> 81,092/22,064 | 181 -> 194 | 3197/2520 -> 3494/2506 | 1141 -> 1464 | 445..3208 -> 426..3373 | 80; 54 -> 150; 88 |
| 314 | 0.492/0.417 -> 0.506/0.419 | 0.075 ± 0.041 -> 0.087 ± 0.051 | 0.468/0.441 -> 0.469/0.444 | 0.3 (16.4) -> 0.3 (12.4) | 83,142/21,408 -> 76,298/21,598 | 288 -> 205 | 4109/1889 -> 3834/2162 | 1463 -> 368 | 500..2953 -> 365..2636 | 191; 108 -> 21; 21 |
| 1000 | 0.483/0.386 -> 0.483/0.387 | 0.097 ± 0.048 -> 0.096 ± 0.042 | 0.482/0.436 -> 0.468/0.426 | 0.4 (15.4) -> 0.6 (16.4) | 48,783/15,547 -> 27,645/16,505 | 241 -> 79 | 3218/2462 -> 2705/2641 | 1027 -> 1009 | 492..2889 -> 518..3259 | 45; 45 -> 55; 36 |

Confined species under the null are 0.0 on every seed in both settings. Ceiling samples are 30-tick rows at 5,990 organisms or more.

Share of organisms by biome, mean of the 30-tick rows from tick 1000 (deep / shallow / Sand / Grassland / Forest / Rock, %):

| seed | barrier | shallow habitat |
|---|---|---|
| 1 | 44.8 / 7.8 / 12.9 / 15.2 / 12.0 / 7.4 | 46.2 / 7.7 / 12.4 / 14.4 / 11.7 / 7.6 |
| 2 | 41.9 / 9.0 / 12.7 / 15.4 / 12.9 / 8.1 | 45.7 / 9.1 / 12.1 / 14.2 / 10.9 / 8.0 |
| 3 | 47.3 / 7.4 / 8.1 / 31.6 / 2.2 / 3.3 | 45.2 / 7.4 / 8.2 / 33.3 / 2.4 / 3.5 |
| 7 | 47.2 / 8.6 / 5.1 / 24.2 / 11.3 / 3.5 | 45.1 / 8.7 / 5.3 / 25.1 / 12.3 / 3.4 |
| 42 | 46.7 / 8.3 / 10.6 / 12.5 / 14.1 / 7.7 | 43.7 / 8.4 / 11.0 / 13.1 / 14.8 / 9.0 |
| 99 | 44.4 / 7.1 / 5.9 / 11.2 / 22.0 / 9.5 | 45.5 / 7.3 / 5.4 / 10.9 / 22.6 / 8.3 |
| 314 | 43.9 / 9.2 / 8.6 / 14.0 / 17.7 / 6.5 | 43.6 / 9.2 / 8.9 / 13.9 / 17.7 / 6.7 |
| 1000 | 42.4 / 8.0 / 3.8 / 20.9 / 23.6 / 1.3 | 41.6 / 8.3 / 3.4 / 21.4 / 24.4 / 1.0 |

(The step 4 note's barrier table rounds a few of these 0.1 to 0.2 differently.)

Plants / grazers by biome at tick 15000 (deep, shallow, Sand, Grassland, Forest, Rock):

| seed | plants, barrier | plants, habitat | grazers, barrier | grazers, habitat |
|---|---|---|---|---|
| 1 | 1778, 274, 697, 575, 399, 405 | 1741, 318, 614, 592, 397, 357 | 862, 152, 160, 320, 300, 75 | 887, 181, 152, 333, 322, 106 |
| 2 | 1621, 324, 571, 576, 356, 451 | 1850, 300, 528, 532, 308, 390 | 885, 219, 175, 367, 375, 80 | 941, 183, 208, 374, 275, 111 |
| 3 | 2206, 329, 366, 1317, 92, 146 | 1836, 216, 291, 910, 59, 136 | 664, 120, 88, 597, 50, 23 | 925, 212, 206, 1065, 74, 70 |
| 7 | 1698, 260, 268, 769, 208, 157 | 1394, 239, 193, 706, 268, 139 | 1102, 220, 103, 704, 442, 64 | 1340, 272, 127, 763, 466, 85 |
| 42 | 2027, 371, 528, 476, 517, 372 | 2198, 358, 537, 567, 496, 419 | 762, 161, 127, 243, 334, 82 | 608, 135, 100, 204, 325, 53 |
| 99 | 1298, 268, 260, 346, 574, 451 | 1705, 255, 171, 383, 567, 413 | 1059, 186, 94, 302, 745, 134 | 1051, 195, 85, 299, 750, 126 |
| 314 | 1874, 345, 429, 521, 573, 367 | 1779, 349, 349, 439, 577, 341 | 782, 178, 114, 315, 429, 71 | 818, 245, 104, 307, 628, 60 |
| 1000 | 1522, 245, 168, 685, 546, 52 | 1256, 218, 133, 592, 454, 52 | 764, 205, 51, 602, 830, 10 | 864, 226, 64, 667, 812, 8 |

Plants and grazers from tick 9000, at 120-tick resolution, and species from tick 9000 (late-window mean):

| seed | plants, barrier -> habitat | grazers, barrier -> habitat | species late mean | species at 5k / 15k |
|---|---|---|---|---|
| 1 | 2,245..4,128 -> 2,390..4,077 | 623..2,837 -> 604..2,786 | 39 -> 41 | 27/34 -> 9/43 |
| 2 | 2,428..3,990 -> 2,881..4,136 | 609..2,654 -> 595..2,628 | 46 -> 40 | 34/53 -> 23/50 |
| 3 | 2,825..4,539 -> 1,860..3,660 | 785..2,668 -> 571..3,174 | 40 -> 38 | 16/56 -> 12/33 |
| 7 | 2,491..4,073 -> 1,963..3,295 | 962..3,509 -> 962..3,827 | 46 -> 51 | 34/43 -> 29/72 |
| 42 | 3,270..4,485 -> 3,038..4,582 | 739..2,579 -> 645..2,683 | 57 -> 26 | 27/57 -> 22/35 |
| 99 | 2,024..3,507 -> 2,240..3,524 | 660..3,208 -> 817..3,355 | 25 -> 30 | 21/22 -> 24/40 |
| 314 | 2,995..4,342 -> 1,804..3,834 | 688..2,530 -> 440..2,629 | 42 -> 27 | 35/46 -> 15/32 |
| 1000 | 1,841..3,556 -> 1,470..2,970 | 534..2,867 -> 662..3,235 | 37 -> 23 | 17/39 -> 22/23 |

Hunters and omnivores (30-tick rows):

| seed | omnivores / hunters at 5k; 15k, barrier | habitat | last hunter tick, barrier -> habitat | rows with omnivores after t3000 | rows with hunters after t5000 |
|---|---|---|---|---|---|
| 1 | 0/0; 0/0 | 0/0; 0/0 | 2011 -> 6541 | 0 -> 10 | 0 -> 2 |
| 2 | 0/0; 0/0 | 0/0; 0/0 | 8401 -> 14581 | 68 -> 4 | 3 -> 15 |
| 3 | 2/0; 2/0 | 0/0; 0/0 | 14281 -> 14761 | 206 -> 60 | 44 -> 44 |
| 7 | 0/0; 0/0 | 0/0; 0/0 | 13501 -> 7561 | 25 -> 10 | 13 -> 10 |
| 42 | 0/0; 0/0 | 0/0; 0/0 | 13051 -> 5221 | 16 -> 5 | 11 -> 6 |
| 99 | 0/0; 2/0 | 0/0; 0/0 | 7681 -> 13591 | 107 -> 27 | 3 -> 10 |
| 314 | 0/0; 0/0 | 0/0; 0/0 | 601 -> 14821 | 11 -> 55 | 0 -> 74 |
| 1000 | 0/0; 0/0 | 0/0; 0/0 | 631 -> 11671 | 22 -> 67 | 0 -> 27 |

Founders in the habitat setting: 59 to 74 of the 400 in ShallowWater (18,038 to 23,580 tiles, 15% to 18% of the founding area), taken proportionally from the land biomes.

## Reading

**No plant extinction in either setting.** The lowest plant count after tick 1000 is 183 (seed 42, habitat), during the founding boom: plants held at about 200 from tick 1000 to 2100 under 1,000 to 1,300 grazers, then recovered to 3,038 or more from tick 9000. Seed 314 habitat's floor is 368 in the same window. Both settings stay on the table.

**The habitat setting does not make shallow water a habitat in any measured sense.** The shallow share of organisms is 7.3% to 9.2% against 7.1% to 9.2%, within 0.3 points on every seed. Plants on shallow water at 15000 are 216 to 358 against 245 to 371, grazers 135 to 272 against 120 to 220. The knob moves about two points of the food-item flow onto a ring 8 tiles wide around each coast, and the ring was already vegetated to 0.13 to 0.22 by niche construction. Founding 15% to 18% of the founders there makes no difference to the shares from tick 1000.

**Separation and crossings move both ways.** The region gap rose on seeds 3, 7, 42 and 314, fell on 1, 2 and 99 and is flat on 1000; the eight-seed mean is 0.087 under the barrier and 0.094 under habitat. Consumer crossings doubled on seed 1, rose on 2 and 99 and fell on the other five (seed 1000 48,783 to 27,645). The founding draw differs between the settings (a fifth founding biome reshuffles every founder), so each pair is two different trajectories, and the spread in the gap between them (-0.035 to +0.057) is the size of a trajectory's own variation. One run per seed cannot separate the knob from the draw here.

**Plant floors and late species fell on most seeds.** Plant floors after tick 1000 are lower on six of eight seeds (seed 42 2,542 to 183, 314 1,463 to 368, 3 1,761 to 859, 7 1,407 to 786), higher on 2 and 99. The late-window plant minimum fell on five seeds. Mean species from tick 9000 fell on five seeds, sharply on 42 (57 to 26), 314 (42 to 27) and 1000 (37 to 23). With one run per seed and a different founding draw, how much of this is the knob is not known; it is the only consistent direction in the comparison, and it is against the habitat setting.

**Hunters.** No hunter is alive at 15000 on any seed in either setting, and no omnivore in the habitat setting (two each on seeds 3 and 99 under the barrier, as in step 4). Hunters show up at more scattered rows after tick 5000 in the habitat setting on five seeds (seed 314 0 to 74, at most a few at a time; last at 14821), and omnivores persist after tick 3000 on no seed in either. The `hunter-emergence` reopen condition is not met.

## Not measured here

- A second run per seed, and the same trajectory with only the knob changed. Because the fifth founding biome reshuffles every founder, the habitat runs differ from the barrier runs by their founding draw as well as by the knob; a run with `water_vegetation` above 0 and the land-only founding biomes would separate the two, and was not run since the plan pairs them as one setting.
- Any value other than 1, and the all-water setting (the plan's option (c)), which the knob does not offer.
- Water vegetation and nutrients over time. The summary line is the end state; the history CSV has no vegetation or nutrient column, so how much of the run's food on water the deposits account for is not known.
- Nutrients in the `shallow-habitat/` runs. Their summaries predate the nutrient column and were not rerun.
