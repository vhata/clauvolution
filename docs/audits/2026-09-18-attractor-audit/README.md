# Attractor audit, 2026-09-18

Step 2 of `plans/2026-09-17-simulation-rules-rethink.md`: the first audit taken after the four review-backlog fixes (#3 to #6) and the corpse energy fountain fix (#8), so the first one measured on a sim that conserves energy at kills, has working biomes, rebuilds its spatial hash every tick, and attributes deaths correctly.

- **Commit:** 984aedd (`main` after #8), run from this branch at 3b5c392, which adds only the full-run history dump and the audit scripts.
- **Seeds:** 1, 2, 3, 7, 42, 99, 314, 1000. Two runs per seed, launched in different batches so same-seed runs never start together. 15000 ticks each at the default 10x virtual time, six compute workers, four runs in parallel on an M4 Max, about ten minutes per batch.
- **Constants:** `PHOTO_OUTPUT_MULTIPLIER` 1.0, predator energy share 0.1, species threshold 1.3, initial population 400, all defaults at the commit.
- **How to repeat:** `scripts/attractor_audit.sh docs/audits/<date>-attractor-audit`, then `scripts/attractor_audit_summary.py` on the directory.
- **Files:** `seed<S>-run<N>.txt` is the headless summary, `seed<S>-run<N>.csv` the 1 Hz population history for the whole run (500 rows; columns are absolute `tick` and `sim_second`). `seed42-runprobe-a` and `-b` are the two simultaneous determinism-probe runs.

## Not comparable to April 2026

The April audit in `docs/ROADMAP.md` ran with a spatial hash rebuilt once per frame rather than per tick, a moisture map on -1..1 that left half the land without vegetation regrowth, killed photosynthesisers that kept photosynthesising and were killed again, a photosynthesis multiplier of 0.5, and inflated species and kill counts. Its seed list was not recorded. Treat the two audits as describing different sims that happen to fall into the same attractor.

## Results at 15000 ticks

| seed | run 1 plants / foragers / predators | run 2 | plant share | species | body size | runs identical |
|---|---|---|---|---|---|---|
| 1 | 1977 / 1 / 22 | 1875 / 125 / 0 | 99% / 94% | 19 / 11 | 0.60 / 0.59 | no |
| 2 | 1998 / 2 / 0 | 1996 / 4 / 0 | 100% / 100% | 12 / 15 | 0.52 / 0.50 | no |
| 3 | 1521 / 476 / 3 | 1438 / 558 / 4 | 76% / 72% | 16 / 15 | 0.57 / 0.57 | no |
| 7 | 1862 / 138 / 0 | 1862 / 138 / 0 | 93% / 93% | 17 / 17 | 0.54 / 0.54 | yes |
| 42 | 1357 / 624 / 19 | 1822 / 177 / 1 | 68% / 91% | 21 / 29 | 0.57 / 0.52 | no |
| 99 | 1939 / 61 / 0 | 1939 / 61 / 0 | 97% / 97% | 16 / 16 | 0.47 / 0.47 | yes |
| 314 | 1668 / 329 / 3 | 1668 / 329 / 3 | 83% / 83% | 22 / 22 | 0.56 / 0.56 | yes |
| 1000 | 1767 / 232 / 1 | 1767 / 232 / 1 | 88% / 88% | 12 / 12 | 0.53 / 0.53 | yes |

## Reading

- **Green world / plant dominance still fires.** 11 of 16 runs ended above 85% plants and every run ended at or above 68%. Seeds 2 and 99 are effectively monocultures on both runs.
- **Predator extinction is worse than April, and no longer an artefact.** Predators peak at roughly 100 to 480 within the first 3 to 5 sim-seconds (the opening burst on the 400-organism seed population), then collapse. 10 of 16 runs ended with at most one predator and 14 of 16 with at most four. Only seed 1 run 1 (22) and seed 42 run 1 (19) still held a population at 15k ticks.
- **Species count** ended between 11 and 29, mean 17.0, in the range April recorded after the threshold change.
- **Body size** ended between 0.47 and 0.60 on every run. The minimal-viable drift persists.
- **Death causes**, as shares of total deaths across the 16 runs: predation 40% to 75%, starvation 9% to 29%, disease 11% to 18%, old age 1% to 18%. Old age had always read zero before the `Killed` marker fixed its attribution.
- **Lock-in is early.** Where plants cross 80% of the population they do so between 42 and 372 sim-seconds (ticks 1260 to 11160); seed 3 on both runs and seed 42 on run 1 never cross it. The 5000-tick view in the corpse-fountain branch was too short to see the plant creep finish.
- **Same-seed runs are reproducible on some seeds and not others.** Seeds 7, 99, 314, 1000 produced bit-identical summaries on their two runs; seeds 1, 2, 3, 42 diverged, seed 42 from 68% to 91% plants. The determinism probe, two simultaneous runs of seed 42, came out bit-identical to each other at 88% plants and 6 predators, a third distinct outcome for the seed after 68% and 91%. Across the whole audit, runs that started at the same moment matched and runs that started at different moments did not: the four seeds that diverged are the four whose first run was in the first batch after launch. Tracked as `determinism-claim-recheck` in `TODO.md`.

## What this settles for step 3

The attractors in the roadmap were not artefacts of the bugs. Made honest, the sim still falls into a plant world and predators still die out, on more seeds than before. The design questions in the plan stand: energy accounting as an invariant, whether speciation should be brain-led or trait-led, and whether biomes and barriers can hold separate regimes apart. The trajectories in the CSVs cover the whole run, including the opening burst where the regime is decided, which is where the heterogeneity question should start.
