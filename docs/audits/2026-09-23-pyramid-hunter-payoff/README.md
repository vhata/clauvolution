# Hunter payoff, 2026-09-23

A measurement for the `hunter-emergence` entry in `TODO.md`, read against step 5 of `plans/2026-09-21-pyramid-top.md` (`docs/audits/2026-09-23-pyramid-size-gate/`). An older sweep of the kill share (0.1, 0.5, 1.0) found no hunters, but it ran before grazing moved to `eat` (step 2), before the strike cost, and before the nearest-eater inputs (step 4). This run repeats the kill-share sweep on the current rules and also measures how many founding hunters have claws. No simulation code or default was changed.

- **Branch:** `roadmap/pyramid-hunter-payoff`, on top of `roadmap/pyramid-size-gate` (step 5) at ef475fc. Documentation only.
- **Sweep:** `kill_transfer_fraction` (`--kill-transfer`) at 0.1 (the default), 0.3, 0.6 and 1.0, on seeds 42, 3 and 7 at 5000 ticks. One run per point; headless runs are deterministic.
- **Constants:** every other default at the branch head: `strike_cost` 1.0, `bite_reach` 1.0, `mouthless_bite_bonus` 0.3, `bite_fraction` 0.3, `founder_diet_spread` 1.0, `population_ceiling` 6000, `animal_efficiency_multiplier` 1.0.
- **How to repeat:** `cargo build --release`, then for each seed S and share K `./target/release/clauvolution --headless 5000 --seed S --kill-transfer K --dump-history seedS-runkK.csv 2> seedS-runkK.txt`. Runs were three in parallel on a shared machine; wall times are not comparable.
- **Files:** `sweep/seed<S>-runk<K>.txt` are the headless summaries of all twelve runs. The CSVs of the three 1.0 runs, where hunter lineages appear, are kept beside them; the other nine were read for the table and not kept. The `Wrote N snapshots to` line in each summary names the scratch path the CSV was first written to. The 0.1 history CSVs are byte-identical to the step 5 runs in `docs/audits/2026-09-23-pyramid-size-gate/`, so those runs are the baseline and their CSVs are not duplicated here.

## Definitions

As in the step 5 note, plus:

- **Hunters:** the strategy label, a non-photosynthesiser with `diet >= 1/3`, founders or descendants.
- **Last tick with a hunter:** the last 1 Hz sample with any hunter alive. 4981 is the last sample of a 5000-tick run.
- **Hunter kills of consumers:** the `kill` column of the `hunters` row of `Consumer-prey gates`. Founder kills are the `founding hunt.` row; descendant kills are the difference, kills by hunters of generation 1 and above. Kills of plants by hunters are not separated in the counters.
- **+ predation energy:** the ledger's cumulative predation flow, energy killers kept after digestion, of consumer and plant victims alike.
- **At ceiling:** 1 Hz samples with 5700 or more organisms, and births blocked at the 6000 ceiling.

## Sweep at 5000 ticks

| kill share | seed | founding hunters: died, mean age, oldest | founders that fired / killed | hunters alive at 5000 | hunters after t1000 (min..max) | last tick with a hunter | hunter kills of consumers (founder / descendant) | grazer kills of consumers | + predation energy | final plants / grazers | plants after t1000 | grazers after t1000 | at ceiling (samples, births blocked) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0.1 | 42 | 76/76, 256, 625 | 25 / 10 | 0 | 0..0 | 601 | 50 (50 / 0) | 18,789 | 12,632 | 988 / 3543 | 320..2199 | 1164..4931 | 46, 646,441 |
| 0.1 | 3 | 98/98, 263, 1463 | 38 / 11 | 0 | 0..1 | 1441 | 367 (367 / 0) | 10,282 | 8,720 | 1043 / 2544 | 256..1426 | 830..4179 | 0, 0 |
| 0.1 | 7 | 83/83, 265, 1144 | 29 / 4 | 0 | 0..1 | 1141 | 229 (229 / 0) | 13,750 | 8,965 | 1013 / 2942 | 497..1945 | 965..4497 | 18, 104,565 |
| 0.3 | 42 | 76/76, 273, 1112 | 25 / 10 | 0 | 0..142 | 3361 | 7,700 (351 / 7,349) | 15,331 | 135,353 | 1832 / 3879 | 741..3046 | 904..4141 | 32, 1,071,390 |
| 0.3 | 3 | 98/98, 283, 1534 | 37 / 10 | 0 | 0..2 | 1531 | 934 (926 / 8) | 10,519 | 25,364 | 1295 / 2222 | 212..1488 | 768..3724 | 0, 0 |
| 0.3 | 7 | 83/83, 309, 3625 | 30 / 4 | 0 | 0..2 | 3601 | 779 (778 / 1) | 21,494 | 13,772 | 0 / 1787 | 0..89 | 668..3469 | 0, 0 |
| 0.6 | 42 | 76/76, 300, 1871 | 25 / 9 | 5 | 5..445 | 4981 | 27,280 (281 / 26,999) | 4,164 | 531,586 | 1376 / 2562 | 83..1894 | 772..3536 | 0, 0 |
| 0.6 | 3 | 98/98, 276, 1346 | 37 / 9 | 0 | 0..3 | 4561 | 590 (562 / 28) | 22,218 | 83,887 | 882 / 2001 | 182..1655 | 678..3698 | 0, 0 |
| 0.6 | 7 | 83/83, 300, 2776 | 30 / 4 | 0 | 0..30 | 2761 | 2,063 (757 / 1,306) | 15,036 | 111,555 | 1 / 2065 | 1..213 | 610..3053 | 0, 0 |
| 1.0 | 42 | 76/76, 278, 1205 | 25 / 10 | 87 | 10..303 | 4981 | 34,548 (383 / 34,165) | 7,864 | 1,178,531 | 0 / 1055 | 0..312 | 572..2660 | 0, 0 |
| 1.0 | 3 | 98/98, 245, 882 | 37 / 9 | 81 | 1..132 | 4981 | 14,907 (278 / 14,629) | 3,264 | 805,936 | 0 / 865 | 0..100 | 346..1846 | 0, 0 |
| 1.0 | 7 | 82/83, 240, 624 | 29 / 4 | 1 | 1..1 | 4981 | 1,091 (1,091 / 0) | 26,442 | 69,922 | 0 / 2303 | 0..89 | 735..3447 | 0, 0 |

