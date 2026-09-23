# Nearest-eater inputs, 2026-09-23

Step 4 of `plans/2026-09-21-pyramid-top.md`: four brain inputs giving the direction, closeness and size ratio of the nearest non-photosynthesiser in sense range. The step's done criterion is that the mean lifetime of founding hunters on seeds 42, 3 and 7 at 5000 ticks is longer than on the base. It is not met: lifetime rose on two seeds and fell on one, by amounts no larger than a change of founder draws produces on its own.

- **Branch:** `roadmap/pyramid-nearest-eater`, on top of `roadmap/pyramid-reread` (step 2, the strike cost, the perf fix and the founding-hunter summary line).
- **Base:** the same branch point, built and run from this worktree. Its history CSVs are byte-identical to the shipped runs in `docs/audits/2026-09-23-pyramid-attack-cost/`, so those files are the base and are not copied here.
- **Seeds:** 42, 3, 7. One run per seed at 5000 ticks; headless runs are deterministic.
- **Constants:** every default at the branch head, unchanged from the base: `strike_cost` 1.0, `bite_reach` 1.0, `mouthless_bite_bonus` 0.3, `bite_fraction` 0.3, `kill_transfer_fraction` 0.1, `founder_diet_spread` 1.0, `population_ceiling` 6000.
- **How to repeat:** `cargo build --release`, then for each seed `./target/release/clauvolution --headless 5000 --seed S --dump-history seedS-run1.csv 2> seedS-run1.txt`. `scripts/attractor_audit_summary.py docs/audits/2026-09-23-pyramid-nearest-eater` prints the summary table. Runs were three in parallel on a shared machine; wall times are not comparable.
- **Files:** `seed<S>-run1.txt` and `.csv` are the shipped runs. `control/seed<S>-zeroed.txt` are the headless summaries of the control described below (CSVs not kept).

## The control

`NUM_INPUTS` sets how many input ids a founder's random connections are drawn from, so going from 22 to 26 inputs changes every founder's brain and, through the shared RNG, every later draw. The base and the shipped runs therefore start from different founders (91 founding hunters on seed 42 in the base, 76 here). To separate what the inputs tell a brain from the reshuffle, the control is a throwaway build of this branch with the four new inputs held at 0: same founders and same draws at tick 0 as the shipped runs, but the new inputs carry nothing. It is not committed.

## Definitions

As in `docs/audits/2026-09-23-pyramid-attack-cost/`. **Founding hunters** are generation-0 organisms labelled hunter (`diet >= 1/3`, not a photosynthesiser); their mean age at death comes from the `Founding hunters died` summary line. Every founding hunter died before tick 5000 in every run below, so the mean age at death is the mean lifetime. **Kills by diet >= 0** are all kills minus kills by `diet < 0` killers.

## Results at 5000 ticks

| seed | run | founding hunters: died, mean age, oldest | hunters alive | last tick with a hunter | kills by diet >= 0 | grazer kills of consumers | predation deaths | starvation | final plants / grazers | plants after t1000 | grazers after t1000 | at ceiling (engaged, births blocked) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 42 | base | 91/91, 270, 1027 | 0 | 1021 | 874 | 27,416 | 30,380 | 23,654 | 409 / 3160 | 48..409 | 1000..4754 | 0, 0 |
| 42 | control (inputs at 0) | 76/76, 262, 907 | 0 | 901 | 336 | 23,147 | 32,556 | 7,693 | 1196 / 4156 | 962..2747 | 1218..4857 | 2, 1,972,014 |
| 42 | **nearest eater** | 76/76, 256, 625 | 0 | 601 | 258 | 18,789 | 24,491 | 11,383 | 988 / 3543 | 320..2199 | 1164..4931 | 2, 646,441 |
| 3 | base | 94/94, 241, 514 | 0 | 4711 | 330 | 16,019 | 27,447 | 7,550 | 2415 / 2464 | 184..2891 | 649..3307 | 1, 5,450 |
| 3 | control (inputs at 0) | 98/98, 228, 628 | 0 | 691 | 364 | 12,711 | 13,503 | 26,442 | 0 / 2438 | 0..75 | 683..3526 | 0, 0 |
| 3 | **nearest eater** | 98/98, 263, 1463 | 0 | 1441 | 547 | 10,282 | 14,265 | 19,259 | 1043 / 2544 | 256..1426 | 830..4179 | 0, 0 |
| 7 | base | 84/84, 249, 1152 | 0 | 2701 | 402 | 15,925 | 23,104 | 8,180 | 1452 / 2917 | 622..2242 | 753..4346 | 1, 227,481 |
| 7 | control (inputs at 0) | 83/83, 258, 1169 | 1 | 4981 | 164 | 13,889 | 15,596 | 17,554 | 154 / 1917 | 58..285 | 609..2722 | 0, 0 |
| 7 | **nearest eater** | 83/83, 265, 1144 | 0 | 1141 | 450 | 13,750 | 18,101 | 11,982 | 1013 / 2942 | 497..1945 | 965..4497 | 1, 104,565 |

