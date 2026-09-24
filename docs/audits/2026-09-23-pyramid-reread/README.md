# Pyramid re-read, 2026-09-23

Step 3 of `plans/2026-09-21-pyramid-top.md` ("Resolve the TODO and re-read the pyramid"). The first 15k-tick, eight-seed audit on a sim where `eat` grazes, `attack` only kills, and a strike costs the attacker. It is read for hunters first and for the health of the two lower levels second, against the phase 1 audit in `docs/audits/2026-09-20-phase1-audit/`.

- **Commit:** 93280c4 on `roadmap/pyramid-reread`: steps 1 and 2 of the plan (feeding counters; grazing through eat with bite reach 1.0), the strike cost at 1.0, and the behaviour-neutral eat-loop index fix.
- **Seeds:** 1, 2, 3, 7, 42, 99, 314, 1000, the phase 1 list. One run per seed: headless runs are deterministic since #31, and seed 42's founding-hunter line here (91 of 91, mean 270, oldest 1027) is identical to the 5000-tick strike-cost run, as it must be. 15000 ticks at the default 10x virtual time, four in parallel on a shared M4 Max. The first batch took 10 to 13 minutes a run and the second about 75 minutes of wall clock (other agents' sims were running alongside), so wall times are not comparable.
- **Constants:** every default at the commit, no overrides: `strike_cost` 1.0, `bite_reach` 1.0, `mouthless_bite_bonus` 0.3, `bite_fraction` 0.3, `kill_transfer_fraction` 0.1, `founder_diet_spread` 1.0, `population_ceiling` 6000, species threshold 1.0, `leaf_capacity_per_tile` 0.02, `max_food_density` 0.1.
- **How to repeat:** `cargo build --release`, then for each seed `./target/release/clauvolution --headless 15000 --seed S --dump-history seedS-run1.csv > seedS-run1.txt 2>&1`. `scripts/attractor_audit.sh` runs every seed twice and was not used, because two runs of one seed are now identical. `scripts/attractor_audit_summary.py docs/audits/2026-09-23-pyramid-reread` prints the full summary table below.
- **Files:** `seed<S>-run1.txt` is the headless summary, `seed<S>-run1.csv` the 1 Hz history (500 rows). `audit-run.log` is the launcher's log and `README.txt` its one-line header. `probe/` holds the kill-share probe described at the end: `seed<S>-kill1.0.txt` at 5000 ticks, and `seed<S>-kill1.0-15k.txt` and `.csv` at 15000.

## Definitions

As in `docs/audits/2026-09-23-pyramid-attack-cost/`:

- **Hunter:** a non-photosynthesiser with `diet >= 1/3` (the strategy label). **Founding hunters** are the generation-0 ones; the summary's `Founding hunters died` line gives their count, mean age at death and oldest.
- **Last hunter tick:** the last 1 Hz sample with any hunter alive, founder or mutant.
- **Grazer kills of consumers per 1000 consumer-seconds:** `grazer_kills_consumer` summed over the history, divided by the summed grazers, hunters and omnivores over the same samples.
- **Kills by diet >= 0:** all kills minus kills by `diet < 0` killers, summed over the history. It includes kills by plants whose diet is non-negative; plants carry the trait and have attack outputs, and the counters do not separate them.
- **At ceiling:** 1 Hz samples with 5700 or more organisms, of 500. The summary's engagement count and births blocked are beside it. The same count is used for phase 1 below, so it differs from the phase 1 note's own "ticks at ceiling" column.

## Hunters and pyramid health at 15000 ticks

