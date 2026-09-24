# Kill share by victim tissue, 2026-09-23

A measurement for the `hunter-emergence` entry in `TODO.md`, following the kill-share sweep in `docs/audits/2026-09-23-pyramid-hunter-payoff/`. That sweep found hunter lineages persisting at `--kill-transfer 1.0` on seeds 42 and 3, with plants gone on all three seeds, and the share paying for plant kills as well as consumer kills. This run splits the share by the victim's tissue and sweeps the animal share with the plant share held at 0.1, to see whether hunters persist without costing the plant level.

- **Branch:** `roadmap/pyramid-tissue-share`, on top of `roadmap/pyramid-hunter-payoff` at ef78f39.
- **Rule change:** `kill_transfer_fraction` became `kill_transfer_animal` (`--kill-transfer-animal`) and `kill_transfer_plant` (`--kill-transfer-plant`), read through `SimConfig::kill_share(victim_is_plant)`. A photosynthesiser victim uses the plant share, any other victim the animal share. `--kill-transfer K` sets both. Defaults 0.1 / 0.1.
- **New counters (summary only, not in the history CSV):** plants killed by hunters (the strategy label, `diet >= 1/3`, not a photosynthesiser) and the energy they kept, on the line `plants killed by hunters`; and a `Grazer timeline` block, every 500 ticks, with plants, grazers, hunters, mean grazer body size and armour value (labelled grazers, `diet <= -1/3`), and eat grazes (by any eater) in the preceding window, total and per grazer-second. Eat reach is `bite_reach × body size`, so the grazer size column is also the grazers' mean reach in units of `bite_reach`.
- **Constants:** every other default at the branch head: `strike_cost` 1.0, `bite_reach` 1.0, `mouthless_bite_bonus` 0.3, `bite_fraction` 0.3, `founder_diet_spread` 1.0, `population_ceiling` 6000, `animal_efficiency_multiplier` 1.0.
- **How to repeat:** `cargo build --release`, then for each seed S and animal share K `./target/release/clauvolution --headless 5000 --seed S --kill-transfer-animal K --dump-history seedS-aK.csv 2> seedS-aK.txt`. Four runs at a time on a shared machine; wall times are not comparable.
- **Files:** `sweep/seed<S>-a<K>.txt` are the nine sweep summaries; `sweep/seed<S>-default.txt` and `sweep/seed<S>-alias1.0.txt` are the default and `--kill-transfer 1.0` runs on this branch, which carry the new counters for the baseline and the old full share. CSVs are kept for the four runs with a hunter population (seed 42 at 0.3, 0.6 and 1.0, seed 7 at 1.0); the rest were read for the tables and not kept. The `Wrote N snapshots to` line in each summary names the scratch path the CSV was first written to.

## Byte-identity

With no flag, the history CSVs of seeds 42, 3 and 7 at 5000 ticks are byte-identical (`cmp`) to the step 5 runs in `docs/audits/2026-09-23-pyramid-size-gate/`, which the hunter-payoff note records as identical to the base's 0.1 runs. With `--kill-transfer 1.0` they are byte-identical to the base's 1.0 runs in `docs/audits/2026-09-23-pyramid-hunter-payoff/sweep/`, so the alias keeps older audit commands meaning what they meant. The summaries differ from the base's only in the new lines and the `Config override` line names.

## Sweep at 5000 ticks (plant share 0.1)

Definitions as in the hunter-payoff note. "Hunter kills of consumers" is the `kill` column of the `hunters` row of `Consumer-prey gates`, founders from the `founding hunt.` row, descendants the difference. "Plants killed by hunters" is the new counter, with the energy kept after digestion in brackets. "At ceiling" is 1 Hz samples with 5700 or more organisms, and births blocked at the 6000 ceiling. Grazer size and armour are the last timeline row (tick 4981). The 0.1 rows are the default runs on this branch, identical to the base's.

