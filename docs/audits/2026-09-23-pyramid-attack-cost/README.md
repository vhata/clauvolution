# Strike cost, 2026-09-23

A follow-on to step 2 of `plans/2026-09-21-pyramid-top.md`, read against the step 2 result in `docs/audits/2026-09-23-pyramid-graze-eat/`. Step 2 moved grazing to `eat`, but grazer kills of consumers rose, and the diagnosis was that firing `attack` costs nothing while a kill removes a competitor for food. This run gives a strike an energy cost to the attacker and sweeps its size.

- **Branch:** `roadmap/pyramid-attack-cost`, on top of `roadmap/pyramid-graze-eat`.
- **Rule:** every attacker (`attack > 0.5`) with at least one living organism within attack range (4 × body size) pays `strike_cost × claw_power × body size` that tick, booked to the ledger's `movement` flow. It pays whether the strike lands, bounces off the damage or size gate, or loses its target to an earlier attacker. Firing with nobody in reach is free. An attacker killed in the same tick is not charged. The cost does not read the attacker's diet or the target's identity.
- **Seeds:** 42, 3, 7. One run per seed at 5000 ticks. At `--strike-cost 0` the history CSVs are byte-identical to the step 2 runs, so the change is inert at 0; the shipped-default runs in this directory are byte-identical to the sweep's `--strike-cost 1.0` runs.
- **Constants:** every default at the branch head: `strike_cost` 1.0 (new), `bite_reach` 1.0, `mouthless_bite_bonus` 0.3, `bite_fraction` 0.3, `kill_transfer_fraction` 0.1, `founder_diet_spread` 1.0, `population_ceiling` 6000, species threshold 1.0.
- **How to repeat:** `cargo build --release`, then for each seed `./target/release/clauvolution --headless 5000 --seed S --dump-history seedS-run1.csv 2> seedS-run1.txt`; add `--strike-cost C` for a sweep point. `scripts/attractor_audit_summary.py docs/audits/2026-09-23-pyramid-attack-cost` prints the summary table for the shipped runs. Runs were three in parallel on a shared machine; wall times are not comparable.
- **Files:** `seed<S>-run1.txt` and `.csv` are the shipped-default runs. `sweep/seed<S>-runc<C>.txt` are the headless summaries of every sweep point (the CSVs were read for the table below and not kept). Two lines are new in the summary: `Founding hunters died: N of M (mean age A ticks, oldest O)` and, in the predation funnel, `Strikes (paid): N (E energy)`. The CSV has no new columns.

## Definitions

As in the step 2 note, plus:

- **Strike:** an attack intent with a living organism in reach. `strikes` counts them at every cost, including 0.
- **Kills by diet >= 0:** all kills minus kills by `diet < 0` killers. It includes kills by plants with a non-negative diet; plants carry the trait and have attack outputs.
- **Grazer kills per 1000 consumer-seconds:** `grazer_kills_consumer` summed over the history CSV, divided by the sum of grazers, hunters and omnivores over the same samples. It corrects for runs holding more consumers.
- **Founding hunters:** generation-0 organisms whose strategy label is hunter (`diet >= 1/3`, not a photosynthesiser). Counted by `death_system` into `SimStats`.
- **Last tick with a hunter:** the last 1 Hz sample with any hunter-labelled organism alive, founders or mutants.
- **At ceiling:** 1 Hz samples with 5700 or more organisms, against `population_ceiling` 6000; the summary's births-blocked count is beside it.

## Sweep at 5000 ticks

