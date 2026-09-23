# Size gate on consumer prey, 2026-09-23

Step 5 of `plans/2026-09-21-pyramid-top.md`: measure how many attacks by diet-positive attackers on consumer prey the size gate (attacker body size above 0.6 of the prey's) rejects, and change the gate only if it rejects most of them. It does not. The size gate on its own stops 0.4% to 1.4% of hunter attacks that have a consumer in reach; the damage gate stops 58% to 66% on its own and takes part in 74% to 80%. No rule was changed.

- **Branch:** `roadmap/pyramid-size-gate`, on top of `roadmap/pyramid-nearest-eater` (step 4).
- **Change:** counters only. The history CSVs, with the seven new columns removed, are byte-identical to the step 4 shipped runs in `docs/audits/2026-09-23-pyramid-nearest-eater/`, so those runs are the base and every other summary line here matches them.
- **Seeds:** 42, 3, 7. One run per seed at 5000 ticks; headless runs are deterministic.
- **Constants:** every default at the branch head, unchanged: `strike_cost` 1.0, `bite_reach` 1.0, `mouthless_bite_bonus` 0.3, `bite_fraction` 0.3, `kill_transfer_fraction` 0.1, `founder_diet_spread` 1.0, `population_ceiling` 6000.
- **How to repeat:** `cargo build --release`, then for each seed `./target/release/clauvolution --headless 5000 --seed S --dump-history seedS-run1.csv 2> seedS-run1.txt`. The new block in the summary is headed `Consumer-prey gates`. Runs were three in parallel on a shared machine.

## Definitions

- **Bands.** Attackers are consumers (not photosynthesisers) in one of three nested bands: `diet >= 0`; hunters (the strategy label, `diet >= 1/3`); founding hunters (hunters of generation 0). Plants are left out of every band although they carry the diet gene and can fire `attack`.
- **Intent:** `attack > 0.5`. **Strike:** an intent with any living organism within attack range (4 × body size), the act the strike cost charges.
- **In reach:** an intent with at least one living consumer within attack range that no earlier attacker has claimed this tick. Every in-reach attack lands in exactly one of:
  - **kill:** the strike killed a consumer;
  - **plant:** some consumer passed both gates, but a nearer plant that also passed was struck;
  - **size-only:** no consumer passed both gates, and at least one passed the damage gate while none passed the size gate. Lifting the size gate alone would have allowed a kill;
  - **damage-only:** no consumer passed the damage gate, and at least one passed the size gate. Lifting the damage gate alone would have allowed a kill;
  - **both:** every consumer in reach failed both gates;
  - **mixed:** no consumer passed both, but different consumers failed different gates. Zero on every run.
- **Size gate involved** is size-only plus both: the share of in-reach attacks where the size gate rejected every consumer. **Damage gate involved** is damage-only plus both.
- **Ages** are the attacker's age in ticks at the intent, in buckets under 100, 100 to 199, 200 to 299, 300 to 499, 500 to 999, and 1000 and over.

## Rejection shares at 5000 ticks

Hunters (labelled `diet >= 1/3`), of attacks with a consumer in reach:

| seed | intents | strikes | in reach | kill | plant | size-only | damage-only | both | size gate involved | damage gate involved |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 42 | 4,896 | 346 (7%) | 277 (6%) | 50 (18.1%) | 4 (1.4%) | 1 (0.4%) | 161 (58.1%) | 61 (22.0%) | 22.4% | 80.1% |
| 3 | 10,225 | 1,733 (17%) | 1,553 (15%) | 367 (23.6%) | 3 (0.2%) | 22 (1.4%) | 984 (63.4%) | 177 (11.4%) | 12.8% | 74.8% |
| 7 | 6,539 | 994 (15%) | 886 (14%) | 229 (25.8%) | 2 (0.2%) | 4 (0.5%) | 582 (65.7%) | 69 (7.8%) | 8.2% | 73.5% |

All consumer attackers with `diet >= 0`:

| seed | intents | strikes | in reach | kill | size-only | damage-only | both |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 42 | 9,615 | 1,054 | 881 | 127 (14.4%) | 1 (0.1%) | 649 (73.7%) | 100 (11.4%) |
| 3 | 14,635 | 2,847 | 2,478 | 452 (18.2%) | 22 (0.9%) | 1,809 (73.0%) | 192 (7.7%) |
| 7 | 9,570 | 1,698 | 1,534 | 366 (23.9%) | 4 (0.3%) | 1,081 (70.5%) | 78 (5.1%) |

Founding hunters:

| seed | founders | died, mean age, oldest | fired at all (first intent, mean age) | ever had a consumer in reach (first time, mean age) | killed a consumer | intents | in reach | kill | size-only | damage-only | both |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 42 | 76 | 76, 256, 625 | 25 (8) | 22 (75) | 10 | 4,457 | 210 | 50 | 1 | 104 | 51 |
| 3 | 98 | 98, 263, 1463 | 38 (7) | 35 (44) | 11 | 9,574 | 1,402 | 367 | 22 | 862 | 148 |
| 7 | 83 | 83, 265, 1144 | 29 (9) | 25 (64) | 4 | 6,417 | 884 | 229 | 4 | 580 | 69 |

Hunter intents, and hunter attacks with a consumer in reach, by attacker age (under 100 / 100-199 / 200-299 / 300-499 / 500-999 / 1000+):

| seed | intents | in reach |
| --- | --- | --- |
| 42 | 1815 / 1620 / 979 / 467 / 15 / 0 | 122 / 45 / 43 / 63 / 4 / 0 |
| 3 | 3044 / 2581 / 1862 / 1634 / 641 / 463 | 288 / 399 / 232 / 170 / 267 / 197 |
| 7 | 2116 / 1855 / 1082 / 737 / 605 / 144 | 167 / 313 / 162 / 85 / 126 / 33 |

No hunter was alive at 5000 on any run, as in the base.

## Reading

- **The size gate is not what binds.** Lifting it alone would have turned 1, 22 and 4 hunter attacks into kills over a whole run, 0.4% to 1.4% of attacks with a consumer in reach. Where the size gate rejects, the damage gate usually rejects the same consumers too (the "both" column, 8% to 22%). The plan's condition for changing the gate, that it rejects most hunter attacks, is not met, so the gate was not changed and no sweep was run.
- **The damage gate is the one a hunter hits.** On its own it stops 58% to 66% of in-reach hunter attacks and takes part in 74% to 80%. The damage gate asks for claw power × size minus half the prey's armour × size above 0.1. From the founder genome ranges (an estimate, as in `docs/audits/2026-09-23-pyramid-attack-cost/`), a clawless founder's strike force is under 0.15 and most founders have no claw, so against any armoured consumer a founding hunter bounces.
- **Most founding hunters never attack.** Of 76, 98 and 83 founding hunters, 25, 38 and 29 (a third) ever fired `attack`. Those that do, fire almost from birth (first intent at a mean age of 7 to 9 ticks). That corrects the reading in the "Strike cost" decision that founders "starve before a random brain learns to strike": a brain does not change within a life, and whether a founder fires is settled by its wiring at birth. The firing founders fire on most ticks of their lives (about 180 intents each on seed 42, against a mean lifetime of 256).
- **Most attacks hit air.** 83% to 93% of hunter intents have nobody in attack range. The step 4 reading already estimated that a founder's random brain rarely wires the nearest-eater inputs to movement; firing is not the same as chasing.
- **A few founders do kill, and still die.** 10, 11 and 4 founding hunters killed at least one consumer, and between them made every hunter kill on each run (50, 367 and 229; no descendant hunter killed a consumer). On seed 7 four founders made 229 kills. Whether those founders lived longer is not measured here (the counters do not link a founder's kills to its age at death), but no founder lived past 1463 ticks. A rough estimate, not measured: a kill of a 50-energy consumer returns about 2.8 to a hunter at diet +0.5, so a founder that killed every 20 ticks (the four seed 7 killers averaged 57 kills each, over lifetimes not measured) would take in about 0.14 energy a tick, the same order as a size-1 consumer's base metabolism of 0.08 plus parts, movement and the strike cost. Hunting pays about break-even for the best founders and nothing for the rest.
- **When in life.** Hunter intents fall off with age because founders die around 250 ticks; on seed 42 nothing hunter-labelled fired past age 1000. The in-reach attacks at ages 500 and over on seeds 3 and 7 come from a few long-lived organisms, mostly founders (founders made 90% and all but two of the hunters' in-reach attacks on those seeds).

## What this leaves

Step 5's candidate is ruled out by measurement. The binding constraints, in the order an attack meets them, are that two thirds of founding hunters never fire `attack`; that the third that do fire mostly at nothing; that an attack with a consumer in reach bounces off the damage gate three times in four; and that the kills that land pay about break-even. None of this is the size gate. The plan's remaining candidate is the "hunter bridge" open question (omnivores persisting long enough for carnivory to emerge from them), which step 6's audit reads before any new plan is written.
