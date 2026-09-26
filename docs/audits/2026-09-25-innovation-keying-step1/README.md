# Innovation keying, step 1: what keying would do to species distances, 2026-09-25

Step 1 of `plans/2026-09-24-innovation-keying.md` ("Instruments and the re-key function, with no behaviour change"). It measures, offline and on the same saves, what keying innovation numbers by structure would do to the compatibility distances that decide species, under both structural normalisations, and answers the plan's first open question: does keying separate lineages at all? The answer decides whether steps 2 and 3 run.

- **Branch:** `roadmap/innovation-keying-instruments`. The rules are `main` at 2be99b0; the branch adds the re-key function, the per-pass species counts and the offline report, and the same-seed check in the pull request shows the history CSVs are unchanged.
- **Seeds:** 42, 3 and 7, the plan's seeds. One run per seed; headless runs are deterministic.
- **Saves:** for each seed, `--headless 5000 --save-as` (the population at tick 5010) and `--headless 149 --save-as` (every organism alive just before the first classification pass at tick 151: the founders and their first children, all unclassified). Seed 42 also has a 15000-tick save for the table size. The saves were written by a binary built at `main`; they are not committed (about 20 MB each).
- **Offline report:** `cargo run --release -p clauvolution_sim --example species_keying_report -- <5000 save> <149 save>`. Its output per seed is `report-s<S>.txt` here (paths shortened). It re-keys every genome in save order with `clauvolution_genome::rekey_population` and computes each distance under four settings: legacy identity (the numbers as saved) or keyed identity, each with the live normalisation (excess and disjoint over the larger gene count, "larger") or the mean gene count ("mean", under which two genomes sharing no gene score exactly 1.0 from the structural terms).
- **Live counts:** `seed<S>-run1.txt` are the headless summaries of the branch build at 5000 ticks with the new "Species classification passes" block at the end, and `seed<S>-run1.csv` the history with the five new trailing columns (`species_pass_tick` to `species_pass_isolated`). These are the live rule (legacy identity, larger normalisation) as `species_classification_system` evaluates it.

## Definitions

- **Near another species:** within the join threshold of some other species' representative. The plan's "93 to 100%" number.
- **Drifting:** at or past the stay threshold (1.3 times the join threshold) from the organism's own species' representative. **Isolated:** drifting and at or past the join threshold from every other representative, which is what founding a species needs. The plan's "0 / 58 / 1" number.
- **Offline species and representatives:** species ids are the ones saved, which the legacy distance formed; each species' representative is its first member in save order, as the live pass takes the first member in query order. So the keyed columns ask how the keyed distance would see the species the legacy distance made, not what a keyed run would make. Step 2 would answer the second question.
- **Founding pass:** the offline report runs `choose_species`' rule for unclassified organisms (join the nearest representative within the threshold, else found a species) over the pre-founding save in save order. At legacy/larger and join 1.0 it gives 50, 74 and 57 species; the live pass at tick 151 formed 49 on seed 42 and 75 on seed 3 (the first history rows), so the offline pass tracks the live one to within a species or two.

## Keys and the re-key recipe

| seed | organisms at 5010 | connection genes | distinct legacy innovation numbers | distinct connection keys | split keys | hidden neurons (keyed / repeat splits / unplaced) |
| --- | --- | --- | --- | --- | --- | --- |
| 42 | 4549 | 34298 | 989 | 339 | 35 | 2416 (2400 / 16 / 0) |
| 3 | 3579 | 29739 | 757 | 366 | 51 | 3879 (3863 / 16 / 0) |
| 7 | 3760 | 36117 | 948 | 364 | 46 | 1974 (1963 / 11 / 0) |

The recipe placed every hidden neuron on every seed: no neuron lacked its split genes and no connection named a missing neuron. The only fresh ids are repeat splits, a connection split, re-enabled and split again in one genome (11 to 16 per seed). A repeat split's fresh id is per genome, so descendants of one ancestor stop matching on it and its two genes; at under 1% of hidden neurons this does not move the instruments, and the fix for a load migration is in the `rekey_genome` doc comment. Living genomes carry about a third as many distinct keys as legacy numbers, because the same `(from, to)` pair recurs across lineages.

