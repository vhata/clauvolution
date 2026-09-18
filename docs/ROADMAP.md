# Roadmap

What's next, organised by theme. Ordered by how much each serves the project's core motivation: **pure curiosity and joy in watching evolution unfold.**

## Guiding motivation

This is a personal project. The goal isn't to answer a research question, ship a product, or present publicly. Features are judged by whether they:

- **Enrich what you can see and understand while watching** → top priority
- **Enable faster iteration on tuning/balance** → indirectly valuable; well-tuned worlds produce more interesting unfoldings
- **Skip past the watching** (e.g. evolve-until) → deprioritised; the unfolding *is* the point
- **Serve sharing / distribution / external research** → deprioritised; no audience but me

If you're considering a new feature and it doesn't fit any of the first two buckets, question whether it should be built.

## What belongs here

This file holds work that serves the motivation above and is big enough to belong to a theme. Themes, ongoing concerns, and aspirational items all live here. Concrete deferred work that doesn't belong to a theme goes in [`TODO.md`](../TODO.md), and work promoted by a whole-codebase review goes in [`review/BACKLOG.md`](../review/BACKLOG.md). "Grab something from the roadmap" selects this file only.

---

## Theme 1: Comprehension — make the invisible visible

The sim shows WHAT happens (species rising and falling) but hides WHY. These items surface the underlying causes so every moment of watching is richer.

The first two items here have shipped; genome diff view and extinction post-mortem are the open candidates.

### Brain activation heatmap
✅ **Shipped.** The Inspect tab's Brain section draws the selected organism's NEAT network live: inputs on the left, outputs on the right, hidden neurons in between, nodes coloured by this tick's activation, edges coloured by weight sign and faded by signal strength, with every input and output labelled by what it senses or drives. Backed by the `BrainActivations` component that `sensing_and_brain_system` fills each tick.

### Species range heatmap
✅ **Shipped** as the minimap's third mode (press M to cycle Normal → Heatmap → Range). Range dims the terrain, greys every other species, and paints the selected organism's species in bright blocks. Falls back to Normal when nothing is selected. A main-world tile overlay was the alternative shape and was not built.

### Genome diff view
Pick two organisms (or two species representatives), see a side-by-side diff of their traits, body plans, and brain topology with the differences highlighted. Unpacks the "how different are these two?" question from a single number into specifics.

**Shape:**
- Comparison tab or modal in the egui right panel
- Click "compare with..." on an organism, then click a second
- Grid layout: trait | org A | org B | diff
- Body plan shown side-by-side with shared segments greyed out
- Brain topology shown with shared neurons/connections greyed, unique ones coloured

### Extinction post-mortem
When a species dies out, capture a snapshot of its last 30 seconds — population trend, causes of death, competitors, environment. Clickable chronicle entry opens the post-mortem modal.

**Shape:**
- When species classification detects extinction, grab last N population snapshots, deaths by cause, last tile occupation heatmap
- Store per-species post-mortem in PhyloTree node (small — just a few stats)
- Chronicle entries become clickable; open a modal with the post-mortem

---

## Theme 2: New dynamics — richer ecosystem

More kinds of evolution to watch unfold. Each adds a qualitatively new pressure.

**Top pick:** Long-term climate shift, now that symbiosis has shipped.

### Symbiosis
✅ **Shipped (v1).** Genome gets a `symbiosis_rate` trait in [-1.0, +1.0]. Proximity tracker looks for a mutual-nearest neighbour held for 30+ consecutive ticks within 6 world units; once locked, each party transfers `rate * 0.05` energy to its partner per tick (negative rate drains). Graphs tab shows mutual-pair count + avg evolved rate. Inspect tab labels each organism parasite/neutral/donor.

**Follow-ups / known roughness:**
- ~~Dynamic barely bites at v1 numbers.~~ First tuning pass (see DECISIONS.md) shortened the streak threshold 30→10 and tripled transfer rate 0.05→0.15. Pair count now 66–124 per 6000-tick run across three seeds. But:
- **Avg rate still near zero** — no population-level drift toward parasitism or donation after 6000 ticks. The math predicts parasitism should win; the observed data says it isn't happening on this timescale. Probably a combination of (a) most active pairs being between similar-rate organisms so no asymmetric exchange happens, (b) selection vs mutation noise too close. Worth a longer-horizon run (20k+ ticks) and/or a histogram instrumentation to see what rates linked organisms actually have.
- **Metabolic discount for linked organisms** is the most likely unlock if we decide mutualism is worth engineering into existence. Modelled on the existing social-sensing discount. Explicit "thumb on the scale" per DECISIONS.md convention, and needs the same honest framing.
- Visual feedback is text-only in Inspect; a gizmos line between linked pairs would make the dynamic visible in the main view.
- No brain input yet — organisms can't sense "am I linked?" or "what's my partner doing?" Worth adding if we want behavioural co-adaptation.

