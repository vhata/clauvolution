# Hunter-bridge baseline, 2026-09-24

The baseline for `plans/2026-09-24-hunter-bridge.md`, recorded by step 1 ("Instruments: where intermediate diets get their energy and how they die"). It takes the plan's first decision, whether omnivory's disadvantage is structural or ecological, and says whether step 2 runs. The rules are the ones on `main` at 6a0fac1; the only difference in the binary is the new counters, and the same-seed check below shows they do not change the run.

- **Branch:** `roadmap/hunter-bridge-instruments`, the commit that adds `DietBandStats` and `FeedingCounts::omnivore_gates`.
- **Seeds:** 42, 3, 7 and 99. One run per seed at 5000 ticks, since headless runs are deterministic ("Headless runs are deterministic" in `docs/DECISIONS.md`). Seed 99 is the one step 6 seed with a large omnivore population.
- **Constants:** every default at the commit. No overrides.
- **How to repeat:** `cargo build --release`, then for each seed `./target/release/clauvolution --headless 5000 --seed S --dump-history seedS-run1.csv 2> seedS-run1.txt`. Runs were three at a time on an M4 Max with another sweep running; wall times mean nothing.
- **Files:** `seed<S>-run1.txt` is the headless summary; the diet-band timeline (every 500 ticks) and the run totals are at its end, after the energy ledger. `seed<S>-run1.csv` is the 1 Hz history (166 rows). Columns 1 to 64 are unchanged; columns 65 to 185 are new: `omnivore_intents` to `omnivore_rejected_both`, then 17 columns per diet band (`g1_ticks` ... `h2_death_age_sum`), then the 12 off-diagonal cells of the label matrix (`cross_plant_grazer` ... `cross_hunter_omnivore`).

## Definitions

- **Diet bands:** consumers only (not `Genome::is_photosynthesiser`), in the three strategy labels of `classify_strategy`, each split in half: grazer below -2/3 (`g1`) and from -2/3 to -1/3 (`g2`), omnivore above -1/3 and below 0 (`o1`) and from 0 to below 1/3 (`o2`), hunter from 1/3 to below 2/3 (`h1`) and from 2/3 (`h2`). The plan's open question on band edges is settled this way: halves of the labels cost nothing extra over fifths, nest exactly in the labels, and split the omnivore label at 0, which tells a near-grazer from a near-hunter.
- **Income** is the energy the eater kept after digestion, before the `max_organism_energy` clamp, as the ledger books `food`, `grazing` and `predation`: food items, eat bites, consumer kills, plant kills. **Taken** is the plant tissue taken from food items and bites before digestion (after the mouth bonus); kept over taken is the band's **realised plant efficiency**. **Costs** are metabolism, movement (`action_system`) and strikes (`predation_system`).
- **Per organism-tick:** an organism-tick is a tick in which a member of the band paid metabolism. "In/tick" and "cost/tick" divide by it; births and deaths are given per 1000 organism-ticks.
- **Band crossing:** a birth whose child's strategy label (after crossover and mutation) differs from the reproducing parent's. The reproducing parent is the one whose species, generation and position the child takes; the mate only contributes genes through crossover. Births whose mate had another label are counted separately (100, 315, 86 and 256 on the four seeds, of 42k to 58k births), so the convention moves at most that many.
- **Omnivore gates:** the step 5 `GateOutcomes` partition for attackers labelled omnivore.

## Results at 5000 ticks

Final populations and whole-run figures. "Omni / grazer" pairs are labelled omnivores against labelled grazers.

| seed | final plants / grazers / omnivores / hunters | omnivore peak (tick) | last tick with an omnivore | plant floor after 1000 | income per org-tick, omni / grazer | cost per org-tick, omni / grazer | plant tissue taken per org-tick, omni / grazer | realised plant efficiency, omni / grazer | omnivore income from plant tissue | births / deaths per 1000 org-ticks, omni | same, grazer |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 42 | 898 / 3956 / 0 / 0 | 133 (151) | 1051 | 296 | 0.259 / 0.542 | 0.290 / 0.390 | 0.96 / 0.61 | 0.27 / 0.89 | 99.4% | 1.06 / 2.96 | 3.20 / 2.96 |
| 3 | 1127 / 2594 / 1 / 0 | 299 (181) | 4981 | 267 | 0.387 / 0.558 | 0.344 / 0.455 | 1.03 / 0.63 | 0.38 / 0.88 | 99.6% | 2.96 / 3.82 | 4.01 / 3.78 |
| 7 | 1245 / 2673 / 0 / 0 | 100 (121) | 2131 | 351 | 0.251 / 0.559 | 0.293 / 0.450 | 0.82 / 0.64 | 0.30 / 0.88 | 96.6% | 1.09 / 3.84 | 3.78 / 3.57 |
| 99 | 2068 / 3469 / 0 / 0 | 752 (1111) | 2491 | 627 | 0.442 / 0.454 | 0.355 / 0.352 | 1.37 / 0.54 | 0.32 / 0.85 | 99.5% | 2.69 / 2.84 | 3.46 / 3.02 |

