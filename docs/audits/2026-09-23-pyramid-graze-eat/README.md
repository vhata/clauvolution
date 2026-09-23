# Grazing through eat, 2026-09-23

The result of step 2 of `plans/2026-09-21-pyramid-top.md` ("Bite with the mouth"), read against the step 1 baseline in `docs/audits/2026-09-23-pyramid-baseline/`. `eat` now bites the nearest living plant in reach, and `attack` on a plant is an ordinary kill attempt digested at the killer's plant efficiency.

- **Branch:** `roadmap/pyramid-graze-eat`, on top of `roadmap/pyramid-instruments`.
- **Seeds:** 42, 3, 7. One run per seed at 5000 ticks; headless runs are deterministic, and the seed 42 run at bite reach 1.0 reproduced the sweep run's history CSV byte for byte (as did seeds 3 and 7).
- **Constants:** every default at the branch head: `bite_reach` 1.0 (new; see the sweep below), `mouthless_bite_bonus` 0.3 (new, equal to the food-item share), `bite_fraction` 0.3, `kill_transfer_fraction` 0.1, `founder_diet_spread` 1.0, `population_ceiling` 6000, species threshold 1.0. No overrides.
- **How to repeat:** `cargo build --release`, then for each seed `./target/release/clauvolution --headless 5000 --seed S --dump-history seedS-run1.csv 2> seedS-run1.txt`. `scripts/attractor_audit_summary.py docs/audits/2026-09-23-pyramid-graze-eat` prints the table below. Runs were three or four in parallel on an M4 Max with other jobs running; wall times are not comparable.
- **Files:** `seed<S>-run1.txt` is the headless summary, `seed<S>-run1.csv` the 1 Hz history (166 rows). Three columns are new at the end of the CSV: `grazes_eat_by_plant`, `kills_plant_by_consumer`, `plant_kill_energy_consumer`.

## Definitions

As in the baseline note, plus:

- **Grazes through eat:** bites of a living plant by `grazing_system`. `grazes_attack` is 0 by construction now.
- **Plant kills by consumers:** kills of a photosynthesiser by a killer that is not one, and the energy those killers kept after digestion at their plant efficiency. This is the number the plan's open question "should attack on a plant be allowed at all?" asked for.
- **Grazer kill:** killer `diet < 0`, as before. This includes plants with a negative diet; plants carry the trait and have attack outputs.

## Before and after at 5000 ticks

| seed | | predation deaths | grazer kills of consumers | plant kills (by consumers) / energy kept | grazes eat / attack | starvation | plants / grazers at 5000 | plants after t1000, min..max |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 42 | before | 39,338 | 37,055 | 0 | 0 / 1,299,947 | 10,045 | 640 / 1733 | 640..5339 |
| 42 | after | 62,225 | 42,259 | 17,844 (17,504) / 44,628 | 108,957 / 0 | 12,095 | 1493 / 1875 | 196..2490 |
| 3 | before | 19,720 | 18,801 | 0 | 0 / 1,130,131 | 12,383 | 1067 / 1504 | 440..3045 |
| 3 | after | 30,631 | 22,559 | 7,787 (7,596) / 13,977 | 182,345 / 0 | 9,684 | 1108 / 2178 | 220..2470 |
| 7 | before | 20,569 | 19,972 | 0 | 0 / 1,278,505 | 5,948 | 950 / 1511 | 810..5367 |
| 7 | after | 30,401 | 24,813 | 5,440 (5,432) / 12,601 | 59,281 / 0 | 15,356 | 162 / 1897 | 45..1123 |

Full summary table:

| seed | run | plants | grazers | hunters | omnivores | plant % | species | deaths | starv | pred | old | dis | kills | grazer kills | grazes eat/attack | plant kills by consumers / energy kept | body | diet | light | ready p/e |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 3 | 1 | 1108 | 2178 | 0 | 0 | 34% | 44 | 46340 | 9684 | 30631 | 0 | 6025 | 30631 | 22559 | 182345 / 0 | 7596 / 13977.4 | 1.15 | -0.90 | 0.58 | 0% / 1% |
| 7 | 1 | 162 | 1897 | 0 | 0 | 8% | 18 | 49036 | 15356 | 30401 | 0 | 3279 | 30401 | 24813 | 59281 / 0 | 5432 / 12600.5 | 1.33 | -0.91 | 0.76 | 0% / 4% |
| 42 | 1 | 1493 | 1875 | 0 | 0 | 44% | 25 | 82305 | 12095 | 62225 | 0 | 7985 | 62225 | 42259 | 108957 / 0 | 17504 / 44627.7 | 1.04 | -0.95 | 0.76 | 2% / 1% |

Energy flows over the run (photosynthesis / food items / grazing / predation), before then after: seed 42 3.78M / 6.01M / 1.70M / 8.7k, then 1.79M / 6.60M / 0.67M / 51.3k; seed 3 2.40M / 5.06M / 1.11M / 4.0k, then 1.88M / 4.44M / 0.83M / 16.9k; seed 7 3.69M / 4.26M / 1.22M / 5.1k, then 0.81M / 5.37M / 0.35M / 14.3k. Attack intents fell from 9.1M, 8.7M and 9.4M to 1.8M, 9.7M and 2.1M.

## Plants and grazers over time

Plants / grazers every 300 ticks from the history CSVs:

- **Seed 42:** 161/2140, 215/3010, 312/3887, 263/3221, 250/2101, 196/914, 271/1315, 427/2793, 463/3443, 282/2796, 250/1794, 272/891, 663/1218, 2253/2782, 2447/3478, 1677/2719, 1493/1875 at 4981.
- **Seed 3:** 165/1038, 184/1549, 211/2391, 220/2293, 242/1498, 231/672, 322/846, 310/1976, 342/2807, 466/2421, 793/1595, 1071/848, 1586/1114, 2365/2566, 2020/3246, 1427/2862, 1108/2178.
- **Seed 7:** 167/1204, 236/1851, 325/2510, 454/2524, 458/1506, 514/829, 746/1115, 1086/2427, 927/2963, 387/2226, 89/1382, 57/653, 55/987, 97/2130, 91/2797, 109/2437, 162/1897.

Both levels persist on every seed. Grazers peak three times and crash twice on every seed (peaks near ticks 900, 2700 and 4500), as in the baseline. Plants cycle more slowly: they stay at a few hundred through the first grazer cycle and boom only once grazers crash (seeds 42 and 3 near tick 4000, seed 7 near 2500), and seed 7 then spends ticks 3300 to 5000 between 45 and 162 plants. Plants no longer reach the 5000-plus peaks of the baseline.

## Bite reach sweep

The first run used the plan's reach, 3 × body size (the food-item reach). Plants fell from about 150 at tick 30 to 21 or fewer by tick 1500 and were gone or at one or two organisms by 5000 on all three seeds, while grazers lived on food items. The plan names bite reach as the first knob and the mouth bonus as the second; both were swept (`--bite-reach`, `--mouthless-bite`). Final plants / grazers, plants after tick 1000, predation deaths, grazer kills of consumers; 5000 ticks unless marked:

| setting | seed 42 | seed 3 | seed 7 |
| --- | --- | --- | --- |
| reach 3.0 | 1 / 2731, 1..52, 19,267, 16,805 | 0 / 2850, 0..52, 15,644, 15,284 | 0 / 2223, 0..16, 27,423, 26,997 |
| reach 2.0 (3000 ticks) | 0 / 2588, 0..52, 18,832, 16,863 | | |
| reach 1.5 (3000 ticks) | 28 / 3125, 25..118, 26,911, 23,373 | | |
| **reach 1.0** | 1493 / 1875, 196..2490, 62,225, 42,259 | 1108 / 2178, 220..2470, 30,631, 22,559 | 162 / 1897, 45..1123, 30,401, 24,813 |
| reach 0.75 | 2605 / 3389, 744..2605, 47,706, 20,241 | 253 / 1786, 2..361, 19,752, 17,603 | 2206 / 1975, 325..2591, 44,637, 27,751 |
| reach 0.5 | 0 / 1495, 0..2080 (gone by 4800), 73,900, 41,343 | 4476 / 1509, 560..5073, 38,003, 13,803 | |
| reach 3.0, bite fraction 0.1 (3000 ticks) | 1 / 3385, 0..77, 20,913, 18,051 | | |
| reach 3.0, mouthless 0 | 0 / 2367, 0..28, 27,809, 25,486 | | |
| reach 1.0, mouthless 0 | 138 / 3394, 64..389, 27,631, 24,184 | 1166 / 2335, 393..2793, 30,551, 16,937 | 2 / 2166, 2..899, 35,932, 29,532 |

Reach 1.0 with the mouthless share at 0.3 is the only setting that kept both levels alive with a plant floor above 40 on all three seeds; it ships. At 0.75, seed 3 fell to two plants; at 0.5, seed 42 lost its plants and seed 3 became plant-dominated; removing the mouthless bite left seed 7 at two plants. Nothing in the sweep brought grazer kills of consumers close to a small fraction of the baseline.

## Reading

- **Grazing moved to `eat`.** Every bite of a living plant now goes through `eat`: 109k, 182k and 59k bites, none through `attack`. Bites are fewer and larger than before (about 6 energy each on seed 42 against 1.3), because the reach is contact and plants are fuller. Food items remain the largest consumer income (4.4M to 6.6M per run against 0.35M to 0.83M from grazing), as in the baseline. Plants take 2% to 12% of the eat bites (3.8k, 21.9k, 2.9k).
- **The bystander kills did not go away.** Grazer kills of consumers were 42,259, 22,559 and 24,813 against 37,055, 18,801 and 19,972, and predation deaths rose from 39,338, 19,720 and 20,569 to 62,225, 30,631 and 30,401 once plant kills are added. The "done" criterion of step 2 is not met. Per 1000 consumer-seconds, grazer kills of consumers were 64, 57 and 80 in the first 2500 ticks (baseline 102, 92, 87) and 150, 84 and 84 in the second half (baseline 111, 54, 65): lower at first, then higher.
- **Why.** Attack intents fell by four fifths on seeds 42 and 7 (9.1M to 1.8M, 9.4M to 2.1M) once attack stopped feeding anyone, so the behavioural half of the plan's diagnosis holds. But each remaining attack is more likely to kill: kills per intent rose from 0.4% to 3.5% on seed 42. The consumers' mean armour on seed 42 fell from 1.48 to 0.48 (seed 3: 1.04 to 0.51; seed 7 barely moved, 1.10 to 1.03); armour is costly, and once grazes stopped needing claws strong enough to pass plant armour the arms race that kept everyone armoured relaxed, so a strike that used to bounce now kills. On seed 3 attack intents did not fall at all (8.7M to 9.7M). Firing attack is free: a grazer's kill of a consumer returns almost nothing at its animal efficiency, and nothing charges for the attempt, so there is no selection that removes it, and killing a neighbour removes a competitor for food items. The kills are interference competition, not a feeding artefact.
- **Plant kills are a small energy path.** Consumers killed 17.5k, 7.6k and 5.4k plants and kept 44.6k, 14.0k and 12.6k energy from them: 6.7%, 1.7% and 3.6% of the eat-grazing flow, and under 1% of food-item income. Attack on a plant stays a kill attempt; the open question does not need revisiting on these numbers.
- **Hunters still do not exist at 5000 ticks** (0 on every seed), and consumers end strongly plant-leaning (mean diet -0.90 to -0.95).