| animal share | seed | hunters alive at 5000 | hunters after t1000 | last tick with a hunter | hunter kills of consumers (founder / descendant) | plants killed by hunters (kept) | grazer kills of consumers | plants after t1000 | final plants / grazers | at ceiling | grazer size / armour at 5000 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0.1 | 42 | 0 | 0..0 | 601 | 50 (50 / 0) | 23 (3) | 18,789 | 320..2199 | 988 / 3543 | 46, 646,441 | 1.53 / 0.18 |
| 0.1 | 3 | 0 | 0..1 | 1441 | 367 (367 / 0) | 28 (5) | 10,282 | 256..1426 | 1043 / 2544 | 0, 0 | 1.27 / 0.14 |
| 0.1 | 7 | 0 | 0..1 | 1141 | 229 (229 / 0) | 18 (1) | 13,750 | 497..1945 | 1013 / 2942 | 18, 104,565 | 1.40 / 0.11 |
| 0.3 | 42 | 0 | 0..194 | 3601 | 14,503 (312 / 14,191) | 6,204 (604) | 12,463 | 220..2418 | 1627 / 3278 | 25, 382,587 | 1.65 / 0.36 |
| 0.3 | 3 | 0 | 0..4 | 1771 | 908 (908 / 0) | 48 (9) | 10,899 | 176..555 | 391 / 2250 | 0, 0 | 1.40 / 0.14 |
| 0.3 | 7 | 1 | 1..3 | 4981 | 2,537 (2,536 / 1) | 122 (12) | 16,544 | 226..671 | 390 / 2841 | 7, 0 | 1.30 / 0.17 |
| 0.6 | 42 | 50 | 2..387 | 4981 | 29,113 (425 / 28,688) | 15,270 (764) | 5,240 | 11..3007 | 1887 / 1946 | 0, 0 | 1.77 / 1.83 |
| 0.6 | 3 | 0 | 0..1 | 2761 | 512 (512 / 0) | 22 (4) | 15,198 | 95..917 | 778 / 2290 | 0, 0 | 1.29 / 0.14 |
| 0.6 | 7 | 0 | 0..1 | 1261 | 470 (470 / 0) | 14 (1) | 14,789 | 145..568 | 553 / 2303 | 0, 0 | 1.47 / 0.16 |
| 1.0 | 42 | 28 | 17..303 | 4981 | 34,515 (177 / 34,338) | 15,690 (1,556) | 5,370 | 37..2374 | 1902 / 2099 | 0, 0 | 1.82 / 1.80 |
| 1.0 | 3 | 0 | 0..3 | 2671 | 920 (920 / 0) | 46 (8) | 11,288 | 33..875 | 875 / 2213 | 0, 0 | 1.48 / 0.11 |
| 1.0 | 7 | 0 | 0..138 | 3361 | 4,318 (286 / 4,032) | 115 (39) | 10,410 | 2..719 | 719 / 2421 | 0, 0 | 1.51 / 0.73 |

For comparison, the old full share (`--kill-transfer 1.0`, both tissues) on this branch, identical to the base's runs:

| share | seed | hunters alive at 5000 | hunters after t1000 | hunter kills of consumers (founder / descendant) | plants killed by hunters (kept) | plants killed by all consumers (kept) | plants after t1000 | final plants / grazers | grazer size / armour at 5000 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1.0 / 1.0 | 42 | 87 | 10..303 | 34,548 (383 / 34,165) | 1,871 (2,564) | 5,193 (94,930) | 0..312 | 0 / 1055 | 1.77 / 1.91 |
| 1.0 / 1.0 | 3 | 81 | 1..132 | 14,907 (278 / 14,629) | 224 (233) | 338 (2,732) | 0..100 | 0 / 865 | 1.76 / 1.76 |
| 1.0 / 1.0 | 7 | 1 | 1..1 | 1,091 (1,091 / 0) | 2 (5) | 429 (11,201) | 0..89 | 0 / 2303 | 1.31 / 0.48 |