Shipped runs, full summary table:

| seed | run | plants | grazers | hunters | omnivores | plant % | species | deaths | starv | pred | old | dis | kills | grazer kills | grazes eat/attack | plant kills by consumers / energy kept | body | diet | light | ready p/e |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 3 | 1 | 1043 | 2544 | 0 | 0 | 29% | 28 | 49609 | 19259 | 14265 | 11 | 16074 | 14265 | 10282 | 146305 / 0 | 3463 / 7152.8 | 1.03 | -0.96 | 0.70 | 0% / 1% |
| 7 | 1 | 1013 | 2942 | 0 | 0 | 26% | 18 | 52504 | 11982 | 18101 | 18 | 22403 | 18101 | 13750 | 258049 / 0 | 3853 / 7451.4 | 1.14 | -0.94 | 0.73 | 0% / 2% |
| 42 | 1 | 988 | 3543 | 0 | 0 | 22% | 43 | 59747 | 11383 | 24491 | 4 | 23869 | 24491 | 18789 | 334859 / 0 | 5474 / 11799.6 | 1.32 | -0.95 | 0.70 | 0% / 1% |

Plants / grazers every 300 ticks on the shipped runs:

- **Seed 42:** 232/1241, 386/2980, 402/4144, 331/3845, 338/2461, 396/1266, 717/1755, 1330/3838, 1316/4684, 1076/4784, 1071/2846, 1182/1460, 1819/2059, 2031/3969, 1467/4531, 1085/4915.
- **Seed 3:** 153/1050, 208/2151, 287/3472, 256/3031, 275/1922, 321/892, 434/1213, 623/2719, 723/3775, 684/3266, 488/1884, 470/996, 703/1487, 1194/3271, 1373/4109, 1274/3631.
- **Seed 7:** 147/1715, 219/2852, 413/3962, 627/3667, 747/2285, 803/1099, 1163/1459, 1842/3454, 1848/4150, 1477/4291, 1272/2409, 1192/1186, 1339/1580, 1457/3412, 1239/4385, 1071/3926.

## Reading

- **Founding-hunter lifetime did not improve measurably.** Against the base, mean age at death moved 270 to 256 (seed 42), 241 to 263 (seed 3) and 249 to 265 (seed 7): longer on two seeds, shorter on one. Against the control, which has the same founders, it moved 262 to 256, 228 to 263 and 258 to 265. The reshuffle of founders alone (base against control) moves the mean by 8 to 13 ticks, the same size as the changes attributed to the inputs, so at one run per seed none of these differences is distinguishable from the draw.
- **Why a founder barely benefits (an estimate from the founder genome, not measured).** A founder has 3 to 8 random connections, each from one of 26 inputs to one of 9 outputs. The chance that a given connection runs from an eater-direction input to a movement output is (2/26) × (2/9), about 1.7%, so something under a tenth of founders have any such link and about half of those steer away from prey rather than toward it. The rest of a founder's brain ignores the inputs. The inputs can only help a hunter lineage through selection across generations, and founding hunters die at about 250 ticks, probably before most of them reproduce (not measured). Founder lifetime is a poor measure of what this input can do, and the step's criterion measures the founders.
- **No hunter level on any run.** No hunter was alive at 5000 on any shipped run. The last hunter-labelled organism was seen at tick 601, 1441 and 1141, against 1021, 4711 and 2701 in the base; the control kept one on seed 7 to the end. With no hunter level, the last-hunter tick is the fate of occasional diet mutants and is noise.
- **Kills by diet >= 0** were 258, 547 and 450 against 874, 330 and 402 in the base, and 336, 364 and 164 in the control. Higher than the control on seeds 3 and 7, lower on 42; small numbers either way, of the order of 1% to 4% of predation deaths.
- **The pyramid is healthy.** Plants and grazers persist and cycle on all three shipped runs. Plant floors after tick 1000 were 320, 256 and 497 against 48, 184 and 622 in the base; seed 42 no longer spends ticks 3000 to 3600 under 72 plants. The control shows how chaotic these worlds are: with the same founders and the inputs silent, seed 3 lost its plants (none left at 5000) and seed 7 held 58 to 285. Grazer kills of consumers fell on every seed (27,416 to 18,789; 16,019 to 10,282; 15,925 to 13,750), and so did predation deaths; starvation rose on seeds 3 and 7. The ceiling engaged twice on seed 42 (646k births blocked) and once on seed 7, as the base did on seeds 3 and 7.
- **What this leaves for steps 5 and 6.** The input costs nothing measurable in pyramid health and gives any future hunter lineage something to steer at, so it can stay if step 3's audit keeps steps 4 to 6 alive. It does not by itself keep founding hunters alive long enough to be selected. Step 5 (the size gate on consumer prey) acts on the kill rather than the chase and is independent of this change; the plan's "hunter bridge" open question (omnivores persisting long enough for carnivory to emerge from them) looks more likely to be the binding one.