| cost | seed | predation deaths | grazer kills of consumers | per 1000 consumer-s | kills by diet >= 0 | attack intents | strikes / energy paid | starvation | final plants / grazers | plants after t1000 | grazers after t1000 | founding hunters: died, mean age, oldest | last tick with a hunter | at ceiling (samples, births blocked) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | 42 | 62,225 | 42,259 | 107.0 | 2,428 | 1,768,988 | 340,478 / 0 | 12,095 | 1493 / 1875 | 196..2490 | 829..3681 | 91/91, 295, 1883 | 4921 | 8, 2,618 |
| 0 | 3 | 30,631 | 22,559 | 73.0 | 499 | 9,655,603 | 5,626,549 / 0 | 9,684 | 1108 / 2178 | 220..2470 | 598..3352 | 94/94, 234, 898 | 4351 | 0, 0 |
| 0 | 7 | 30,401 | 24,813 | 82.3 | 162 | 2,144,624 | 884,948 / 0 | 15,356 | 162 / 1897 | 45..1123 | 619..3029 | 84/84, 247, 810 | 4381 | 0, 0 |
| 0.1 | 42 | 32,675 | 28,780 | 60.0 | 2,110 | 1,227,881 | 503,698 / 9,251 | 20,706 | 0 / 3750 | 0..322 | 959..5893 | 91/91, 302, 2145 | 2131 | 6, 0 |
| 0.1 | 3 | 17,502 | 16,550 | 60.0 | 299 | 4,983,987 | 2,947,462 / 18,762 | 18,152 | 65 / 1560 | 13..168 | 582..2764 | 94/94, 251, 540 | 1231 | 0, 0 |
| 0.1 | 7 | 24,082 | 20,179 | 65.8 | 423 | 4,538,581 | 2,714,269 / 16,849 | 18,410 | 123 / 2112 | 115..394 | 682..3143 | 84/84, 253, 1072 | 1051 | 0, 0 |
| 0.3 | 42 | 35,712 | 26,618 | 60.9 | 2,099 | 3,317,723 | 1,688,187 / 33,900 | 10,865 | 229 / 2271 | 229..955 | 935..4419 | 91/91, 293, 1869 | 4711 | 0, 0 |
| 0.3 | 3 | 11,682 | 10,873 | 36.0 | 290 | 5,941,407 | 3,539,948 / 56,691 | 15,876 | 117 / 2361 | 27..118 | 609..3540 | 94/94, 258, 673 | 931 | 0, 0 |
| 0.3 | 7 | 34,178 | 30,494 | 86.0 | 163 | 2,657,427 | 1,227,565 / 32,591 | 16,162 | 180 / 1495 | 76..1082 | 790..3754 | 84/84, 257, 784 | 4591 | 0, 0 |
| **1.0** | 42 | 30,380 | 27,416 | 61.5 | 874 | 1,050,586 | 519,726 / 61,494 | 23,654 | 409 / 3160 | 48..409 | 1000..4754 | 91/91, 270, 1027 | 1021 | 0, 0 |
| **1.0** | 3 | 27,447 | 16,019 | 53.2 | 330 | 9,713,382 | 5,693,838 / 213,005 | 7,550 | 2415 / 2464 | 184..2891 | 649..3307 | 94/94, 241, 514 | 4711 | 12, 5,450 |
| **1.0** | 7 | 23,104 | 15,925 | 40.9 | 402 | 4,653,916 | 1,939,716 / 131,482 | 8,180 | 1452 / 2917 | 622..2242 | 753..4346 | 84/84, 249, 1152 | 2701 | 21, 227,481 |
| 3.0 | 42 | 36,785 | 25,521 | 42.7 | 4,590 | 2,848,858 | 915,763 / 141,050 | 6,709 | 1111 / 4161 | 761..2063 | 1341..5104 | 91/91, 242, 563 | 631 | 78, 1,621,068 |
| 3.0 | 3 | 14,353 | 7,970 | 22.5 | 379 | 10,509,317 | 5,571,943 / 418,141 | 8,534 | 1659 / 2673 | 407..3085 | 652..4166 | 94/94, 245, 824 | 1831 | 36, 382,948 |
| 3.0 | 7 | 13,403 | 9,171 | 19.0 | 352 | 3,157,246 | 1,210,021 / 125,158 | 7,791 | 1226 / 4191 | 1104..2209 | 974..4743 | 84/84, 232, 903 | 4171 | 54, 1,043,593 |
| 10.0 | 42 | 8,236 | 4,587 | 9.2 | 1,113 | 5,039,818 | 1,567,838 / 295,087 | 5,724 | 2723 / 3275 | 921..3215 | 1232..5068 | 91/91, 224, 538 | 4981 | 88, 4,852,274 |
| 10.0 | 3 | 3,250 | 2,180 | 4.2 | 237 | 4,588,480 | 1,216,236 / 173,686 | 7,814 | 1844 / 4155 | 831..2204 | 1216..5134 | 94/94, 247, 622 | 1021 | 75, 2,294,936 |
| 10.0 | 7 | 2,661 | 1,947 | 4.1 | 56 | 4,599,722 | 1,796,518 / 234,587 | 6,562 | 2309 / 3690 | 1334..2754 | 1070..4405 | 84/84, 235, 726 | 4321 | 71, 3,198,358 |