Final trait averages over all organisms at 1.0 against 0.1: body size 1.79 / 1.77 / 1.31 against 1.32 / 1.03 / 1.14 on seeds 42 / 3 / 7; mean armour on the seed 42 and 3 runs rose from about 0.1 to 1 to 2 by tick 1200 and stayed there, against 0.1 to 0.2 at 0.1. Max generation at 1.0 was 88, 77 and 63 against 47, 47 and 40.

## Founding-hunter claws

The founder genome (`Genome::new_minimal_with_diet`) gives each founder a torso plus one to three random segments, each a claw with probability 1/8, of size 0.3 to 1.5; `attack_power` is drawn from 0 to 0.1 and `armor` from 0 to 0.1. Claw power is the sum of claw sizes plus `attack_power`, and the strike force the damage gate reads is claw power × body size (body size 0.5 to 1.5). The gate passes when strike force − 0.5 × prey armour value × prey size is above 0.1.

- **From the code (estimated):** the chance a founder has at least one claw is the mean over one, two and three extra segments of 1 − (7/8)^n, about 23%. A clawless founder's strike force is at most 0.1 × 1.5 = 0.15 and passes the gate only against almost unarmoured prey; a Monte Carlo of the draw puts 10% of clawless hunters above 0.1 against zero armour, and none against a consumer of armour value 0.19 and size 1.3 (the step 5 run's final averages). A clawed founder's claw power has a median of about 1.0, and it passes against almost anything a founder meets.
- **From the founders (measured):** `--headless 1 --seed S --save-as` writes every founder's genome at tick 10, before any has changed. Counting the generation-0 hunters in those saves (a few have already died by then, so the hunter counts on seeds 3 and 7 are 95 and 82 against 98 and 83 in the summaries):

| seed | hunters | with a claw | claw power of clawed, median (range) | strike force, median of all hunters | clawless, largest strike force | damage gate passes, clawed vs clawless vs all, against every founder consumer | both gates, all hunters |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 42 | 76 | 25 (33%) | 1.00 (0.36..2.49) | 0.072 | 0.124 | 95% / 2% / 33% | 29% |
| 3 | 95 | 22 (23%) | 0.88 (0.46..1.50) | 0.062 | 0.139 | 96% / 3% / 25% | 21% |
| 7 | 82 | 17 (21%) | 0.96 (0.47..1.55) | 0.052 | 0.141 | 95% / 2% / 21% | 19% |

The pairwise pass rate against founder consumers (19% to 29% through both gates) matches the step 5 measurement of hunter attacks with a consumer in reach ending in a kill, 18% to 26%. The gate is binary on claws: a clawed founder almost always passes, a clawless one almost never. Only a fifth to a third of founding hunters can kill anything, and the founders that did kill (10, 11 and 4) are fewer than the clawed ones (25, 22 and 17); how many of the killers were clawed is not linked in the counters.

## Reading