### Long-term climate shift
Very slow sinusoidal temperature drift over many seasons (e.g. a 30-minute cycle vs the 60-second year). Multi-generational selection pressure distinct from seasons.

**Shape:**
- Slow cosine drift on global temperature multiplier
- Optional: random climate events ("volcanic winter" lasting many seasons)
- Interesting in combination with seed-based terrain (deserts expand/contract, forests migrate)

### Larger body-plan mutations
Currently body parts are variations on torso+attachments. Rare macro-mutations could reshuffle the entire plan — swap torso type, rearrange attachment slots, introduce novel asymmetries. Big jumps, low frequency (~0.1% per reproduction).

**Shape:**
- In the mutation function, rare rolls for structural changes
- New BodyPlan variants: radial symmetry, stacked torsos, asymmetric plans
- Pair with Cambrian-spark event to trigger bursts

### Disease evolution (follow-ups)
V2 shipped. Potential next upgrades if the basics work well:
- Multiple strains with their own severity/duration/virulence stats, mutating over time (pathogen coevolution)
- Infection inheritance (vertical transmission) so mother→offspring infection is possible
- Species-specific resistance so diseases target lineages

---

## Theme 3: Time mastery — scrub, don't skip

Real evolution takes many generations. When watching *is* the point, skipping ahead conflicts with the motivation — but scrubbing backward to re-savour something you noticed is valuable.

### Replay / timeline scrubbing
Record a run as periodic snapshots. Scrub backward through time with a slider. Population graphs become scrubbable — click a point in history to see the world at that moment.

**Shape:**
- Periodic (every N seconds) snapshots of organism state + tilemap + phylo state
- Snapshots are lightweight (no brain eval, just state) and ring-buffered
- Timeline scrubber in the header or as a new mode
- Scrubbing pauses live sim; resume returns to live

### Interesting-moment auto-screenshot
Automatically capture screenshots on notable events: new species, mass extinctions, convergent evolution detected, population peaks/crashes, arms-race milestones.

**Shape:**
- Already have the detection logic in place (chronicle triggers)
- Add a screenshot request when those events fire
- Saved to sessions/<name>/highlights/ with descriptive filenames

### Evolve-until mode *(deprioritised)*
Run at unlimited speed until a triggering event, then pause. "Skip to the punchline" — useful in a research framing but works against the joy-of-watching motivation. Documented but not chasing.

---

## Theme 4: Tuning infrastructure — serve the watching indirectly

The recent disease-tuning session made the pain clear: eyeballing 2 minutes of sim to find out a parameter is wrong is a slow feedback loop. These items make tuning fast and measurable, which in turn makes the live sim richer.

### Headless mode
✅ **Shipped (v2).** `--headless <ticks>` CLI flag runs the sim without rendering/UI and prints a final summary. `--speed N` multiplies virtual time so FixedUpdate can fire faster than 30Hz wall-clock.

**v2 correction to the v1 framing:** I earlier claimed headless was pegged at 30Hz "because the sim is CPU-bound." Half right — the sim IS CPU-bound at ~85 ticks/sec on an M4 Max, but v1 was actually bottlenecked on Bevy's virtual-time pacing (30Hz virtual → 30Hz wall). Unchained, it runs 2.8× faster.

Measured (1500 ticks, seed 1, M4 Max, 6 compute workers):
- `--speed 1` → 50.07s (1.0×, baseline)
- `--speed 5` → 17.86s (2.8×)
- `--speed 10` → 17.62s (2.8×)
- `--speed 50` → 17.95s (2.8×)

Past `--speed 5` the CPU is the floor. To push further we'd need less per-tick compute (more Rayon, GPU compute for brains, or fewer organisms).

What headless gives us:
- Runs without a display (ssh, CI, servers)
- Scriptable (no keyboard/clicks required)
- Single summary at the end instead of live graphs
- 2.8× speedup via `--speed N` for fast validation loops

### Session seeds — full reproducibility
⚠️ **Partial (v1).** `--seed <u64>` CLI flag + SimRng resource makes food regen, mutation, disease rolls, reproduction and asteroid targeting all derive from the master seed.

