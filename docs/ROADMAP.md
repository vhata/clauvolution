# Roadmap

What's next, organised by theme. Ordered by how much each serves the project's core motivation: **pure curiosity and joy in watching evolution unfold.**

## Guiding motivation

This is a personal project. The goal isn't to answer a research question, ship a product, or present publicly. Features are judged by whether they:

- **Enrich what you can see and understand while watching** → top priority
- **Enable faster iteration on tuning/balance** → indirectly valuable; well-tuned worlds produce more interesting unfoldings
- **Skip past the watching** (e.g. evolve-until) → deprioritised; the unfolding *is* the point
- **Serve sharing / distribution / external research** → deprioritised; no audience but me

If you're considering a new feature and it doesn't fit any of the first two buckets, question whether it should be built.

---

## Theme 1: Comprehension — make the invisible visible

The sim shows WHAT happens (species rising and falling) but hides WHY. These items surface the underlying causes so every moment of watching is richer.

**Top pick:** Brain activation heatmap. You already watch creatures move and compete; this lets you watch them *think*.

### Brain activation heatmap
When an organism is selected, show its neural network in real time — which inputs are currently firing, which connections are active, which outputs are being driven. Probably lives in the Inspect tab alongside the creature portrait.

**Shape:**
- Expose the brain's last-tick input and output values (already computed, just not exposed)
- Add a neural-network renderer in the Inspect tab (reuses Creature Portrait design for the brain DAG part)
- Colour nodes by activation level, connections by signal flow
- Label input nodes with what they sense ("energy", "food dir x", "group size", etc.)

### Species range heatmap
Click a species in the Phylo tab and the minimap (or a world overlay) highlights only where that species lives. Reveals niche partitioning you can't see now.

**Shape:**
- SelectedSpecies resource (already exists conceptually via SelectedOrganism → species_id)
- Minimap gets a third mode: Range (shows only this species at high contrast, everything else faded)
- Or a main-world overlay: semi-transparent coloured tiles where the species lives

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

**Top pick:** Symbiosis. After disease, it's the next piece of the richer-ecosystem puzzle.

### Symbiosis
Two organisms spending extended time in close proximity develop an "energy link". A genome trait determines whether they drain (parasite), donate (altruist), or both (mutualist). Evolution decides whether symbiotic pairs outcompete solo organisms.

**Shape:**
- Proximity tracker per organism (nearest-stable-neighbour over N ticks)
- New genome trait: `symbiosis_rate` (-1.0 drain to +1.0 donate)
- When link forms, energy transfer happens per tick based on both parties' rates
- Selection sorts out viable strategies

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
Run the sim without rendering or UI, as fast as the CPU allows. Emit a final summary (end-of-run stats, death breakdown, trait averages) and/or dump the full PopulationHistory as JSON.

**Shape:**
- New `--headless <ticks>` (or `--headless-until <condition>`) CLI flag on the existing binary
- App uses `MinimalPlugins` + `ScheduleRunnerPlugin` when headless; skips `RenderPlugin` and `UiPlugin`
- Override `Time::<Virtual>` or manually step FixedUpdate to run faster than wall-clock
- Final report: JSON dump of SimStats + final PopulationHistory snapshot + top species list
- Optional `--seed <N>` for reproducibility
- Optional `--no-session` to skip session-directory creation

**Gotchas:**
- Bevy `DefaultPlugins` pull in windowing/rendering — need MinimalPlugins
- `Time::<Fixed>` at 30Hz is wall-clock driven by default; need to step manually or use `ScheduleRunnerPlugin::run_loop(Duration::ZERO)`
- `Session::new()` creates a directory regardless — add `Session::new_ephemeral()` for headless

### Session seeds — full reproducibility
Currently `terrain_seed` is deterministic but everything else (initial placement, mutation, food spawning, reproduction) uses `rand::thread_rng()`. Two runs with the same terrain still diverge at tick 0.

**Goal:** a single `--seed N` makes the entire simulation bit-reproducible.