Plants killed by all consumers (kept) in the split runs: 13,261 (15,051), 3,079 (6,837), 2,251 (4,636) at 0.3; 19,468 (9,820), 2,035 (4,689), 1,258 (2,584) at 0.6; 21,386 (14,259), 1,115 (2,376), 779 (1,253) at 1.0, on seeds 42 / 3 / 7. At 0.1: 5,474 (11,800), 3,463 (7,153), 3,853 (7,451).

## Grazer timeline

Excerpts from the `Grazer timeline` blocks (tick: plants, grazers, hunters, grazer size, grazer armour, eat grazes per grazer-second over the preceding 500 ticks). The full blocks are in each summary.

- **Seed 42, 0.1:** grazer size rises steadily with no hunters, 1.12 at 511 to 1.53 at 4981; armour 0.07 to 0.18. Grazes per grazer-second 0.17 to 1.30.
- **Seed 42, 1.0 split:** hunters peak at 303 by tick 1021; plants fall from 317 to 73 in the same window while grazes per grazer-second are 0.11 and plant kills are 887 (against 578 at 0.1). Grazer armour jumps from 0.06 to 1.09 by 1021 and stays at 1.1 to 1.9; size reaches 1.82. Plants recover to 2,299 by 4021, with grazers at 785 to 982 over 3511 to 4021.
- **Seed 7, 1.0 split:** hunters reach 138 at 1021; plants fall from 138 to 50 by 1021 and to 3 to 26 between 1501 and 3511, with grazes per grazer-second at 0.00 to 0.04 over that stretch. Plants recover to 719 by 4981 once hunters are gone.
- **Seed 3, 1.0 split (no hunter population, at most 3 after t1000):** plants drift from 229 at 1021 to 40 at 3001, grazes per grazer-second 0.04 to 0.11, grazer size 1.27 to 1.42 against 1.18 to 1.26 at 0.1. At 0.6 (at most 1 hunter) plants stay between 100 and 280 through tick 3500 with grazer size at the 0.1 baseline's (1.15 to 1.24).

## Reading

