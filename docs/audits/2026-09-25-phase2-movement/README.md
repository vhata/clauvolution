# Phase 2 step 2: movement cost interpolated by aquatic adaptation, 15k ticks

Step 2 of `plans/2026-09-25-phase2-biomes.md`. `action_system` now charges `land + aquatic × (water − land)` from the two `TerrainType` movement tables on every tile, with the fin bonus on water, the limb bonus on land and the 0.5 floor as before, and `AQUATIC_MOVE_COST_WEIGHT` is removed. The rule and its reasoning are in `docs/DECISIONS.md` ("Movement cost interpolates by aquatic adaptation").

- **Commit:** 023405f on `roadmap/phase2-movement-cost`, stacked on the step 1 instruments branch.
- **Seeds:** 1, 2, 3, 7, 42, 99, 314, 1000, one run each (headless runs are deterministic), 15000 ticks, no overrides. Three at a time; wall times in the summaries are not comparable with the baseline's.
- **How to repeat:** `cargo build --release`, then for each seed `./target/release/clauvolution --headless 15000 --seed S --dump-history seedS-run1.csv > seedS-run1.txt 2>&1`.
- **Baseline:** `docs/audits/2026-09-25-phase2-baseline/` (same seeds and ticks, the instruments with the old rule). Definitions of regions, crossings, separation and aquatic bands are in its README.

Additional definitions used here:

- **Deep-water share by band:** deep-water consumer organism-ticks over all consumer organism-ticks in the band, summed over the run. Bands in brackets held under 100,000 consumer organism-ticks over the run.
- **Late band mix:** the share of consumer organism-ticks in each aquatic band over ticks 14000 to 15000, standing in for the aquatic distribution of consumers at 15000.
- **At ceiling:** 1 Hz samples with 5700 or more organisms, of 500.
- **Omnivore samples after t3000:** 1 Hz samples after tick 3000 with any omnivore alive.

## Before and after

| seed | deep-water share of consumer ticks aq0 / aq1 / aq2 / aq3 | consumer crossings | plant crossings | new-region events | region separation / null | final plants / grazers | plant floor after t1000 | grazers after t1000 (after) | at ceiling | species at 5k / 15k | omnivores / hunters at 5k; 15k (after) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 31% / 38% / 36% / (35%) -> 26% / 36% / (41%) / (-) | 0 -> 0 | 0 -> 0 | 0 -> 0 | 1.000/1.000 -> 1.000/1.000 | 3336/2663 -> 3192/2472 | 597 -> 754 | 322..3204 | 150 -> 93 | 22/37 -> 16/51 | 0/0; 0/1 |
| 2 | 30% / 33% / (34%) / (-) -> 24% / 31% / (39%) / (-) | 101,425 -> 43,273 | 16,069 -> 15,509 | 224 -> 107 | 0.559/0.488 -> 0.561/0.474 | 2904/3096 -> 3140/2858 | 447 -> 924 | 400..3593 | 132 -> 125 | 19/60 -> 21/47 | 0/0; 0/0 |
| 3 | 34% / 35% / 39% / (-) -> 29% / 30% / (30%) / (-) | 59,046 -> 33,304 | 5,554 -> 6,728 | 112 -> 65 | 0.851/0.844 -> 0.852/0.849 | 2294/2755 -> 3192/2586 | 267 -> 376 | 503..3373 | 28 -> 94 | 25/41 -> 20/52 | 0/0; 0/0 |
| 7 | 33% / 27% / 27% / (33%) -> 20% / 27% / (29%) / (-) | 33,502 -> 16,259 | 4,231 -> 6,415 | 187 -> 165 | 0.932/0.937 -> 0.919/0.926 | 1725/3443 -> 2813/2690 | 351 -> 592 | 489..3250 | 2 -> 70 | 38/33 -> 12/44 | 0/0; 0/0 |
| 42 | 7% / 7% / (7%) / (-) -> 7% / 7% / 8% / (14%) | 0 -> 0 | 0 -> 0 | 0 -> 0 | 1.000/1.000 -> 1.000/1.000 | 3131/2869 -> 2950/3050 | 296 -> 944 | 1044..4203 | 243 -> 267 | 40/82 -> 49/89 | 0/0; 0/0 |
| 99 | 57% / 59% / 59% / (63%) -> 50% / 57% / 57% / 59% | 0 -> 0 | 0 -> 0 | 0 -> 0 | 1.000/1.000 -> 1.000/1.000 | 2229/2495 -> 1524/1475 | 627 -> 1058 | 292..2417 | 54 -> 0 | 22/54 -> 17/31 | 0/0; 0/0 |
| 314 | 23% / 21% / 17% / (23%) -> 15% / 18% / 24% / (-) | 28,704 -> 19,931 | 2,195 -> 5,435 | 202 -> 124 | 0.986/0.986 -> 0.986/0.986 | 2718/2880 -> 3666/2331 | 129 -> 395 | 579..3657 | 114 -> 124 | 26/51 -> 16/26 | 0/0; 0/0 |
| 1000 | 42% / 35% / 34% / (39%) -> 28% / 30% / 40% / 45% | 0 -> 0 | 0 -> 0 | 0 -> 0 | 1.000/1.000 -> 1.000/1.000 | 2231/3213 -> 4062/1935 | 562 -> 661 | 406..3144 | 152 -> 237 | 21/43 -> 29/76 | 0/0; 0/0 |

