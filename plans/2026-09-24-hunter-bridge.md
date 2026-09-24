# Plan: the hunter bridge, so that an intermediate diet can live long enough for carnivory to grow from it

**Status:** planned 2026-09-24. Not started.
**Scope:** `hunter-emergence` in `TODO.md`, through the one candidate `plans/2026-09-21-pyramid-top.md` left open: the omnivore bridge. Not phase 2 of `docs/design/simulation-rules.md`, and not `consumer-ceiling-regulation` or `species-count-above-tuned-band`, which are re-measured by this plan's audit but not worked by it.

## Why

The pyramid-top plan ran all six steps and ended without a hunter level ("Pyramid-top outcome" in `docs/design/simulation-rules.md`; `docs/audits/2026-09-24-pyramid-step6/`). On eight seeds at 15000 ticks, no seed had a hunter (a non-photosynthesiser with `diet >= 1/3`) alive at 5000 or 15000, and none held more than two at once after tick 1500. What is measured about why:

- **Founding hunters die at about 250 ticks whatever the setting.** Mean age at death was 223 to 279 on the eight step 6 seeds, 240 to 309 across every kill share in `docs/audits/2026-09-23-pyramid-hunter-payoff/`, and moved by no more than a change of founder draws with the nearest-eater inputs ("Nearest-eater inputs" in `docs/DECISIONS.md`).
- **Two thirds are wired never to fire `attack`.** 25 to 38 of 76 to 98 founding hunters per seed ever fired, and those fire from a mean age of 7 to 9 ticks, so firing is set at birth ("Size gate on consumer prey: measured, left alone"). 83% to 93% of hunter intents have nobody in range.
- **Most founders have no claw, so the damage gate stops most strikes.** 21% to 33% of founding hunters have a claw; a clawed founder passes the damage gate against 95% to 96% of founder consumers and a clawless one against 2% to 3% (`docs/audits/2026-09-23-pyramid-hunter-payoff/`, "Founding-hunter claws"). In step 6 the damage gate alone stopped 49% to 87% of founding-hunter attacks with a consumer in reach; the size gate alone stopped 0 to 22 attacks per seed.
- **Payoff binds, but raising it costs the plants.** At `--kill-transfer 1.0` hunters were alive at 5000 on seeds 42, 3 and 7 (87, 81 and 1), descendant lineages from the clawed minority on 42 and 3, and plants were gone on all three. Splitting the share by victim tissue ("Kill share by victim tissue") kept plants alive on every run, but left hunters at 5000 on one seed of three at every animal share (0.3, 0.6, 1.0), with plant floors after tick 1000 falling to 37 / 33 / 2 at 1.0 against 320 / 256 / 497 at 0.1.
- **Omnivores do not persist.** Consumer diet ended at -0.96 to -0.97 on all eight step 6 seeds, with at most three omnivores alive after tick 3000. The step 6 history CSVs (the `omnivores` column) show the shape: every seed founds 80 to 107 omnivores, several grow at first (seed 3 to 245 by tick 121), and every seed has none at tick 3001. Seed 99 is the exception that says most: 723 omnivores at tick 991 against 382 grazers, a peak of 752 at 1111, then 19 at 2191 and 0 at 2671 while grazers rose from 459 to 3427.

The pyramid-top plan named the remaining candidate: the diet axis is continuous, so intermediate diets exist, and a hunter lineage could grow from omnivores whose diet drifts up while they are still fed by plants. The founding hunter has no such bridge because it digests `((1 - 1/3) / 2)^2`, 11%, or less of plant tissue, food items included (`Genome::plant_efficiency`, read by `action_system` for food items and `grazing_system` for bites). The question this plan answers is why intermediate diets vanish within a few thousand ticks, and whether the answer is a rule or an outcome.

## Decisions to take

Each is taken in the step named, with the numbers, and becomes a `docs/DECISIONS.md` entry in the pull request that takes it.