Table size at 15000 ticks, seed 42 (`report-s42-15000.txt`): 6000 organisms in 82 species carry 69606 connection genes, 1249 distinct legacy numbers, 524 distinct connection keys and 76 split keys (8122 hidden neurons: 8089 keyed, 33 repeat splits, none unplaced). Keys carried by living genomes grew from 339 to 524 between tick 5010 and tick 15010 (legacy numbers grew from 989 to 1249 over the same stretch). A few hundred entries is no memory concern. These count keys the living population carries; keys issued over a whole run can only be counted by a keyed run, which is step 2. The 15000-tick instruments tell the same story as 5000: near another species 99.0% legacy against 99.5% keyed (larger), isolated drifters 0 on both, 48.5% of cross-species pairs sharing a gene under legacy numbers.

## The instruments at join 1.0, stay 1.3, tick 5010

Near another species (% of organisms) and drifting / isolated (organisms), under each identity and normalisation:

| seed | species | near other: legacy/larger | keyed/larger | legacy/mean | keyed/mean | drifting / isolated: legacy/larger | keyed/larger | legacy/mean | keyed/mean |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 42 | 40 | 99.0 | 99.5 | 98.6 | 99.2 | 57 / 2 | 56 / 2 | 71 / 4 | 70 / 4 |
| 3 | 23 | 99.6 | 99.6 | 66.4 | 67.0 | 21 / 1 | 18 / 1 | 27 / 14 | 23 / 5 |
| 7 | 37 | 100.0 | 100.0 | 96.9 | 97.2 | 30 / 0 | 84 / 0 | 46 / 1 | 99 / 48 |

Median distance to the nearest other species' representative, legacy/larger against keyed/larger: 0.624 / 0.616 (seed 42), 0.760 / 0.755 (seed 3), 0.684 / 0.679 (seed 7). The reports have the same table at join thresholds 0.8 to 1.8.

The live counts over every pass after founding (branch build, legacy/larger) agree with the offline snapshot: near another species on 99.8% (seed 42), 99.3% (seed 3) and 98.7% (seed 7) of organisms per pass on average (lowest single pass 99.1%, 97.4%, 92.2%), and of the organism-passes past the stay threshold (4272, 2531 and 2592 summed over 32 passes), 22, 21 and 40 were isolated. The founding pass at tick 151 formed 49, 75 and 55 species. Seed 42 now forms species after founding, unlike the 2026-09-23 measurement in DECISIONS, because the rules have changed since; the rate is still low, one isolated organism per 200 drifting ones.

## Gene sharing and the weight term

| seed | organism-to-other-species-rep pairs sharing a gene at 5010, legacy / keyed | weight term when shared, legacy / keyed | founder pairs (tick 1) sharing a gene, legacy / keyed | weight term when shared, keyed |
| --- | --- | --- | --- | --- |
| 42 | 67.4% / 67.6% | 0.884 / 0.879 | 0.0% / 12.3% | 0.666 |
| 3 | 49.6% / 51.5% | 0.710 / 0.716 | 0.1% / 12.1% | 0.660 |
| 7 | 53.8% / 60.0% | 0.831 / 0.805 | 0.0% / 11.7% | 0.666 |

The founder columns come from a `--headless 1` save (419, 432 and 413 founders; `report-founders-tick1-s<S>.txt`), where nobody is related yet; the pre-founding saves mix in the founders' first children, who share genes with their parents by descent (5 to 16% of pairs under legacy numbers).

## The founding pass

Pair distance over the pre-founding population (705, 1589 and 1515 organisms), median (p10 to p90), and the species the founding pass would form at join 1.0 and 1.1:

| seed | legacy/larger | keyed/larger | legacy/mean | keyed/mean |
| --- | --- | --- | --- | --- |
| 42 | 1.006 (0.842 to 1.138); 50 at 1.0, 13 at 1.1 | 1.018 (0.845 to 1.235); 41, 5 | 1.135 (1.070 to 1.212); 353, 19 | 1.146 (1.071 to 1.300); 201, 24 |
| 3 | 0.899 (0.567 to 1.104); 74, 9 | 0.905 (0.566 to 1.179); 63, 11 | 1.089 (0.582 to 1.201); 337, 19 | 1.093 (0.581 to 1.248); 222, 22 |
| 7 | 0.938 (0.768 to 1.092); 57, 5 | 0.956 (0.764 to 1.178); 52, 9 | 1.090 (1.015 to 1.186); 328, 21 | 1.096 (0.905 to 1.249); 202, 22 |