- **Splitting the share keeps the plants.** At every animal share with plants held at 0.1, plants were alive at 5000 on every seed (390 to 1,902), against 0 on all three seeds under the old full share. The old full share handed a plant killer 1.0 × plant energy × its plant efficiency per kill, about three times a mouthed bite's 0.3; on seed 42 consumers kept 94,930 energy from 5,193 plant kills at 1.0 / 1.0, against 14,259 from 21,386 plant kills under the split. So the payoff on plant kills was a large part of the plant loss at 1.0 on seed 42. On seeds 3 and 7 the full-share runs lost their plants early and made few plant kills afterwards, so the counters there do not separate the plant payoff from the hunters' effect.
- **Hunters do not persist on most seeds under the split.** Hunters were alive at 5000 on one seed at every animal share: seed 7 at 0.3 (one founder that lived the whole run; 1 descendant kill), and seed 42 at 0.6 and 1.0 (descendant populations of 50 and 28, 28,688 and 34,338 descendant kills). Seed 3, which held 81 hunters at 1.0 / 1.0, lost its hunters by tick 2761 at 0.6 and 2671 at 1.0, and never grew a descendant population. Seed 7 at 1.0 grew one (138 at tick 1021, 4,032 descendant kills) and lost it at 3361. Hunter plant kills paid seed 3's full-share hunters only 233 energy in the whole run, so their persistence at 1.0 / 1.0 was not fed by the plant payoff directly. What was different is not isolated; one run per seed diverges from the first changed kill, and the plant payoff also went into grazers the hunters ate, which is not measured.
- **Plant floors fall with the animal share even where plants survive.** Against 0.1's floors after tick 1000 of 320 / 256 / 497, the split runs gave 220 / 176 / 226 at 0.3, 11 / 95 / 145 at 0.6 and 37 / 33 / 2 at 1.0. Only 0.3 on seeds 42 and 3 is above half the baseline. Where a hunter population forms (seed 42 at 0.6 and 1.0, seed 7 at 1.0) the floor comes during the first hunter peak, plant kills rise, and hunters themselves kill many plants (15,270 and 15,690 on seed 42, about a third of their kills, keeping 0.05 to 0.1 energy each): a hunter strikes the nearest passing target, and a plant is often nearest. Where no hunter population forms (seed 3 at 0.6 and 1.0) the floor still falls, to 95 and 33.
- **What can be said about the grazer side.** Grazer body size rises through every run, hunters or not (1.12 to 1.53 on seed 42 at 0.1), so bigger eat reach is a background drift, not a hunter effect alone. Hunters add armour, not only size: grazer armour rose tenfold wherever a hunter population formed (0.1 to 1.1 to 1.9 on seed 42, to 0.3 to 1.1 on seed 7) and stayed at baseline where none did. In the windows where plants fall fastest, total eat bites were lower than at 0.1 over the same ticks (1,300 against 7,061 on seed 7 and 2,246 against 11,919 on seed 42, ticks 511 to 1021), while plant kills were higher (255 against 171, 887 against 578), so the plants were not being grazed down harder; plant starvation is not split from other starvation in the counters. On seed 3 at 1.0 grazers were 8% to 13% larger than at 0.1 through the plant decline with no hunters present, which is consistent with a reach effect but does not show it; at 0.6 on the same seed grazer size was at baseline and the floor still fell to 95. The cause of the lower floors is not isolated by these counters.
- **Floors vary a lot between single runs.** The step 4 note recorded plant floors of 48 / 184 / 622 at the step 3 head and 320 / 256 / 497 after a change that only altered founder draws. A factor of two in a floor from one run per seed is within that spread, so the 0.3 row (220 / 176 / 226) is not distinguishable from the baseline, and the 1.0 row (37 / 33 / 2) probably is.
- **The ceiling does not engage.** Only seed 42 at 0.3 reached it (25 samples, 382,587 births blocked), against 46 samples on seed 42 at 0.1. No run with a hunter population reached it.

## Decision

The criterion for shipping a nonzero animal default was hunters alive at 5000 on at least two of three seeds and plant floors after tick 1000 no worse than about half the 0.1 baseline on every seed. No animal share meets the hunter half: 0.3, 0.6 and 1.0 each kept hunters on one seed. 0.6 and 1.0 also fail the floor half on every seed, and 0.3 fails it narrowly on seed 7 (226 against 248). The 15k-tick check was therefore not run.

Shipped: `kill_transfer_animal` 0.1 and `kill_transfer_plant` 0.1, so default behaviour is unchanged. The split stays as a knob, because it separates the two payoffs cleanly and removes the plant extinction the full share caused, and any later hunter candidate (founder claws, an omnivore bridge) can be swept against it.

## What this leaves

1. **Hunters need more than payoff on most seeds.** A higher animal share lets a lineage persist on seed 42 at 0.6 and above, not on 3 or 7. The hunter-payoff note's second item, founder claws weighted toward diet-positive founders, was reserved for exactly this case ("hunters that persist on one seed and not the others"). It reads diet in a founder rule and raises bystander kills by clawed grazers too; it should be swept with the animal share at 0.6 or 1.0 and the plant share at 0.1.
2. **Hunter plant kills are bycatch.** Hunters killed 15k plants on seed 42 for 0.05 to 0.1 energy each and struck them because they were nearest. Whether that costs the plant floor more than hunting thins grazers is not measured; a counter of plant deaths by cause (grazing starvation against kills) would separate them.
3. **The plant floor under a hunter population is the other constraint.** Where hunters did persist, the floor fell during the first hunter peak, with low grazing. The grazer-side candidate (eat reach not scaling with body size) is not supported by the bite counts in those windows, so it is not the first thing to try.