- **Is omnivory's disadvantage structural or ecological? (step 1)** Structural means the digestion curve alone rules it out. With `plant_efficiency = ((1 - diet) / 2)^2` and `animal_efficiency = ((1 + diet) / 2)^2` (`crates/clauvolution_genome/src/lib.rs`, `Genome::plant_efficiency`, `Genome::animal_efficiency`), the two sum to `(1 + diet^2) / 2`: a specialist digests one tissue fully, a generalist at 0 digests a quarter of each. While almost all consumer income is plant tissue (1.2M to 2.1M eat bites a run in step 6, against 0 to 855 kills by `diet >= 0` killers after tick 1000), plant efficiency is monotone in diet and every step toward carnivory is a pure loss. Ecological means the curve permits an omnivore but the world removes it: competition with grazers for the same plants, predation, or disease. Seed 99 is evidence for the second reading (omnivores out-numbered grazers until grazers boomed) and the phase 1 tuning pass stated the first ("the squared curve pushes omnivores toward whichever side feeds them, which is the plant side", "Diet axis tuning pass"). Neither is measured. **Recommendation:** decide from step 1's per-band energy budget, not from argument. If omnivores' income is almost all plant tissue and they fall as grazers rise, the curve is binding and step 2 is the lever. If their income matches grazers' per capita and they die of something else, the curve is not the lever and step 2 is skipped.

- **Reshape the efficiency curve? (step 2)** Options: (a) keep the exponent at 2; (b) make the exponent a `SimConfig` field, default 2, and sweep it between 1 and 2, so a generalist digests more than a quarter of each tissue (at exponent 1 a generalist digests half of each and the sum is 1 everywhere); (c) a curve with a flat top, so that a band around each end digests fully and only the far side pays; (d) an exponent below 1, which makes a generalist better than a specialist on total digestion. "Why the squared curve" in `docs/DECISIONS.md` records that a linear split gives no reason to specialise and says the exponent is a tuning knob, but no sweep of it is recorded in the diet tuning pass. **Recommendation:** (b), swept at 1.5 and 1.0. It is one number, it reads only the organism's own diet to set its own digestion (which the rule already does), and it defaults to today's behaviour. Reject (d): it removes the reason for the axis to have two ends. Keep (c) as the fallback if (b) at 1.0 still leaves no intermediates, since it is a new curve shape rather than a knob on the existing one. The design table's warning applies to any flatter curve: "no tuning gives a trophic pyramid when the apex can also graze". At exponent 1 a hunter at `diet = 1/3` digests a third of plant tissue, so step 2 must measure how much of labelled hunters' income is plant tissue, and a setting that produces hunters living mostly on plants is not a hunter level.

- **Reconsider the kill share by tissue? (step 3)** Options: (a) leave both shares at 0.1; (b) sweep the animal share again on top of a step 2 setting; (c) scale the share by the killer's diet. The tissue split already exists as a knob (`SimConfig::kill_share` in `crates/clauvolution_core/src/lib.rs`, `--kill-transfer-animal`), so (b) needs no code. **Recommendation:** (b), and only if step 2 leaves intermediate diets alive that kill but do not climb the axis. The tissue-share note found that payoff lets a lineage persist on the seed where a clawed lineage already exists and nowhere else; on a population that has intermediates to start from, the same sweep reads differently. Reject (c): a reward that reads the killer's diet is a rule that reads diet, already rejected under "Strike cost" and "Kill share by victim tissue".

- **Is weighting founder claws toward diet-positive founders founder staging?** Yes. It reads the founder's diet to decide its weapon, which is staging by trophic level, and the design doc rejects staging because it "programmes the pyramid into the opening" (decisions table, "Initial conditions"). The hunter-payoff and tissue-share notes proposed it; this plan does not run it. A diet-blind change to the founder draw (a higher claw probability in `BodySegmentGene::random` for every founder, or a different `FOUNDER_OUTPUT_BIAS_RANGE` in `Genome::new_minimal_with_diet`) is not staging, but it raises every clawed grazer's bystander kills too ("Strike cost"), and it acts on founders, who die at about 250 ticks, where the bridge question is about lineages. **Recommendation:** out of this plan; it is an open question below if the bridge fails.

- **Time, or a larger world?** Time is measured: omnivores are gone by tick 3001 on all eight seeds, and 15000-tick runs (the step 3 re-read and step 6) brought none back, so a longer run has nothing to grow from. A larger world could give omnivores refuges where grazers are sparse, which is what seed 99's early run looks like, but `SimConfig::world_width` and `world_height` are fixed at 512 with no command-line override, and the design doc holds world size at 512 until phase 2's barriers are measured (decisions table, "Barriers"). **Recommendation:** neither in this plan. Phase 2's biomes are the principled source of refuges, and a bridge that works only in a bigger world is a phase 2 finding.