| seed | hunters at 5k / 15k | last hunter tick | founding hunters: died, mean age, oldest | final plants / grazers | plant floor after t1000 | grazers after t1000 | deaths: predation / starvation / disease | grazer kills of consumers (per 1000 consumer-s) | kills by diet >= 0 after t1000 | at ceiling (samples of 500; engagements, births blocked) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 0 / 0 | 11911 | 94/94, 259, 1898 | 3376 / 2624 | 1948 | 740..3605 | 54,240 / 14,412 / 70,075 | 25,365 (23.8) | 3,606 | 223; 9, 6.02M |
| 2 | 0 / 0 | 751 | 87/87, 240, 757 | 1525 / 3389 | 216 | 731..4544 | 46,283 / 40,723 / 63,928 | 32,464 (26.5) | 0 | 83; 3, 0.86M |
| 3 | 0 / 0 | 9391 | 94/94, 241, 514 | 2610 / 3081 | 184 | 649..4207 | 72,152 / 17,043 / 67,662 | 38,884 (35.2) | 992 | 135; 6, 2.23M |
| 7 | 0 / 0 | 11191 | 84/84, 249, 1152 | 2294 / 3209 | 622 | 751..4346 | 41,132 / 25,678 / 86,625 | 26,372 (22.6) | 530 | 68; 4, 0.31M |
| 42 | 0 / 0 | 1021 | 91/91, 270, 1027 | 2032 / 3293 | 48 | 885..4797 | 64,150 / 51,642 / 87,072 | 47,813 (34.8) | 19 | 75; 4, 1.14M |
| 99 | 0 / 0 | 841 | 94/94, 249, 844 | 2787 / 2680 | 588 | 547..3473 | 29,138 / 20,125 / 70,255 | 19,414 (20.5) | 80 | 85; 5, 0.47M |
| 314 | 0 / 0 | 661 | 91/91, 266, 662 | 2940 / 3058 | 502 | 865..4007 | 53,187 / 20,128 / 76,566 | 31,375 (24.8) | 0 | 190; 8, 2.63M |
| 1000 | 0 / 0 | 841 | 89/89, 264, 862 | 2108 / 3407 | 433 | 903..4416 | 16,741 / 29,140 / 96,040 | 10,857 (8.3) | 2 | 128; 7, 1.09M |

Full summary table (`scripts/attractor_audit_summary.py`):

| seed | run | plants | grazers | hunters | omnivores | plant % | species | deaths | starv | pred | old | dis | kills | grazer kills | grazes eat/attack | plant kills by consumers / energy kept | body | diet | light | ready p/e |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 1 | 3376 | 2624 | 0 | 0 | 56% | 74 | 139289 | 14412 | 54240 | 562 | 70075 | 54240 | 25365 | 1776633 / 0 | 24848 / 54218.9 | 0.91 | -0.97 | 0.32 | 2% / 5% |
| 2 | 1 | 1525 | 3389 | 0 | 0 | 31% | 40 | 151279 | 40723 | 46283 | 345 | 63928 | 46283 | 32464 | 1067924 / 0 | 12558 / 25336.8 | 1.03 | -0.97 | 0.52 | 1% / 0% |
| 3 | 1 | 2610 | 3081 | 0 | 0 | 46% | 70 | 157198 | 17043 | 72152 | 341 | 67662 | 72152 | 38884 | 1458635 / 0 | 32187 / 62484.8 | 1.11 | -0.97 | 0.62 | 0% / 0% |
| 7 | 1 | 2294 | 3209 | 0 | 0 | 42% | 49 | 153909 | 25678 | 41132 | 474 | 86625 | 41132 | 26372 | 1275225 / 0 | 13919 / 25806.5 | 1.07 | -0.97 | 0.56 | 0% / 1% |
| 42 | 1 | 2032 | 3293 | 0 | 0 | 38% | 60 | 202915 | 51642 | 64150 | 51 | 87072 | 64150 | 47813 | 683799 / 0 | 14822 / 30619.8 | 1.21 | -0.97 | 0.50 | 0% / 1% |
| 99 | 1 | 2787 | 2680 | 0 | 0 | 51% | 52 | 119828 | 20125 | 29138 | 310 | 70255 | 29138 | 19414 | 1702659 / 0 | 9294 / 18465.6 | 1.03 | -0.97 | 0.50 | 0% / 1% |
| 314 | 1 | 2940 | 3058 | 0 | 0 | 49% | 58 | 150280 | 20128 | 53187 | 399 | 76566 | 53187 | 31375 | 1723542 / 0 | 19625 / 44448.0 | 1.00 | -0.97 | 0.48 | 4% / 19% |
| 1000 | 1 | 2108 | 3407 | 0 | 0 | 38% | 68 | 143123 | 29140 | 16741 | 1202 | 96040 | 16741 | 10857 | 1543289 / 0 | 5303 / 9990.2 | 1.22 | -0.98 | 0.60 | 0% / 1% |

