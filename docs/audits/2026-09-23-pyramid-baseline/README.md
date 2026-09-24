# Pyramid baseline, 2026-09-23

The baseline for `plans/2026-09-21-pyramid-top.md`, recorded by step 1 ("Instruments, with no behaviour change"). Step 2 moves grazing from the `attack` output to `eat`; these are the numbers it is judged against. The rules are the ones on `main` at e1a7f50; the only difference in the binary is the new counters, and the same-seed check below shows they do not change the run.

- **Branch:** `roadmap/pyramid-instruments`, the commit that adds `FeedingCounts` to `PredationStats`.
- **Seeds:** 42, 3, 7. One run per seed at 5000 ticks, since headless runs are deterministic (see "Headless runs are deterministic" in `docs/DECISIONS.md`).
- **Constants:** every default at the commit (`bite_fraction` 0.3, `kill_transfer_fraction` 0.1, `founder_diet_spread` 1.0, `population_ceiling` 6000, species threshold 1.0). No overrides.
- **How to repeat:** `cargo build --release`, then for each seed `./target/release/clauvolution --headless 5000 --seed S --dump-history seedS-run1.csv 2> seedS-run1.txt`. `scripts/attractor_audit_summary.py docs/audits/2026-09-23-pyramid-baseline` prints the table below from the summaries. The runs were three in parallel on an M4 Max with other jobs running; wall times (390 s to 545 s) are not comparable to anything.
- **Files:** `seed<S>-run1.txt` is the headless summary, `seed<S>-run1.csv` the 1 Hz history (166 rows). The history's last seven columns are the new per-second counters: `grazes_eat`, `grazes_attack`, `kills_consumer`, `kills_plant`, `grazer_kills`, `grazer_kills_consumer`, `attacks_no_plant_in_reach`.

## Definitions

- **Grazer kill:** a kill whose killer has `diet < 0`. This is wider than the grazer strategy label (`diet <= -1/3`) so that plant-leaning omnivores count too.
- **Consumer / plant kill:** by the victim, `Genome::is_photosynthesiser` or not. Under the current rules an attack on a plant is a graze, so plant kills are 0 by construction.
- **Grazes through eat / attack:** bites of a living plant, by the output that took them. `eat` does not graze under the current rules, so the eat count is 0 by construction.
- **No plant in reach:** attack intents (`attack > 0.5`) with no living photosynthesiser within attack range (4 x body size), counting plants another attacker had already claimed that tick.

## Results at 5000 ticks

| seed | plants | grazers | hunters | omnivores | species | deaths | starvation | predation | old age | disease | consumer / plant kills | grazer kills (of consumers) | grazes eat / attack | attack intents | no plant in reach | consumer diet |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 42 | 640 | 1733 | 0 | 0 | 25 | 69,017 | 10,045 | 39,338 | 39 | 19,595 | 39,338 / 0 | 37,055 (37,055) | 0 / 1,299,947 | 9,077,603 | 6,434,434 | -0.87 |
| 3 | 1067 | 1504 | 0 | 0 | 24 | 44,155 | 12,383 | 19,720 | 91 | 11,961 | 19,720 / 0 | 18,801 (18,801) | 0 / 1,130,131 | 8,735,306 | 5,991,085 | -0.91 |
| 7 | 950 | 1511 | 0 | 0 | 24 | 44,992 | 5,948 | 20,569 | 93 | 18,382 | 20,569 / 0 | 19,972 (19,972) | 0 / 1,278,505 | 9,399,633 | 6,326,887 | -0.70 |

Kills of consumers by killers with `diet >= 0` (predation minus grazer kills): 2,283 on seed 42, 919 on seed 3, 597 on seed 7. From the history, 2,051, 350 and 513 of them happened by tick 1000, while founding omnivores and hunters were alive. After tick 2500 grazer kills are 100%, 94% and 100% of consumer kills.

## Reading

- **Predation is grazer-on-grazer.** Grazer kills are 94% to 97% of all kills over the run, and every one of them is of a consumer. Predation is 57% (seed 42), 45% (seed 3) and 46% (seed 7) of all deaths, against starvation at 15%, 28% and 13%. This is the problem `graze-attack-output-split` names, measured directly for the first time rather than inferred from "no hunters alive, yet predation deaths".
- **Most attacks fire with no plant to bite.** 71%, 69% and 67% of attack intents had no living plant within reach. Each is an attack that can only hit a consumer or nothing. Grazes succeed on 13% to 14% of intents and kills on 0.2% to 0.4%.
- **All plant feeding goes through `attack`.** 1.13M to 1.30M grazes per run, none through `eat`. Step 2 should move most of this column to the `eat` side.
- **Hunters do not exist at 5000 ticks**, as in the phase 1 audit, and consumers end strongly plant-leaning (mean diet -0.70 to -0.91).

## Same-seed check

The counters do not change the run. For each seed the same command was run with a binary built from `main` (e1a7f50) and one built from this branch. With the seven new columns removed (`cut -d, -f1-47`), the branch's history CSV is byte-identical to `main`'s:

| seed | md5, main | md5, branch with new columns removed |
| --- | --- | --- |
| 42 | `2ad4f6eca64dad205d6614e3f267e6bf` | `2ad4f6eca64dad205d6614e3f267e6bf` |
| 3 | `ac2c5a3673919e01e82259b1dc07067c` | `ac2c5a3673919e01e82259b1dc07067c` |
| 7 | `21ff44b7045b56b8fccb0ef80271a39b` | `21ff44b7045b56b8fccb0ef80271a39b` |

The summaries differ only in the added lines (the two kill lines under "by Predation" and "No plant in reach" in the funnel), in the funnel's "Grazes" line, now "Grazes (eat/attack): 0 / N" with the same N, and in the output file name and wall time.