**Shape:**
- New `SimRng(StdRng)` resource in core, seeded from master seed at startup
- Replace every `rand::thread_rng()` / `rand::random()` in sim/genome/world/body with `sim_rng.0` accessed via `ResMut<SimRng>`
- CLI flag: `--seed <u64>` overrides the default
- Save files persist the seed for replay

**Gotchas:**
- Bevy system parallelism: two systems that mutate `SimRng` in parallel would race. All sim systems are already `.chain()`-ed — should be safe. Verify.
- User-triggered events (asteroid selection, etc.) must use `SimRng` too
- Bevy internal RNG may affect visuals but not gameplay; only care about gameplay determinism

**Nice-to-have after v1:**
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

Headless mode (Theme 4) will make this much faster once it lands.

**Current tuning state:**
- **Disease (v2 pass in progress).** First pass too mild. Second pass: radius 12→20, drain ×2, background ×3, added direct mortality. Not yet validated in a run. May need further adjustment.
- **Plant dominance attractor.** Plant density competition shipped but recent runs still go 100% plants in ~50 seconds. Density formula might need to be steeper (current: `1/(1 + others × 0.2)`; steeper: `× 0.4`). Initial seeding is 30% plants — could reduce to 20%.
- **Predation energy pyramid (10% trophic efficiency).** Thermodynamically motivated but worth sanity-checking — are predators ever viable, or does 10% make them unsustainable?
- **Quadratic body/armor/claw costs.** Prevents "stack everything" meta — but if nobody evolves big bodies or heavy armor, the cost may be too punishing.