Plants / grazers every 1500 ticks from tick 1501:

- **Seed 1:** 2872/3127, 2697/3303, 3225/2774, 3581/2419, 3063/1273, 2445/886, 2419/1909, 2383/3605, 2733/3267, 3376/2624.
- **Seed 2:** 287/1624, 470/2837, 1377/3761, 1403/2699, 1239/1287, 1174/1011, 1546/2324, 1657/4343, 1752/4247, 1525/3389.
- **Seed 3:** 184/1385, 1436/2522, 2854/3067, 3436/2560, 2754/1445, 1426/983, 1705/2034, 1980/4019, 2404/3593, 2610/3081.
- **Seed 7:** 904/1684, 1057/3695, 2051/3949, 1801/3293, 1374/1357, 648/773, 1277/1814, 2111/3791, 2203/3797, 2294/3209.
- **Seed 42:** 186/2162, 51/3148, 261/4692, 1293/4372, 1171/2043, 880/1242, 739/2033, 1243/3454, 1726/4243, 2032/3293.
- **Seed 99:** 894/1451, 1189/2008, 2181/2924, 2326/2323, 1664/1004, 1587/664, 2164/1673, 2640/3360, 2825/3175, 2787/2680.
- **Seed 314:** 505/1784, 1417/3283, 2130/3865, 2631/3366, 2086/1533, 1827/1145, 1993/2570, 2331/3669, 2721/3277, 2940/3058.
- **Seed 1000:** 518/1930, 1648/4349, 1946/4054, 1794/3408, 1406/1469, 1342/962, 1579/2117, 1722/3966, 1909/4091, 2108/3407.

## Against phase 1

Phase 1 ran two runs per seed at commit 59759bc, before grazing moved to `eat` and before the strike cost. Per 1000 consumer-seconds (predation deaths, then starvation deaths), and samples at the ceiling, phase 1 run 1 / run 2 then this audit:

| seed | predation per 1000 consumer-s | starvation per 1000 consumer-s | samples at ceiling | mean consumers |
| --- | --- | --- | --- | --- |
| 1 | 83.3 / 79.8, then 50.9 | 78.4 / 62.2, then 13.5 | 27 / 36, then 223 | 1021 / 1115, then 2131 |
| 2 | 103.0 / 91.5, then 37.7 | 101.8 / 86.3, then 33.2 | 6 / 4, then 83 | 952 / 1102, then 2455 |
| 3 | 69.4 / 81.0, then 65.3 | 69.7 / 76.3, then 15.4 | 206 / 71, then 135 | 686 / 1189, then 2210 |
| 7 | 94.3 / 86.1, then 35.3 | 122.8 / 73.2, then 22.0 | 0 / 0, then 68 | 993 / 1291, then 2329 |
| 42 | 99.1 / 148.7, then 46.6 | 87.4 / 95.8, then 37.5 | 42 / 12, then 75 | 1467 / 1216, then 2751 |
| 99 | 82.8 / 74.1, then 30.8 | 77.2 / 91.2, then 21.3 | 17 / 3, then 85 | 991 / 952, then 1890 |
| 314 | 70.4 / 74.8, then 42.1 | 69.4 / 95.7, then 15.9 | 56 / 0, then 190 | 1369 / 1089, then 2526 |
| 1000 | 115.7 / 100.0, then 12.8 | 115.2 / 123.2, then 22.2 | 0 / 0, then 128 | 1082 / 989, then 2620 |

