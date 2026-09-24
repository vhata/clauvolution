# Hunter-bridge digestion exponent sweep, 2026-09-24

Step 2 of `plans/2026-09-24-hunter-bridge.md` ("The digestion exponent as a knob, swept once"). Step 1 (`docs/audits/2026-09-24-hunter-bridge-baseline/`) found omnivory's disadvantage structural: omnivores keep about a third of the plant tissue they take against about 0.88 for grazers, run at an energy deficit and breed below replacement. This note sweeps the exponent of the digestion curve, `plant_efficiency = ((1 - diet) / 2)^x` and `animal_efficiency = ((1 + diet) / 2)^x`, at 1.5 and 1.0 against the default 2.0, and reads the result with step 1's counters. The knob ships at 2.0.

- **Branch:** `roadmap/hunter-bridge-exponent`, stacked on `roadmap/hunter-bridge-instruments` (#78). The only rule change is `SimConfig::diet_efficiency_exponent` (`--diet-exponent`).
- **Seeds:** 42, 3, 7 and 99, one run per seed (headless runs are deterministic). 5000 ticks at 1.5 and 1.0; 15000 ticks at 1.0, the more promising value. The 2.0 rows are step 1's baseline runs, which are the same binary behaviour (see the same-seed check below).
- **Constants:** every default except `--diet-exponent`.
- **How to repeat:** `cargo build --release`, then for each seed `./target/release/clauvolution --headless 5000 --seed S --diet-exponent X --dump-history seedS-expX.csv 2> seedS-expX.txt`, and `--headless 15000` for the `-15k` files. Three runs at a time on an M4 Max; wall times mean nothing.
- **Files:** `seed<S>-exp1.5.txt/.csv` and `seed<S>-exp1.0.txt/.csv` are the 5000-tick runs; `seed<S>-exp1.0-15k.txt/.csv` the 15000-tick runs. Each 5000-tick exponent 1.0 CSV is the first 166 rows of the matching 15k CSV, byte for byte. The summary files' `Config override` line records the exponent. Column layout as in the baseline note.

## Definitions

As in the baseline note: diet bands, income (energy kept after digestion), plant tissue taken, realised plant efficiency ("plant kept": kept over taken from food items and bites), organism-ticks, band crossings against the reproducing parent. In addition:

- **Omnivores after 3000:** the largest 1 Hz omnivore count after tick 3000; "at end" is the final sample; "last tick" the last sample with any omnivore.
- **After 3000 (income / cost):** labelled omnivores' income and cost per organism-tick summed over samples after tick 3000.
- **Hunter income from animal tissue:** labelled hunters' consumer-kill income over their whole income (food items, bites, consumer kills, plant kills).
- **Plant floor after 1000:** the smallest 1 Hz plant count after tick 1000. **At ceiling:** 1 Hz samples with 5700 or more organisms, with the summary's engagement count.

## Results

| exponent | seed | omnivores: max after 3000 / at end / last tick | omnivore in / cost per org-tick, run | after 3000 | plant kept, omni / grazer | omnivore births / deaths per 1000 org-ticks | omnivore -> hunter crossings | hunters at 5000 / at 15000 / max after 1500 | hunter income from animal tissue | plants at end / floor after 1000 | grazers after 1000 | at ceiling (samples; engagements) | consumer diet | species |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2.0 | 42 | 0 / 0 / 1051 | 0.259 / 0.290 | none alive | 0.27 / 0.89 | 1.06 / 2.96 | 1 | 0 / - / 1 | 8.9% | 898 / 296 | 1214..5069 | 54 of 166; 2 | -0.93 | 40 |
| 2.0 | 3 | 1 / 1 / 4981 | 0.387 / 0.344 | 0.000 / 0.119 | 0.38 / 0.88 | 2.96 / 3.82 | 3 | 0 / - / 2 | 57.3% | 1127 / 267 | 718..3989 | 0 of 166; 0 | -0.95 | 25 |
| 2.0 | 7 | 0 / 0 / 2131 | 0.251 / 0.293 | none alive | 0.30 / 0.88 | 1.09 / 3.84 | 1 | 0 / - / 0 | 30.0% | 1245 / 351 | 846..4112 | 1 of 166; 0 | -0.93 | 38 |
| 2.0 | 99 | 0 / 0 / 2491 | 0.442 / 0.355 | none alive | 0.32 / 0.85 | 2.69 / 2.84 | 2 | 0 / - / 1 | 6.1% | 2068 / 627 | 321..3865 | 26 of 166; 1 | -0.91 | 22 |
| 1.5 | 42 | 1 / 0 / 3991 | 0.371 / 0.319 | 0.000 / 0.125 | 0.33 / 0.81 | 2.48 / 2.80 | 4 | 0 / - / 0 | 6.2% | 1417 / 974 | 1459..4892 | 82 of 166; 3 | -0.78 | 21 |
| 1.5 | 3 | 0 / 0 / 1741 | 0.372 / 0.291 | none alive | 0.44 / 0.87 | 3.69 / 4.16 | 8 | 0 / - / 0 | 43.0% | 1 / 1 | 592..2882 | 0 of 166; 0 | -0.89 | 24 |
| 1.5 | 7 | 0 / 0 / 751 | 0.333 / 0.293 | none alive | 0.38 / 0.81 | 2.08 / 4.62 | 0 | 0 / - / 0 | 16.9% | 882 / 111 | 661..3694 | 0 of 166; 0 | -0.80 | 27 |
| 1.5 | 99 | 0 / 0 / 2341 | 0.332 / 0.284 | none alive | 0.46 / 0.85 | 1.82 / 2.35 | 4 | 0 / - / 1 | 5.6% | 1621 / 418 | 579..3777 | 9 of 166; 1 | -0.90 | 24 |
| 1.0 | 42 | 17 / 0 / 4831 | 0.586 / 0.420 | 0.590 / 0.498 | 0.60 / 0.78 | 3.90 / 4.12 | 7 | 0 / - / 0 | 1.5% | 1787 / 452 | 945..4312 | 21 of 166; 1 | -0.60 | 23 |
| 1.0 | 3 | 2 / 0 / 4501 | 0.457 / 0.336 | 0.462 / 0.547 | 0.56 / 0.82 | 3.95 / 4.22 | 9 | 0 / - / 1 | 21.5% | 1701 / 122 | 729..3369 | 0 of 166; 0 | -0.60 | 11 |
| 1.0 | 7 | 0 / 0 / 1621 | 0.497 / 0.379 | none alive | 0.57 / 0.83 | 3.41 / 4.94 | 0 | 0 / - / 0 | 19.7% | 0 / 0 | 623..3360 | 0 of 166; 0 | -0.82 | 3 |
| 1.0 | 99 | 36 / 0 / 4021 | 0.449 / 0.370 | 0.384 / 0.533 | 0.58 / 0.77 | 3.16 / 3.12 | 14 | 0 / - / 1 | 0.9% | 2437 / 664 | 317..2553 | 0 of 166; 0 | -0.57 | 23 |
| 1.0, 15k | 42 | 17 / 0 / 12301 | 0.583 / 0.419 | 0.499 / 0.449 | 0.60 / 0.82 | 3.88 / 4.14 | 7 | 0 / 0 / 1 | 1.5% | 3163 / 452 | 945..4312 | 195 of 500; 7 | -0.73 | 66 |
| 1.0, 15k | 3 | 2 / 0 / 8521 | 0.456 / 0.336 | 0.274 / 0.394 | 0.56 / 0.87 | 3.95 / 4.22 | 9 | 0 / 1 / 3 | 21.5% | 2296 / 122 | 662..4152 | 49 of 500; 2 | -0.91 | 43 |
| 1.0, 15k | 7 | 0 / 0 / 1621 | 0.497 / 0.379 | none alive | 0.57 / 0.91 | 3.41 / 4.94 | 0 | 0 / 0 / 0 | 19.7% | 0 / 0 | 623..3703 | 0 of 500; 0 | -0.93 | 17 |
| 1.0, 15k | 99 | 36 / 0 / 6301 | 0.449 / 0.370 | 0.371 / 0.521 | 0.58 / 0.88 | 3.16 / 3.12 | 14 | 0 / 0 / 1 | 0.9% | 2639 / 664 | 317..3180 | 37 of 500; 2 | -0.94 | 46 |

Omnivore births and deaths per 1000 organism-ticks after tick 3000 at exponent 1.0 (5000-tick runs): 3.44 / 4.88 on seed 42, 0.00 / 5.15 on seed 3, 0.67 / 6.14 on seed 99. Founding hunters died at a mean age of 296, 272, 272 and 318 ticks at 1.0 (seeds 42, 3, 7, 99) and 280, 256, 227, 295 at 1.5, against 256, 264, 263 and 279 at 2.0.

For the 15000-tick comparison at the default, the step 6 audit of the same rules (`docs/audits/2026-09-24-pyramid-step6/`, on commit 6a0fac1; the step 1 counters do not change a run) gives, for seeds 42, 3, 7 and 99: hunters at 15k 0 / 0 / 0 / 0, last hunter tick 2161 / 4231 / 691 / 12421, final plants 3131 / 2294 / 1725 / 2229, at ceiling 243 / 28 / 2 / 54 samples of 500, species 82 / 41 / 33 / 54.

## Reading

- **No exponent produces a hunter level.** No seed had a hunter alive at tick 5000 at either exponent, and at 15000 ticks at exponent 1.0 one seed (3) had a single hunter at the final sample, one of a few short runs of one to three hunters (samples from 4591 to 4801, from 7321 to 8341 and, with gaps, from 13651 to 15001). That hunter earned nothing after tick 3000: labelled hunters' income after 3000 was 0.000 per organism-tick against a cost of 0.142. No seed held more than three hunters at once after tick 1500.
- **Exponent 1.0 keeps omnivores alive longer, not indefinitely.** At 1.0 omnivores were alive after tick 3000 on three seeds of four (peaks of 17, 2 and 36 on 42, 3 and 99), against one seed and one organism at 2.0. None was alive at 5000 on any seed, and at 15000 ticks the last omnivore was at 12301, 8521, 1621 and 6301. After tick 3000 the band bred below replacement on every seed where it survived (births per 1000 organism-ticks 3.44, 0.00, 0.67 against deaths of 4.88, 5.15, 6.14), and on seeds 3 and 99 its income was below its cost (0.46 against 0.55, 0.38 against 0.53). At 1.5 the last omnivore was at tick 3991, 1741, 751 and 2341.
- **The digestion gap narrows but stays.** Realised plant efficiency for omnivores went from 0.27 to 0.38 at 2.0 to 0.56 to 0.60 at 1.0, but grazers' went from 0.85 to 0.89 to 0.77 to 0.83, because grazers moved inward too: consumer diet at 5000 ended at -0.57 to -0.82 at 1.0 against -0.91 to -0.95 at 2.0, and on seed 99 most grazers sat in the inner grazer band (-2/3 to -1/3) through the run. The ratio of omnivore to grazer plant efficiency rose from 0.30 to 0.43 at 2.0 to 0.68 to 0.77 at 1.0, and that is still enough for a shortage to remove omnivores first. By tick 15000 consumer diet had returned toward the grazer end on three seeds (-0.91, -0.93, -0.94).
- **Omnivores that survive still do not kill.** 0.0% to 0.4% of omnivore income after tick 3000 came from consumer kills at 1.0 (0.3% to 0.9% over the run), so the step 3 premise, intermediates that kill but do not cross into the hunter band, is not met.
- **Flatter curves feed hunters on plants, the design warning.** At 1.0 founders near the hunter edge grew on plant tissue: hunters went from 120 to 266 by tick 241 on seed 42 and from 110 to 183 by tick 331 on seed 99, before dying out by about tick 1050. Labelled hunters' whole-run organism-ticks were about 102k on 42 and 99 against 22k to 35k at 2.0, with income above cost (0.34 against 0.26, 0.35 against 0.31), and 1.5% and 0.9% of that income was from animal tissue. Omnivore-to-hunter crossings rose (7, 9, 0, 14 against 1, 3, 1, 2), and hunter-to-omnivore crossings rose further (30, 9, 2, 59), so the traffic between the bands is mostly the plant-fed founding hunter population drifting down. That is "no tuning gives a trophic pyramid when the apex can also graze" in the design table, measured.
- **Plants pay on some seeds.** At 1.0 seed 7 lost its plants: 127 at tick 211, 23 at 931, none in any sample from tick 2191 to 15000 except a single plant at 13861, with grazers kept going by food items (2544 at 15000, 17 species). At 1.5 seed 3 fell to one plant (floor 1 after tick 1000, 1 at 5000). Elsewhere plants persisted and cycled, and the floors after tick 1000 moved in both directions against the baseline (1.0: 452 / 122 / 0 / 664 against 296 / 267 / 351 / 627). The flatter curve raises what every consumer near the middle keeps from a plant, and in the opening boom that is most of them.
- **The ceiling:** at 15000 ticks and 1.0, seeds 42, 3, 7 and 99 spent 195, 49, 0 and 37 samples of 500 at 5700 organisms or more, against 243, 28, 2 and 54 at the default in step 6. No hunter level appeared to regulate consumers, so the ceiling reading is inside the spread these seeds show.

## Decision

Neither value meets the plan's shipping bar ("keeps omnivores alive after tick 3000 on most of the four seeds, plants and grazers persist and cycle, and any hunters it produces take most of their income from animal tissue"): 1.0 leaves omnivores alive after 3000 on three seeds for a while but on none at 5000, loses the plants on seed 7, and its hunters live on plants; 1.5 does less for omnivores and nearly loses the plants on seed 3. The knob ships at 2.0 with these numbers.

The plan's fallback, a flat-topped curve (option (c)), was to be tried "if neither value leaves intermediates alive after tick 3000". At 1.0 some do, so the fallback's trigger is narrowly not met; step 3 (the animal kill share on top of the step 2 setting) needs intermediates that kill, which these do not. The reading above points past the curve's shape: flattening it helps the grazers next to the omnivores as much as it helps the omnivores, so the gap that a shortage acts on survives any curve under which a specialist out-digests a generalist, which the plan requires. What is left is the plan's "If the bridge fails" open question.

## Same-seed check

The knob at its default does not change the run. With a binary built from `roadmap/hunter-bridge-instruments` (3f5b020) and one built from this branch, `--headless 2000 --seed S --dump-history` gave byte-identical history CSVs, and summaries identical except for the output file name and wall time:

| seed | md5, #78 branch | md5, this branch |
| --- | --- | --- |
| 42 | `e2a2d2da2c86b186bd63b990a36a4ace` | `e2a2d2da2c86b186bd63b990a36a4ace` |
| 3 | `27508e08422876a6e102eb8ebda46894` | `27508e08422876a6e102eb8ebda46894` |

The first build of this branch computed the curve with `powf(2.0)` and did not match: `powf` differs from `h * h` in the last bit for some inputs. The shipped code multiplies at exactly 2.0 and calls `powf` otherwise, and a unit test checks the default against `h * h` bit for bit over the diet axis.
