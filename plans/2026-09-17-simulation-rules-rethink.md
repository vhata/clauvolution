# Plan: rethink the simulation's rules against an honest baseline

**Date:** 2026-09-17
**Status:** proposed
**Scope:** the simulation's rules (energy, speciation, world heterogeneity). Not the code structure.

## Why now

The motivation for a rethink was three complaints about the running sim: it stagnates, organisms are not noticeably divergent, and the world seems too small for real niches or biomes. The four review-backlog branches merged today (#3, #4, #5, #6) changed what those complaints mean.

The review found bugs. The branches found that the ecology was balanced on top of them:

- **Energy was being created at three separate points.** Small parents paid less for reproduction than the child received (#6). Several attackers could each be paid for the same victim in one tick (#6). And a killed photosynthesiser refills energy the same tick it dies, survives at zero health, and is killed and paid for again on later ticks (`predation-corpse-energy-fountain`, TODO.md). A one-line experiment gating photosynthesis on health flipped a 1000-tick seed-42 run from 758 plants and 44 predators to 48 plants and 641 predators. The current plant-dominated regime depends on that fountain.
- **Half the land had no biome.** Moisture was normalised to -1..1 while every consumer assumed 0..1, so roughly half the land had negative vegetation capacity and no regrowth, and Forest was confined to the top fifth of the range (#5). Sand fell from about two thirds of land to a sixth on seed 42 once corrected.
- **Headless and GUI were not the same sim.** The spatial hash was rebuilt per frame, not per tick, so every headless number ever taken was measured with stale neighbours (#3). The GUI speed control rescaled the fixed timestep and so slowed every virtual-time timer relative to headless (#4).
- **The measurements themselves were off.** The species count, the population readout, and the kill count were all wrong in the headless summary (#6). The review's clippy baseline of zero was a broken counting command (`clippy-baseline-toolchain`).

The consequence is that the attractor states recorded in `docs/ROADMAP.md` (green world in 7 of 8 seeds, predator extinction in 6 of 8) were observed on a sim that did not conserve energy and had no working biomes. Nobody yet knows what this sim does when it is correct. A rethink is still the right move, but it cannot start from the current observations, and it should be about rules, not architecture. The crate layout and ECS shape held up well under four parallel branches touching the same files.

## The plan

Three steps, in order. Each is a normal unit of work through a branch and a pull request.

### 1. Close the corpse energy fountain and re-tune

Claim `predation-corpse-energy-fountain` from `TODO.md`. Killed organisms must not earn energy after death, from photosynthesis or from symbiosis transfer, and must not be killable a second time. Kills must not exceed predation deaths in the headless summary afterwards.

This is a mechanism change with a tuning pass attached, per the contract's rule for new dynamics. The experiment above shows the balance will move a long way. Expect to adjust `PHOTO_OUTPUT_MULTIPLIER`, the predation energy fraction, or initial seeding, and record whatever is chosen in `docs/DECISIONS.md`. The goal of the tuning pass is only that the sim is watchable again, not that it is balanced. Balance is the design doc's job.

Done when: kills are at or below predation deaths on every seed of the audit below, no energy is created at any birth, kill, or transfer, and a GUI run at seed 42 reaches 5000 ticks without either plants or predators going to zero.

### 2. Re-run the 8-seed audit for a baseline

Re-run the 8-seed, 15k-tick headless audit described in `docs/ROADMAP.md` on the resulting `main`, using `--dump-history` so every run leaves a CSV. Record the results in the roadmap's attractor-states section as a dated block alongside the April numbers, with the reasons the two are not comparable.

Two things to note while doing it. The moisture fix roughly doubled per-tick cost (`moisture-fix-tick-cost`), so the audit will take about twice as long as April's; measure it rather than fight it, unless it becomes impractical, in which case `spatial-hash-organisms-only` is the first lever. And the determinism claim in `docs/DECISIONS.md` may be stale (`determinism-claim-recheck`); run seed 42 twice and record whether the two runs match, because if they do the integration tests in roadmap Theme 4 become possible immediately.

Done when: the roadmap has a dated block of eight final strategy breakdowns, death-cause splits, species counts, and trait averages, taken on a named commit.

### 3. Write the design doc

With the baseline in hand, write `docs/design/simulation-rules.md` answering three questions. Each one maps onto one of the original complaints, and each surfaced this week as a design call rather than a bug.

**Energy conservation as an invariant.** Three of this week's defects were energy minting. The question is whether the simulation should carry a per-tick energy ledger that asserts energy in equals energy out plus declared losses, exposed on the Graphs tab and as a headless summary line. This is cheap to build, would have caught all three defects, and would make every future tuning number trustworthy. The doc should decide the accounting boundary (what counts as a loss, how photosynthesis income and event deaths enter) and whether the assertion is a debug assert, a warning, or a chronicle entry.

**What speciation is measuring.** Two read-only findings in the review ledger bear directly on divergence. `compatibility-body-term-unbounded`: the morphology term in `compatibility_distance` sums nine raw trait deltas and can exceed 2 on its own against a threshold of 1.3, so speciation is driven by trait drift rather than brain topology, and the April threshold sweep was tuning around it without naming it. `crossover-single-blend-factor`: all nine traits blend with one shared factor, so offspring lie on a line between the parents and cannot recombine one parent's speed with the other's armour. The doc should decide whether species are meant to be brain-led or trait-led, normalise the distance accordingly, and decide per-trait crossover, then propose how to observe divergence (the genome diff view in roadmap Theme 1 is the natural instrument).

**Heterogeneity versus size.** The complaint was that the world is too small for niches. `docs/DECISIONS.md` already records that plants spread to about one per tile at 2000 organisms, so the world is sparse, not cramped, and the fountain experiment shows it can support a predator-heavy regime. The question is whether distinct biomes and dispersal barriers create separate selective regimes, which the moisture fix only just made observable. The doc should propose how to measure regime separation (species range against biome, trait averages per biome) before proposing new terrain, and only then decide whether world size, biome thresholds (`biome-threshold-retune`), or barriers are the lever.

Done when: the doc has a decision and a first implementation shape for each question, each shape is small enough to be one or two pull requests, and the roadmap links to it.

## Not in scope

- Re-architecting crates, schedules, or the ECS layout. Nothing found this week argues for it.
- The remaining review backlog entries (chronicle dedupe, volcano range, screenshot tour, render fixes). They are independent of this plan and can be picked up whenever someone is in those files.
- Performance work beyond what step 2 needs to finish in reasonable time.

## Open questions

- Should step 1 land the tuning pass in the same pull request as the mechanism fix, or as a second pull request once the audit shows where the balance settled? The contract says budget a follow-up pass; it does not say it must be separate.
- Does the energy ledger belong in step 1 as instrumentation for the tuning pass, rather than waiting for the design doc? It would make step 1's done-when condition checkable by the sim itself.

## References

- Pull requests #3, #4, #5, #6 (review-backlog fixes) and #7 (follow-up TODO entries).
- `review/2026-09-17-0756-full.md`: findings `compatibility-body-term-unbounded`, `crossover-single-blend-factor`, `mate-energy-unscaled-and-unpaid`, `hidden-neuron-ids-not-innovated`.
- `TODO.md`: `predation-corpse-energy-fountain`, `moisture-fix-tick-cost`, `determinism-claim-recheck`, `biome-threshold-retune`, `spatial-hash-organisms-only`.
- `docs/ROADMAP.md`: "Attractor states" and "Ecosystem tuning" sections.