Phase 1 had no hunters, and attack on a plant was a graze, so its predation deaths were all consumer kills by grazers and omnivores, the like-for-like comparison for this audit's grazer kills of consumers (8.3 to 35.2 per 1000 consumer-seconds; predation above also counts plant kills). Death shares across all runs: phase 1 predation 43%, starvation 42%, disease 15%, old age 1%; this audit predation 31%, starvation 18%, disease 51%, old age under 1%.

## Reading

- **No hunter level on any seed, at 5000 or at 15000.** Zero hunters at both marks on all eight. After tick 1500 no seed ever held more than one hunter at a time; the late "last hunter" ticks (11911 on seed 1, 11191 on seed 7, 9391 on seed 3) are single diet mutants that lived a few samples each (22, 8 and 25 samples over 13,500 ticks). On the other five seeds the last hunter was seen between ticks 661 and 1021, as in phase 1 ("gone by tick 600 to 1000"). Hunters fall from 22 to 31 at tick 301 to 0 to 5 by tick 601 on every seed.
- **Founding hunters die at the same age as always.** Every founding hunter died on every seed; mean age at death 240 to 270 ticks, oldest 514 to 1898. The 5000-tick runs of steps 2 and the strike cost gave 234 to 295. Neither the output split nor the strike cost nor another 10,000 ticks moves this number.
- **Hunters do not re-emerge from omnivores.** After tick 3000 no seed held more than two omnivores (seed 3; one on four others, none on three). The consumers' mean diet was -0.58 to -0.94 at tick 1501, -0.64 to -0.97 at 5011, and -0.97 to -0.98 at 15000 on every seed. Seed 3 is the slow one (-0.58 at 1501, -0.64 at 5011) and still converged. The squared digestion curve drives every consumer lineage to the plant end and nothing pulls one back up. Kills by diet >= 0 killers after tick 1000 were 0 to 3,606 per seed over 14,000 ticks (0 on seeds 2 and 314, 2 on 1000, 19 on 42); with at most one hunter alive at a time, most of those are plants and rare mutants.
- **Time alone does not produce hunters.** This was the cheapest `hunter-emergence` candidate, and it fails on eight of eight seeds.
- **Plants and grazers persist and cycle on every seed.** Plant share ended between 31% and 56% (phase 1: 2% to 76%). Plant floors after tick 1000 were 48 to 1948; only seed 42 dipped under 100 (51 at tick 3001) and it recovered to 2032. Phase 1 had seven of sixteen runs under 100 and seed 7 ending at 14 and 18 plants; here seed 7 ends at 2294. Grazers ranged from 547 to 4797 after tick 1000 and crash and recover every 1800 ticks or so, in step across all eight seeds (troughs near ticks 1900, 3700, 5500, 7300, 9100, 10900, 12700 and 14500 on every seed), so the cycle is locked to the seasons, as it was in phase 1. Species ended between 40 and 74 against 7 to 32 in phase 1, well above the 10-to-30 band the species threshold was tuned to; the populations are about twice as large.
- **Competition kills are down to roughly a quarter to a half of phase 1 per consumer.** Grazer kills of consumers ran at 8.3 to 35.2 per 1000 consumer-seconds against phase 1 predation of 69 to 149 (all of it grazer and omnivore kills of consumers). Per seed the ratio is about 11% (seed 1000) to 48% (seed 3). Predation's share of deaths fell from 43% to 31%. The absolute counts (10,857 to 47,813 grazer kills of consumers per run) are not a small fraction of phase 1's 23,773 to 90,491 predation deaths, because consumers are about twice as numerous.
- **The ceiling is now the regulator on every seed.** Every seed spent 68 to 223 of 500 samples at 5700 organisms or more (14% to 45% of the run), engaged the ceiling 3 to 9 times, and blocked 0.31M to 6.02M births. In phase 1, 6 of 16 runs never came near it and the median was about 9 samples. The first engagement came at tick 931 to 7951, after the first grazer cycle, so the 5000-tick runs of step 2 and the strike cost (at most 21 of 166 samples) did not see it. The strike cost shipped because at 5000 ticks 1.0 was "the largest value that keeps the population energy-limited"; at 15000 ticks that does not hold at 1.0 either.
- **Disease has taken over the deaths that predation and starvation gave up.** 51% of deaths against 15% in phase 1, 64k to 96k per run. Infection spreads by proximity, and a world pinned at the ceiling is a dense world; starvation per consumer fell by two thirds to four fifths. What regulates consumers now is the ceiling's birth refusal plus density-driven disease, not food and not predators.