What works: same seed → identical state for first ~50 ticks.
What doesn't: runs diverge after that due to Bevy's parallel task pool and archetype-based Query iteration order.
Enough for "same config, comparable outcomes" validation; not enough for exact integration-test bounds or bit-identical replay.

**For full determinism (follow-up):**
- Force single-threaded Bevy task pool (config `TaskPoolPlugin` with 1 worker) — costs parallelism but recovers determinism
- Alternatively: sort query results by Entity ID before iterating anywhere order-sensitive (species classification, etc.)
- Investigate whether HashMap iteration order (`seen_species`, etc.) contributes — swap to BTreeMap or explicit sorts
- `SimRng` state is not saved, so a loaded world re-seeds from `terrain_seed` and diverges from the original run immediately

**Nice-to-have once determinism holds:**
- `--compare-seed N --feature-a disease_on --feature-b disease_off` — runs two sims with same seed, one feature toggled. Direct A/B testing of changes.

### Parameter sweep mode (builds on headless)
Headless runs with different parameter combinations, outputs a CSV or comparison view.

**Shape:**
- `--sweep config.toml` loads a matrix of parameters, runs each, writes one row per run with summary stats
- Or `--runs N --param mutation_rate=0.1,0.3,0.5` for ad-hoc sweeps

### Integration tests
Test crate that spawns an app in headless mode, runs N ticks, asserts invariants. Examples:
- "After 1000 ticks, population > 100 AND species > 1" — catches death-spirals
- "Organisms never have negative energy" — invariant check
- "After an asteroid, 60-80% of organisms are gone" — behaviour check
- "Disease resistance increases after 2000 ticks with disease enabled" — selection check

Runs as part of `cargo test`. Takes seconds per test. Catches regressions. Depends on session seeds for deterministic assertions.

---

# Ongoing concerns

Recurring work that matters for the whole lifetime of the project. Revisit periodically and every time a new simulation dynamic is added.

## Ecosystem tuning

Every simulation dynamic has numeric parameters that need tuning. The goal is not "perfect balance" — it's that no single strategy dominates forever, multiple causes of death contribute, and selection pressure is visible in evolving traits.

**The tuning loop:**
1. Run the sim for a few minutes
2. Open Graphs tab — check death cause breakdown, strategy ratios, trait trends
3. Identify imbalances (one cause dominating, traits flatlining, etc.)
4. Adjust parameters
5. Re-run and compare

Headless mode (Theme 4) is the fast version of this loop: `cargo run --release -- --headless 15000 --seed 42 --dump-history run.csv` gives the same numbers without watching, and `--species-threshold` lets one constant be swept without a recompile.

**Current tuning state:**
- ~~**Disease (v2 pass in progress).**~~ Validated across four seeds at 15k ticks after v3 tweak (quadratic resistance protection). Disease kills 5-10% of organisms consistently, infected populations roughly halve vs linear-protection baseline, evolved resistance nudges up 1-3 percentage points. Accepted as a background pressure; not a primary selection driver (predation dominates at 55-75%). See DECISIONS.md for the tuning journey.
- **Corpse energy fountain closed (2026-09-17).** Killed photosynthesisers used to keep photosynthesising and get killed again, so plants never really lost organisms to predation and predators were fed from corpses; every tuning number below was taken in that regime. With kills final, `PHOTO_OUTPUT_MULTIPLIER` went back to 1.0 (see DECISIONS.md). Outcomes at 5000 ticks are now bistable: seed 42 holds all three strategies, seed 3 oscillates between plants and foragers, seeds 1 and 2 drift to plant monoculture with predators fading. The 8-seed audit needs re-running on this main before any of the lines below are trusted.
- ~~**Plant dominance attractor.**~~ Broken by `PHOTO_OUTPUT_MULTIPLIER = 0.5` (see DECISIONS.md), measured with the corpse fountain in place. Density competition alone didn't bite because the world is too large for plants to actually cluster. Validated across four seeds: plant share 38–79%, foragers 7–61%, predators 0.5–14%. Lesson: two independent pressures on the same strategy isn't "double the pressure" if one of them doesn't engage in the actual operating regime.
- **Starvation vs predation split.** After the attribution fix (below), both causes contribute meaningfully — starvation 1.1k–2.7k, predation 2k–9.4k across seeds 1–4 in 1500 ticks. Predation often dominates now, which is the opposite of the earlier "starvation is everything" reading. Disease still small (<5% of deaths) and old-age essentially zero.
- ~~**Predators don't actually predate.**~~ Mis-diagnosis. Instrumentation showed 5k–43k kill events per 1500-tick run — predation was always happening. The bug was in death-cause attribution: `metabolism_system` regens health by 0.005/tick, which ran between `predation_system` setting `health = 0` and `death_system` reading it, so every predated victim was re-classified as Starvation. Fixed by gating regen on `health > 0.0` — a corpse doesn't heal. See DECISIONS.md.
- **Predation energy pyramid (10% trophic efficiency).** Thermodynamically motivated but worth sanity-checking — are predators ever viable, or does 10% make them unsustainable?
- **Quadratic body/armor/claw costs.** Prevents "stack everything" meta — but if nobody evolves big bodies or heavy armor, the cost may be too punishing.

