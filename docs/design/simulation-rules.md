# Simulation rules: direction and design

**Status:** approved in discussion 2026-09-18; phase 0 shipped 2026-09-18 (see "Phase 0 outcome"); phase 1 shipped 2026-09-20 (see "Phase 1 outcome"); phase 2 not started
**Source:** step 3 of `plans/2026-09-17-simulation-rules-rethink.md`
**Baseline:** `docs/audits/2026-09-18-attractor-audit/` on commit 984aedd

This document sets the direction for the simulation's rules and decomposes it into phases. Each phase is one or more ordinary units of work through branches and pull requests, each with its own `docs/DECISIONS.md` entries and its own audit. The crate layout, schedules, and ECS shape are out of scope; nothing found in the review or the audit argues for changing them.

## Why

Three complaints started this: the sim stagnates, organisms are not noticeably divergent, and the world seems too small for real niches or biomes. Between 2026-09-17 and 2026-09-18 the four review-backlog fixes and the corpse energy fountain fix removed every known way the sim was lying about itself, and the attractor audit then measured it honestly for the first time. The attractors were not artefacts. On 16 runs across eight seeds at 15k ticks, 11 ended above 85% plants and none below 68%; predators peaked in the first few sim-seconds and collapsed on almost every run; body size settled between 0.47 and 0.60 everywhere.

Reading the current rules against that result explains it. The food web has no herbivore level: foragers eat terrain-spawned food items, not living plants, and predators attack any organism, for whom a plant is the easiest prey there is. Nothing needs anything else to exist. Terrain is a resource map that acts on where food appears, not on what kind of body can live there, so the same lineages are simply denser where food is denser. A global population cap of 2000 couples every region to every other. The speciation distance is dominated by an unnormalised body term, so species are trait-led by accident. And no instrument could see energy being created, which is how three minting bugs survived every tuning pass.

## What a good run looks like

From the discussion that produced this document. After ten or fifteen minutes of watching:

- **Ecosystems, not populations.** Species need each other. Remove one level and the level below overruns its food and crashes. Too many hunters and the hunters starve. The balance is fine and visible.
- **Different biomes hold different ecosystems.** Desert, forest, and ocean support different kinds of life because they cost different bodies different amounts to live in.
- **Species that visibly differ.** Two species can be pointed at and the reason they are not the same can be seen: what they eat, where they live, what they look like.
- **Not the same plant soup every run.** Divergence across runs matters, but only as a consequence of the three points above, not as a goal in itself.

Ranked: geography first, legibility close behind, coexistence over time welcome but secondary.

## Decisions

| Question | Decision | Rejected | Why |
| --- | --- | --- | --- |
| What makes biomes hold different life | Trait pressure (heat tolerance, aquatic axis, terrain-aware movement cost) and barriers arising from that pressure | Resource differences only; explicit dispersal limits; a bigger world alone | Resource differences give the same species at different densities. Explicit limits are a rule about movement rather than a property of the world. Distance alone does not separate regimes that lock in globally at 2000 organisms. |
| How species come to need each other | A diet axis on the genome, herbivore to carnivore, making digestion a specialisation | Universal attack tuned harder; diet axis plus plant defences in one step | No tuning gives a trophic pyramid when the apex can also graze. Plant defences are the first follow-on once the axis is watchable. |
| What a species is | Trait-led: body term normalised to 0..1 and weighted 1.0; NEAT terms at 0.5 each | Brain-led; balanced | A species should be a way of making a living that can be seen from outside. The diet and biome traits will carry the ecological meaning. Brain terms stay so a behavioural split can still become a species. |
| Energy accounting | A visible ledger: per-flow accumulators, a residual line on the Graphs tab and in the headless summary, a debug assertion on the residual | Debug assertion only; a finite environment energy pool | The residual is the bug detector that would have caught all three minting bugs; the flows are a tuning instrument. A finite pool is a different sim. |
| Carrying capacity | Emergent from energy; the hard cap becomes a high safety ceiling that logs when it engages | Keep the global cap; cap per region | The global cap is a hidden shared resource coupling every region. A per-region cap invents a boundary the sim does not otherwise have. |
| Initial conditions | Founders seeded per biome with modest starting energy | Leave the founding boom; stage founders by trophic level | The boom decides the regime before anything adapts. Staging programmes the pyramid into the opening, against the project's principle. |
| Barriers | Terrain as barrier, body as key: movement cost by terrain and body plan, generator tuned toward continents | Explicit dispersal limits; a bigger world alone | Same mechanism as the trait pressure seen from the movement side, and a barrier visible on the minimap is one a lineage can be watched crossing. World size stays at 512 until this is measured; a bigger world is a follow-on. |
| Sequencing | Foundations first, then diet, then geography | Geography first; dynamics in impact order with instruments as needed | The project's rule is instrument, then add the dynamic, then tune. A two-strategy soup gives biomes little to sort; grazers and hunters give them a pyramid to place. |