| seed | consumer ticks by band aq0/aq1/aq2/aq3 (%), whole run | late band mix (%), ticks 14000-15000 | share of all organism-ticks on water | movement energy paid (M) | of it on deep water (M) | biome separation / null | omnivore samples after t3000 | last hunter tick |
|---|---|---|---|---|---|---|---|---|
| 1 | 17/47/36/0 -> 88/12/0/0 | 41/29/29/0 -> 90/10/0/0 | 47% -> 45% | 5.4 -> 8.2 | 1.12 -> 3.76 | 0.408/0.350 -> 0.419/0.335 | 0 -> 67 | 14611 -> 15001 |
| 2 | 57/43/0/0 -> 78/22/0/0 | 93/7/0/0 -> 91/9/0/0 | 62% -> 62% | 5.3 -> 11.0 | 1.26 -> 5.17 | 0.410/0.368 -> 0.408/0.362 | 0 -> 20 | 12661 -> 14191 |
| 3 | 96/4/0/0 -> 73/27/0/0 | 100/0/0/0 -> 66/33/0/0 | 57% -> 56% | 6.3 -> 10.7 | 1.82 -> 5.24 | 0.422/0.381 -> 0.413/0.368 | 32 -> 5 | 4231 -> 14971 |
| 7 | 3/78/19/0 -> 98/2/0/0 | 2/81/17/0 -> 99/1/0/0 | 58% -> 58% | 6.5 -> 10.5 | 1.23 -> 4.57 | 0.411/0.356 -> 0.436/0.354 | 0 -> 6 | 691 -> 8251 |
| 42 | 40/60/0/0 -> 7/91/2/0 | 32/68/0/0 -> 18/80/2/0 | 31% -> 32% | 7.1 -> 10.7 | 0.32 -> 1.65 | 0.408/0.360 -> 0.413/0.363 | 22 -> 13 | 2161 -> 14641 |
| 99 | 82/15/2/0 -> 1/3/13/82 | 99/1/0/0 -> 0/0/0/100 | 68% -> 64% | 4.5 -> 7.0 | 1.79 -> 2.02 | 0.576/0.563 -> 0.541/0.522 | 0 -> 148 | 12421 -> 7381 |
| 314 | 6/69/25/0 -> 12/86/1/0 | 2/90/8/0 -> 15/85/0/0 | 35% -> 36% | 6.0 -> 10.6 | 0.73 -> 3.54 | 0.414/0.372 -> 0.387/0.360 | 45 -> 102 | 451 -> 421 |
| 1000 | 6/81/14/0 -> 2/88/9/1 | 0/87/13/0 -> 3/95/3/0 | 49% -> 51% | 6.7 -> 11.7 | 1.60 -> 4.89 | 0.467/0.395 -> 0.464/0.399 | 0 -> 9 | 721 -> 481 |

Plants / grazers every 1500 ticks from tick 1501, after the change:

- **Seed 1:** 1043/755, 2062/1641, 2643/2250, 3517/1983, 3024/927, 2696/638, 2825/1439, 2880/2614, 3091/2908, 3192/2472.
- **Seed 2:** 1416/1022, 1708/2016, 2799/2706, 3320/2427, 2758/1260, 2564/888, 2242/1677, 2413/3300, 2512/3487, 3140/2858.
- **Seed 3:** 613/1144, 792/2165, 2356/3182, 2595/2500, 2137/1176, 2031/741, 2110/1627, 3005/2992, 3229/2771, 3192/2586.
- **Seed 7:** 843/1199, 827/1825, 793/2311, 1498/2170, 1703/1048, 1893/684, 2409/1615, 2797/3203, 3012/2988, 2813/2690.
- **Seed 42:** 2575/2080, 2502/3493, 3149/2849, 3381/2619, 3516/2087, 2389/1367, 2231/3312, 2266/3734, 2887/3113, 2950/3050.
- **Seed 99:** 1453/582, 1772/1356, 1784/1574, 1668/1560, 1515/688, 1433/417, 1786/1110, 1896/2032, 2053/2398, 1524/1475.
- **Seed 314:** 470/1491, 1012/1738, 1806/2498, 2040/2065, 2693/1156, 2636/781, 3117/1881, 2612/3387, 2891/3107, 3666/2331.
- **Seed 1000:** 950/949, 3511/2486, 2972/3027, 4141/1856, 3599/1303, 2797/768, 2751/1998, 3843/2157, 3999/2001, 4062/1935.

## Reading

**The flatness is gone, but the gradient is shallow.** aq0 is the band with the lowest deep-water share on every seed; before, the share was flat across bands and on seeds 7, 314 and 1000 aq0 had the highest. On the four seeds with more than one region, aq0 consumers still spend 15% to 29% of their organism-ticks in deep water, against 23% to 34% before. Low-aquatic consumers pay 10× there and go anyway.

**Crossings fell for consumers, not for plants.** Consumer crossings fell 57% on seed 2, 44% on seed 3, 51% on seed 7 and 31% on seed 314. Plant crossings (moves, not budding: a newborn starts on its birth region) were flat on seed 2 and rose on 3, 7 and 314, so plants now make 17% to 28% of crossings against 7% to 14% before. This bears on the plan's open question about plants crossing water: slow plants still cross, and a plant's aquatic value is drifting like anyone's.

**Region separation did not move.** Seed 2 stays about 0.08 above its null (0.561 against 0.474; before 0.559 against 0.488). Seeds 3, 7 and 314 are at their nulls, as before. A movement cost alone does not make a barrier on these maps. The likely reasons are the two the plan already schedules: food items still spawn on water (21% to 59% of all food items, eaten in the same share), so a consumer in deep water is fed there (step 5), and most seeds have one landmass (steps 3 and 4). The table was not turned up.

**Selection now acts on the aquatic trait.** This is the largest effect. On seeds 1 and 7 consumers converged on aq0 (90% and 99% of late consumer organism-ticks, from 41% and 2%). On seed 99, 73% water, they converged on aq3 (100%, from 99% aq0): the first population in any audit to become aquatic. Seeds 42, 314 and 1000 kept an aq1 majority, and seeds 2 and 3 an aq0 majority with a larger aq1 share than before.

**Movement costs more.** Movement energy paid over the run rose by 50% to 108%, and on deep water about three to five times as much was paid (1.1 times on seed 99, whose consumers became aquatic).

**Plants and grazers persist and cycle on every seed.** Every plant floor rose (376 to 1058, from 129 to 627). Grazer minima after tick 1000 fell on every seed (292 to 1044, from 321 to 1214). Seed 99 ends at 2,999 organisms and never reached the ceiling; the eight-seed ceiling-sample sum went from 875 to 1010, with seed 7 (2 to 70) and seed 1000 (152 to 237) up and seeds 1 and 99 down.

**Species at 15k** rose on five seeds and fell on three (99: 54 to 31, 314: 51 to 26, 2: 60 to 47). One run per seed, so these are single trajectories.

**Hunters.** Seed 1 has one hunter alive at 15000, and hunters were alive at scattered samples from tick 13111 on (never more than two). The hunter label earned 498 energy from consumer kills over the whole run. These are single mutants of the kind the parked `hunter-emergence` entry already lists (the step 1 baseline had its last hunter at tick 14611 on seed 1, 12661 on seed 2 and 12421 on seed 99), but the reopen condition in the plan, "hunters alive at 15000 on some seed", is met by the letter on seed 1. Omnivores appear at more samples after tick 3000 on six seeds (up to 148 samples on seed 99, at most 10 alive at a time) and on no seed do they persist. No seed had a hunter at 5000.

## Not measured here

- A second run per seed. Headless runs are deterministic, so a second run would reproduce the first; spread across perturbations is not measured.
- Whether the movement energy of a grazer is short of the channel widths on seeds 2, 3 and 7. Crossings fell, so the plan did not require it.