The sweep was planned as 0, 0.1, 0.3 and 1.0. 3.0 and 10.0 were added after 1.0 to bracket the trend, since 1.0 was the best point and the top of the planned range.

Plants / grazers every 300 ticks at the shipped 1.0:

- **Seed 42:** 153/121, 168/1967, 209/3058, 220/3993, 248/3204, 185/2061, 204/1048, 305/1779, 324/3258, 167/3263, 51/3146, 50/2033, 71/1060, 117/1797, 187/3684, 268/4754, 351/3868.
- **Seed 3:** 152/117, 176/1122, 225/1713, 222/2405, 210/2184, 188/1338, 238/666, 411/939, 960/2127, 1604/2777, 1428/2434, 1175/1513, 1204/728, 1912/1138, 2815/2400, 2838/3159, 2425/2773.
- **Seed 7:** 152/142, 182/1334, 310/2000, 596/2697, 783/2409, 877/1571, 900/775, 1164/1171, 1215/2416, 1094/3837, 1081/3612, 1190/2252, 1201/1119, 1623/1645, 2223/3615, 2033/3963, 1623/4206.

Shipped runs, full summary table:

| seed | run | plants | grazers | hunters | omnivores | plant % | species | deaths | starv | pred | old | dis | kills | grazer kills | grazes eat/attack | plant kills by consumers / energy kept | body | diet | light | ready p/e |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 3 | 1 | 2415 | 2464 | 0 | 1 | 49% | 25 | 44573 | 7550 | 27447 | 2 | 9574 | 27447 | 16019 | 216552 / 0 | 11109 / 18453.3 | 0.89 | -0.63 | 0.62 | 0% / 1% |
| 7 | 1 | 1452 | 2917 | 0 | 0 | 33% | 24 | 51774 | 8180 | 23104 | 3 | 20487 | 23104 | 15925 | 243250 / 0 | 6805 / 13306.5 | 1.05 | -0.95 | 0.72 | 0% / 4% |
| 42 | 1 | 409 | 3160 | 0 | 0 | 11% | 30 | 67649 | 23654 | 30380 | 0 | 13615 | 30380 | 27416 | 32530 / 0 | 2135 / 5026.9 | 1.05 | -0.96 | 0.71 | 1% / 3% |

Energy flows over the shipped runs (photosynthesis / food items / grazing / predation), step 2 then 1.0: seed 42 1.79M / 6.60M / 0.67M / 51.3k, then 0.44M / 6.90M / 0.19M / 8.2k; seed 3 1.88M / 4.44M / 0.83M / 16.9k, then 2.57M / 3.90M / 0.85M / 21.9k; seed 7 0.81M / 5.37M / 0.35M / 14.3k, then 2.48M / 5.39M / 1.24M / 15.6k. Strikes cost 61k, 213k and 131k energy, 3% to 16% of the movement flow.