## Phase 0: foundations

Four independent pieces, each its own branch. None is a new dynamic; all are instruments or corrections, and they touch different systems, so they can proceed in parallel.

### Energy ledger

An `EnergyLedger` resource in `clauvolution_core` with one accumulator per flow: photosynthesis income, food eaten, predation transfer, symbiosis transfer, metabolism, reproduction spent by parents, reproduction received by children, energy lost at death, and energy destroyed at the `max_organism_energy` clamp. Each system that already moves energy adds to the matching accumulator at the point it moves it. A `ledger_system` at the end of the `FixedUpdate` chain sums live organism energy, compares the change since the previous tick against the net of the flows, and records the residual.

The Graphs tab gets a flows chart and a residual line; the headless summary gets a ledger block with cumulative flows and the largest residual seen. The residual is a `debug_assert!` at near zero and a chronicle warning in release builds.

Done when the residual is zero to floating-point rounding over a 5000-tick run on three seeds, and the flows block appears in the headless summary.

### Emergent carrying capacity

Remove the 2000-organism cap from `reproduction_system`. Add `SimConfig.population_ceiling` at 6000 whose only effect is to block births and write a chronicle entry each time it engages, so it cannot quietly become the rule again.

Done when a 15k-tick run on the audit seeds never touches the ceiling and the population plateaus at a level the ledger's photosynthesis and food flows account for.

**Phase 0 finding (resolved 2026-09-20: the ceiling is 6000 now that canopy light sharing and grazing set the carrying capacity; see `docs/DECISIONS.md`, "Emergent carrying capacity", update).** The ceiling shipped at 2000, not 6000. With the cap removed, every audit seed ran to the 6000 ceiling within 630 to 780 ticks and pinned there at 5700 to 6000 plants, at 2.5x to 3.4x the tick cost: a plant alone on its tile has all the light it needs and nothing consumes it, so under the current rules there is no energy-derived plateau below 6000 and the cap is the carrying capacity. The mechanism and the instrument are in (the ceiling is the only birth limiter, each engagement is a chronicle episode, and the headless summary counts engagements and blocked births), so the cap is now visible as the rule it is. The raise to a safety ceiling is sequenced after phase 1, once grazing gives plants a consumer, and the plateau is measured again then. See `docs/DECISIONS.md`, "Emergent carrying capacity".

### Per-biome seeding

`spawn_initial_population` places founders in proportion to each biome's land area, with a floor per biome so no biome starts empty, and gives every founder energy near `reproduction_energy_threshold` rather than well above it. The first minutes become colonisation rather than a feeding frenzy on free energy.

Done when the first 300 ticks of a seed-42 run show no predator peak above the founding predator count in the population history.

### Speciation normalisation

In `compatibility_distance`, divide the body term by the number of traits and by each trait's range so it lies in 0..1, weight it at 1.0, and weight the three NEAT terms at 0.5 each. Give each scalar trait its own crossover blend factor instead of one shared factor, so offspring can combine one parent's speed with the other's armour. Sweep the threshold once to recover a species count in the audit's range.

Done when species count at 15k ticks on the audit seeds is between roughly 10 and 30, and the `docs/DECISIONS.md` entry records the weights and the sweep.

## Phase 0 outcome

