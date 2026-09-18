# Design decisions

This is the "why did we do that?" document. Each entry records a non-obvious choice, the alternatives considered, and the tradeoff we accepted. New entries should follow the same shape.

Not an exhaustive list of every tweak — just the decisions where someone reading the code might reasonably wonder "why this way and not another?"

---

## Simulation biology

### Energy pyramid — predators get 10% of prey's stored energy
**Chosen:** when a predator kills prey, it receives `prey_energy × 0.1`.
**Alternatives:** full energy transfer (100%), half (50%), fixed constant.
**Why:** thermodynamically honest — most energy is lost as heat in real ecosystems. Also functions as a balance mechanism: predators can't sustain themselves indefinitely on abundant prey, preventing predator-dominated attractors.
**Accepted tradeoff:** predators need dense prey to thrive; in sparse populations they struggle. This is realistic but can mean predator lineages fail on some seeds.

### One kill per victim per tick — first attacker wins
**Chosen:** `predation_system` tracks the victims claimed during the current tick. The first attacker to land a kill on a target takes the 10% energy transfer; later attackers skip that target and carry on scanning for another one.
**Alternatives:** split the transfer between every attacker that picked the victim; pay every attacker in full (the previous behaviour, by omission).
**Why:** several attackers can pick the same victim in one tick. Before this rule each of them was paid 10% of the victim's unmodified energy and each pushed a kill, so one death was paid several times over and `PredationStats.kills` exceeded predation deaths (4724 kills against 1946 predation deaths at the baseline seed of the 2026-09-17 review). Splitting the transfer would also conserve energy, but the payout would then depend on how many neighbours happened to fire in the same tick, which is noise rather than a selective signal. First-wins keeps the payout a fixed fraction of the victim's energy, and the attacker that loses the race is free to find another target in the same tick.
**Accepted tradeoff:** which attacker wins is query iteration order, not a contest of size or speed. At one tick of resolution this is indistinguishable from simultaneity, and the loser pays nothing for the attempt.

### Child starting energy is a fraction of what the parent paid
**Chosen:** a child is born with `CHILD_ENERGY_FRACTION` (0.8) of the parent's actual reproduction cost, which is `reproduction_energy_cost × (0.5 + body_size × 0.5)`. At body size 1.0 that is 32 energy, the value every child received before; at the 0.3 floor it is 20.8.
**Alternatives:** a fixed child energy regardless of parent size (the previous behaviour); the child receives the full cost with no overhead; neither cost nor child energy scaled with size.
**Why:** the parent's cost scaled with body size but the child's energy was a fixed `reproduction_energy_cost × 0.8` = 32. Below body size 1.0 the parent paid less than the child received, so every birth created energy from nothing (2 at size 0.5, 6 at the 0.3 floor); above 1.0 the sign flipped and reproduction taxed large bodies. That is a gradient toward small bodies that no decision had chosen, and it sat underneath every observation about body-size drift elsewhere in this document. Tying the child to the parent's actual payment makes each birth conserve or lose energy at every size.
**Accepted tradeoff:** small-bodied children start with less energy than before and have less runway to their first meal, so small lineages lose a subsidy they had been evolving under. Watch the body-size average after this change; the 2026-09-17 review baseline sat at 0.81 for `--headless 1000 --seed 42`.

### Quadratic costs for body, armor, claws, speed
**Chosen:** metabolism cost scales as `trait²` for these four traits.
**Alternatives:** linear costs, tiered cliffs, no extra cost.
**Why:** prevents "stack everything" meta. If costs were linear, evolution would converge on maxed-out big armored fast predators. Quadratic costs force trade-offs — you can be fast *or* armored *or* big, but not all three cheaply.
**Accepted tradeoff:** organisms might never evolve extreme traits because the marginal cost becomes prohibitive. If we see everyone converging to tiny low-trait organisms, that's the sign we've overtuned it.

### Plant density competition — `yield × 1/(1 + others_on_tile × 0.3)`
**Chosen:** photosynthesis yield drops inversely with the number of other plants on the same tile.
**Alternatives:** no competition (100% plant worlds), linear penalty (too harsh — nobody survives clustering), carrying-capacity cliff (abrupt and unfair).
**Why:** biologically honest (real plants shade each other) and naturally caps monocultures without banning clustering.
**Accepted tradeoff:** on its own, this mechanism **doesn't** cap plant dominance. Swept the coefficient 0.2 → 0.5 → 2.0 in a 512² world with ~2000 organisms and plants still reached 90%+. The world is large enough that plants naturally spread to ~1 per tile, so density rarely bites. Density competition is real pressure *when plants cluster*, but it's not the thing that prevents monoculture. That's what `PHOTO_OUTPUT_MULTIPLIER` is for (separate entry).