**When adding a new dynamic:** expect the first version to be wrong. Budget a follow-up tuning pass. Instrument first (Graphs tab should surface the dynamic's effect), then tune.

## Attractor states — observed, not theoretical

8-seed headless audit at 15k ticks (April 2026) surfaced which of these actually fire and how often. Below are the observed facts, not the guesses.

**Caveat added 2026-09-17:** the April audit ran with a stale spatial hash, a moisture map with no working biomes, and the corpse energy fountain (see DECISIONS.md), so its numbers describe a different sim. The plan in `plans/2026-09-17-simulation-rules-rethink.md` re-runs it. Until then the bullets below are history, not current state.

- **Green world / plant dominance** ⚠️ **still fires regularly.** 7/8 audit seeds ended >85% plants, 1/8 hit 99.8% (seed 314 — full monoculture). `PHOTO_OUTPUT_MULTIPLIER = 0.5` stopped the *extreme* case from v1 but hasn't solved the attractor. **Next levers:** lower initial plant seeding (30% → 15%), drop `PHOTO_OUTPUT_MULTIPLIER` further (0.5 → 0.35), or investigate whether forager/predator initial conditions are themselves disadvantaged.
- **Predator extinction** ⚠️ **fires 6/8 seeds.** Predators peak at 5-37 then crash to ≤7. Never establish stable populations. 10% trophic efficiency may be too strict, OR predators' problem is upstream — prey (foragers) also crashing so predators starve second-order. Probably tied to plant dominance; fixing that may ease this.
- **Species stagnation** ✅ **addressed by tightening compatibility threshold 2.0 → 1.3** (see DECISIONS.md). Mean final species count 10.7 → 15.3; one audit seed now produces a healthy 22-species / 80-predator ecosystem. Some seeds still regress to baseline pattern though — the threshold change isn't a silver bullet, just tilts the odds.
- **Minimal viable organism drift** — observed body-size decline in plant-dominated worlds (0.5 → 0.4 range). Symptom of plant dominance, not a separate attractor. Should ease when we fix the green-world pressure.
- **Starvation-dominated mortality** — theoretical but NOT happening. Audit shows only 0.5-7.7% of deaths are starvation; predation dominates at 55-75%. Food supply isn't the bottleneck; the bottleneck is that plant biomass can't convert into forager/predator biomass efficiently.

**Re-run 2026-09-18**, on commit 984aedd (the four review-backlog fixes and the corpse fountain closed, `PHOTO_OUTPUT_MULTIPLIER` 1.0), seeds 1, 2, 3, 7, 42, 99, 314 and 1000, two runs per seed, 15k ticks. Full summaries, whole-run CSVs and the method are in [`docs/audits/2026-09-18-attractor-audit/`](audits/2026-09-18-attractor-audit/README.md); repeat with `scripts/attractor_audit.sh`. Not comparable to April line by line: different physics, a different multiplier, and April's seed list was never recorded.

| seed | run 1 plants / foragers / predators | run 2 | plant share | species | body size | runs identical |
|---|---|---|---|---|---|---|
| 1 | 1977 / 1 / 22 | 1875 / 125 / 0 | 99% / 94% | 19 / 11 | 0.60 / 0.59 | no |
| 2 | 1998 / 2 / 0 | 1996 / 4 / 0 | 100% / 100% | 12 / 15 | 0.52 / 0.50 | no |
| 3 | 1521 / 476 / 3 | 1438 / 558 / 4 | 76% / 72% | 16 / 15 | 0.57 / 0.57 | no |
| 7 | 1862 / 138 / 0 | 1862 / 138 / 0 | 93% / 93% | 17 / 17 | 0.54 / 0.54 | yes |
| 42 | 1357 / 624 / 19 | 1822 / 177 / 1 | 68% / 91% | 21 / 29 | 0.57 / 0.52 | no |
| 99 | 1939 / 61 / 0 | 1939 / 61 / 0 | 97% / 97% | 16 / 16 | 0.47 / 0.47 | yes |
| 314 | 1668 / 329 / 3 | 1668 / 329 / 3 | 83% / 83% | 22 / 22 | 0.56 / 0.56 | yes |
| 1000 | 1767 / 232 / 1 | 1767 / 232 / 1 | 88% / 88% | 12 / 12 | 0.53 / 0.53 | yes |

- **Green world / plant dominance still fires.** 11 of 16 runs ended above 85% plants and every run ended at or above 68%. Seeds 2 and 99 are effectively monocultures on both runs.
- **Predator extinction is worse than April, and no longer an artefact.** Predators peak at roughly 100 to 480 within the first 3 to 5 sim-seconds (the opening burst on the 400-organism seed population), then collapse. 10 of 16 runs ended with at most one predator and 14 of 16 with at most four. Only seed 1 run 1 (22) and seed 42 run 1 (19) still held a population at 15k ticks.
- **Species count** ended between 11 and 29, mean 17.0, in the range April recorded after the threshold change.
- **Body size** ended between 0.47 and 0.60 on every run. The minimal-viable drift persists.
- **Death causes**, as shares of total deaths across the 16 runs: predation 40% to 75%, starvation 9% to 29%, disease 11% to 18%, old age 1% to 18%. Old age had always read zero before the `Killed` marker fixed its attribution.
- **Lock-in is early.** Where plants cross 80% of the population they do so between 42 and 372 sim-seconds (ticks 1260 to 11160); seed 3 on both runs and seed 42 on run 1 never cross it. The 5000-tick view in the corpse-fountain branch was too short to see the plant creep finish.
- **Same-seed runs are reproducible on some seeds and not others.** Seeds 7, 99, 314, 1000 produced bit-identical summaries on their two runs; seeds 1, 2, 3, 42 diverged, seed 42 from 68% to 91% plants. The determinism probe, two simultaneous runs of seed 42, came out bit-identical to each other at 88% plants and 6 predators, a third distinct outcome for the seed after 68% and 91%. Across the whole audit, runs that started at the same moment matched and runs that started at different moments did not: the four seeds that diverged are the four whose first run was in the first batch after launch. Tracked as `determinism-claim-recheck` in `TODO.md`.

**Direction:** the response to this audit is [`docs/design/simulation-rules.md`](design/simulation-rules.md). Each of its phases ends with the audit re-run and a new dated block here.

**Observational trigger:** when a running sim trends toward any of these, it's time to tune. The Graphs tab has current-state readouts for plant/forager/predator ratios and death cause breakdown to make this visible.

## Code health

- Big files worth splitting if they grow further: `clauvolution_sim/src/lib.rs` (~1500 lines), `clauvolution_ui/src/lib.rs` (~1450), `clauvolution_render/src/lib.rs` (~1350). Counts are refreshed by each code review in `review/`.
- When a function in one of those crosses 100 lines, it's probably ready to move to its own module
- Two settled calls worth not re-litigating: cosmetic overlay systems (minimap viewport rect, trails, infection halos) silently skip on a missing camera, and clippy's `type_complexity` and `too_many_arguments` lints are silenced crate-wide in the three Bevy-heavy crates because aliasing the `Query` signatures individually didn't improve readability

Concrete tech-debt items live in `TODO.md`; review-derived work lives in `review/BACKLOG.md`.

---

# Design docs

Larger design pieces that deserve their own document:

- [Simulation rules: direction and design](design/simulation-rules.md) — the approved direction for the sim's rules after the 2026-09-18 audit: energy ledger, emergent carrying capacity, per-biome seeding, trait-led speciation, a diet axis, and biomes as pressure and barrier, in phases
- [Creature Portrait — detailed inspect visualization](design/creature-portrait.md) — large, detailed rendering of selected organism with brain DAG (v1 shipped; v2 polish is tracked in `TODO.md` as `creature-portrait-v2-polish`)

---

# Aspirational

Items in the Core Vision that don't yet have a clear implementation shape.

**Brains:**
- Cognitive speciation — separated populations diverge cognitively
- Sentience spectrum — communication, deception, play

**Emergent Dynamics:**
- Climate shift (has a sketch above in Theme 2 — moved from Aspirational when the shape became clear)