All four pieces merged on 2026-09-18: the energy ledger (#12), speciation normalisation (#13), per-biome seeding (#14), and the population ceiling (#15), with the follow-ups filed in `TODO.md` (#16). Each landed as one squashed, formatted commit under the quality gates from #11.

| Piece | Done-when | Result |
| --- | --- | --- |
| Energy ledger | Residual zero to rounding over 5000 ticks on three seeds | Met. Largest residual under 0.001 per tick against live totals near 225k. Found and fixed a fourth minting bug in symbiosis transfer. |
| Speciation normalisation | Species count in the audit's range at 15k ticks | Met at threshold 1.0 (13 to 36 species at 5000 ticks). The sweep has a cliff: 1.1 collapses to four to six species, so the sweep must be repeated whenever a trait is added. |
| Per-biome seeding | No predator peak above the founding count in the first 300 ticks | Not met. Founder placement and energy are in; the boom is driven by food regeneration refilling toward its ceiling, not by starting conditions (`founding-boom-food-regen`). |
| Emergent carrying capacity | A 15k run that never touches the ceiling | Not met. Shipped at 2000 with instrumentation. Without the cap every seed ran to 6000 plants at roughly three times the tick cost; nothing consumes plants, so the cap is the carrying capacity until phase 1. |

Three findings from phase 0 change how phase 1 should be approached, and the phase 1 tuning pass has to address each before its done-when means anything:

- **The energy clamp destroys about as much energy as foragers eat** (`energy-clamp-waste`): 7.40M destroyed against 7.62M eaten over 5000 ticks on seed 42. A forager near the 120 cap keeps almost nothing from a meal. The diet axis makes eating a specialisation, so what a specialist can keep has to be decided in the same pass.
- **The population cap is the carrying capacity.** The ceiling raise is sequenced after grazing gives plants a consumer; when it is raised, the plateau and the tick cost (`reproduction-linear-scans`) are measured again.
- **Water is neither barrier nor habitat.** The movement code picks the cost table by tile rather than by the organism's aquatic adaptation, so deep water is cheaper to cross than sand and no audit run has shown geographic isolation. (Fixed in phase 2 step 2: the cost now interpolates between the two tables by aquatic adaptation; see "Movement cost interpolates by aquatic adaptation" in `docs/DECISIONS.md`.) Water tiles grow no food. Whether oceans become habitat, with the colonisation of land as something to watch, or stay a barrier that aquatic specialists unlock, is a phase 2 design decision to make before the terrain work (`oceans-as-habitat`).

Same-seed runs remain non-reproducible when started at different moments (`determinism-claim-recheck`), so every comparison in phase 1 carries that spread.

## Phase 1: the diet axis

One new dynamic with a tuning pass.

### Mechanism

The genome gains `diet` in -1..1, mutating like any other scalar trait. It sets two digestion efficiencies: plant tissue at `((1 - diet) / 2)²` and animal tissue at `((1 + diet) / 2)²`. A specialist at either end digests one kind fully; a generalist at zero digests a quarter of each. The middle is a real cost, so the axis has two attractors of its own and omnivory has to earn its place.

### Two kinds of eating

An attack on a photosynthesiser becomes a graze. The grazer takes a bite, a fixed fraction of the plant's energy scaled by the grazer's plant efficiency, and the plant lives on with less. Plants become a renewable food rather than a one-shot meal, which is what lets a grazer population persist on a plant population. An attack on anything else stays a kill, with the existing 10% transfer scaled by the attacker's animal efficiency. Terrain food items count as plant tissue and their regeneration is reduced so that living plants become the primary food; whether they go entirely is decided in the tuning pass.

### What changes elsewhere

Strategy classification becomes plant, grazer, hunter, and omnivore, from photosynthesis rate and diet. Population history, the Graphs strategy plot, the headless summary, species naming, and the audit summary script follow. The ledger's transfer flow splits into grazing and predation. Brain inputs stay as they are; a "nearest photosynthesiser" sense is a follow-on if grazers cannot find food.

### Tuning pass

Three knobs: bite fraction, the efficiency exponent, and food-item regeneration. The bite fraction starts small, because the risk is plants going extinct before grazers specialise. Watched through the ledger flows and the four-way strategy plot.

### Done when

On the audit seeds at 15k ticks, plants, grazers, and hunters all persist on most seeds. And the interdependence test: a headless run with animal digestion forced to zero shows grazers overrunning plants and crashing. That is the "you need wolves to keep the deer in check" claim made checkable, and its numbers go in `docs/DECISIONS.md`.

## Phase 1 outcome

Shipped 2026-09-19 to 2026-09-20 as #18 (trait and instruments), #19 (grazing and digestion), #20 (tuning knobs and the finding that no knob bounds plant growth), and then, because that finding was structural, `plans/2026-09-20-plant-physics.md`: #21 (instruments), #22 (drag from photosynthetic surface), #23 (light shared over a canopy, the ceiling raised to 6000, and the diet-pass defaults of founder spread 1.0 and bite 0.3). The audit is `docs/audits/2026-09-20-phase1-audit/`.

| Done-when | Result |
| --- | --- |
| Plants, grazers and hunters persist on most seeds at 15k ticks | Plants and grazers on sixteen of sixteen runs, cycling against each other; hunters on none. |
| Interdependence test: hunting off shows grazers overrunning plants | Cannot run: the baseline has no hunters. Grazers are held from below by their plants, visibly (plants fell below 100 in seven runs and recovered in five). |
| Threshold re-swept | 1.0 kept; the cliff at 1.1 softened to 10 to 18 species with larger, cycling populations. |
| Ceiling measured (diet plan step 4) | Raised to 6000 in #23; eight of sixteen audit runs touched it, seven briefly. |

Two things the phase did not anticipate. First, the diet axis on its own could not work: consumers collapsed under any knob because plant growth was unbounded and plants could move, so the phase grew two physical couplings (a leaf is a sail; light is shared over a canopy) before its own tuning could mean anything. Those belong to the "what makes biomes hold different life" and "carrying capacity" rows of the decisions table as much as to this phase. Second, the top of the pyramid is not a tuning problem: a founding hunter digests almost nothing but meat, has no bridge while its brain is random, and starves in a few hundred ticks at any kill share; and the one attack output that a grazer must use to bite a plant also kills its neighbours (43% of all deaths with no hunters alive). Both are filed as design questions (`hunter-emergence`, `graze-attack-output-split`) and are the open thread phase 2 inherits.

**Update 2026-09-23 (`plans/2026-09-21-pyramid-top.md`, steps 1 to 3).** The output question is settled: `eat` bites a living plant in contact reach with the mouth bonus, `attack` is only a kill attempt, and a strike costs the attacker `1.0 × claw power × body size` (see "Grazing through eat; an attack on a plant is a kill" and "Strike cost" in `docs/DECISIONS.md`). The 15k-tick re-read on the same eight seeds (`docs/audits/2026-09-23-pyramid-reread/`) found plants and grazers persisting and cycling on eight of eight, with plant share 31% to 56% and only one seed dipping under 100 plants (it recovered). Grazer kills of consumers fell to 8 to 35 per 1000 consumer-seconds, against 69 to 149 predation deaths in phase 1, and predation's share of deaths from 43% to 31%. Hunters did not appear: none alive at 5000 or 15000 on any seed, founding hunters dead at a mean of 240 to 270 ticks, and no omnivore lineage left to become one. So the first half of "Two things" above is still open (`hunter-emergence`), and the second exposed a new one. With the competition kills mostly gone and no predator, nothing energetic holds consumers down. Every seed spent 14% to 45% of the run at the 6000 ceiling and disease became half of all deaths (`consumer-ceiling-regulation`). A probe at kill share 1.0 held a cycling hunter population to tick 15000 on seed 42 and to 11341 on seed 3, and it kept seed 42 off the ceiling, but it paid out on plant kills too and plants went extinct on two of four seeds. The done-when row for hunters still reads "on none".

**Pyramid-top outcome, 2026-09-24 (`plans/2026-09-21-pyramid-top.md`, all six steps).** Steps 4 and 5 were tried and did not bring a hunter level: the nearest-eater inputs (step 4, "Nearest-eater inputs") left founding-hunter lifetime within the spread a change of founder draws produces, and the size gate (step 5, "Size gate on consumer prey: measured, left alone") stops almost no hunter attack on its own, so it was left as it is. The kill share was split by victim tissue and shipped at 0.1 for both ("Kill share by victim tissue"). The step 6 audit on main (`docs/audits/2026-09-24-pyramid-step6/`), eight seeds at 15000 ticks and the first on the species stay-threshold fix (#45), is the new baseline:

| Done-when | Result |
| --- | --- |
| Attack means a kill attempt, grazers feed with mouths | Met: every graze on every seed goes through `eat` (1.23M to 2.12M bites a run, none through attack). |
| Predation deaths with no hunters alive a small fraction of phase 1 | Partly: grazer kills of consumers run at 11 to 39 per 1000 consumer-seconds against phase 1's 69 to 149 predation deaths, 14% to 36% of each seed's phase 1 rate (mean of its two runs), and predation is 32% of deaths against 43%. Consumers are about twice as numerous, so absolute counts are not a small fraction. |
| Hunters alive at 15k on most of eight seeds | Not met: none at 5000 or 15000 on any seed. Founding hunters die at a mean of 223 to 279 ticks; no seed held more than two hunters at once after tick 1500. |
| Interdependence test run or blocked on one named thing | Blocked on `hunter-emergence`. |

Plants and grazers persist and cycle on all eight seeds, with plant floors after tick 1000 of 129 to 627 (none under 100) and plant share 33% to 56% at the end. The ceiling still regulates consumers on seven of eight seeds (28 to 243 of 500 samples at 5700 or more) and disease is 49% of deaths (`consumer-ceiling-regulation`). A control build without #45 gave the same pyramid on the same seeds: no hunters, the same death shares to within half a point, and per-seed swings in ceiling time in both directions. #45 lowered the species count at 15000 on six of eight seeds (mean 61 to 50), which is still above the band the threshold was tuned for (`species-count-above-tuned-band`).

Why no hunter level, as measured: about two thirds of founding hunters never fire `attack`, most attacks by the rest bounce off the damage gate, and the 3 to 11 founders per seed that kill a consumer still die. Time does not grow carnivory from omnivory: consumers converge on diet -0.96 to -0.97 and no seed holds more than three omnivores after tick 3000. The one change that has formed a hunter lineage is a larger kill share for animal victims (0.6 to 1.0), on one or two seeds of three or four and with plant floors falling, so it is not shipped. The next candidate is the plan's own open question, an omnivore bridge: why intermediate diets do not persist long enough for carnivory to emerge from them. That is a new plan. Phase 2 therefore starts on a two-level pyramid, as phase 1 did.

Of the plan's open questions, three are settled: an attack on a plant stays a kill attempt, digested at plant efficiency (step 2; consumers' plant kills were under 7% of the eat-grazing flow); bite reach is 1.0 × body size, contact (step 2's sweep); and older genomes gain the new inputs by migration, with no save break (step 4, "Brain inputs grow by migration"). The fourth, the hunter bridge, is what remains, together with `consumer-ceiling-regulation` and `species-count-above-tuned-band`.

Same-seed divergence, which every comparison in this phase carried as a caveat, was found and fixed after the audit ran (#31): headless virtual time followed the wall clock. Audits from here on are reproducible per seed.

## Phase 2: biomes as pressure and barrier

One dynamic in three parts with a tuning pass.

### Trait pressure

Two new genome traits: `heat_tolerance` in -1..1 and `aquatic` in 0..1. Every tile gets a heat value from biome and elevation: sand hot, high rock cold, forest and grassland temperate, water neutral. Metabolism pays a per-tick multiplier that grows with the distance between the tile's heat and the organism's tolerance, so a lineage that lives where it is suited is cheaper to run. Photosynthesis light per biome and vegetation regrowth already differ and stay as they are.

### Terrain-aware movement

The flat ten-times deep-water cost becomes a cost table by terrain and body: water cheap for high `aquatic` with fins and ruinous without, rock steep for large bodies, sand moderate and hot. Oceans and ranges become barriers for most lineages and habitat for the ones that pay to specialise. This replaces one special case with the general rule.

### A world with continents

The terrain generator is tuned so passable land forms several masses separated by deep water and split by rock. A unit test in `clauvolution_world` asserts at least two large passable land components on the default seed. Phase 0's per-biome seeding then gives each mass its own founders.

### What you see

Population by biome as a Graphs series, tolerance and aquatic values in Inspect, and one chronicle event: the first time a species has members on a landmass it did not start on. The existing species range heatmap already shows the geography. A regime-separation number, the mean share of each species' members that live in its dominant biome, goes to the Graphs tab and the headless summary.

### Tuning pass

The steepness of the heat multiplier and the movement cost table, against the ledger and the per-biome plot.

### Done when

On most audit seeds at 15k ticks, at least two biomes have different dominant species, the separation number is well above what a shuffled species-to-organism assignment gives, and at least one crossing event appears in the chronicle per run.

## Phase 3: follow-ons

In rough order:

- **Plant defences** as evolvable traits that cost energy (toxins, thorns), giving plants and grazers an arms race of their own.
- **A bigger world**, once phase 2 shows what separation looks like at 512 and the performance work below makes it affordable.
- **A "nearest photosynthesiser" brain input** if grazers cannot find food with the existing senses.
- **Long-term climate shift** from the roadmap, which becomes far more interesting once tolerance traits exist for it to push against.

## Cross-cutting

### Performance is on the critical path

Emergent carrying capacity and continents both push population up, and the per-tick cost roughly doubled when the moisture fix grew more vegetation and more food. The existing `TODO.md` items `spatial-hash-organisms-only`, `photosynthesis-density-cache`, and `rayon-remaining-systems` are prerequisites for phases 1 and 2 rather than optional improvements. The doubling was measured on 2026-09-23: the food-eating loop in `action_system` costs eaters × food × eaten items per tick, and all three grow with the food supply; it was nearly half the CPU of a 1000-tick run. the behaviour-neutral fix, a per-snapshot eaten flag, landed in #50, and `sensing-per-organism-cost` is what dominates after it.

### Measurement

Every phase ends with `scripts/attractor_audit.sh` on the same eight seeds, two runs each, and a dated block in the roadmap's attractor-states section beside the previous ones. Same-seed divergence (`determinism-claim-recheck`) stays a stated caveat on every comparison until it is resolved.

### Risks

- The diet axis could send plants extinct faster before grazers specialise. The bite fraction starts small and the ledger shows the grazing flow directly.
- Barriers can isolate a founding population into extinction. That is an acceptable outcome, but it must be visible, hence the crossing event and the per-biome plot.
- Trait-led speciation plus three new traits could over-speciate. The threshold is swept once after each phase, and the weights are recorded each time.
- Each new trait is a new brain-agnostic dimension of selection. If lineages converge on the trait optimum without behavioural change, that is a finding about the brain inputs, not a failure of the phase.

### Out of scope

Crate layout, schedules, the ECS shape, the render and UI structure, and everything in `review/BACKLOG.md`. This design is about rules.

## Open questions

To decide before the phase that owns them starts:

- **Phase 2: water as habitat or barrier.** See "Phase 0 outcome" and `oceans-as-habitat` in `TODO.md`. This changes what the terrain generator should produce, so it precedes the continents work.
- **Phase 1: what a specialist keeps.** The clamp at `max_organism_energy` discards most of a full forager's income (`energy-clamp-waste`); the diet axis tuning pass decides whether the cap rises, scales with body size, or feeds reproduction readiness.

Small enough to settle in the implementing pull request:

- Whether terrain food items survive phase 1 at all, or only as a seasonal supplement.
- The exact heat value per biome and how elevation contributes, which the phase 2 tuning pass decides.

## References

- `plans/2026-09-17-simulation-rules-rethink.md`: the plan this document completes.
- `docs/audits/2026-09-18-attractor-audit/README.md`: the baseline.
- `docs/ROADMAP.md`: attractor states, ecosystem tuning, Theme 2 dynamics.
- `docs/DECISIONS.md`: energy pyramid, quadratic costs, species threshold, `Killed` marker, photosynthesis multiplier revision.
- `review/2026-09-17-0756-full.md`: findings `compatibility-body-term-unbounded`, `crossover-single-blend-factor`, `mate-energy-unscaled-and-unpaid`.