### Photosynthesis output multiplier — raw scalar on yield
**Chosen:** a global `PHOTO_OUTPUT_MULTIPLIER` multiplier on the photosynthesis energy equation. 0.5 from the original tuning; 1.0 since 2026-09-17 (see the revision at the end of this entry).
**Alternatives:** steeper density penalty (see above — doesn't help), more aggressive metabolism costs for photosynthesisers (biased against a strategy rather than rebalancing), reducing sunlight (same effect, worse name).
**Why:** density competition on its own wasn't enough because plants don't cluster densely in a large world. The blunt lever is lowering the raw photosynthesis yield so plant energy intake becomes comparable to what foraging yields — at which point foragers can actually compete. Tuning journey: 2.0 (original) → 1.0 → 0.7 all produced 90%+ plant monocultures. 0.5 consistently produces diverse ecologies across four tested seeds (plant share ranges 38%–79%, foragers 7%–61%, predators 0.5%–14%).
**Accepted tradeoff:** at first read the death breakdown looked starvation-dominant (~85% of deaths). That turned out to be a misattribution bug in `metabolism_system` (see "Health regen gate" below) — once fixed, the split is more like ~30% starvation, ~50–80% predation, rest disease. **Revised framing: predation is the primary selection pressure, not starvation.** In a live run after the fix, one second showed Predation 84% / Disease 8% / Starvation 7% / Old age 1%. Point stands: foragers earn their place because photosynthesis is no longer a free lunch — they just earn it by being predated less than alternatives, not by out-eating the shortfall. Also notable: only ~7 organisms are genome-classified `SpeciesStrategy::Predator` (`claw_power > 0.5`), yet hundreds of kills per second happen — most predation is done by foragers with modest claws. The strategy label is decorative for display, not load-bearing for dynamics.

**Revised 2026-09-17, back to 1.0:** every value in the tuning journey above was measured while a killed photosynthesiser kept photosynthesising and could be killed and paid for again (see "Kills are explicit" below). Plants were never actually losing organisms to predation, so the multiplier was being lowered against a plant strategy that could not lose. With kills final, 0.5 sent plants extinct by 5000 ticks on seed 3 and left 29 on seed 42. Sweep at 5000 ticks over seeds 1, 2, 3 and 42: 0.75 kept plants alive everywhere with predators at 3, 5, 32 and 10; 1.0 kept plants alive everywhere with predators at 21, 1, 28 and 49. Seed 42 at 1.0 was then run five times: plants 1165 to 1468, foragers 530 to 825, predators 2 to 49, and at 0.5 three times: plants 3 to 29. Same-seed runs are not reproducible at 5000 ticks (see `determinism-claim-recheck` in TODO.md), so the plant-side conclusion is robust across that spread and the predator counts are not a tuning signal at all. 1.0 chosen. The predator energy share (0.1) was also swept at 1.0: 0.15 and 0.2 did not sustain predators but flipped seeds between plant monoculture and plant extinction (seed 42 at 0.15: 0 plants, 809 predators; seed 3 at 0.2: 0 plants, 610 predators), so it stays at 0.1. Seeds 1 and 2 still drift to plant monoculture at 1.0 and predators decline slowly on every seed; the outcomes are bistable rather than tunable, which is a question for the simulation-rules design doc (`plans/2026-09-17-simulation-rules-rethink.md`), not for this constant.
### Kills are explicit — a `Killed(cause)` marker, and a corpse earns nothing
**Chosen:** `predation_system` inserts `Killed(DeathCause::Predation)` on each victim as well as zeroing its energy and health. `photosynthesis_system` and `symbiosis_transfer_system` filter on `Without<Killed>`. `death_system` despawns any organism with energy ≤ 0, health ≤ 0, or a `Killed` marker, and takes the cause from the marker when there is one; otherwise it attributes by old age, then disease, then starvation. `predation_system` also skips targets already at zero health.
**Alternatives:** gate photosynthesis and symbiosis on `health > 0` and keep inferring cause from health (smaller change, but leaves old-age deaths, which also reach zero health, counted as predation); despawn victims inside `predation_system` (loses the death marker and the per-cause bookkeeping in one place); reorder `death_system` to run directly after `predation_system` (still lets a corpse be paid for by symbiosis and photosynthesis on the next tick if anything revived it).
**Why:** `death_system` only despawned on energy ≤ 0. A killed photosynthesiser had its energy refilled by `photosynthesis_system` in the same tick, survived at zero health, could never regenerate because of the regen gate below, and was killed and paid for again on later ticks. In the 2026-09-17 accounting branch a diagnostic counted 1297 of 3039 kills landing on targets already at zero health, 1959 of them photosynthesisers; kills exceeded predation deaths on every run (4238 against 2476 at `--headless 1000 --seed 42` after the one-kill-per-victim rule). Predators were being fed from corpses, and the plant-dominated balance depended on it. Recording the cause at the kill also fixes old-age attribution: the "by Old age" line had always read 0 because health reaching zero was read as predation.
**Accepted tradeoff:** one more component and one more thing for a future system between `predation_system` and `death_system` to remember to filter on. Kills and predation deaths are now equal by construction (3389 and 3389 at the seed above). Removing the corpse income moves the ecology a long way; see the 2026-09-17 revision of the photosynthesis output multiplier entry above.

### Health regen gate — "a corpse doesn't heal"
**Chosen:** `metabolism_system`'s health regen step (`health += 0.005`) is gated on `health > 0.0`. Organisms whose health has been set to zero don't regen.
**Alternatives:** reorder systems so `death_system` runs before `metabolism_system`, make `predation_system` despawn victims directly, introduce a `Dying(cause)` marker component, use a large-negative sentinel for health that regen can't overcome.
**Why:** when the photosynthesis multiplier tuning shipped, death-cause attribution looked broken — 0 predation deaths across every seed despite thousands of attack events. Root cause: `predation_system` sets victim `health = 0` and `energy = 0`, then `metabolism_system` runs in the same tick and regens health to ~0.005 (because the `min(1.0)` clamp didn't have a corresponding `max(0.0)` floor for the dead), then `death_system` reads the victim as energy-dead but health-alive and classifies them as Starvation. Gating the regen on `health > 0.0` is the minimal invariant-preserving fix and reads naturally — a fatally wounded organism shouldn't regenerate between ticks. It also keeps the `death_system` cause-priority chain simple (health ≤ 0 → Predation) without introducing new components or ordering constraints.
**Accepted tradeoff:** a one-liner behavioural rule buried in `metabolism_system` that a future reader might not connect to attribution correctness. Mitigated by a comment at the gate explaining the predation-attribution coupling. Since the `Killed` marker (entry above) the gate is no longer what makes attribution correct; it stays as a guard so a zero-health organism never heals in the tick it dies.

### Disease: direct mortality + energy drain, not drain alone
**Chosen:** infected organisms suffer both a per-tick energy drain AND a small per-tick chance of direct death. Resistance protection is quadratic — `drain × (1 - res × 0.5)²` and `mortality × (1 - res)²`.
**Alternatives:** drain only (first implementation), direct mortality only, linear (1-res) protection (v2 pass, superseded).
**Why direct mortality:** drain-only failed on photosynthesisers. Plants refill energy from sunlight faster than disease drained it, so they were immune in practice. Adding direct mortality that ignores energy reserves ensures disease can't be "sun-bathed" through.
**Why quadratic resistance:** at linear, resistance of 10% gave 90% mortality factor vs 100% for res=0 — nearly no fitness delta across the observed evolved range (5-20%), so selection drifted. Squared amplifies the delta at the top of the range (res=50% goes from 50% factor to 25%) and leaves the bottom mostly unchanged. Validated at 15k ticks across seeds 1, 7, 42, 99: infected population roughly halved on 3 of 4 seeds (359/276/266/359 vs linear 458/638/267/378), resistance +1-3 percentage points.
**Accepted tradeoff:** Disease remains a background mortality pressure (~5-10% of deaths), not a primary evolutionary driver. Predation dominates the fitness landscape at 55-75% of deaths, so resistance is always a second-order concern; even with the quadratic bump, resistance doesn't climb to the range where squared really pays off (40%+). Shipping because (a) infected populations dropping is a monotone improvement, (b) the math now correctly rewards high resistance when it does appear. The direct-mortality path still only zeroes `energy` (not `health`) so `death_system` correctly attributes the death to Disease rather than Predation.

### Symbiosis tuning: loose threshold + modest transfer
**Chosen:** `SYMBIOSIS_RANGE = 6.0`, `SYMBIOSIS_LINK_THRESHOLD = 10` ticks, `SYMBIOSIS_TRANSFER_RATE = 0.15` energy per tick at |rate|=1.0.
**Why this set of numbers (observed, not theorised):** v1 shipped with range 6, threshold 30, transfer 0.05 and produced 3–35 pairs per 6000-tick seed — a barely-present dynamic. Widening range to 12 actually produced FEWER pairs (3–4) because more nearby candidates made the "nearest stays nearest" streak flaky. Shortening the streak to 10 ticks was the real unlock — pair count jumped to 66–124 per seed, so the mechanic now contributes something visible every tick rather than a handful of links per run. Transfer was tripled to 0.15 to give any individual link a measurable fitness impact; doubling further to 0.30 produced no extra selection signal and was reverted.
**Open question on selection:** avg evolved `symbiosis_rate` still sits within ±0.12 of zero after 6000 ticks across three seeds — no drift toward parasitism or donation that the math would predict. Hypotheses: (1) most actual pairs are between organisms with similar rates (e.g. near-neutral plants rooted nearby) so the asymmetric-pair case that produces selection is rare; (2) the mutation rate + link magnitude combination is too weak vs lifespan; (3) parasites that do get a donor partner drive the donor to extinction fast, so both alleles drop. Follow-up: (a) run 20k+ ticks to see if drift resolves given enough time, (b) instrument the histogram of rates in linked pairs specifically, (c) consider a small metabolic discount for being in any active link (gives mutualists an a-priori reason to exist, like the social-sensing discount does for clustering).
**Accepted tradeoff:** the dynamic's machinery is live and visible in the Graphs tab (pair count climbing over time, organisms in the Inspect tab label themselves parasite/neutral/donor), but we're not seeing a population-level specialisation emerge yet. Shipping the tuning that works (more pairs) rather than waiting for the tuning that delivers the evolutionary outcome we hypothesised.

### Social sensing: metabolic discount, not behaviour reward
**Chosen:** organisms near same-species kin get a small (~5%) metabolic discount via formula `1 - (count/(count+5)) × 0.05`.
**Alternatives:** direct "reward for clustering" bonus (rejected as intellectually dishonest), no discount (rejected as not producing visible social behaviour).
**Why:** biologically justified as "reduced vigilance cost" — real animals in groups spend less energy watching for predators. Diminishing returns so infinite clustering isn't useful. This is a selection pressure gradient, not a behaviour prescription. Evolution decides whether to exploit grouping, ignore it, or find something better.
**Accepted tradeoff:** this is still a "thumb on the scale" — without it, social behaviour wouldn't emerge because the sim doesn't have enough latent group benefits (dilution of predation, etc. are too weak). We accept that a gentle nudge is the cost of seeing emergent sociality in reasonable time.

### Component presence for boolean state (Infection)
**Chosen:** `Infection` is a component; its presence means the organism is sick, its absence means healthy.
**Alternatives:** `Infection { is_infected: bool }` field, an `Option<Infection>` on a health struct.
**Why:** Bevy-idiomatic. Queries like `Query<..., Without<Infection>>` and `With<Infection>` read naturally and the ECS can skip healthy organisms entirely. Adding `Infection` to an entity is an actual state change, not a flag flip on an always-present blob.
**Accepted tradeoff:** adding/removing a component has slight overhead vs flipping a bool, but at 2000 organisms it's negligible.

## Brains & evolution

### NEAT with 22 inputs / 9 outputs (no hard-coded behaviour)
**Chosen:** each organism has a per-organism neural network with specific labelled inputs and outputs, but no behaviour is pre-programmed.
**Alternatives:** scripted behaviour trees, finite state machines, heuristic AI.
**Why:** the core goal of the project — watch evolution discover behaviour. Scripted behaviour wouldn't be evolution, it'd be a game.
**Accepted tradeoff:** early-generation organisms behave poorly until selection produces useful circuits. Initial-diversity seeding (30% photosynthesisers) compensates by guaranteeing *some* strategy works out of the gate.

### Species classification threshold 1.3 with 1.3× hysteresis
**Chosen:** organisms classified by NEAT compatibility distance, threshold 1.3 to join a species, 1.69 (1.3×) to stay in it. Runs every 5 seconds.
**Alternatives:** 2.0 (original — observed stagnation), 1.5 (moderate), 1.0 (too tight — broke ecosystems), no speciation, per-generation classification.
**Why 1.3:** audit of 8 seeds at 15k ticks showed every single run lost species at roughly the same rate (halving across the run) with threshold 2.0 — small lineages drifting into the gene-space "territory" of larger existing species and getting absorbed. Tightening lets drifting organisms cross into "new species" territory instead. Tuning sweep 2.0 → 1.5 → 1.3 → 1.0: 1.0 preserves species count almost perfectly but over-speciates so hard that minority strategies (foragers, predators) can't find same-species mates and collapse (2/3 seeds hit full plant monoculture). 1.3 is the valley: mean final species count 15.3 vs 10.7 at baseline, one seed produced a healthy 22-species / 80-predator ecosystem.
**Accepted tradeoff:** species names collide slightly more often (the word-list naming scheme has a finite combination count), plant dominance attractor is unchanged (needs a different lever), and mate-finding gets mildly harder for drift-cases. Hysteresis still 1.3× so an organism stays in its current species up to distance 1.69 — prevents classification flip-flopping at the new tighter threshold.

### Species naming — habitat + descriptor + strategy, children inherit two of three
**Chosen:** three-word names like "Swamp Dwarf Moss" (habitat = Swamp, descriptor = Dwarf, strategy noun = Moss). Child species inherit their parent's habitat and strategy noun, only varying the descriptor.
**Alternatives:** random names, Latin binomial generation, ID numbers.
**Why:** evolutionary trees are more readable when related species have related names. Three-word structure mimics real taxonomy enough to be legible.
**Accepted tradeoff:** word-list approach means occasional collisions — two unrelated lineages might happen to share a name by virtue of similar traits and modular arithmetic on species ID.

## World & environment

### Seasons: 60-second year, sinusoidal light & food regen
**Chosen:** a full spring/summer/autumn/winter cycle every 60 real-time seconds.
**Alternatives:** longer cycles (more realistic but slow to watch), no seasons, seasonal on/off only.
**Why:** creates regular environmental pressure on a timescale where you can actually observe adaptation within one viewing session. 60 seconds is short enough that you'll see winter affect populations multiple times in a run.
**Accepted tradeoff:** not a lifetime pressure — organisms can't "adapt" within one season. Multi-generational pressure needs long-term climate shift (which is on the roadmap).

### Terrain noise ranges: elevation -1..1, moisture 0..1
**Chosen:** `generate_noise_map` normalises each noise map to 0..1. `TileMap::generate` remaps only elevation to -1..1; moisture keeps 0..1.
**Alternatives:** normalise both to -1..1 (what shipped originally), remap moisture with `(m + 1) / 2` at each consumer.
**Why:** elevation needs a signed range because water is "below zero" (DeepWater under -0.3, ShallowWater under -0.05). Nothing about moisture is signed. Every consumer assumes 0..1: the biome thresholds in `Tile::from_elevation_moisture` (Sand/Rock below 0.25, Forest above 0.6), the vegetation carrying capacity `nutrients × moisture` in `tile_dynamics_system`, the `min(1.0)` clamp in `niche_construction_system`, and the ice age's `moisture *= 0.7`. With -1..1, roughly half the land had negative moisture, so its carrying capacity was negative and vegetation decayed to zero with no regrowth; Forest was confined to the top fifth of the range; and the ice age made dry (negative) tiles wetter, since scaling a negative number toward zero raises it. On the default 512² world at seed 42, Sand went from about 65% of land to 16% and Grassland from 12% to 46% once the range was corrected.
**Accepted tradeoff:** the biome thresholds were left at 0.25 and 0.6, so Rock (dry and above elevation 0.6) is now rare to absent on some seeds; it was only 0.2% of land before. Any retuning of thresholds, `PHOTO_OUTPUT_MULTIPLIER`, or initial seeding in response to the greener world belongs to a separate tuning pass after the attractor audit is re-run. Saves regenerate terrain from the seed, so saves made before this change load with a different biome layout.

### Deep water 10× movement cost
**Chosen:** land organisms crossing deep water pay 10× energy cost per step.
**Alternatives:** soft gradient, impassable barrier, no penalty.
**Why:** creates real geographic isolation between landmasses so allopatric speciation actually happens. A soft gradient wouldn't reliably isolate populations; a hard barrier would feel like a wall.
**Accepted tradeoff:** some aquatic-adapted organisms cross freely which can feel odd — but that's the selection pressure the trait exists to respond to.

## UI & rendering

### Single tabbed right panel vs multiple always-visible panels
**Chosen:** one right-side panel with tabs (Inspect / Phylo / Graphs / Chronicle / Events / Help). You see one tab's content at a time.
**Alternatives:** multiple fixed panels (original layout, plus the bevy_egui migration initially), floating windows, dockable panels.
**Why:** the world view is the primary content — maximise its screen area. Related info is usually consulted separately (graphs for trends, chronicle for events, phylo for species). Rarely needed simultaneously. Simpler to build and tune across window sizes.
**Accepted tradeoff:** can't see graphs AND chronicle at the same time. We can add an "eject to window" button later if that becomes a real need. It hasn't.

### egui_plot for charts instead of ASCII sparklines
**Chosen:** real line charts via `egui_plot` with pan/zoom/legend.
**Alternatives:** keep ASCII sparklines, roll our own chart rendering.
**Why:** egui_plot is built for this exact use case, gives us multi-series charts, interactive zoom, tooltip values for free. Sparklines were cute but couldn't show per-cause-death breakdowns usefully.
**Accepted tradeoff:** pulls in another dependency version-locked to bevy_egui.

### egui pointer-capture check in PreUpdate
**Chosen:** `UiInputState.pointer_over_ui` is populated in PreUpdate by querying egui's `ctx.is_pointer_over_area()`.
**Alternatives:** check in the same system that runs the panel (too early — side panel not drawn yet that frame), check in PostUpdate (too late — Update systems already consumed inputs).
**Why:** egui retains layout state across frames. At the start of a frame, egui still knows where its panels were drawn *last* frame. For cursor-based gating that's accurate enough — the user moves the cursor 1 frame before they click/scroll.
**Accepted tradeoff:** 1-frame lag on pointer-over-UI detection. At 60fps that's 16ms — imperceptible.

### Bevy gizmos for trails and infection halos
**Chosen:** use `gizmos.linestrip_2d()` / `gizmos.circle_2d()` for visual overlays.
**Alternatives:** per-segment entities, per-organism child meshes, custom shader.
**Why:** gizmos batch into a single draw call automatically. For 2000 potential trails and N infected organisms, gizmos are essentially free to render. Per-entity approaches would create entity-management headaches (spawn on infect, despawn on recover, sync per tick).
**Accepted tradeoff:** gizmos are "for debug" in Bevy's docs and have simple visual options. For our case (faint lines, simple circles) that's fine.

### Photosynthesisers render at z=0.3, active organisms at z=1.0
**Chosen:** plants render behind active organisms, no outline. Actives render with outline.
**Alternatives:** all at same z, all with outlines, all without outlines.
**Why:** visual clarity. When you're watching the sim, plants are "ground cover" and actives are "the things moving around". Separating z-orders makes the screen parseable.
**Accepted tradeoff:** in a plant-heavy world the active organisms can get obscured if they walk into dense foliage — but that matches the biology.

## Architecture & performance

### Unified event bus (WorldEventRequest) for keyboard + UI
**Chosen:** both hotkeys and UI buttons emit the same `WorldEventRequest` event; one system consumes.
**Alternatives:** duplicate the effect logic for keyboard vs UI, shared resource flags instead of events.
**Why:** one code path means one place for cooldown logic, chronicle logging, and effect application. Adding a new triggering mechanism (e.g. REST API, scheduled events) just means another event source — no logic duplication.
**Accepted tradeoff:** requires slightly more plumbing (defining events, reading events) than direct function calls. Worth it for the symmetry.

### Virtual time pause instead of near-zero timestep for pausing
**Chosen:** pausing the sim calls `Time::<Virtual>::pause()`. Unpausing calls `unpause()`.
**Alternatives:** set `Time::<Fixed>` timestep to something huge so no ticks fire (what we had first — broken).
**Why:** the original approach let virtual time keep accumulating while paused, which filled the fixed timestep accumulator. On unpause, Bevy tried to "catch up" by running thousands of ticks, freezing the app. Pausing virtual time halts accumulation entirely.
**Accepted tradeoff:** none worth noting — this is just the right way to do it in Bevy. Pause and relative speed are independent fields on `Time<Virtual>`, so pausing at 16× and unpausing resumes at 16×.

### Frustum culling in the render systems, not by Bevy's built-in
**Chosen:** in `sync_organism_transforms` and `sync_food_transforms`, check each entity's position against the camera viewport and set `Visibility::Hidden` if off-screen. Margin-padded to prevent pop-in.
**Alternatives:** rely on Bevy's built-in visibility culling (doesn't apply to 2D Mesh2d entities by default), render everything (wasted GPU), draw order spatial partition.
**Why:** a 4-float-comparison per entity is trivial vs rendering cost for thousands of off-screen organisms. Big win at 2000 organisms.
**Accepted tradeoff:** a small margin (~20px scaled) is used to prevent entities flickering at the camera edge — means we draw slightly more than strictly necessary.

### Headless mode: `--speed N` virtual-time multiplier, capped by CPU
**Chosen:** `--headless` runs with `MinimalPlugins` and `--speed N` (default 10) scales `Time::<Virtual>::relative_speed`. FixedUpdate fires as fast as the CPU can sustain, bounded by Bevy's `max_delta` catchup cap.
**Same mechanism in the GUI:** the `[` / `]` speed keys write `SimSpeed::multiplier`, as does headless `--speed`, and `sim_speed_system` applies that to `Time<Virtual>` in both modes. The fixed timestep stays at 30 Hz everywhere. Until September 2026 the GUI instead rescaled the fixed timestep (`set_timestep_hz(30 × multiplier)`), which shrank the delta every virtual-time timer saw inside `FixedUpdate`: at 16× the species classifier ran every 2400 ticks instead of every 150, the 1 Hz history sampler every 480 ticks, and "sim time" in the header fell 16× behind headless for the same tick count. Measured at 4× with a scripted tour: classification passes at sim time 18s/38s/58s/1m18s before, 5s/10s/15s/20s after. `--species-threshold` was likewise only applied in the headless startup chain; it now applies in both. Both modes running the same per-tick simulation is what makes headless sweeps a valid stand-in for what is watched. Consequence for `--script` tours: `at_seconds` is virtual time, so after a `set_speed` action the remaining triggers arrive `multiplier` times sooner in wall-clock terms.
**Alternatives considered:** tiny fixed timestep (changes the semantic meaning of one tick for virtual-time timers like species classification), manual schedule invocation (breaks timers entirely), keeping it at 1× wall-clock (what v1 did).
**Why this is where we landed:** relative_speed leaves every sim timer semantically intact (classification still runs every 5 virtual-seconds, seasons still 60 virtual-seconds, etc.) — only the mapping from virtual to wall-clock changes. Measured speedup is 2.8× (50s → 17.6s for 1500 ticks on M4 Max). Beyond `--speed 5` further multiplication does nothing because the CPU is the floor at ~85 ticks/sec.
**What was wrong with the v1 assumption:** I previously claimed headless couldn't go faster than 30Hz because "the sim is CPU-bound at 30Hz." Half right. The sim IS CPU-bound at ~85 ticks/sec, but v1 was bottlenecked on Bevy's virtual-time pacing at 30Hz wall-clock — which is a configurable thing, not a compute thing. Fixing the pacing gets us to the real CPU ceiling.
**Accepted tradeoff:** speedup caps at whatever your CPU sustains per-tick, which depends heavily on organism count and system count. For further speedup we still need less per-tick compute (more Rayon, GPU compute for brains, or fewer organisms). Note also that non-determinism beyond ~50 ticks (from parallel task pool / HashMap ordering) is unchanged by `--speed` — integration tests needing bit-identical replay need single-threaded mode, which costs Rayon and runs slower.

### Spatial hash is rebuilt inside the fixed tick, not once per frame
**Chosen:** `update_spatial_hash` (defined in `clauvolution_world`) is scheduled by `SimPlugin` as the second system in the `FixedUpdate` chain, directly after `tick_counter_system`. `WorldPlugin` does not register it.
**Alternatives:** rebuild in `PreUpdate` once per frame (what shipped originally); rebuild lazily on first query per tick; maintain the hash incrementally as positions change.
**Why:** positions change once per fixed tick, and a frame can run many fixed ticks — every frame in headless at the default `--speed 10`, and any GUI frame above 1× or after a hitch. With a per-frame rebuild, every tick after the first in a frame queried neighbour cells from the start of the frame, so headless sweeps and the watched sim were not running the same physics, and every tuning number taken from headless before this change was measured with a stale hash. Rebuilding at the head of the tick is the smallest change that makes each tick self-consistent. Lazy and incremental variants would save the rebuild cost on ticks that do not need it, but the rebuild is a linear pass over positions and is not where the tick spends its time.
**Accepted tradeoff:** the hash reflects positions at the start of the tick. Systems after `action_system` (predation, disease, symbiosis, reproduction) query with post-move positions against pre-move cells; because every reader re-checks real distance after the lookup, the effect is that an entity which moved across a cell boundary this tick can be missed, never falsely matched. That one-move lag is bounded by per-tick speed and is accepted. The ordering is enforced by the chain, and a `debug_assert` in `sensing_and_brain_system` (which runs before anything moves) fails a debug build if the hash and positions ever disagree.

### Rayon parallelism via Bevy `par_iter_mut`, not rayon crate directly
**Chosen:** Used Bevy's `Query::par_iter_mut` in `sensing_and_brain_system`. Bevy's task pool wraps Rayon under the hood.
**Alternatives considered:** direct rayon (pull in as dep), manual collect → `par_iter` → apply, custom thread pool.
**Why:** Bevy's idiomatic parallel iteration is already set up, safe with Bevy's ECS borrow checker, and requires no extra deps or custom thread pools. Writes to per-organism components are conflict-free because each iteration gets its own `Mut<T>`.
**Accepted tradeoff:** we don't control the pool size; Bevy picks based on cores. Measured ~1.4 effective cores used in practice — less than the 6 available because brain eval isn't the sole per-tick bottleneck. Spatial queries in sensing and the rest of the tick (metabolism, reproduction, disease) also contribute. Further parallelisation of those would pay off; wasn't in scope for v1.

### Screenshotting egui overlays: Bevy + bevy_egui can't do it out of the box
**Context:** we wanted `S` (manual) and the `--script` tour to produce images that include the header bar, side panel, and minimap legend. With Bevy 0.15.3 and bevy_egui 0.33.0, out-of-the-box `Screenshot::primary_window()` captures the main camera's render but **not** the egui overlay on top of it. The panel appears to "vanish" in the saved image.

**Why this happens (the short version):**
- bevy_egui draws directly to `window.swap_chain_texture_view` instead of going through Bevy's `ViewTarget` abstraction. See `bevy_egui-0.33.0/src/egui_node.rs` line ~285 — the render target is hardcoded to the swap chain.
- Bevy's `Screenshot` API works by replacing the window target's `OutputColorAttachment` with a capture texture, so anything rendered via `ViewTarget` (main camera) lands in the capture. Egui, rendering straight to the swap chain, completely bypasses this redirection.
- We'd happily read back from the swap chain after egui draws, except Bevy configures the surface with `TextureUsages::RENDER_ATTACHMENT` only (see `bevy_render-0.15.3/src/view/window/mod.rs` line ~356). Without `COPY_SRC` on the swap chain texture, wgpu refuses the readback.

**Why this feels like a Bevy/bevy_egui failure:** three small integration decisions — egui bypassing `ViewTarget`, screenshot going through `ViewTarget`, swap chain not being `COPY_SRC` — combine to make "capture a frame of the app as the user sees it" impossible with the stock APIs. Each decision is individually defensible; together they box us out. Fixing any one of them upstream would resolve this.

**Chosen:** render the frame a second time into an `Image` we own. `begin_screenshot` (in `clauvolution_render::screenshot_with_egui`) creates an image with `RENDER_ATTACHMENT | COPY_SRC | TEXTURE_BINDING`, spawns a secondary `Camera2d` targeting it with the main camera's transform and projection cloned, and attaches bevy_egui's `EguiRenderToImage` to the primary window entity so the same egui context is also drawn onto that image. bevy_egui's default graph edge draws egui *before* the camera (its use case is "egui as a texture in the scene"), which would let the camera clear over the UI, so a small `ExtractSchedule` system flips the edge so egui composites after the camera. A few frames later a `Readback::texture` observer receives the pixels, converts BGRA to RGBA, strips wgpu's row padding, writes the PNG, and despawns the secondary camera. One capture is in flight at a time; the tour script waits on it.

**Alternatives considered:**
- Patch Bevy's surface configuration to add `COPY_SRC` to the swap chain and copy it out after `egui_pass`. Was the first plan; rejected because it modifies Bevy's window setup and depends on the platform surface accepting `COPY_SRC`.
- `screencapture` CLI shell-out on macOS. Shipped briefly, reverted — platform-specific, captures the entire monitor, required `osascript` to activate our window first.
- Fork bevy_egui to render via `ViewTarget`. Ongoing upstream maintenance burden for a personal project.
- Render everything (camera + egui) to an intermediate texture we own, then blit to the swap chain for display. Needs bevy_egui cooperation we don't have.
- Accept that screenshots miss egui. Would have been fine if we didn't want UI overlays in README images.

**Accepted tradeoff:** the world is rendered twice for the capture frame, and the graph-edge flip is coupled to bevy_egui's internal node labels and its `setup_new_render_to_image_nodes_system`, so a bevy_egui upgrade may need the patch refitted. The legacy `--screenshot` tour still uses Bevy's stock `Screenshot::primary_window()` and therefore produces camera-only images; `--script` is the egui-aware path.

### Incremental release builds
**Chosen:** `[profile.release] incremental = true` in Cargo.toml.
**Alternatives:** default non-incremental release (much slower rebuilds).
**Why:** during active development, rebuild times matter. After first build, single-file changes rebuild in seconds rather than minutes.
**Accepted tradeoff:** slightly larger release artifacts, potentially slightly worse runtime perf. For this project we don't care — there's no "distribution" binary.

### 10 workspace crates, one per domain
**Chosen:** sim/world/genome/brain/body/phylogeny/render/ui/core/app as separate crates.
**Alternatives:** one big crate with modules, three crates (sim / render / app), two crates (headless + app).
**Why:** enforces that sim crates can't import from render or ui. Compile parallelism when only one crate changes. Clear ownership — a bug in rendering lives in one place, a bug in genetics lives in another.
**Accepted tradeoff:** extra Cargo.toml files, slight compile-time overhead for workspace resolution, component/resource types have to live in `core` to be shared.
