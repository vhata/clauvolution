# Ceiling leaf-capacity probe, 2026-09-25

A measurement-only follow-up for the `TODO.md` entry `consumer-ceiling-regulation`. The consumer ceiling sweep (`docs/audits/2026-09-24-consumer-ceiling-sweep/`) found that no food or strike-cost setting keeps the world off the 6000-organism ceiling. Plants fill whatever room consumers leave, and plants are held by light, not by consumers. Its suggested next probe was a lower `--leaf-capacity` combined with `--strike-cost 0.3`, on seed 42 first. This is that probe. No code or default was changed.

- **What the lever is:** `SimConfig::leaf_capacity_per_tile` (default 0.02) is the leaf area one tile can fully light. A photosynthesiser's light share is `min(1, window tiles × capacity / leaf area in the window)` over its 9 by 9 canopy window. At 0.02 the world has room for roughly 3200 fully lit plants (`docs/DECISIONS.md`, "Canopy light sharing"). Lowering it shades every plant in proportion, so the plant level's standing crop and its income both fall. The 3000-tick capacity sweep in that entry, run with no ceiling, had plants held under 400 at 0.01 and eaten out by tick 2500 at 0.005.
- **Values:** 0.015, 0.012 and 0.01 were chosen first, as cuts of 25%, 40% and 50%. All three emptied the plant level on seed 42, so 0.0175 and 0.019 (cuts of 12.5% and 5%) were added to find where it tips. Each value was run alone and with `--strike-cost 0.3`, so the two effects can be separated.
- **Seeds:** all ten settings on seed 42. The only setting that cut seed 42's ceiling time with plants alive at 15000, 0.0175 alone and with strike 0.3, was repeated on seeds 3 and 7. One run per seed, since headless runs are deterministic. 15000 ticks at the default 10x virtual time, at most three at once on a machine shared with other agents, 409 to 1036 s a run, so wall times are not comparable.
- **Commit and baselines:** main at 2be99b0. The default and strike 0.3 rows are the step 6 audit (`docs/audits/2026-09-24-pyramid-step6/`, seed<S>-run1) and the 2026-09-24 sweep (`strike-0.3/`), not re-run. The build here has changed under `crates/` since those runs (PRs #78 and #81 add counters and a digestion-exponent flag with an unchanged default). To check that the old runs are still valid baselines, a default seed 42 run of 600 ticks on this build was compared with the step 6 history. All 64 columns the two files share match on all 20 rows.
- **How to repeat:** `cargo build --release`, then from the repo root `xargs -P3 -L1 docs/audits/2026-09-25-ceiling-leaf-probe/run.sh < docs/audits/2026-09-25-ceiling-leaf-probe/<jobs file>`. The jobs files are `jobs-seed42.txt`, `jobs-seed42-fine.txt` and `jobs-seeds-3-7.txt`. Each line is `<setting dir> <seed> <flags>`, and `run.sh` runs `./target/release/clauvolution --headless 15000 --seed S <flags> --dump-history <dir>/seedS.csv > <dir>/seedS.txt 2>&1`.
- **Files:** one directory per setting, holding `seed<S>.txt` (headless summary) and `seed<S>.csv` (1 Hz history, 500 rows). `probe-run.log` is the launcher log.

## Definitions

The same as the 2026-09-24 sweep:

- **At ceiling:** the number of 1 Hz samples (out of 500) with 5700 or more organisms, then the summary's engagement count and births blocked. **First at 5700+** is the tick of the first such sample. **Plants / grazers at 5700+** is the mean of each over those samples.
- **Death shares:** predation, starvation and disease as shares of all deaths in the summary. Old age is under 1% and is left out.
- **Plant floor:** the fewest plants in any sample after tick 1000. **Grazers after t1000:** their minimum and maximum over the same samples.
- **Consumer mean:** grazers plus hunters plus omnivores, averaged over the samples after tick 1000. **Plant mean** and **organism mean** are the same for plants and for all organisms. No run had more than 20 hunters and omnivores together at any sample after tick 1000, and none had a hunter at 15000.
- **Species:** the living species count in the final summary.

## Results at 15000 ticks

| setting | seed | at ceiling (samples; engagements, births blocked) | first at 5700+ | plants / grazers at 5700+ (mean) | deaths: predation / starvation / disease | final plants / grazers | plant floor after t1000 | grazers after t1000 | consumer mean after t1000 | plant mean after t1000 | organism mean after t1000 | species at 15k |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| default (step 6) | 42 | 243; 8, 9.20M | 2401 | 2002 / 3987 | 36.3% / 14.3% / 49.2% | 3131 / 2869 | 296 | 1214..5069 | 3191 | 1789 | 4980 | 82 |
| strike 0.3 (2026-09-24) | 42 | 194; 8, 3.85M | 2251 | 2147 / 3834 | 47.7% / 11.0% / 41.3% | 2618 / 3379 | 421 | 889..4945 | 2815 | 1909 | 4724 | 64 |
| leaf 0.019 | 42 | 204; 8, 5.20M | 2251 | 2348 / 3632 | 43.8% / 13.2% / 42.8% | 2477 / 3515 | 571 | 878..4861 | 2713 | 1980 | 4693 | 37 |
| leaf 0.019 + strike 0.3 | 42 | 162; 5, 5.50M | 2251 | 2760 / 3224 | 54.7% / 22.7% / 22.6% | 0 / 2519 | 0 | 647..4184 | 2357 | 1726 | 4083 | 13 |
| leaf 0.0175 | 42 | 133; 6, 2.34M | 4171 | 2114 / 3861 | 27.3% / 21.0% / 51.3% | 2907 / 3093 | 94 | 949..4714 | 2744 | 1512 | 4257 | 57 |
| leaf 0.0175 + strike 0.3 | 42 | 167; 6, 4.61M | 2581 | 1862 / 4116 | 36.6% / 16.6% / 46.6% | 2241 / 3565 | 322 | 968..4967 | 2988 | 1452 | 4440 | 54 |
| leaf 0.015 | 42 | 0; 0, 0 | never | - | 19.6% / 71.5% / 8.9% | 0 / 2536 | 0 | 744..3823 | 2184 | 4 | 2188 | 8 |
| leaf 0.015 + strike 0.3 | 42 | 0; 0, 0 | never | - | 31.5% / 64.1% / 4.4% | 0 / 2395 | 0 | 618..3678 | 2034 | 8 | 2043 | 20 |
| leaf 0.012 | 42 | 0; 0, 0 | never | - | 13.7% / 53.0% / 33.3% | 1 / 2922 | 0 | 873..4395 | 2510 | 11 | 2520 | 13 |
| leaf 0.012 + strike 0.3 | 42 | 0; 0, 0 | never | - | 23.5% / 61.4% / 15.1% | 0 / 2788 | 0 | 811..3913 | 2262 | 2 | 2264 | 18 |
| leaf 0.01 | 42 | 0; 0, 0 | never | - | 7.8% / 50.3% / 41.9% | 594 / 3702 | 18 | 852..5076 | 2534 | 301 | 2835 | 30 |
| leaf 0.01 + strike 0.3 | 42 | 0; 0, 0 | never | - | 15.7% / 64.6% / 19.7% | 0 / 2663 | 0 | 788..4654 | 2593 | 0 | 2594 | 16 |
| default (step 6) | 3 | 28; 2, 0.02M | 9781 | 2231 / 3641 | 25.0% / 24.7% / 50.1% | 2294 / 2755 | 267 | 718..3989 | 2255 | 1379 | 3634 | 41 |
| strike 0.3 (2026-09-24) | 3 | 0; 0, 0 | never | - | 44.5% / 25.5% / 30.0% | 1952 / 3137 | 197 | 679..3819 | 2164 | 1128 | 3292 | 31 |
| leaf 0.0175 | 3 | 0; 0, 0 | never | - | 28.5% / 67.9% / 3.6% | 3 / 2931 | 0 | 536..3793 | 1725 | 3 | 1727 | 19 |
| leaf 0.0175 + strike 0.3 | 3 | 0; 0, 0 | never | - | 25.8% / 43.0% / 31.2% | 1009 / 2584 | 39 | 582..3921 | 2008 | 421 | 2429 | 14 |
| default (step 6) | 7 | 2; 0, 0 | 4531 | 1584 / 4132 | 29.3% / 25.8% / 44.8% | 1725 / 3443 | 351 | 845..4153 | 2406 | 1168 | 3574 | 33 |
| strike 0.3 (2026-09-24) | 7 | 0; 0, 0 | never | - | 27.8% / 34.4% / 37.7% | 977 / 3056 | 353 | 730..4073 | 2276 | 721 | 2997 | 35 |
| leaf 0.0175 | 7 | 0; 0, 0 | never | - | 31.1% / 30.8% / 38.0% | 1808 / 2506 | 76 | 537..3508 | 1912 | 962 | 2874 | 43 |
| leaf 0.0175 + strike 0.3 | 7 | 0; 0, 0 | never | - | 28.9% / 38.8% / 32.3% | 1271 / 3342 | 44 | 679..4012 | 2157 | 437 | 2594 | 34 |

Samples at the ceiling on seed 42: default 243, strike 0.3 194, leaf 0.019 204, 0.019 + strike 162, 0.0175 133, 0.0175 + strike 167, and 0 for every value from 0.015 down, alone or with strike 0.3.

Plants / grazers every 1500 ticks from tick 1501:

- **Leaf 0.019, seed 42:** 1021/1767, 2531/3468, 2853/3145, 2801/3193, 1667/1643, 1419/1308, 1507/3742, 1066/4781, 1362/4638, 2477/3515.
- **Leaf 0.019 + strike 0.3, seed 42:** 1524/2064, 2513/3487, 2649/3351, 3489/2506, 2811/2007, 1853/1360, 1512/1745, 0/2955, 0/3309, 0/2519.
- **Leaf 0.0175, seed 42:** 96/2203, 651/3967, 1679/4321, 1892/3718, 1337/1591, 1094/1017, 1695/2336, 2098/3898, 2598/3397, 2907/3093.
- **Leaf 0.0175 + strike 0.3, seed 42:** 331/2741, 659/3856, 1545/4452, 2283/3717, 2544/2099, 1547/1320, 1197/2603, 1203/4795, 1467/4532, 2241/3565.
- **Leaf 0.015, seed 42:** 26/1854, 9/3372, 1/3406, 0/2688, 0/1273, 1/880, 0/1924, 0/3116, 0/3460, 0/2536.
- **Leaf 0.015 + strike 0.3, seed 42:** 25/1972, 28/3315, 26/3479, 0/2483, 0/1183, 0/775, 0/1663, 0/1702, 0/3072, 0/2395.
- **Leaf 0.012, seed 42:** 49/2281, 16/3790, 14/4312, 6/3071, 5/1360, 10/951, 6/1953, 1/3498, 0/4014, 1/2922.
- **Leaf 0.012 + strike 0.3, seed 42:** 20/2190, 0/3014, 0/3401, 0/2729, 0/1283, 0/898, 0/1935, 0/3274, 0/3659, 0/2788.
- **Leaf 0.01, seed 42:** 67/2321, 46/4122, 23/4463, 27/2997, 201/1356, 386/894, 551/1837, 826/3402, 596/4301, 594/3702.
- **Leaf 0.01 + strike 0.3, seed 42:** 0/2200, 0/3941, 2/4539, 0/3459, 0/1548, 1/1016, 0/2252, 0/3358, 0/3690, 0/2663.
- **Leaf 0.0175, seed 3:** 19/1377, 3/2191, 0/2656, 0/1706, 0/866, 0/575, 0/1405, 1/2790, 0/3742, 3/2931.
- **Leaf 0.0175 + strike 0.3, seed 3:** 68/1676, 254/3193, 532/3105, 146/1334, 57/897, 124/707, 647/2013, 1127/3313, 1052/3548, 1009/2584.
- **Leaf 0.0175, seed 7:** 81/1647, 687/2947, 1544/3286, 1382/2350, 529/945, 443/757, 836/1564, 1465/2894, 1690/3301, 1808/2506.
- **Leaf 0.0175 + strike 0.3, seed 7:** 58/1640, 60/3071, 148/3552, 367/2753, 718/1530, 217/767, 151/1625, 751/3202, 1189/4012, 1271/3342.

The baseline rows' series are in the 2026-09-24 sweep's README.

## Reading

Measured, from the table above:

- **From 0.015 down, the plant level collapses on seed 42, alone or with strike 0.3.** Plants fall under 10 by tick 3000 at 0.015 and never recover. At 0.012, and at 0.01 with strike 0.3, they are at or near zero from tick 1500 on. The ceiling is never reached because the plants are gone, and starvation becomes 50% to 72% of deaths. Grazers live on food items (2034 to 2593 consumers on average). Leaf 0.01 alone is the exception: plants hold at 18 to 885 after tick 4501 (the 18 is at tick 5161, after an earlier low of 23 at 4501), and 201 to 885 after tick 7500. That is not monotone with 0.012 and 0.015, which lost theirs. On one run per setting, this reads as the plant level surviving by chance, not a safe setting.
- **Where the plants collapse, it starts in the founding boom.** Tick by tick, plants on seed 42 at 0.015 peaked at 176 around tick 300 while grazers climbed past 3000, and were at 46 by tick 1200. At the default they reached 339 and held. The average light share at tick 600 was 0.59 at 0.015 and 0.45 at 0.01, against 0.74 at the default. Shaded plants grow too slowly to get ahead of the grazer boom.
- **0.0175 alone gives the largest cut to seed 42's ceiling time, but the plant level nearly goes.** It drops from 243 samples to 133, from 8 engagements to 6, and from 9.20M blocked births to 2.34M, and the first 5700+ sample moves from tick 2401 to 4171. Plants and grazers persist and cycle on seed 42, but the plant floor is 94, against 296 at the default (96 at tick 1501). On seed 3 the same setting loses its plants: under 10 from tick 1771, 0 from tick 3661 through tick 11580, then 1 to 3 on and off to the end, and 3 at the end. Seed 7 keeps its plants with a floor of 76, against 351 at the default.
- **Adding strike 0.3 to 0.0175 protects plants on seeds 42 and 3, but less of seed 42's ceiling time goes away.** On seed 42 the floor rises from 94 to 322 and ceiling samples are 167 (133 alone, 194 at strike 0.3 alone). On seed 3 plants survive, with a floor of 39, 57 at tick 7501, and 1009 at the end. On seed 7 the floor falls from 76 to 44, with plants at 58 to 367 for the first 6000 ticks. Seeds 3 and 7 never reach 5700 under either setting, but they did not reach it at strike 0.3 alone either, with higher plant floors (197 and 353).
- **0.019 alone looks like a milder strike 0.3.** It cuts seed 42's ceiling samples to 204 (strike 0.3 alone: 194), and blocked births to 5.20M. The plant floor is 571 and the death mix is close to strike 0.3's (43.8% predation, 42.8% disease). Species fall to 37 (82 at the default, 64 at strike 0.3).
- **0.019 with strike 0.3 had plenty of plants for most of the run, then lost them all.** Plants were at 1512 to 3489 every 1500 ticks up to tick 10501, then 0 from tick 11701 on. In one population crash, plants went from 2086 to 97 between ticks 10201 and 11251, and grazers from 3850 to 647 and back up to 1808. The grazers recovered on food items and ate the last plants. Its 162 ceiling samples all came before the collapse, and it ends with 13 species.
- **Consumers barely move.** For settings that keep their plants, the consumer mean on seed 42 is 2713 to 2988, against 3191 at the default and 2815 at strike 0.3. The shorter ceiling time comes from fewer plants (plant mean 1452 to 1980), not fewer consumers. That is the same pattern the strike 0.3 run showed in the earlier sweep.

Estimated or not settled by this probe:

- **The window between "less ceiling" and "no plants" is narrow and depends on the seed.** Within 12.5% of the default, the same value (0.0175) kept plants on seeds 42 and 7 and lost them on seed 3. Adding strike 0.3 flipped which seeds kept a comfortable floor, and 0.019 with strike 0.3 lost plants at tick 11700 after 10000 healthy ticks. A default in this range would put plant extinction within reach of ordinary seed-to-seed spread. That is a reading of fourteen runs across three seeds, not a measured extinction rate.
- **The seed 42 gains are within the spread seen elsewhere.** The #45 control in the step 6 audit moved one seed's ceiling samples from 140 to 2 from a small perturbation. 243 to 133 on one run is the largest change here, but it is one run.
- **Why the lever cannot do what is asked of it:** a lower capacity takes room at the ceiling from plants, as intended. But it also shrinks the plant level's income, so each grazer boom takes plants closer to zero. Grazers survive the trough on food items, so nothing lets the plant level recover before it hits zero. The consumer mean does not fall. Nothing here lowers consumer numbers, which is what the TODO entry says the ceiling is doing. This is an interpretation of the series above.

## Recommendation

Change no default on this evidence. Lowering `leaf_capacity_per_tile`, alone or with `--strike-cost 0.3`, does not keep the world off the ceiling safely:

- From 0.015 down, it keeps the world off the ceiling only by emptying the plant level.
- 0.0175 and 0.019 shorten seed 42's ceiling time. But on at least one of the runs, each loses the plant level entirely: 0.0175 on seed 3, and 0.019 with strike 0.3 on seed 42.

Next steps, in order:

1. The design's answer to `consumer-ceiling-regulation` is a hunter level, but `hunter-emergence` is parked (2026-09-25) under `plans/2026-09-25-phase2-biomes.md`, which keeps the ceiling out of scope as work and re-reads it in every phase 2 audit. Every energetic lever with a flag has now been tried: food, strike cost and leaf capacity. Each one moves room between plants and grazers, and none lowers the consumer mean.
2. The plant collapses here all run through grazers surviving the trough on food items (`founding-boom-food-regen`, and the food-items observation in "Canopy light sharing"). If an energetic route is still wanted, making grazers depend on plants more than on food items would come before any further change to leaf capacity. That is a design question, not a flag sweep.
3. Do not sweep leaf capacity further with the current levers. The useful range is within about 12.5% of the default (0.0175 is a 12.5% cut) and flips plant survival from seed to seed.
