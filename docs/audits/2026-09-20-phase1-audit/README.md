# Phase 1 audit, 2026-09-20

The audit that closes phase 1 of `docs/design/simulation-rules.md` (the diet axis) and step 3 of `plans/2026-09-20-plant-physics.md` (surface drag and canopy light sharing). It is the first audit taken on a sim where plants have a consumer and light is a shared resource.

- **Commit:** 59759bc on `roadmap/plant-physics-canopy` (pull request #23, which raised the ceiling to 6000 and shipped founder diet spread 1.0 and bite fraction 0.3 as defaults), run before the headless determinism fix in #31.
- **Seeds:** 1, 2, 3, 7, 42, 99, 314, 1000. Two runs per seed, launched in different batches. 15000 ticks each at the default 10x virtual time, four runs in parallel on an M4 Max with other jobs running, about 17 minutes per run.
- **Constants:** all defaults at the commit: `leaf_capacity_per_tile` 0.02, `CANOPY_RADIUS` 4, `photo_drag` 1.0, `bite_fraction` 0.3, `kill_transfer_fraction` 0.1, `founder_diet_spread` 1.0, `population_ceiling` 6000, species threshold 1.0, `max_food_density` 0.1.
- **How to repeat:** `scripts/attractor_audit.sh docs/audits/<date>-phase1-audit`, then `scripts/attractor_audit_summary.py` on the directory. Runs from #31 onward are reproducible per seed; these were not.
- **Files:** `seed<S>-run<N>.txt` is the headless summary, `seed<S>-run<N>.csv` the 1 Hz history for the whole run (500 rows). `seed42-runprobe-a` and `-b` are the two simultaneous determinism-probe runs. `audit-run.log` is the launcher's log.

## Not comparable to 2026-09-18

The previous audit ran under a 2000 ceiling with kills of plants, no digestion, no grazing, per-tile shading that never engaged, founders in a narrow diet band, and a predator label based on claws. Every one of those changed between the two audits. Treat the two as different sims; what they share is the seed list and the method.

## Results at 15000 ticks

| seed | run | plants | grazers | hunters | omnivores | plant % | species | deaths | starv | pred | old | dis | kills | body | diet | light | ready p/e |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 1 | 1245 | 1058 | 0 | 0 | 54% | 32 | 103435 | 40036 | 42599 | 1949 | 18851 | 42599 | 1.08 | -0.97 | 0.81 | 0% / 0% |
| 1 | 2 | 1315 | 1423 | 0 | 0 | 48% | 25 | 107107 | 34720 | 44472 | 2620 | 25295 | 44472 | 1.12 | -0.96 | 0.78 | 0% / 1% |
| 2 | 1 | 223 | 886 | 0 | 0 | 20% | 12 | 108772 | 48543 | 49001 | 173 | 11055 | 49001 | 1.67 | -0.96 | 0.67 | 0% / 2% |
| 2 | 2 | 331 | 827 | 0 | 0 | 29% | 13 | 109868 | 47570 | 50396 | 162 | 11740 | 50396 | 1.55 | -0.97 | 0.72 | 0% / 1% |
| 3 | 1 | 335 | 1431 | 0 | 0 | 19% | 22 | 85921 | 24047 | 23773 | 1968 | 36133 | 23773 | 1.54 | -0.83 | 0.84 | 0% / 1% |
| 3 | 2 | 85 | 1212 | 0 | 0 | 7% | 21 | 115769 | 45349 | 48300 | 465 | 21655 | 48300 | 1.79 | -0.97 | 0.68 | 0% / 0% |
| 7 | 1 | 18 | 978 | 0 | 0 | 2% | 8 | 118943 | 60973 | 46808 | 165 | 10997 | 46808 | 1.89 | -0.95 | 0.88 | 0% / 1% |
| 7 | 2 | 14 | 837 | 0 | 0 | 2% | 14 | 119812 | 47269 | 55640 | 276 | 16627 | 55640 | 1.93 | -0.96 | 0.59 | 0% / 0% |
| 42 | 1 | 758 | 1277 | 0 | 0 | 37% | 14 | 165488 | 64113 | 72806 | 1296 | 27273 | 72806 | 1.37 | -0.96 | 0.59 | 0% / 0% |
| 42 | 2 | 3601 | 1164 | 0 | 0 | 76% | 28 | 177524 | 58211 | 90491 | 1785 | 27037 | 90491 | 0.74 | -0.96 | 0.42 | 0% / 1% |
| 42 | probe-a | 855 | 1398 | 0 | 0 | 38% | 11 | 155636 | 61493 | 67279 | 1163 | 25701 | 67279 | 1.37 | -0.95 | 0.60 | 0% / 0% |
| 42 | probe-b | 855 | 1398 | 0 | 0 | 38% | 11 | 155636 | 61493 | 67279 | 1163 | 25701 | 67279 | 1.37 | -0.95 | 0.60 | 0% / 0% |
| 99 | 1 | 571 | 754 | 0 | 0 | 43% | 7 | 90381 | 38221 | 41085 | 373 | 10702 | 41085 | 1.30 | -0.95 | 0.69 | 0% / 1% |
| 99 | 2 | 313 | 854 | 0 | 0 | 27% | 15 | 88801 | 43382 | 35314 | 733 | 9372 | 35314 | 1.43 | -0.96 | 0.51 | 0% / 2% |
| 314 | 1 | 1124 | 1400 | 0 | 0 | 45% | 22 | 121890 | 47523 | 48238 | 1045 | 25084 | 48238 | 1.24 | -0.98 | 0.80 | 0% / 1% |
| 314 | 2 | 1193 | 1027 | 0 | 0 | 54% | 18 | 104044 | 52115 | 40878 | 771 | 10280 | 40878 | 1.08 | -0.96 | 0.73 | 0% / 0% |
| 1000 | 1 | 371 | 1092 | 0 | 0 | 25% | 24 | 133972 | 62352 | 62536 | 895 | 8189 | 62536 | 1.55 | -0.96 | 0.76 | 0% / 0% |
| 1000 | 2 | 376 | 983 | 0 | 0 | 28% | 16 | 112869 | 60919 | 49415 | 483 | 2052 | 49415 | 1.50 | -0.96 | 0.89 | 0% / 3% |

Per-run extremes over the whole history (plants min / max, grazers min / max, population range over the second half, ticks spent at the ceiling):

| run | plants min / max | grazers min / max | population, second half | ticks at ceiling |
|---|---|---|---|---|
| 1-1 | 159 / 5355 | 100 / 2280 | 1376 to 3000 | 570 |
| 1-2 | 160 / 5607 | 101 / 2269 | 1556 to 3560 | 1020 |
| 2-1 | 8 / 4070 | 156 / 2769 | 227 to 1265 | 30 |
| 2-2 | 49 / 3909 | 132 / 3012 | 319 to 1786 | 0 |
| 3-1 | 157 / 5936 | 20 / 1562 | 639 to 6000 | 6000 |
| 3-2 | 43 / 5795 | 103 / 2794 | 391 to 2395 | 2070 |
| 7-1 | 12 / 3034 | 134 / 2330 | 258 to 1762 | 0 |
| 7-2 | 8 / 3130 | 145 / 3248 | 300 to 2067 | 0 |
| 42-1 | 156 / 5244 | 184 / 4102 | 983 to 4011 | 1110 |
| 42-2 | 157 / 4631 | 208 / 3119 | 1004 to 5186 | 270 |
| 99-1 | 45 / 4002 | 95 / 3297 | 281 to 1599 | 450 |
| 99-2 | 158 / 4248 | 99 / 2172 | 460 to 1966 | 0 |
| 314-1 | 160 / 5584 | 119 / 3640 | 587 to 3231 | 1380 |
| 314-2 | 161 / 1792 | 120 / 3337 | 530 to 2902 | 0 |
| 1000-1 | 151 / 3734 | 154 / 2335 | 532 to 2767 | 0 |
| 1000-2 | 56 / 895 | 143 / 2783 | 336 to 1767 | 0 |

## Reading

- **Every run holds a plant level and a grazer level at 15000 ticks.** Plant share ended between 2% and 76% (the previous audit: 68% to 100%, eleven of sixteen above 85%). Grazers ended between 754 and 1431 on every run. Populations cycle: plants boom, grazers boom on them, plants crash, grazers starve back, plants recover, with periods of a few thousand ticks and amplitudes visible in the extremes table (plants between 8 and 5936 on the same seed).
- **The new risk is the other way.** Plants fell below 100 at some point in seven of sixteen runs (minimum 8 on seed 2 and seed 7) and recovered in five of them. Seed 7 ended at 14 and 18 plants on its two runs, the grazers (837 and 978) living mostly on terrain food items. Overgrazing to near-extinction is now a live attractor; a plant world is not.
- **Hunters do not exist.** Zero on all sixteen runs, as on every 5000-tick configuration tried. Time does not produce them. See `hunter-emergence` in `TODO.md`.
- **Grazers specialise fully.** The eaters' mean diet ended between -0.83 and -0.98 on every run; the axis is under selection and it points one way.
- **Body size doubled and more.** 0.74 to 1.93 against 0.47 to 0.60 in the previous audit. The minimal-viable-organism drift is gone; whether the new size is grazing (a bigger bite range) or the ceiling lottery no longer favouring the small is for the trait plots.
- **Species** ended between 7 and 32, mean about 18, the same range as before at threshold 1.0.
- **The ceiling is a rule some runs meet.** Eight of sixteen touched 6000, seven of them for under 2100 ticks during the first plant boom. Seed 3 run 1 sat pinned as a plant monoculture for 6000 ticks (ticks 1000 to 7000) and then its grazers returned and ate it down to 335 plants. The lost opening is a delay, not a loss.
- **Light share** ended between 0.42 and 0.89 and plant readiness read 0% on every run; the canopy is doing what it was built to do.
- **Death causes** across all 2.18M deaths: starvation 41%, predation 43%, disease 15%, old age 1%. Predation with no hunters is grazers killing grazers through the shared attack output (`graze-attack-output-split` in `TODO.md`); the ordering hypothesis for it was tested and disproved in #32.
- **Same-seed runs still diverge at this commit** (seed 42: 37% against 76% plant share) while the two simultaneous probe runs are bit-identical, exactly as in the previous audit. #31 found the cause (headless virtual time followed the wall clock) and fixes it; the next audit is reproducible per seed.
- **Wall clock:** about 1000 to 1060 s per 15000-tick run with four concurrent, about 14 ticks per second at populations of 300 to 6000, roughly three times the cost of the 2000-cap sim.

## What this settles

Phase 1's done-when was "plants, grazers and hunters all persist on most seeds at 15k ticks, and the interdependence test shows grazers overrunning plants when hunting is switched off." Two of three levels persist on sixteen of sixteen runs; the third never appears, and the interdependence test cannot run without it. The trophic pyramid has a base and a middle. What it needs next is not a constant but a mechanism by which a hunter can make a living before it starves, and a way for a grazer to bite a plant without the same output killing its neighbour. Both are filed as design questions. Phase 2 (biomes as pressure and barrier) can proceed on the plants-and-grazers world; hunters are the open thread it inherits.
