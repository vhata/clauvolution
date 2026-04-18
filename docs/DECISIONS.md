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
**Chosen:** a global `PHOTO_OUTPUT_MULTIPLIER = 0.5` multiplier on the photosynthesis energy equation.
**Alternatives:** steeper density penalty (see above — doesn't help), more aggressive metabolism costs for photosynthesisers (biased against a strategy rather than rebalancing), reducing sunlight (same effect, worse name).
**Why:** density competition on its own wasn't enough because plants don't cluster densely in a large world. The blunt lever is lowering the raw photosynthesis yield so plant energy intake becomes comparable to what foraging yields — at which point foragers can actually compete. Tuning journey: 2.0 (original) → 1.0 → 0.7 all produced 90%+ plant monocultures. 0.5 consistently produces diverse ecologies across four tested seeds (plant share ranges 38%–79%, foragers 7%–61%, predators 0.5%–14%).
**Accepted tradeoff:** at first read the death breakdown looked starvation-dominant (~85% of deaths). That turned out to be a misattribution bug in `metabolism_system` (see "Health regen gate" below) — once fixed, the split is more like ~30% starvation, ~50–80% predation, rest disease. **Revised framing: predation is the primary selection pressure, not starvation.** In a live run after the fix, one second showed Predation 84% / Disease 8% / Starvation 7% / Old age 1%. Point stands: foragers earn their place because photosynthesis is no longer a free lunch — they just earn it by being predated less than alternatives, not by out-eating the shortfall. Also notable: only ~7 organisms are genome-classified `SpeciesStrategy::Predator` (`claw_power > 0.5`), yet hundreds of kills per second happen — most predation is done by foragers with modest claws. The strategy label is decorative for display, not load-bearing for dynamics.

### Health regen gate — "a corpse doesn't heal"
**Chosen:** `metabolism_system`'s health regen step (`health += 0.005`) is gated on `health > 0.0`. Organisms whose health has been set to zero don't regen.
**Alternatives:** reorder systems so `death_system` runs before `metabolism_system`, make `predation_system` despawn victims directly, introduce a `Dying(cause)` marker component, use a large-negative sentinel for health that regen can't overcome.
**Why:** when the photosynthesis multiplier tuning shipped, death-cause attribution looked broken — 0 predation deaths across every seed despite thousands of attack events. Root cause: `predation_system` sets victim `health = 0` and `energy = 0`, then `metabolism_system` runs in the same tick and regens health to ~0.005 (because the `min(1.0)` clamp didn't have a corresponding `max(0.0)` floor for the dead), then `death_system` reads the victim as energy-dead but health-alive and classifies them as Starvation. Gating the regen on `health > 0.0` is the minimal invariant-preserving fix and reads naturally — a fatally wounded organism shouldn't regenerate between ticks. It also keeps the `death_system` cause-priority chain simple (health ≤ 0 → Predation) without introducing new components or ordering constraints.
**Accepted tradeoff:** a one-liner behavioural rule buried in `metabolism_system` that a future reader might not connect to attribution correctness. Mitigated by a comment at the gate explaining the predation-attribution coupling.

### Disease: direct mortality + energy drain, not drain alone
**Chosen:** infected organisms suffer both a per-tick energy drain AND a small per-tick chance of direct death (scaled by severity × (1-resistance)).
**Alternatives:** drain only (first implementation), direct mortality only.
**Why:** drain-only failed on photosynthesisers. Plants refill energy from sunlight faster than disease drained it, so they were immune in practice. Adding direct mortality that ignores energy reserves ensures disease can't be "sun-bathed" through.
**Accepted tradeoff:** slightly less intuitive than a pure energy model. The direct-mortality path only zeroes `energy` (not `health`) so `death_system` correctly attributes the death to Disease rather than Predation.

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

### Species classification threshold 2.0 with 1.3× hysteresis
**Chosen:** organisms classified by NEAT compatibility distance, threshold 2.0 to join a species, 2.6 (1.3×) to stay in it. Runs every 5 seconds.
**Alternatives:** strict threshold (lots of flip-flopping), no speciation (one species forever), per-generation classification.
**Why:** species should be stable enough to track over time but responsive enough that genuine divergence produces a new species. The 1.3× hysteresis prevents classification flip-flopping.
**Accepted tradeoff:** species names can feel slightly "sticky" — a lineage that drifts gradually won't speciate as often as it might in a stricter system.

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
**Accepted tradeoff:** none worth noting — this is just the right way to do it in Bevy.

### Frustum culling in the render systems, not by Bevy's built-in
**Chosen:** in `sync_organism_transforms` and `sync_food_transforms`, check each entity's position against the camera viewport and set `Visibility::Hidden` if off-screen. Margin-padded to prevent pop-in.
**Alternatives:** rely on Bevy's built-in visibility culling (doesn't apply to 2D Mesh2d entities by default), render everything (wasted GPU), draw order spatial partition.
**Why:** a 4-float-comparison per entity is trivial vs rendering cost for thousands of off-screen organisms. Big win at 2000 organisms.
**Accepted tradeoff:** a small margin (~20px scaled) is used to prevent entities flickering at the camera edge — means we draw slightly more than strictly necessary.

### Headless mode: `--speed N` virtual-time multiplier, capped by CPU
**Chosen:** `--headless` runs with `MinimalPlugins` and `--speed N` (default 10) scales `Time::<Virtual>::relative_speed`. FixedUpdate fires as fast as the CPU can sustain, bounded by Bevy's `max_delta` catchup cap.
**Alternatives considered:** tiny fixed timestep (changes the semantic meaning of one tick for virtual-time timers like species classification), manual schedule invocation (breaks timers entirely), keeping it at 1× wall-clock (what v1 did).
**Why this is where we landed:** relative_speed leaves every sim timer semantically intact (classification still runs every 5 virtual-seconds, seasons still 60 virtual-seconds, etc.) — only the mapping from virtual to wall-clock changes. Measured speedup is 2.8× (50s → 17.6s for 1500 ticks on M4 Max). Beyond `--speed 5` further multiplication does nothing because the CPU is the floor at ~85 ticks/sec.
**What was wrong with the v1 assumption:** I previously claimed headless couldn't go faster than 30Hz because "the sim is CPU-bound at 30Hz." Half right. The sim IS CPU-bound at ~85 ticks/sec, but v1 was bottlenecked on Bevy's virtual-time pacing at 30Hz wall-clock — which is a configurable thing, not a compute thing. Fixing the pacing gets us to the real CPU ceiling.
**Accepted tradeoff:** speedup caps at whatever your CPU sustains per-tick, which depends heavily on organism count and system count. For further speedup we still need less per-tick compute (more Rayon, GPU compute for brains, or fewer organisms). Note also that non-determinism beyond ~50 ticks (from parallel task pool / HashMap ordering) is unchanged by `--speed` — integration tests needing bit-identical replay need single-threaded mode, which costs Rayon and runs slower.

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

**Chosen:** patch the surface configuration to add `COPY_SRC`, then add a custom render-graph node that runs *after* `egui_pass`, copies the swap chain texture into a buffer, and saves it as PNG. Bevy-native, keeps egui's direct-to-swapchain rendering intact.

**Alternatives considered:**
- `screencapture` CLI shell-out on macOS. Shipped briefly, reverted — platform-specific, captures the entire monitor, required `osascript` to activate our window first.
- Fork bevy_egui to render via `ViewTarget`. Ongoing upstream maintenance burden for a personal project.
- Use `EguiRenderToImage` on a secondary entity to render egui to an image, composite with the main camera's output. Requires duplicating every UI-draw system across two contexts or somehow sharing paint jobs — invasive.
- Render everything (camera + egui) to an intermediate texture we own, then blit to the swap chain for display. Needs bevy_egui cooperation we don't have.
- Accept that screenshots miss egui. Would have been fine if we didn't want UI overlays in README images.

**Accepted tradeoff:** we depend on `TextureUsages::COPY_SRC` being supported on the platform's swap chain surface (Metal on macOS definitely does; DX12 and Vulkan typically do; WebGPU has some restrictions). If a target platform later rejects `COPY_SRC` on the surface, the screenshot path breaks and we fall back to Bevy's egui-less capture. Also, modifying Bevy's surface setup couples us to internal Bevy details — any Bevy upgrade may require re-fitting the patch.

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