## Reading

**Keying barely moves the distances that decide species.** Under the live normalisation it changes the share of organisms near another species by at most half a point (99.0 to 99.5, 99.6 to 99.6, 100.0 to 100.0) and the isolated drifters not at all (2, 1, 0 on both identities). Under the mean normalisation it changes the share by at most 0.6 points (98.6 to 99.2, 66.4 to 67.0, 96.9 to 97.2).

The reason is that the premise does not hold at 5000 ticks. The TODO entry and the "Species classification" DECISIONS entry read the high near-other share as the ceiling for unrelated genomes, which score at most 1.0 apart. But half to two-thirds of organism-to-other-species pairs already share genes under the legacy numbers (49.6 to 67.4%), because most species alive at 5000 descend from a few founding lineages, and the nearest other species' representative sits at a median 0.62 to 0.76, well under the 1.0 ceiling. Keying only adds shared genes to pairs that had none, which raises the share by 0.2 to 6.2 points. What keeps drifting organisms near another species is that the species are close relatives, not that unrelated brains are capped.

**Where keying does act, the weight term cancels it.** Among true founders, keying makes about 12% of pairs share a gene, and when they do, the weight term adds 0.66 on average (two independent draws from `NEW_WEIGHT_RANGE`, halved, as the plan warned). The structural term falls (median 0.875 to 0.833 on seed 42) and the weight term more than makes it up: median pair distance over the pre-founding population rises slightly under keying (1.006 to 1.018, 0.899 to 0.905, 0.938 to 0.956) and fewer pairs fall under 1.0 (47.2 to 43.6% on seed 42). The founding pass still forms somewhat fewer species at 1.0 under keying (50 to 41, 74 to 63, 57 to 52), since the greedy pass needs only one representative within reach; either way keying neither shatters the founders nor separates them. Under the mean normalisation keying pulls about 40% of the founding pass's species back together at 1.0 (353 to 201, 337 to 222, 328 to 202) through the same shared genes, but that is still several times the live 49 to 75.

**The mean normalisation does not give step 3 a usable window with or without keying.** Founder pair distances under it sit in a narrow band (keyed p10 to p90: 1.07 to 1.30 on seed 42), so the founding pass falls from about 200 species at join 1.0 to about 22 at 1.1. At 1.1, where the founding count is reasonable, the population at 5000 is back where the live rule has it: near another species 99.9%, 85.9% and 99.7%, and isolated drifters 0, 0 and 6 (keyed/mean), against 2, 1 and 0 for legacy/larger at 1.0. The only cell with many more isolated drifters is seed 7 keyed/mean at 1.0 (48), which is also a cell that shatters the founding pass into 202 species. Seed 3's lower near-other share under the mean normalisation (66 to 67% at 1.0) comes from the normalisation, not from keying, and is gone by 1.1 (85.9 to 90.7%).

## Verdict

**Keying does not separate lineages; the plan stops here.**

The numbers behind it, per seed 42 / 3 / 7 at join 1.0: organisms near another species 99.0 / 99.6 / 100.0% under legacy numbers against 99.5 / 99.6 / 100.0% keyed (live normalisation), and isolated drifters 2 / 1 / 0 against 2 / 1 / 0. Cross-species pairs already share genes by descent in 49.6 to 67.4% of cases, and keying adds 0.2 to 6.2 points. The one setting that raises isolation (keyed with the mean normalisation, 48 isolated on seed 7) shatters the founding pass into 201 / 222 / 202 species, and at the first threshold that does not (1.1: 24 / 22 / 22 founding species) isolation is 0 / 0 / 6. Steps 2 and 3 should not run on this evidence. The re-key function and its identical-outputs test stay as built, since they cost nothing at runtime and are the load migration if keying is ever revisited; the per-pass counts stay on the Graphs tab and in the headless summary as the instrument for whatever replaces this plan.

What the measurements point at instead is outside this plan's scope and is recorded in the `species-distance-unrelated-ceiling` TODO entry: at 5000 ticks, species are close relatives whose representatives sit 0.6 to 0.8 apart, so a drifting organism nearly always fits a sibling species. That is a question about the representative and the join rule (or the body term's share), not about innovation identity.