## The plan

Each step is a branch and a pull request. Step 1 is counting only; steps 2 and 3 are one lever each and each is conditional on the step before it; step 4 is the audit. Every step that changes a rule records the before and after numbers in `docs/DECISIONS.md` and shows its effect in the Graphs tab or the headless summary before any tuning.

### 1. Instruments: where intermediate diets get their energy and how they die

Add counters, with no behaviour change, that answer the structural-or-ecological question:

- **Energy income by diet band and tissue.** For consumers in diet bins (at least the three strategy labels; finer bins such as fifths of the axis if the cost is small), the energy kept from food items, from eat bites, from consumer kills and from plant kills, and the metabolism, movement and strike cost paid. The income points are `action_system` (food items), `grazing_system` (bites) and `predation_system` (kills); the costs are in `metabolism_system` and `action_system`. `EnergyFlows` is already a per-organism scratch record for the parallel systems, so a per-band accumulator can follow the same pattern.
- **Deaths by band and cause,** with mean age at death, from `death_system`'s existing `Killed(cause)` reading.
- **Births by band, and the parent-to-child band crossing count** in `reproduction_system`: how many children land in a different strategy label from their parent. This says whether omnivore lineages die out or drift into grazers, which the history's falling mean diet cannot tell apart.
- **Omnivore attack outcomes,** extending the step 5 `GateOutcomes` bands with an omnivore band, so it is known whether omnivores fire `attack`, and how often the damage gate stops them.

Print them in the headless summary as a per-band block every 500 ticks, like the existing `Grazer timeline` block, and add the omnivore income split to the Graphs tab. Reading the diet band for counting is an instrument, not a rule. Record the baseline on seeds 42, 3, 7 and 99 at 5000 ticks (99 because it is the one step 6 seed with a large omnivore population), and check the history CSVs are byte-identical to main with any new columns removed.

Done when: the summary prints the per-band block, the Graphs tab shows omnivore income by tissue, a `docs/audits/` note records the baseline per seed, and the note takes the structural-or-ecological decision above and says whether step 2 runs.

### 2. The digestion exponent as a knob, swept once

If step 1 finds the curve binding: move the exponent in `Genome::plant_efficiency` and `Genome::animal_efficiency` to a `SimConfig` field (`diet_efficiency_exponent`, default 2.0, flag `--diet-exponent`). The genome methods do not see `SimConfig` today, so they take the exponent as an argument, and every caller passes it: `kill_digestion_efficiency`, `action_system`'s food-item digestion, `grazing_system`, and the Inspect panel in `clauvolution_ui`. `SimConfig` is not saved beyond `terrain_seed`, so saves are unaffected. At the default, history CSVs must be byte-identical to step 1's.

Sweep 1.5 and 1.0 on the four step 1 seeds at 5000 ticks, reading step 1's counters: omnivores alive after tick 3000, their income by tissue, band crossings from omnivore to hunter, hunters alive at 5000, labelled hunters' share of income from animal tissue, and plant floors after tick 1000 against step 1's baseline. If neither value leaves intermediates alive after tick 3000, try the flat-topped curve once at the same seeds and record it, or stop and say so.

Done when: the decision entry records the sweep. A value ships only if it keeps omnivores alive after tick 3000 on most of the four seeds, plants and grazers persist and cycle, and any hunters it produces take most of their income from animal tissue; otherwise the knob ships at 2.0 with the numbers.

### 3. The animal kill share on top of the step 2 setting

If step 2 leaves intermediates alive that kill but do not cross into the hunter band: sweep `--kill-transfer-animal` at 0.3 and 0.6 with `--kill-transfer-plant` at 0.1 and the step 2 exponent, on the same four seeds at 5000 ticks. No code change. The tissue-share note's shipping bar applies: hunters alive at 5000 on most seeds and plant floors after tick 1000 no worse than about half the baseline.

Done when: the sweep is recorded under "Kill share by victim tissue", and the share either ships with its numbers or stays at 0.1 with them.