## Kill-share probe

Phase 1 concluded that kill share was not what bound hunters ("Diet axis tuning pass" in `docs/DECISIONS.md`: gone by tick 600 to 1000 at shares of 0.1, 0.5 and 1.0). That was measured while `attack` was also how grazers fed, so kill counts were dominated by grazes and bystander kills. Once the audit above showed no hunters at the default share, the cheapest check was whether that conclusion still holds. `--kill-transfer 1.0`, every other default, seeds 42, 3 and 7 at 5000 ticks, then 42, 3, 7 and 1 at 15000:

| seed | hunters at 5k / 15k | hunters at 3001 / 6001 / 9001 / 12001 | last hunter tick | founding hunters: mean age, oldest | final plants / grazers | plant floor after t1000 | predation income (kill share 0.1 run) | kills by diet >= 0 | at ceiling |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 42 | 5 / 10 | 1 / 7 / 19 / 31 | 15001 | 331, 4109 | 108 / 1724 | 17 | 3.49M (34.9k) | 92,478 | 0 |
| 3 | 75 / 0 | 103 / 60 / 0 / 0 | 11341 | 269, 1430 | 0 / 1547 | 0 | 1.24M (66.9k) | 45,098 | 0 |
| 7 | 0 / 0 | 0 / 0 / 0 / 0 | 2491 | 259, 1512 | 0 / 2464 | 0 | 0.12M (28.2k) | 539 | 0 |
| 1 | 0 / 0 | 0 / 0 / 0 / 0 | 2671 | 281, 1216 | 223 / 998 | 2 | 0.70M (57.5k) | 1,961 | 44 |

- **On two seeds a hunter level forms once kills pay.** Seed 42 holds hunters through the whole run, cycling between 0 and 102 (7 at tick 6001, 78 at 7801, 102 at 9601, 31 at 12001, 10 at 15001), with 92k kills by diet >= 0 killers and 3.49M energy of predation income, second only to food items (20.2M) and ten times grazing (0.33M). Seed 3 holds a hunter population of 13 to 134 from tick 600 to 6600, a few to 21 through tick 10,200, and loses it at 11341. This is the first time any configuration has held hunters past tick 3000.
- **Founding hunters still die young.** Mean age 259 to 331 against 241 to 270 at the default share. The hunters that persist are later lineages (seed 42 has one hunter at tick 3001 and grows from there), so a founder's lifetime is not the measure that predicts a hunter level.
- **The share is not shippable as a knob.** It pays out on plant kills too: a grazer that kills a plant keeps the whole plant at its plant efficiency, which is a graze worth several times a bite. Plants went extinct on seeds 3 and 7 (by tick 12,000 and 7,500) and fell to 2 on seed 1; seed 42 bottomed at 17. The probe says payoff binds for a lineage that survives the founder phase; it does not give a sim to ship. It is not a proposed change, and it is recorded here so the next step does not re-derive it.

## What this settles

The output split (`graze-attack-output-split`) did what it was for: grazing goes through `eat` on every seed (0.68M to 1.78M bites, none through attack), attack is a kill attempt, and grazer kills of consumers per consumer are a quarter to a half of phase 1's. What is left is not about outputs: consumer numbers are held by the ceiling and by disease for 14% to 45% of every run. Hunters are absent at 5000 and 15000 on every seed, founders die at 240 to 270 ticks, and time does not bring them back through omnivory. Steps 4 to 6 are still needed. The kill-share probe adds one thing the plan did not have: with attack meaning attack, a hunter level forms on two of four seeds when a kill pays enough, so payoff is now part of what binds, alongside whatever kills the founders.