## Reading

- **A strike cost lowers competition kills, and the effect grows with the cost from 1.0 up.** Grazer kills of consumers per 1000 consumer-seconds were 107, 73 and 82 at 0; 62, 53 and 41 at 1.0; 43, 23 and 19 at 3.0; 9, 4 and 4 at 10.0. Only 10.0 reaches the step 2 criterion of "a small fraction". Below 1.0 the sweep is not monotone (seed 7 at 0.3 rose to 86), and at one run per seed the low points are within the run-to-run spread of these chaotic worlds.
- **What the kills were doing.** At 3.0 and 10.0 the world sits at the population ceiling for a large part of the run (36 to 88 of 166 samples, up to 4.9M births blocked), where at 0 it touched the ceiling once, on one seed. Bystander kills were regulating consumer numbers. Remove them and neither food nor plants hold the population below 6000 on these seeds: the safety ceiling becomes the regulator, which `docs/DECISIONS.md` ("Emergent carrying capacity") says it should not be. At 1.0 the ceiling engages once late in the run on seeds 3 and 7 (12 and 21 samples) and not on 42, close to the step 2 behaviour.
- **The cost does not select attack away; it selects claws away.** Attack intents at 1.0 were 1.05M, 9.71M and 4.65M against 1.77M, 9.66M and 2.14M; seed 7 fired more. Mean claw power at tick 5000 (all organisms) fell from 0.23, 0.07 and 0.16 to 0.13, 0.04 and 0.07. A strike's cost scales with claw power, so a lineage that keeps firing pays less by growing smaller claws, and a strike with small claws bounces off the damage gate more often.
- **Plants and grazers persist and cycle at 1.0.** Grazers boom and crash three times in 5000 ticks on every seed, as before. Plant floors after tick 1000 are 48, 184 and 622 against 196, 220 and 45: seed 7 no longer spends 1700 ticks under 170 plants, but seed 42 holds 50 to 71 plants from tick 3000 to 3600 and its starvation doubles (12,095 to 23,654). Seeds 3 and 7 starve less (9,684 to 7,550; 15,356 to 8,180). At 0.1, seed 42 lost its plants by tick 3000; at 0.3, seed 3 held 27 to 118 plants.
- **Founding hunters are not further starved at 1.0, and they were not surviving before.** Every founding hunter died on every run at every cost. Mean age at death at 1.0 was 270, 241 and 249 ticks against 295, 234 and 247 at 0; the oldest was 1027, 514 and 1152 against 1883, 898 and 810. At 3.0 and 10.0 means were 224 to 247. The founders die of starvation before a random brain learns to strike, so the strike cost barely touches them. Hunter-labelled mutants lasted to tick 1021, 4711 and 2701 at 1.0 against 4921, 4351 and 4381 at 0; with no hunter level on any run, the last-hunter tick is noise from occasional diet mutants.
- **Why 1.0 and not higher.** A killer is offered a tenth of the victim's energy and keeps it times its digestion efficiency. For a consumer victim holding 50 energy, a hunter at diet +0.5 keeps about 2.8; a grazer at diet -0.9 keeps under 0.02. From the founder genome ranges (not measured): a founder has one to three random parts, each a claw with probability 1/8 and size 0.3 to 1.5, `attack_power` 0 to 0.1 and body size 0.5 to 1.5, so a clawless founder strikes with a force under 0.15 and a clawed one with about 0.2 to 2.4. At 1.0 per unit of force a clawed hunter's kill of a 50-energy consumer still pays more than the strike for most of that range, while a grazer's kill is a loss. At 3.0 a founder with force 1 pays more than the kill returns, and at 10.0 every strike costs several times a kill, so predation as an energy path would be closed to the hunters steps 4 to 6 are trying to produce. Combined with the ceiling reading above, 1.0 is the largest value that keeps both the population energy-limited and a hunter's kill profitable.