**When adding a new dynamic:** expect the first version to be wrong. Budget a follow-up tuning pass. Instrument first (Graphs tab should surface the dynamic's effect), then tune.

## Attractor states to watch for

Stable-but-boring configurations the sim can fall into. Each should get counter-pressure so the world doesn't stay there.

- **Green world** — plants win everything. Partially addressed by plant density competition; disease is intended to finish the job.
- **Predator starvation collapse** — too many predators, prey crashes, predators starve. Natural but boring if it always happens. Energy pyramid (10%) is supposed to prevent it.
- **Minimal viable organism** — everyone converges to a tiny, cheap, photo-surface-only organism that barely moves. Watch for low body_size + low speed averages plus no predators.
- **Genetic stagnation** — species count stabilises low, traits flatline. Suggests mutation rate or structural mutation rate is too low.

Observing the sim trend toward any of these = trigger to tune.

## Code health

- Known rough edges tracked in `CLAUDE.md` under "Known rough edges"
- Big files worth splitting if they grow further: `clauvolution_sim/src/lib.rs` (~1200 lines), `clauvolution_render/src/lib.rs` (~1100), `clauvolution_ui/src/lib.rs` (~830)
- When a function in one of those crosses 100 lines, it's probably ready to move to its own module

### Known tech debt (not blocking, worth addressing when touching nearby code)

**Performance / allocation hotspots:**
- `photosynthesis_system` allocates a new `HashMap<(u32,u32), u32>` every tick (30/sec) for plant density counting. Two passes over all organisms. At 2000 organisms this is fine, but could reuse a cached resource for the count.
- Food positions are collected into a new `Vec` every tick in both `sensing_and_brain_system` and `action_system`. The spatial hash already has food — could query it directly.
- `reproduction_system` calls `genome.clone()` multiple times when setting up mate candidates. Genomes are large (neurons + connections + body segments). Could pass references via a lookup table.
- `render` systems clone mesh/material handles frequently — handles are cheap (Arc-like) but the pattern obscures that.

**Magic numbers that should be named:**
- ~~Disease tuning literals~~, ~~bloom durations~~, ~~plant density coefficient~~, ~~extinction cooldown~~, ~~species classification period~~, ~~species hysteresis factor~~ — all done (named consts at top of `clauvolution_sim/src/lib.rs`).
- Still inline: random click-radius, frustum margin, disease severity clamps, NEAT innovation/mutation thresholds in genome crate. Lower priority — not frequently tuned.
- Future: promote the tuning consts to a `SimConfig`-style resource so they can be edited live via the UI without recompile (would serve the tuning loop even more directly).

**Large multi-concern functions:**
- `sensing_and_brain_system` (~114 lines) does spatial querying, input assembly, social sensing computation, and brain evaluation in one loop. Splitting the sensing pass from the brain-eval pass would also enable Rayon parallelism (ROADMAP theme 4).
- `reproduction_system` (~114 lines) mixes mate finding, genome crossover/mutation, and child spawning. Natural split along those three concerns.

**Inconsistent patterns:**
- Event-based triggering vs direct Commands usage varies across systems. `mass_extinction_input_system` uses `WorldEventRequest` but `action_system` spawns food entities directly. Should decide: is every world mutation an event, or only user-triggered ones? Right now it's "user-triggered only", which is fine — just document.
- Some systems use explicit chaining (`.chain()`), others rely on default Bevy ordering within a tuple. Be deliberate about which.

**Name duplication:**
- `body_descriptor`, `habitat_word`, `strategy_noun` in `clauvolution_phylogeny` use the same `pick!` macro three times with only the word lists differing. Unifying saves a few lines and makes adding a fourth category trivial.

**Silent failure spots:**
- Many `let Ok(...) else { return }` patterns in `click_select_system`, `camera_control_system` — if window or camera are ever absent, the system silently does nothing. In normal flow these are always present, but a `warn!` on the else arm would make engine-state bugs easier to diagnose.
- `save_system` wraps save writes in `.expect("Failed to write save file")` — panics on disk full / permissions. For a personal tool this is fine but worth noting.

**Validation / save compatibility:**
- Loaded save files get no structural validation — a corrupted `genome.neurons` list would just produce a weird organism.
- `disease_resistance` uses `#[serde(default)]` for backward-compat with old saves. Other fields don't. When adding new genome fields, default them too, or the load will fail.

**Type complexity:**
- Several Bevy `Query` type signatures are 150+ characters. Named `type` aliases would help readability — clippy warns about this.

---

# Cool ideas to try

Small-to-medium features that aren't on the critical path but would be fun. Pick one when in the mood.

### Symbiosis starter
Lighter version of full symbiosis — energy transfer between nearby stable pairs, no genome trait yet. See whether the dynamic works at all before investing in evolvability.

### Clickable chronicle entries
Click a chronicle entry → if it's about a species, switch to Phylo tab and highlight that species. If it's about a location, focus camera there.

---

# Design docs

Larger design pieces that deserve their own document:

- [Creature Portrait — detailed inspect visualization](design/creature-portrait.md) — large, detailed rendering of selected organism with brain DAG

---

# Backlog

Items that don't directly serve the joy-of-watching motivation. Here for completeness.

## Performance scaling

Three complementary approaches, roughly in order of bang-for-buck:

### Rayon parallelization for brain evaluation
The `sensing_and_brain_system` iterates organisms sequentially, but brain evaluation is pure computation with no side effects — textbook `par_iter`. Could halve simulation cost on multi-core machines.

```rust
// Conceptual
inputs.par_iter().map(|i| brain.evaluate(i)).collect()
```

### GPU instanced rendering
Currently each organism gets its own `ColorMaterial`. True instanced rendering would pack per-instance data into a single buffer and draw all organisms in one draw call. Bitmask shader trick: each instance carries a feature bitmask, shader scales absent parts to zero — no entity churn for LOD changes.

### GPU compute for neural net batching
The big one. Pad all NEAT networks to uniform max size, flatten into GPU buffers, evaluate all brains in a single compute shader dispatch. Only worth it at 10k+ organisms.

## Accessibility

### WASM+WebGPU browser build
Run in a browser without installing anything. Only matters if you ever want to share the sim. Needs perf work first.

## Sharing

### Organism export / import
Save an interesting creature to a file. Load it into another sim as seed population. JSON export of a single organism's genome. `--seed-with creature.json` CLI flag.

---

# Aspirational

Items in the Core Vision that don't yet have a clear implementation shape.

**Brains:**
- Cognitive speciation — separated populations diverge cognitively
- Sentience spectrum — communication, deception, play

**Emergent Dynamics:**
- Climate shift (has a sketch above in Theme 2 — moved from Aspirational when the shape became clear)