How omnivores die, where their children go, and what their attacks do:

| seed | omnivore deaths: starvation / predation / disease | grazer deaths: same | mean age at death, omni / grazer | omnivore births: stay / to grazer / to hunter | grazer births to omnivore | omnivore attack intents | with a consumer in reach | of those, stopped by the damage gate alone | kills of consumers |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 42 | 68% / 30% / 3% | 23% / 33% / 44% | 338 / 311 | 58 / 11 / 1 | 30 | 20,807 | 8,094 | 97.3% | 85 |
| 3 | 57% / 40% / 3% | 46% / 26% / 27% | 260 / 249 | 274 / 90 / 3 | 114 | 54,310 | 22,397 | 98.2% | 216 |
| 7 | 63% / 37% / 1% | 30% / 30% / 40% | 260 / 264 | 29 / 8 / 1 | 22 | 8,871 | 2,244 | 84.4% | 303 |
| 99 | 38% / 57% / 5% | 31% / 21% / 49% | 352 / 280 | 1998 / 67 / 2 | 98 | 598,066 | 138,545 | 98.1% | 2,384 |

Consumer diet ended at -0.93, -0.95, -0.93 and -0.91. No hunter was alive at 5000 on any seed; labelled hunters kept 0.05 to 0.09 energy per organism-tick against costs of 0.25 to 0.26, 43% to 94% of it from plant tissue.

Seed 99 through time (from the history): omnivores 162 / 394 / 654 / 692 / 424 / 129 / 31 / 3 / 0 at ticks 301, 601, 901, 1201, 1501, 1801, 2101, 2401, 2701, against grazers 130 / 287 / 339 / 421 / 401 / 322 / 798 / 2184 / 3486 and plants 221 / 522 / 1170 / 1440 / 977 / 701 / 703 / 688 / 1200. By 500-tick window, omnivores against grazers:

| ticks | income per org-tick | cost per org-tick | plant tissue taken per org-tick | realised plant efficiency | births per 1000 org-ticks |
| --- | --- | --- | --- | --- | --- |
| 0 to 500 | 0.541 / 0.607 | 0.303 / 0.245 | 2.11 / 0.91 | 0.26 / 0.67 | 4.86 / 4.98 |
| 500 to 1000 | 0.524 / 0.537 | 0.353 / 0.271 | 1.77 / 0.84 | 0.29 / 0.63 | 3.81 / 6.52 |
| 1000 to 2000 | 0.377 / 0.461 | 0.365 / 0.333 | 1.01 / 0.73 | 0.37 / 0.62 | 1.64 / 4.28 |
| 2000 to 3000 | 0.295 / 0.418 | 0.366 / 0.325 | 0.72 / 0.50 | 0.41 / 0.83 | 0.90 / 4.18 |

## Reading

