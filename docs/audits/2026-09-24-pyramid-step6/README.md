# Pyramid step 6 audit, 2026-09-24

Step 6 of `plans/2026-09-21-pyramid-top.md` ("Audit and outcome"). The eight-seed, 15k-tick audit on main after every step of the plan has landed: grazing through `eat`, attack as a kill attempt only, the strike cost, the nearest-eater inputs, the size-gate counters and the kill share split by victim tissue (shipped at 0.1 / 0.1). It is also the first audit on the species stay-threshold fix (#45), which landed on main after every measurement in steps 1 to 5 was taken and which changes mating, since mates must share a species. It is read for hunters first, then for the two lower levels, against the step 3 re-read in `docs/audits/2026-09-23-pyramid-reread/`. It is the new baseline for pyramid work.

- **Commit:** 6a0fac1 on `main`.
- **Seeds:** 1, 2, 3, 7, 42, 99, 314, 1000, as in the re-read. One run per seed: headless runs are deterministic. 15000 ticks at the default 10x virtual time, four in parallel on a shared machine (8 to 11 minutes a run), so wall times are not comparable.
- **Constants:** every default at the commit, no overrides: `strike_cost` 1.0, `bite_reach` 1.0, `mouthless_bite_bonus` 0.3, `bite_fraction` 0.3, `kill_transfer_animal` and `kill_transfer_plant` 0.1, `founder_diet_spread` 1.0, `population_ceiling` 6000, species threshold 1.0 with the 1.3 stay rule, `leaf_capacity_per_tile` 0.02, `max_food_density` 0.1.
- **How to repeat:** `cargo build --release`, then for each seed `./target/release/clauvolution --headless 15000 --seed S --dump-history seedS-run1.csv > seedS-run1.txt 2>&1`. `scripts/attractor_audit_summary.py docs/audits/2026-09-24-pyramid-step6` prints the full summary table below.
- **Files:** `seed<S>-run1.txt` is the headless summary, `seed<S>-run1.csv` the 1 Hz history (500 rows). `audit-run.log` is the launcher's log and `README.txt` its one-line header. `control/` holds the #45 control described below: `seed<S>-no45.txt` and `.csv`, and `control-run.log`.

## Definitions

As in the re-read:

- **Hunter:** a non-photosynthesiser with `diet >= 1/3`. **Founding hunters** are the generation-0 ones; the summary's `Founding hunters died` line gives their count, mean age at death and oldest, and the `Consumer-prey gates` block (step 5) says how many ever fired `attack` and how many killed a consumer.
- **Hunters at 5k:** the 1 Hz sample at tick 5011. **At 15k:** the final summary.
- **Last hunter tick:** the last 1 Hz sample with any hunter alive, founder or mutant.
- **Grazer kills of consumers per 1000 consumer-seconds:** `grazer_kills_consumer` summed over the history, divided by the summed grazers, hunters and omnivores over the same samples.
- **Kills by diet >= 0 after t1000:** kills of consumers and plants minus kills by `diet < 0` killers, summed over samples after tick 1000. It includes kills by plants whose diet is non-negative. Recomputed here for the re-read too with the same script; that gives 3,611, 993, 535 and 83 on seeds 1, 3, 7 and 99 against the 3,606, 992, 530 and 80 the re-read note printed, a window-boundary difference of a few kills.
- **At ceiling:** 1 Hz samples with 5700 or more organisms, of 500, with the summary's engagement count and births blocked beside it.
- **Species:** the living species count in the history at tick 5011 and in the final summary. The summary does not report how many species formed after the founding pass, so that number is not available at 15k.

## Hunters and pyramid health at 15000 ticks

| seed | hunters at 5k / 15k | last hunter tick | founding hunters: died, mean age, oldest | founding hunters that fired / killed a consumer | final plants / grazers | plant floor after t1000 | grazers after t1000 | deaths: predation / starvation / disease | grazer kills of consumers (per 1000 consumer-s) | kills by diet >= 0 after t1000 | at ceiling (samples of 500; engagements, births blocked) | species at 5k / 15k |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 0 / 0 | 14611 | 76/76, 224, 411 | 34 / 11 | 3336 / 2663 | 597 | 555..3701 | 26,687 / 22,600 / 63,456 | 15,869 (15.3) | 490 | 150; 6, 4.81M | 22 / 37 |
| 2 | 0 / 0 | 12661 | 97/97, 244, 584 | 29 / 10 | 2904 / 3096 | 447 | 708..3989 | 69,184 / 23,169 / 71,054 | 39,675 (35.2) | 834 | 132; 7, 1.42M | 19 / 60 |
| 3 | 0 / 0 | 4231 | 98/98, 264, 1544 | 38 / 11 | 2294 / 2755 | 267 | 718..3989 | 37,314 / 36,856 / 74,807 | 25,944 (23.1) | 271 | 28; 2, 0.02M | 25 / 41 |
| 7 | 0 / 0 | 691 | 83/83, 263, 710 | 29 / 4 | 1725 / 3443 | 351 | 845..4153 | 43,272 / 38,038 / 66,091 | 27,817 (23.1) | 55 | 2; 0, 0 | 38 / 33 |
| 42 | 0 / 0 | 2161 | 76/76, 256, 669 | 25 / 10 | 3131 / 2869 | 296 | 1214..5069 | 57,300 / 22,644 / 77,734 | 29,628 (18.9) | 469 | 243; 8, 9.20M | 40 / 82 |
| 99 | 0 / 0 | 12421 | 94/94, 279, 961 | 34 / 6 | 2229 / 2495 | 627 | 321..3865 | 30,979 / 28,568 / 65,609 | 10,509 (11.1) | 855 | 54; 2, 1.10M | 22 / 54 |
| 314 | 0 / 0 | 451 | 80/80, 231, 463 | 32 / 7 | 2718 / 2880 | 129 | 802..4455 | 41,961 / 28,923 / 75,078 | 26,688 (21.2) | 1 | 114; 6, 0.66M | 26 / 51 |
| 1000 | 0 / 0 | 721 | 81/81, 223, 731 | 26 / 3 | 2231 / 3213 | 562 | 713..4026 | 64,229 / 21,028 / 71,550 | 45,176 (38.6) | 0 | 152; 5, 2.76M | 21 / 43 |

Full summary table (`scripts/attractor_audit_summary.py`):

| seed | run | plants | grazers | hunters | omnivores | plant % | species | deaths | starv | pred | old | dis | kills | grazer kills | grazes eat/attack | plant kills by consumers / energy kept | body | diet | light | ready p/e |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 1 | 3336 | 2663 | 0 | 0 | 56% | 37 | 113496 | 22600 | 26687 | 753 | 63456 | 26687 | 15869 | 1885038 / 0 | 9740 / 22746.0 | 0.83 | -0.96 | 0.45 | 10% / 41% |
| 2 | 1 | 2904 | 3096 | 0 | 0 | 48% | 60 | 163702 | 23169 | 69184 | 295 | 71054 | 69184 | 39675 | 1646039 / 0 | 27835 / 59035.0 | 1.07 | -0.96 | 0.58 | 2% / 6% |
| 3 | 1 | 2294 | 2755 | 0 | 0 | 45% | 41 | 149255 | 36856 | 37314 | 278 | 74807 | 37314 | 25944 | 1323872 / 0 | 9556 / 18041.0 | 1.17 | -0.97 | 0.54 | 0% / 0% |
| 7 | 1 | 1725 | 3443 | 0 | 0 | 33% | 33 | 147666 | 38038 | 43272 | 265 | 66091 | 43272 | 27817 | 1225419 / 0 | 14457 / 26023.1 | 1.32 | -0.97 | 0.54 | 0% / 0% |
| 42 | 1 | 3131 | 2869 | 0 | 0 | 52% | 82 | 157914 | 22644 | 57300 | 236 | 77734 | 57300 | 29628 | 1662701 / 0 | 27071 / 72035.7 | 0.98 | -0.97 | 0.64 | 18% / 57% |
| 99 | 1 | 2229 | 2495 | 0 | 0 | 47% | 54 | 125474 | 28568 | 30979 | 318 | 65609 | 30979 | 10509 | 1281269 / 0 | 17375 / 32459.4 | 1.10 | -0.97 | 0.49 | 0% / 0% |
| 314 | 1 | 2718 | 2880 | 0 | 0 | 49% | 51 | 146229 | 28923 | 41961 | 267 | 75078 | 41961 | 26688 | 1406345 / 0 | 14731 / 27984.4 | 1.12 | -0.96 | 0.46 | 0% / 1% |
| 1000 | 1 | 2231 | 3213 | 0 | 0 | 41% | 43 | 157716 | 21028 | 64229 | 909 | 71550 | 64229 | 45176 | 2115980 / 0 | 18728 / 38128.2 | 1.33 | -0.96 | 0.69 | 0% / 0% |

Plants / grazers every 1500 ticks from tick 1501:

- **Seed 1:** 1028/1607, 1297/2410, 1566/2835, 2419/2324, 2297/1044, 2005/890, 2221/2277, 2738/3259, 2987/3010, 3336/2663.
- **Seed 2:** 776/1620, 2435/3562, 2297/3703, 2726/3274, 2227/1544, 1689/1041, 1802/1943, 1771/2868, 2519/3481, 2904/3096.
- **Seed 3:** 306/1877, 467/3024, 1108/3930, 1801/3078, 1379/1371, 1271/870, 1693/1932, 2161/3178, 2445/3528, 2294/2755.
- **Seed 7:** 483/1870, 1108/3310, 1602/4072, 1709/3116, 1233/1410, 961/935, 998/1911, 1192/3467, 1306/3931, 1725/3443.
- **Seed 42:** 394/2718, 1153/4845, 1325/4673, 2105/3895, 2170/2190, 2006/1619, 1814/2984, 1521/4479, 2007/3993, 3131/2869.
- **Seed 99:** 977/401, 1925/3258, 2832/3168, 2934/2679, 1992/1097, 1619/701, 1626/1578, 1955/2772, 2299/3396, 2229/2495.
- **Seed 314:** 129/1921, 910/3340, 1699/4297, 2208/3450, 1490/1460, 1221/1036, 1637/2181, 2159/3839, 2470/3526, 2718/2880.
- **Seed 1000:** 585/1392, 2118/3208, 2311/3684, 2699/3300, 2312/1616, 1816/1125, 1976/2163, 2130/3808, 2092/3111, 2231/3213.

## Against the re-read

The re-read ran at 93280c4 (steps 1 and 2 and the strike cost). Since then main gained the nearest-eater inputs (which change every founder's brain draw), the step 5 counters, the tissue split at 0.1 / 0.1, and #45. Re-read then this audit:

| seed | last hunter tick | founding hunter mean age | final plants / grazers | plant floor after t1000 | grazer kills of consumers per 1000 consumer-s | at ceiling (samples) | species at 5k / 15k |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 11911, then 14611 | 259, then 224 | 3376 / 2624, then 3336 / 2663 | 1948, then 597 | 23.8, then 15.3 | 223, then 150 | 39 / 74, then 22 / 37 |
| 2 | 751, then 12661 | 240, then 244 | 1525 / 3389, then 2904 / 3096 | 216, then 447 | 26.5, then 35.2 | 83, then 132 | 24 / 40, then 19 / 60 |
| 3 | 9391, then 4231 | 241, then 264 | 2610 / 3081, then 2294 / 2755 | 184, then 267 | 35.2, then 23.1 | 135, then 28 | 25 / 70, then 25 / 41 |
| 7 | 11191, then 691 | 249, then 263 | 2294 / 3209, then 1725 / 3443 | 622, then 351 | 22.6, then 23.1 | 68, then 2 | 24 / 49, then 38 / 33 |
| 42 | 1021, then 2161 | 270, then 256 | 2032 / 3293, then 3131 / 2869 | 48, then 296 | 34.8, then 18.9 | 75, then 243 | 30 / 60, then 40 / 82 |
| 99 | 841, then 12421 | 249, then 279 | 2787 / 2680, then 2229 / 2495 | 588, then 627 | 20.5, then 11.1 | 85, then 54 | 15 / 52, then 22 / 54 |
| 314 | 661, then 451 | 266, then 231 | 2940 / 3058, then 2718 / 2880 | 502, then 129 | 24.8, then 21.2 | 190, then 114 | 24 / 58, then 26 / 51 |
| 1000 | 841, then 721 | 264, then 223 | 2108 / 3407, then 2231 / 3213 | 433, then 562 | 8.3, then 38.6 | 128, then 152 | 27 / 68, then 21 / 43 |

Death shares across all eight runs: re-read predation 31.0%, starvation 18.0%, disease 50.8%, old age 0.3%; this audit predation 31.9%, starvation 19.1%, disease 48.7%, old age 0.3%.

## The #45 control

The comparison above mixes two changes that alter dynamics: the nearest-eater inputs and #45. To separate them, the control is a throwaway build of 6a0fac1 with #45's code change reverse-applied (`git diff 81a68e2^ 81a68e2 -- crates` applied in reverse; nothing else touched, not committed), run on the same eight seeds at 15000 ticks. On seeds 42, 3 and 7 its history for the first 5000 ticks is identical, on all 57 shared columns, to the step 4 runs in `docs/audits/2026-09-23-pyramid-nearest-eater/`, so every other change on main since step 4 (#43, #44, #60, the step 5 counters, the tissue split) is behaviour-neutral, and the control is the step 4 to 5 rules at 15k without #45. The shipped runs diverge from it at tick 331, the first 1 Hz sample after the second classification pass.

Control (no #45) then this audit:

| seed | last hunter tick | founding hunter mean age | final plants / grazers | plant floor after t1000 | deaths: predation / starvation / disease | grazer kills of consumers per 1000 consumer-s | at ceiling (samples; engagements) | species at 5k / 15k |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 12361, then 14611 | 224, then 224 | 3413 / 2584, then 3336 / 2663 | 694, then 597 | 49,013 / 19,236 / 67,678, then 26,687 / 22,600 / 63,456 | 24.4, then 15.3 | 146; 6, then 150; 6 | 26 / 79, then 22 / 37 |
| 2 | 7981, then 12661 | 251, then 244 | 2900 / 3100, then 2904 / 3096 | 151, then 447 | 39,665 / 28,772 / 74,509, then 69,184 / 23,169 / 71,054 | 23.9, then 35.2 | 107; 6, then 132; 7 | 24 / 75, then 19 / 60 |
| 3 | 1441, then 4231 | 263, then 264 | 1700 / 2373, then 2294 / 2755 | 256, then 267 | 61,941 / 45,097 / 57,144, then 37,314 / 36,856 / 74,807 | 37.0, then 23.1 | 0; 0, then 28; 2 | 28 / 47, then 25 / 41 |
| 7 | 14131, then 691 | 265, then 263 | 2358 / 3399, then 1725 / 3443 | 497, then 351 | 41,760 / 25,640 / 78,473, then 43,272 / 38,038 / 66,091 | 22.4, then 23.1 | 140; 6, then 2; 0 | 18 / 54, then 38 / 33 |
| 42 | 12511, then 2161 | 256, then 256 | 2995 / 3005, then 3131 / 2869 | 320, then 296 | 56,712 / 30,194 / 87,807, then 57,300 / 22,644 / 77,734 | 25.0, then 18.9 | 161; 7, then 243; 8 | 43 / 57, then 40 / 82 |
| 99 | 9691, then 12421 | 278, then 279 | 2254 / 2446, then 2229 / 2495 | 1053, then 627 | 32,422 / 20,813 / 72,204, then 30,979 / 28,568 / 65,609 | 17.0, then 11.1 | 66; 5, then 54; 2 | 17 / 29, then 22 / 54 |
| 314 | 1741, then 451 | 248, then 231 | 2205 / 3133, then 2718 / 2880 | 173, then 129 | 54,979 / 32,409 / 75,282, then 41,961 / 28,923 / 75,078 | 25.4, then 21.2 | 30; 1, then 114; 6 | 24 / 60, then 26 / 51 |
| 1000 | 751, then 721 | 219, then 223 | 1787 / 3579, then 2231 / 3213 | 919, then 562 | 53,750 / 25,216 / 79,672, then 64,229 / 21,028 / 71,550 | 26.0, then 38.6 | 125; 6, then 152; 5 | 27 / 86, then 21 / 43 |

Control death shares: predation 32.1%, starvation 18.7%, disease 48.8%, old age 0.3%. The control also had no hunter at 5000 or 15000 on any seed.

## Reading

- **No hunter level on any seed, at 5000 or at 15000.** Zero hunters at both marks on all eight, as in the re-read and in the #45 control. After tick 1500 no seed held more than two hunters at once (seeds 1, 2 and 3 briefly held two). The late last-hunter ticks (14611 on seed 1, 12661 on seed 2, 12421 on seed 99) are diet mutants alive for 39, 66 and 12 samples of 450; on four seeds the last hunter was seen by tick 2161. The plan's done-when for hunters, alive at 15k on most of the eight seeds, is not met.
- **Founding hunters still die at about 250 ticks.** Every founding hunter died on every seed, at a mean age of 223 to 279 ticks (re-read 240 to 270), oldest 411 to 1544. 25 to 38 of 76 to 98 founding hunters ever fired `attack`, and 3 to 11 per seed killed a consumer. Of their attacks with an unclaimed consumer in reach, the damage gate alone stopped 49% to 87% and the size gate alone 0 to 22 attacks per seed, which is the step 5 finding again on eight seeds.
- **Carnivory does not come back through omnivory.** No seed held more than three omnivores after tick 3000, and the consumers' mean diet ended at -0.96 to -0.97 everywhere. Kills by diet >= 0 killers after tick 1000 were 0 to 855 per seed over 14,000 ticks.
- **Plants and grazers persist and cycle on every seed.** Plant share ended between 33% and 56% (re-read 31% to 56%). Plant floors after tick 1000 were 129 to 627; no seed went under 100, where the re-read had seed 42 at 48. Grazers ranged from 321 to 5069 after tick 1000 and still crash and recover in step with the seasons across all eight seeds (the troughs near ticks 7500 to 9000 appear on every seed, as in the re-read).
- **Competition kills are about where the re-read left them.** Grazer kills of consumers ran at 11.1 to 38.6 per 1000 consumer-seconds (re-read 8.3 to 35.2). Per seed they moved in both directions by up to a factor of four (seed 1000 from 8.3 to 38.6, seed 42 from 34.8 to 18.9). Predation was 31.9% of deaths against 31.0%.
- **The ceiling is still the regulator on seven of eight seeds.** Seven seeds spent 28 to 243 of 500 samples at 5700 organisms or more and engaged the ceiling 2 to 8 times; seed 42 blocked 9.20M births, the most of any run so far. Seed 7 is the exception, with 2 samples near the ceiling and no engagement. Summed over the eight seeds: 875 samples, against 987 in the re-read and 775 in the control. Disease was 48.7% of deaths.
- **Species counts are still above the tuned band.** 33 to 82 at 15000 (mean 50), against 40 to 74 (mean 59) in the re-read and 29 to 86 (mean 61) in the control. At 5000 they were 19 to 40, against 15 to 39 and 17 to 43. Seed 7 is the only seed whose count fell between 5000 and 15000.

## What #45 changed

Read against the control, which differs from this audit only by #45:

- **Nothing in the pyramid.** Hunters are absent at 5000 and 15000 on every seed with and without it. Founding-hunter mean ages match to within 7 ticks on seven seeds (seed 314: 248 without, 231 with). Death shares across the eight seeds agree to within 0.4 percentage points for each cause. Plants and grazers persist on all eight either way.
- **Per-seed outcomes move a lot, in both directions.** Seed 7 went from 140 samples at the ceiling to 2, seed 314 from 30 to 114 and seed 3 from 0 to 28; seed 1's predation deaths fell from 49k to 27k while seed 2's rose from 40k to 69k. With one run per seed and the runs diverging from tick 331, these are what a perturbation to mating does to a 15k-tick trajectory, not a direction; the eight-seed sums (ceiling samples 775 to 875; death shares unchanged) show no consistent effect.
- **Fewer species alive at 15000 on six of eight seeds.** Mean 61 to 50; lower on seeds 1, 2, 3, 7, 314 and 1000, higher on 42 and 99. At 5000 the counts are about the same (means 26 and 27). #45's own 5000-tick measurement found it spreads speciation over the run and cuts churn between species; at 15000 the living count ends lower on most seeds, which fits species being lineages that can go extinct rather than labels reassigned each pass, but how many species formed is not in the summary, so that reading is not measured here.
- **Against the re-read, the step 4 inputs account for the rest.** The control against the re-read isolates the nearest-eater inputs and their founder redraw: species means 59 to 61, ceiling samples 987 to 775, hunters absent on both. Neither change brings a hunter level.

## Interdependence test

Not run. The plan runs it (`--animal-efficiency 0`) only if a hunter level exists, and no seed has a hunter alive at 15000 in this audit or its control. It is blocked on `hunter-emergence`.

## What this settles

Every step of the plan has now been tried on eight seeds at 15000 ticks with the combined rules, and none produced a hunter level: time (step 3), the nearest-eater inputs (step 4) and the size gate (step 5, measured and left alone) do not keep founding hunters alive past about 250 ticks or let a descendant lineage form. The two lower levels are healthier than in phase 1 on every seed. What binds hunters, in the order an attack meets it: about two thirds of founding hunters never fire `attack`, most attacks by those that do bounce off the damage gate, and the few founders that kill still die. Raising the kill share for animal victims is the one change that has formed a hunter lineage (seeds 42 and 3 at 1.0 in the re-read's probe; seed 42 at 0.6 and 1.0 in `docs/audits/2026-09-23-pyramid-tissue-share/`), on one or two seeds of three or four and with plant floors falling, so it was not shipped. The plan's remaining candidate, the omnivore bridge, is a new plan.