### 4. Audit and outcome

The eight-seed, 15000-tick audit on main (seeds 1, 2, 3, 7, 42, 99, 314, 1000, one run per seed), read against `docs/audits/2026-09-24-pyramid-step6/`: hunters at 5000 and 15000, omnivores after tick 3000, band crossings, hunters' income by tissue, plant floors, time at the ceiling (`consumer-ceiling-regulation`), and species at 15000 (`species-count-above-tuned-band`). If a hunter level exists, run the interdependence test from `plans/2026-09-19-diet-axis.md` (`--animal-efficiency 0`) and record it. Update `hunter-emergence` per `docs/TODO_GUIDE.md` and write the outcome into `docs/design/simulation-rules.md`.

Done when: hunters are alive at 15000 on most of the eight seeds, or the design doc says why not, with this plan's counters, and names the next candidate.

## What done looks like for the whole plan

It is known, with numbers, why intermediate diets vanish: whether the digestion curve rules them out or the world removes them. If the curve was the reason, a flatter one has been tried and either keeps omnivores alive long enough for carnivory to emerge from them or is recorded as not enough. A hunter level persists on most seeds and lives on animal tissue, or the reason it cannot is written down with this plan's counters and the interdependence test is blocked on one named thing.

## Not in scope

- Founder staging by trophic level, including founder claws or wiring weighted by diet. Rejected in the design doc's decisions table and above.
- Any rule that reads the diet gene to pick a target, or to set a reward by the killer's diet. Programmed behaviour. Digestion reading the eater's own diet is the existing rule and stays the only place diet acts.
- An efficiency exponent below 1, or any curve under which a generalist out-digests a specialist.
- A larger world or longer runs as the lever. Measured for time; deferred to phase 2 for space.
- Phase 2 (biomes as pressure and barrier), plant defences, and a separate `graze` output, as in the pyramid-top plan.
- Working `consumer-ceiling-regulation` or `species-count-above-tuned-band`. The audit re-measures both.

## Open questions

- **Band edges for the instruments.** The strategy labels split at ±1/3 (`classify_strategy` in `clauvolution_phylogeny`). An omnivore at -0.3 is nearly a grazer and one at +0.3 nearly a hunter, so finer bins may be needed to see the bridge; step 1 decides from the cost.
- **Which parent a band crossing counts against.** Crossover blends diet ("Crossover blends each scalar trait with its own factor"), so a child of two parents in different bands has two candidate parent bands. Step 1 picks one convention and records it.
- **Hunters that graze.** A flatter curve gives a founding hunter a food-item and bite bridge as well as giving omnivores one; at exponent 1 a hunter at `diet = 1/3` digests a third of plant tissue. Whether that is a bridge or an apex that also grazes is what step 2's income-by-tissue reading has to show.
- **Food items as the bridge.** Food items are plant tissue digested at plant efficiency, and the founding stock is used up in the first few hundred ticks (`founding-boom-food-regen`). If step 1 shows omnivores living on food items and dying as the items run out, the bridge question and that entry are the same question.
- **If the bridge fails.** The narrower levers the step 6 note left, a diet-blind founder claw probability or output-bias range, and whether a founder's wiring fires `attack` at all, act on founders rather than lineages. They are the next candidates, as a separate plan, if this one ends with intermediates that persist and still do not climb.

## References

- `TODO.md`: `hunter-emergence`, `consumer-ceiling-regulation`, `species-count-above-tuned-band`, `founding-boom-food-regen`.
- `docs/design/simulation-rules.md`: decisions table, "Phase 1 outcome", "Pyramid-top outcome".
- `docs/DECISIONS.md`: "Diet trait: instrumented before it acts" ("Why the squared curve"), "Diet axis tuning pass", "Strike cost", "Size gate on consumer prey: measured, left alone", "Kill share by victim tissue", "Nearest-eater inputs".
- `docs/audits/2026-09-23-pyramid-hunter-payoff/`, `docs/audits/2026-09-23-pyramid-tissue-share/`, `docs/audits/2026-09-23-pyramid-size-gate/`, `docs/audits/2026-09-24-pyramid-step6/`: the numbers this plan reads against.
- `plans/2026-09-21-pyramid-top.md`: the open question this plan takes up.
