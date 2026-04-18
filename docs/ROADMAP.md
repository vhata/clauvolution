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

**Top pick:** Long-term climate shift, now that symbiosis has shipped.

### Symbiosis
✅ **Shipped (v1).** Genome gets a `symbiosis_rate` trait in [-1.0, +1.0]. Proximity tracker looks for a mutual-nearest neighbour held for 30+ consecutive ticks within 6 world units; once locked, each party transfers `rate * 0.05` energy to its partner per tick (negative rate drains). Graphs tab shows mutual-pair count + avg evolved rate. Inspect tab labels each organism parasite/neutral/donor.

**Follow-ups / known roughness:**
- Dynamic barely bites at v1 numbers. 3000-tick headless runs show only ~26 mutual pairs across ~2000 orgs and the rate doesn't drift much — budget a tuning pass. Candidates: relax the 30-tick streak, widen the 6-unit contact range, bump the 0.05 transfer rate, or give linked pairs a metabolic discount so mutualists have a reason to stay.
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
- ~~**Plant dominance attractor.**~~ Broken by `PHOTO_OUTPUT_MULTIPLIER = 0.5` (see DECISIONS.md). Density competition alone didn't bite because the world is too large for plants to actually cluster. Validated across four seeds: plant share 38–79%, foragers 7–61%, predators 0.5–14%. Lesson: two independent pressures on the same strategy isn't "double the pressure" if one of them doesn't engage in the actual operating regime.
- **Starvation vs predation split.** After the attribution fix (below), both causes contribute meaningfully — starvation 1.1k–2.7k, predation 2k–9.4k across seeds 1–4 in 1500 ticks. Predation often dominates now, which is the opposite of the earlier "starvation is everything" reading. Disease still small (<5% of deaths) and old-age essentially zero.
- ~~**Predators don't actually predate.**~~ Mis-diagnosis. Instrumentation showed 5k–43k kill events per 1500-tick run — predation was always happening. The bug was in death-cause attribution: `metabolism_system` regens health by 0.005/tick, which ran between `predation_system` setting `health = 0` and `death_system` reading it, so every predated victim was re-classified as Starvation. Fixed by gating regen on `health > 0.0` — a corpse doesn't heal. See DECISIONS.md.
- **Predation energy pyramid (10% trophic efficiency).** Thermodynamically motivated but worth sanity-checking — are predators ever viable, or does 10% make them unsustainable?
- **Quadratic body/armor/claw costs.** Prevents "stack everything" meta — but if nobody evolves big bodies or heavy armor, the cost may be too punishing.

**When adding a new dynamic:** expect the first version to be wrong. Budget a follow-up tuning pass. Instrument first (Graphs tab should surface the dynamic's effect), then tune.

## Attractor states to watch for

Stable-but-boring configurations the sim can fall into. Each should get counter-pressure so the world doesn't stay there.

- **Green world** — plants win everything. Addressed by `PHOTO_OUTPUT_MULTIPLIER = 0.5` (see DECISIONS.md); density competition alone didn't bite.
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
- ~~Food positions collected into a new Vec twice per tick~~ — now unified behind a single `FoodSnapshot` resource built once per tick.
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
- ~~`body_descriptor`, `habitat_word`, `strategy_noun` `pick!` triplet~~ — done, unified to a shared `pick()` function.

**Silent failure spots:**
- ~~`click_select_system` and `camera_control_system` window/camera silent returns~~ — now use `warn_once!` so engine-state bugs leave a single log breadcrumb.
- Remaining overlay systems (minimap viewport rect, trails, infection halos) still silently skip on missing camera — kept silent because they're cosmetic and can't confuse input behaviour.
- `save_system` wraps save writes in `.expect("Failed to write save file")` — panics on disk full / permissions. For a personal tool this is fine but worth noting.

**Validation / save compatibility:**
- ~~Loaded save files get no structural validation~~ — basic validation now runs on load (genome shape checks, non-finite position clamping, broken organisms skipped with a warn).
- `disease_resistance` uses `#[serde(default)]` for backward-compat with old saves. Other fields don't. When adding new genome fields, default them too, or the load will fail.

**Type complexity:**
- ~~Bevy `Query` type signatures flagged by clippy~~ — silenced crate-wide with `#![allow(clippy::type_complexity, clippy::too_many_arguments)]` in the three Bevy-heavy crates. Aliasing individually didn't improve readability; Bevy idiom accepts these.

---

# Cool ideas to try

Small-to-medium features that aren't on the critical path but would be fun. Pick one when in the mood.

### Symbiosis starter
Lighter version of full symbiosis — energy transfer between nearby stable pairs, no genome trait yet. See whether the dynamic works at all before investing in evolvability.

### Clickable chronicle entries
Click a chronicle entry → if it's about a species, switch to Phylo tab and highlight that species. If it's about a location, focus camera there.

### Prettier creature portrait (v2)
V1 shipped — literal geometry per segment type (ellipses, triangles, lines). Reads the anatomy but looks rough. Follow-up polish:
- Curved / jointed limbs instead of single line segments
- Layered fin art with veins or gradients
- Shaded torso (subtle radial gradient, not flat fill)
- Idle breathing animation (slight torso scale oscillation synced to Age)
- Armor plates that stack/segment visibly for multiple ArmorPlate genes
- Proper bilateral-pair alignment so mirrored parts line up along a centre axis rather than jittering on attachment_slot offsets
- See [design doc](design/creature-portrait.md) for the full vision (metaballs, L-systems, etc. are still future / optional)

---

# Design docs

Larger design pieces that deserve their own document:

- [Creature Portrait — detailed inspect visualization](design/creature-portrait.md) — large, detailed rendering of selected organism with brain DAG (v1 shipped — see "Prettier creature portrait" above for v2 follow-ups)

---

# Backlog

Items that don't directly serve the joy-of-watching motivation. Here for completeness.

## Performance scaling

Three complementary approaches, roughly in order of bang-for-buck:

### Rayon parallelization for brain evaluation
⚠️ **Partial (v2).** `sensing_and_brain_system`, `metabolism_system`, and the photosynthesis second pass all use `Query::par_iter_mut` now. Compute pool capped at 6 workers by default (overridable via `CLAU_WORKERS` env var) — leaves cores free for other OS tasks so the sim doesn't peg the laptop.

**Caveat on benchmarking:** headless runs at a 30Hz fixed timestep paced by virtual time, so 1500 ticks always take ~50s regardless of per-tick cost. Real speedup shows at fast sim speeds (`]` key, up to 16x) where each real-time second has to accommodate more ticks — that's where the extra Rayon headroom matters.

**Follow-ups for further speedup:**
- Parallelise remaining O(n) systems: predation, disease effects, niche construction (the last two currently share `SimRng` / `Commands`, need refactoring)
- Cache or batch spatial hash queries (currently ~2000 radius queries per tick)
- GPU compute shader for brain eval (below) for big wins at 10k+ organisms

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