- **Omnivores live on plant tissue.** 96.6% to 99.6% of what they keep comes from food items, bites and plant kills; consumer kills are 0.4% to 3.4%. Their attacks do not feed them: 84% to 98% of omnivore attacks with a consumer in reach are stopped by the damage gate alone, and kills are 1% to 14% of those attacks.
- **They are not short of food; they cannot digest it.** Omnivores take more plant tissue per organism-tick than grazers on every seed (0.82 to 1.37 against 0.54 to 0.64 over the run, 2.5 times as much on seed 99) and keep 0.27 to 0.38 of it, against 0.85 to 0.89 for grazers. That ratio is the digestion curve: an omnivore at diet -0.2 digests `(1.2 / 2)^2` = 0.36 of plant tissue, a grazer at -0.9 digests 0.90. Per organism-tick they keep a half (seeds 42, 7) to seven tenths (seed 3) of a grazer's income over the run; seed 99 is the exception, below.
- **On two seeds omnivores run a deficit from the start, on a third from tick 500.** Income per organism-tick is below cost on seeds 42 and 7 over the run and in every window it was alive in, and births per 1000 organism-ticks are about a third of their deaths; on seed 3 income exceeds cost only before tick 500 (0.43 against 0.34) and falls short after (0.31 against 0.37, then 0.14 against 0.38). On every seed the band is kept alive by founders and by grazers' children mutating into it (30, 114, 22 and 98 grazer births became omnivores, as many as or more than the omnivore births that became grazers). Omnivore lineages die out as omnivores more than they drift into grazers: 16% to 25% of omnivore births crossed to grazer on seeds 42, 3 and 7, 3% on seed 99, and 1 to 3 per seed crossed to hunter.
- **Seed 99 is the curve binding through competition.** While plants were plentiful, omnivores took twice the tissue grazers did and matched their income (0.54 against 0.61, then 0.52 against 0.54), and they out-bred grazers into the thousands. They fell from 692 at tick 1201 to 129 at 1801 while grazers were flat (421, 401, 322) and plants fell from 1440 to 701, before the grazer boom: the shortage cut both bands' intake, and at a third of the digestion the omnivores went to break-even (income 0.377 against cost 0.365) while grazers kept a surplus (0.461 against 0.333). The grazer boom after tick 2000 finished what the shortage started. Predation, the other candidate, does not single omnivores out: it took 74% and 49% of omnivore deaths in the 500 to 1000 and 1000 to 2000 windows against 88% and 60% of grazer deaths. Disease took fewer omnivores than grazers on every seed (1% to 5% of deaths against 27% to 49%). Omnivores died about as old as grazers or older (mean age at death 260 to 352 against 249 to 311).

## Decision: structural

The plan's test was: "If omnivores' income is almost all plant tissue and they fall as grazers rise, the curve is binding and step 2 is the lever. If their income matches grazers' per capita and they die of something else, the curve is not the lever and step 2 is skipped." Omnivores' income is almost all plant tissue on all four seeds. Their per-capita income matches grazers' only on seed 99 and only while plants are abundant, and it does so by taking two to three times as much tissue; when plant tissue per head falls, the digestion ratio (0.27 to 0.41 against 0.62 to 0.91) is what separates the band that starves from the band that grows. No cause of death singles them out. The measured shape is the second half of the structural reading with one refinement: on seed 99 omnivores fall as plants fall, just before grazers rise, not after. Neither competition nor predation is removing a band that would otherwise pay its way; the digestion curve makes them pay two to three times as much plant tissue for the same energy, so any shortage removes them first.

**Step 2 runs:** `diet_efficiency_exponent` as a `SimConfig` knob at default 2.0, swept at 1.5 and 1.0 on these four seeds, read against this note's counters. At exponent 1 an omnivore at -0.2 digests 0.6 of plant tissue rather than 0.36, and a grazer at -0.9 digests 0.95 rather than 0.9; the question step 2 answers is whether that gap is small enough for omnivores to hold on through a shortage. Step 2's shipping bar also has to read hunters' income by tissue: labelled hunters here already take 43% to 94% of their small income from plants.

## Same-seed check

The counters do not change the run. For each seed the same command was run with a binary built from `origin/main` (6a0fac1) and one built from this branch. With the new columns removed (`cut -d, -f1-64`), the branch's history CSV is byte-identical to `main`'s:

| seed | md5, main | md5, branch with new columns removed |
| --- | --- | --- |
| 42 | `1ed657ee3d887a17a097a921dba6af81` | `1ed657ee3d887a17a097a921dba6af81` |
| 3 | `f3f613acca8d4ce288dde74380a26629` | `f3f613acca8d4ce288dde74380a26629` |
| 7 | `1b7bebf917dc3209d8222ea095ab01a4` | `1b7bebf917dc3209d8222ea095ab01a4` |
| 99 | `68b5d20509c4dfe156a7a862b351e41f` | `68b5d20509c4dfe156a7a862b351e41f` |

The summaries differ only in the added block at the end (the diet-band timeline and run totals) and in the output file name and wall time.
