# Consumer ceiling sweep, 2026-09-24

A measurement-only proof of concept for the `TODO.md` entry `consumer-ceiling-regulation`. With no hunters and few competition kills, the 6000-organism population ceiling holds consumers down on most seeds: in the step 6 audit (`docs/audits/2026-09-24-pyramid-step6/`) seven of eight seeds engaged it at 15000 ticks and disease was 48.7% of deaths. `docs/DECISIONS.md` ("Emergent carrying capacity") says energy should keep the population under the ceiling and the ceiling should only be a safety net. The entry names three energetic candidates: food-item regeneration, metabolism, and a lower strike cost. This sweep turns the ones that have a CLI flag, at 15000 ticks, and changes no code or default.

- **Commit:** 07cbb76 (the head of `roadmap/pyramid-step6-audit`, PR #76). It has no change under `crates/` or in `Cargo.lock` since 6a0fac1, the step 6 audit's commit, and the first 600 ticks of a default seed 42 run on this build match the step 6 history byte for byte, so the step 6 default runs are the baseline here and were not re-run.
- **Seeds:** 42, 3 and 7, one run each (headless runs are deterministic). 15000 ticks at the default 10x virtual time, at most three at once on a machine shared with another agent's profiling, 256 to 513 s a run, so wall times are not comparable.
- **Levers and values:**
  - **Food items:** `--max-food-density` 0.02, 0.05 and 0.2 around the default 0.1. There is no separate regeneration flag. `food_regeneration_system` spawns `deficit_ratio × food_regen_rate × season multiplier × max_food` items a tick, so this one flag scales both the standing stock and the refill rate.
  - **Strike cost:** `--strike-cost` 0.3 and 3.0 around the default 1.0.
  - **Metabolism:** no CLI flag exists (`crates/clauvolution_app/src/cli.rs`), so it was not swept.
- **How to repeat:** `cargo build --release`, then for each setting and seed `./target/release/clauvolution --headless 15000 --seed S --max-food-density D --dump-history <dir>/seedS.csv > <dir>/seedS.txt 2>&1` (or `--strike-cost C`).
- **Files:** one directory per setting (`food-0.02`, `food-0.05`, `food-0.2`, `strike-0.3`, `strike-3.0`), each holding `seed<S>.txt` (headless summary) and `seed<S>.csv` (1 Hz history, 500 rows). `sweep-run.log` is the launcher log and `README.txt` its header.

## Definitions

As in the step 6 audit, where the same measure exists:

- **At ceiling:** 1 Hz samples with 5700 or more organisms, of 500, with the summary's engagement count and births blocked. **First at 5700+** is the tick of the first such sample. **Plants / grazers at 5700+** is the mean of each over those samples, which says what the crowded world is made of.
- **Death shares:** predation, starvation and disease as shares of all deaths in the summary. Old age is under 1% in every run and is left out.
- **Plant floor:** the fewest plants in any sample after tick 1000. **Grazers after t1000:** their minimum and maximum over the same samples.
- **Consumer mean:** grazers plus hunters plus omnivores, averaged over the samples after tick 1000. **Organism mean** is the same for all organisms. No run had a hunter or omnivore at 15000.
- **Species:** the living species count in the final summary.

## Results at 15000 ticks

| setting | seed | at ceiling (samples; engagements, births blocked) | first at 5700+ | plants / grazers at 5700+ (mean) | deaths: predation / starvation / disease | final plants / grazers | plant floor after t1000 | grazers after t1000 | consumer mean after t1000 | organism mean after t1000 | species at 15k |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| default (step 6) | 42 | 243; 8, 9.20M | 2401 | 2002 / 3987 | 36.3% / 14.3% / 49.2% | 3131 / 2869 | 296 | 1214..5069 | 3191 | 4980 | 82 |
| default (step 6) | 3 | 28; 2, 0.02M | 9781 | 2231 / 3641 | 25.0% / 24.7% / 50.1% | 2294 / 2755 | 267 | 718..3989 | 2255 | 3634 | 41 |
| default (step 6) | 7 | 2; 0, 0 | 4531 | 1584 / 4132 | 29.3% / 25.8% / 44.8% | 1725 / 3443 | 351 | 845..4153 | 2406 | 3574 | 33 |
| food 0.02 | 42 | 277; 9, 10.62M | 961 | 3587 / 2403 | 37.9% / 9.7% / 51.0% | 3507 / 2490 | 2212 | 680..3611 | 1991 | 5337 | 47 |
| food 0.02 | 3 | 54; 3, 0.13M | 8071 | 3676 / 2232 | 52.8% / 9.6% / 37.5% | 4030 / 1970 | 1535 | 289..2811 | 1186 | 4089 | 57 |
| food 0.02 | 7 | 144; 7, 1.91M | 2431 | 3989 / 1980 | 38.7% / 10.8% / 48.8% | 4227 / 1770 | 1598 | 235..2622 | 1334 | 4761 | 49 |
| food 0.05 | 42 | 189; 7, 5.21M | 2791 | 2890 / 3095 | 32.2% / 12.7% / 53.6% | 3080 / 2918 | 1392 | 619..3814 | 2234 | 4820 | 72 |
| food 0.05 | 3 | 27; 1, 0.00M | 9811 | 2948 / 2879 | 33.7% / 19.8% / 46.0% | 2463 / 2187 | 503 | 374..3063 | 1475 | 3464 | 65 |
| food 0.05 | 7 | 146; 7, 1.14M | 4201 | 2761 / 3208 | 40.1% / 13.0% / 46.7% | 3332 / 2666 | 653 | 595..3870 | 2159 | 4586 | 70 |
| food 0.2 | 42 | 241; 9, 6.84M | 691 | 406 / 5583 | 23.9% / 18.0% / 58.0% | 1386 / 4613 | 30 | 1709..5957 | 4284 | 4667 | 56 |
| food 0.2 | 3 | 28; 0, 0 | 931 | 18 / 5802 | 8.4% / 42.8% / 48.7% | 0 / 4575 | 0 | 1208..5951 | 3452 | 3459 | 18 |
| food 0.2 | 7 | 54; 3, 0.55M | 511 | 40 / 5921 | 32.6% / 43.4% / 24.0% | 0 / 3573 | 0 | 1044..5995 | 3272 | 3275 | 19 |
| strike 0.3 | 42 | 194; 8, 3.85M | 2251 | 2147 / 3834 | 47.7% / 11.0% / 41.3% | 2618 / 3379 | 421 | 889..4945 | 2815 | 4724 | 64 |
| strike 0.3 | 3 | 0; 0, 0 | never | - | 44.5% / 25.5% / 30.0% | 1952 / 3137 | 197 | 679..3819 | 2164 | 3292 | 31 |
| strike 0.3 | 7 | 0; 0, 0 | never | - | 27.8% / 34.4% / 37.7% | 977 / 3056 | 353 | 730..4073 | 2276 | 2997 | 35 |
| strike 3.0 | 42 | 253; 8, 13.10M | 2341 | 1819 / 4174 | 13.1% / 17.8% / 67.5% | 1716 / 4281 | 846 | 1346..4747 | 3342 | 4973 | 42 |
| strike 3.0 | 3 | 38; 2, 0.15M | 4411 | 2029 / 3904 | 19.2% / 22.7% / 57.7% | 2169 / 2618 | 216 | 753..4216 | 2270 | 3806 | 59 |
| strike 3.0 | 7 | 91; 5, 0.51M | 6121 | 1947 / 4023 | 13.1% / 21.2% / 64.8% | 2008 / 3371 | 408 | 696..4291 | 2430 | 3975 | 60 |

Samples at the ceiling summed over the three seeds: default 273, food 0.02 475, food 0.05 362, food 0.2 323, strike 0.3 194, strike 3.0 382.

Plants / grazers every 1500 ticks from tick 1501:

- **Default, seed 42:** 394/2718, 1153/4845, 1325/4673, 2105/3895, 2170/2190, 2006/1619, 1814/2984, 1521/4479, 2007/3993, 3131/2869.
- **Default, seed 3:** 306/1877, 467/3024, 1108/3930, 1801/3078, 1379/1371, 1271/870, 1693/1932, 2161/3178, 2445/3528, 2294/2755.
- **Default, seed 7:** 483/1870, 1108/3310, 1602/4072, 1709/3116, 1233/1410, 961/935, 998/1911, 1192/3467, 1306/3931, 1725/3443.
- **Food 0.02, seed 42:** 3730/1542, 3981/2019, 3130/2870, 3738/2262, 3868/1532, 3041/1042, 2747/2862, 3471/2528, 2977/3023, 3507/2490.
- **Food 0.02, seed 3:** 2971/1138, 2332/1524, 2331/1529, 3335/1358, 2675/514, 2374/436, 2985/1146, 3205/2021, 3466/2530, 4030/1970.
- **Food 0.02, seed 7:** 3326/886, 3811/2164, 4171/1829, 4443/1555, 3901/761, 3121/616, 3075/1354, 2824/1861, 2873/1855, 4227/1770.
- **Food 0.05, seed 42:** 1546/1242, 2680/2897, 3007/2993, 3740/2257, 2993/1537, 1952/1018, 2197/1999, 2275/3722, 2625/3375, 3080/2918.
- **Food 0.05, seed 3:** 768/958, 1199/1587, 1805/1986, 2286/1749, 1993/765, 2073/614, 2382/1383, 2814/2563, 2936/2982, 2463/2187.
- **Food 0.05, seed 7:** 1473/1473, 2138/2703, 2784/3212, 3291/2650, 2476/1171, 2309/1018, 2346/1951, 2173/3812, 2219/3777, 3332/2666.
- **Food 0.2, seed 42:** 142/5177, 101/5893, 94/5906, 81/5919, 43/2877, 95/1810, 495/3973, 729/5271, 1001/4996, 1386/4613.
- **Food 0.2, seed 3:** 36/3096, 30/5060, 10/5693, 3/4171, 1/1910, 0/1276, 0/2823, 2/5050, 0/5877, 0/4575.
- **Food 0.2, seed 7:** 21/3589, 6/5992, 1/5641, 0/3917, 0/1951, 0/1208, 0/2291, 0/4000, 0/4422, 0/3573.
- **Strike 0.3, seed 42:** 626/2589, 1180/4819, 1553/4441, 2501/3499, 2115/1526, 2184/1199, 1919/2269, 2070/3929, 2322/3677, 2618/3379.
- **Strike 0.3, seed 3:** 232/1693, 350/2883, 1330/3716, 1574/2971, 1599/1469, 855/833, 1184/1682, 1602/2844, 1841/3547, 1952/3137.
- **Strike 0.3, seed 7:** 475/2064, 1282/3616, 966/3370, 775/2714, 476/1183, 353/809, 569/1775, 926/3309, 947/3848, 977/3056.
- **Strike 3.0, seed 42:** 1019/2907, 1465/4533, 1776/4223, 2444/3552, 2249/2510, 1376/1454, 1333/4528, 1628/4371, 1583/4417, 1716/4281.
- **Strike 3.0, seed 3:** 255/1846, 680/3294, 1775/4209, 2092/3099, 1644/1325, 1235/804, 1619/1733, 1991/3039, 2274/3609, 2169/2618.
- **Strike 3.0, seed 7:** 439/1731, 1339/2920, 1878/3316, 2160/2839, 1639/1379, 1312/968, 1545/2141, 1781/4219, 1948/4045, 2008/3371.

## Reading

Measured, from the table above:

- **No setting keeps all three seeds off the ceiling.** Seed 42 spends 189 to 277 of 500 samples at 5700 or more under every setting, against 243 at the default, and engages the ceiling 7 to 9 times in every run.
- **Less food cuts consumers, and plants take the room.** At 0.05 and 0.02 the consumer mean after tick 1000 falls on every seed (seed 42 3191 to 2234 and 1991; seed 3 2255 to 1475 and 1186; seed 7 2406 to 2159 and 1334), and starvation's share of deaths falls with it, but plants grow into the space: the plant floor rises from 267 to 351 at the default to 503 to 1392 at 0.05 and 1535 to 2212 at 0.02, and the crowded samples turn plant-heavy (3587 to 3989 plants of about 6000 at 0.02). Ceiling time goes up, not down: 362 and 475 samples in all against 273, and seed 7 goes from 2 samples to 146 and 144. Grazers still crash and recover with the seasons at both values, with the troughs lower (grazer minimum after tick 1000 of 235 to 680 against 718 to 1214).
- **More food removes plants.** At 0.2 plants died out by the end on seeds 3 and 7 (floors 0) and fell to a floor of 30 on seed 42, and the ceiling samples are almost all grazers (5583 to 5921). This collapses the bottom level.
- **A higher strike cost raises ceiling time.** At 3.0 predation falls to 13% to 19% of deaths and disease rises to 58% to 68%; ceiling samples are 253, 38 and 91 (382 in all), blocked births on seed 42 reach 13.10M. This matches the 5000-tick strike-cost sweep in `docs/DECISIONS.md` ("Strike cost").
- **A lower strike cost is the only setting that lowers ceiling time on all three seeds.** At 0.3 seed 42 goes from 243 samples to 194 and from 9.20M blocked births to 3.85M; seeds 3 and 7 never reach 5700 (28 and 2 at the default). Predation rises to 28% to 48% of deaths and disease falls to 30% to 41% (45% to 50% at the default). Plants and grazers persist and cycle on all three; plant floors are 421, 197 and 353 against 296, 267 and 351. Seed 7 ends with 977 plants against 1725 and its plants run lower through the second half (353 to 977 against 961 to 1725). The consumer mean moves little (2815, 2164, 2276 against 3191, 2255, 2406), so the lower ceiling time comes mostly from fewer plants and a lower organism mean (4724, 3292, 2997 against 4980, 3634, 3574).
- **Species counts move with no common direction** except at food 0.2, where the two seeds that lost their plants fell to 18 and 19.

Estimated or not settled by this sweep:

- **The strike 0.3 gain is within per-seed spread.** One run per seed, and the #45 control in the step 6 audit showed a small perturbation to mating move a single seed's ceiling samples from 140 to 2. Seeds 3 and 7 were already near zero at the default, so the only seed where 0.3 had room to show an effect is 42, and there it still sat at the ceiling for 39% of the run. The 5000-tick sweep also found the strike cost non-monotone below 1.0 (0.1 lost seed 42's plants). A lower strike cost is plausible as a partial lever, not demonstrated.
- **Why food alone cannot work, read from the mix at the ceiling:** the ceiling counts plants and consumers together, and plants are held by light and leaf capacity, not by consumers' food. Taking energy from consumers hands their share of the 6000 to plants, and giving consumers more lets them eat the plants out. Nothing in this sweep lowers both levels at once. That is an interpretation of the plant / grazer mixes above, not a separate measurement.

## Recommendation

Change no default on this evidence. None of the energetic levers with a flag keeps the population energy-limited on all three seeds without collapsing a level: lower food and a higher strike cost both raise ceiling time, higher food removes plants, and a lower strike cost helps only where the default was already off the ceiling, with seed 42 still pinned.

Next steps, in order:

1. Keep `hunter-emergence` as the first answer, as the TODO entry says: the kill-share probe in `docs/audits/2026-09-23-pyramid-reread/` is still the only run that kept seed 42 off the ceiling for 15000 ticks.
2. If an energetic lever is still wanted, the missing half is the plant level. `--leaf-capacity` exists and was not in this sweep's brief; a lower leaf capacity combined with `--strike-cost 0.3` is the next two-flag probe, on seed 42 first, since it is the seed every setting here left at the ceiling.
3. A metabolism lever needs a flag or constant change first, which is code and out of scope for a measurement pass.