- **On the current rules, the payoff does bind, and a higher share lets hunter lineages persist.** At 1.0, hunters were alive at 5000 on all three seeds (87, 81 and 1). On seeds 42 and 3 the hunters are a descendant population: they never fell below 10 and 1 after tick 1000, peaked at 303 and 132, and made 34,165 and 14,629 kills of consumers against 383 and 278 by founders. At 0.6, seed 42 held 5 to 445 hunters through the whole run with 27k descendant kills, seed 7 kept a descendant population until tick 2761, and seed 3 had a lone survivor to 4561. At 0.3 seed 42 had a descendant hunter population until 3361. At 0.1 no descendant hunter ever killed a consumer. This is the reverse of the old sweep's result, so what changed in between (grazing through `eat`, the strike cost, the nearest-eater inputs) is what made payoff matter.
- **Seed 7 at 1.0 is a single founder.** One founding hunter was alive from tick 0 to 5000, killing 1,091 consumers; no descendant hunter killed anything and the hunter count was 1 at every sample after tick 1000. A kill share of 1.0 is enough for a lucky founder to live off kills indefinitely, but its offspring did not inherit the trick, or did not keep a hunter diet.
- **Founders die as before.** Mean founding-hunter age at death is 240 to 309 ticks at every share, against 256 to 265 at 0.1. The lineages that persist come from the few founders that live long enough to reproduce, not from founders in general living longer. A founder energy bridge would target a number the sweep says does not need moving.
- **The cost is the plant level.** At 1.0 plants were gone by tick 5000 on all three seeds (floors after tick 1000 of 0, 0 and 0, against 320, 256 and 497 at 0.1). At 0.6 plants persisted on seeds 42 and 3 (floors 83 and 182) and not on 7; at 0.3 they persisted on 42 and 3 and not on 7. Seed 7 lost its plants at every share above 0.1, including 0.3, where no hunter population formed, so on seed 7 at least part of the loss does not come through hunters. The plant loss at 1.0 on seeds 42 and 3 started early: plants fell from 269 at tick 631 to 26 at tick 991 on seed 42, and from 461 at tick 751 to 15 at tick 1231 on seed 3, in the same window as the first hunter peak (672 hunters on seed 42 at tick 751). The mechanism is not isolated. What is measured: the kill share also pays for plant kills, and the energy consumers kept per plant kill rose from about 2 at 0.1 to 8 to 26 at 1.0; but plant kills by consumers in the first 1500 ticks did not rise with the share (1,174 against 1,035 on seed 42, 323 against 436 on seed 3), and the plants-killed totals fell once plants were scarce. What is estimated: a hunter population selects its prey for size and armour (both gates read them; consumer armour rose tenfold and body size by 35% to 70% on seeds 42 and 3), and eat reach is `bite_reach × body size`, so bigger grazers bite from further away, the regime in which the step 2 bite-reach sweep lost its plants. Neither is shown to be the cause.
- **Grazer kills of consumers fell where hunters persisted** (18,789 to 7,864 on seed 42 and 10,282 to 3,264 on seed 3 at 1.0), and no run above 0.1 except seed 42 at 0.3 reached the population ceiling. Hunters take over the population regulation that the strike cost audit found bystander kills were doing.
- **Omnivores do not bridge.** Omnivores (`-1/3 < diet < 1/3`) peaked after tick 1000 at 2 to 176 across all twelve runs, mostly under 20, and were at 0 to 4 at 5000. The descendant hunters at 0.6 and 1.0 arose while omnivores were rare. The counters do not trace ancestry, so whether those lineages descend from founding hunters or from omnivores that drifted up the diet axis is not measured.

## What this leaves

A higher kill share lets a hunter lineage persist on two of three seeds at 1.0 and one of three at 0.6, but at 1.0 it costs the plant level on every seed. It does not "clearly work", so no setting is recommended to ship. In order:

1. **Separate the hunter's payoff from the plant kill's.** The kill share pays for plant kills and consumer kills alike, so the sweep cannot tell whether the plant loss comes from the payoff on plant kills or from the hunters themselves. The next experiment is a kill share per victim tissue, for example `kill_transfer_animal` swept at 0.6 and 1.0 while plant victims stay at 0.1, with a counter for plant kills by hunters. It is a small rule change in `predation_system` (`kill_digestion_efficiency` already branches on tissue) and needs its own PR. Done looks like hunters alive at 5000 on at least two seeds with plant floors after tick 1000 near the 0.1 baseline. If plants survive, the split share is a candidate default. If they still collapse, the loss is a cascade through the grazers (size and armour against hunters, then reach against plants), and the lever is on that side, for example eat reach not scaling with body size.
2. **Claws for founders second.** Only 21% to 33% of founding hunters can pass the damage gate, and that is a draw fixed at birth, like firing. Weighting the founder draw toward claws for diet-positive founders would widen the base hunter lineages grow from, but the sweep shows lineages already grow from the clawed minority once a kill pays. It is also a founder rule that reads diet, and claws raise every clawed grazer's bystander kills too. Worth a run only if the tissue split gives hunters that persist on one seed and not the others.
3. **Not the founder energy bridge or the omnivore bridge, on this evidence.** Founder lifetimes did not move with the payoff, and the persisting lineages formed without an omnivore population. Either question reopens if the tissue split fails.
