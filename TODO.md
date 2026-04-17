# Clauvolution

An evolution simulator where you watch life emerge, adapt, compete, and speciate in real time.

## What's Done

- [x] Rust + Bevy ECS workspace
- [x] Per-organism NEAT neural networks with 20 inputs, 9 outputs, recurrent memory
- [x] Organisms sense food, neighbors, terrain, species, health, signals — brains decide everything
- [x] Procedural terrain — oceans, deserts, grasslands, forests, rock
- [x] Biome-aware food spawning + terrain-dependent movement costs
- [x] Photosynthesis — organisms with photo surfaces gain energy from light
- [x] Body segments — torso, limbs, fins, eyes, mouth, photo surfaces, claws, armor plates
- [x] Predation — attack and eat other organisms (claws vs armor, size advantage)
- [x] Energy pyramid — 10% trophic efficiency (thermodynamics, not tuning)
- [x] Sexual reproduction with genome crossover
- [x] Niche construction — organisms terraform tiles
- [x] Mass extinction events — asteroid (X), ice age (I), volcanic eruption (V)
- [x] Brain memory — 3 recurrent slots
- [x] Organism aging and natural death
- [x] Generation tracking
- [x] Species classification with distinct colours + hysteresis for stability
- [x] Phylogenetic tree with ancestry tracking and lineage grouping
- [x] World chronicle — automatic event log (speciation, extinction, convergence, seasons)
- [x] Convergent evolution detection (summarised, not per-pair spam)
- [x] Population sparkline graphs with per-strategy breakdown + fitness tracking
- [x] Strategy breakdown (Plants / Predators / Foragers counts)
- [x] Click-to-inspect with full stat panel
- [x] Help overlay (H key) explaining everything for newcomers
- [x] Action flash — organisms pulse when eating/attacking/reproducing
- [x] Initial diversity seeding — 30% photosynthesizers at start
- [x] Chemical signalling between organisms (brain input + output)
- [x] Seasonal cycles — 60-second year, affects light and food production
- [x] Geographic barriers — oceans/mountains isolate populations for speciation
- [x] Fitness tracking — average lifespan sparkline
- [x] Dynamic LOD — body parts re-render when zoom changes
- [x] Species stability — higher threshold, slower reclassification, hysteresis
- [x] Save/load — F5 saves to session directory, --load restores
- [x] Named sessions — cosmic three-word names, logs + screenshots per session
- [x] Manual screenshots (S key) saved to session directory
- [x] JetBrains Mono font for proper Unicode rendering
- [x] Photosynthesizers render as ground cover (behind active organisms)
- [x] Pause/play, speed control, smooth camera
- [x] Screenshot verification mode (--screenshot)
- [x] Seed-based terrain generation (same seed = same map, saved in save files)
- [x] Species naming — three-word trait-based names with taxonomy-like inheritance
- [x] Minimap with click-to-navigate — world overview showing terrain, organisms, camera viewport
- [x] Population heatmap toggle — minimap density view coloured by strategy (M key)
- [x] Social sensing — group size + average group signal brain inputs, group metabolic discount (~5%)
- [x] Performance — frustum culling, food hidden at far zoom, virtual time cap (100ms) prevents death spiral
- [x] UI panel backgrounds — semi-transparent dark backing on all text panels for readability
- [x] Death markers — red flash for predation kills, amber for starvation/old age
- [x] Parent species tracking — inspect panel shows organism's lineage
- [x] Bloom events — solar bloom (B), nutrient rain (N), Cambrian spark (J)
- [x] Plant density competition — photosynthesizers shade each other, caps monoculture dominance

## What's Next (prioritised)

### 1. Proper UI panels (bevy_egui)

**Motivation:** Current text panels are fixed-size, can't scroll, clip at edges, and overlap at certain window sizes. No way to click a species to focus on it, filter the chronicle, or expand/collapse tree nodes. The inspect panel has to manually dodge the minimap. All of this goes away with a real UI layer.

**Library choice:** `bevy_egui` — standard integration of the `egui` immediate-mode library with Bevy. Verify version compatibility with Bevy 0.15 before starting (historically `bevy_egui` lags Bevy releases; may need a specific pinned version).

#### Layout strategy

Start with **fixed side panels** (`egui::SidePanel`), not dockable or floating. Reasons: preserves the current mental model, predictable layout, less setup. Dockable (`egui_dock` crate) can come later if users want rearrangement.

Proposed layout:
```
┌──────────────────────────────┬─────────────┐
│                              │  Minimap    │ ← stays as Bevy UI ImageNode
│                              │  (160x160)  │
│         World view           │─────────────│
│         (Bevy 2D)            │  Right tab  │
│                              │  panel:     │
│                              │  • Inspect  │
│                              │  • Phylo    │
│                              │  • Events   │
├──────────────────────────────┴─────────────┤
│  Bottom panel (collapsible):               │
│  • Stats  • Graphs  • Chronicle            │
└────────────────────────────────────────────┘
```

Keyboard shortcuts (B, N, J, X, I, V, etc.) still work, but the right panel also gets buttons for discoverability.

#### Migration phases

Incremental — one panel at a time. Old text UI stays working until each panel is replaced, then we delete the old system.

**Phase 1: Setup (~30 min)**
- Add `bevy_egui = "0.31"` (or whatever matches Bevy 0.15) to workspace deps
- Add `EguiPlugin` to the app
- Confirm egui renders over the Bevy world with no conflicts
- Add a keyboard-input gate: hotkeys should NOT fire when egui has keyboard focus (`EguiContexts::ctx_mut().wants_keyboard_input()`)

**Phase 2: Stats panel (easy warmup)**
- Simplest panel, pure text readout
- Egui `SidePanel::top` or a corner window
- Proves the integration works end-to-end
- Delete old `StatsText` entity and `update_stats_text` system

**Phase 3: Help overlay**
- Static text in a modal window, toggled by H
- Use `egui::Window::new("Help").open(&mut help_visible)`
- Egui handles the close button automatically
- Delete old help overlay

**Phase 4: Chronicle**
- Scrollable list of events — this is where egui shines
- `egui::ScrollArea::vertical()` + iterate entries
- Add filter checkboxes: hide season changes, hide extinctions, etc.
- Delete old `ChronicleText`

**Phase 5: Population graphs**
- Switch from ASCII sparklines to `egui_plot` — proper line charts
- Multiple series overlaid: organisms, plants, predators, foragers, food
- Zoom/pan built in
- Legend with toggleable series
- Delete old `GraphText` and sparkline code

**Phase 6: Phylogenetic tree**
- Recursive tree widget using `egui::CollapsingHeader` per species
- Click a species → set `SelectedSpecies` resource, highlight on minimap, focus camera on a random member
- Show expanded stats per species (peak pop, age, child count, traits)
- Delete old `PhyloText`

**Phase 7: Inspect panel**
- Tab in the right panel
- Same stats as now but with proper layout (grid/table, not format-string alignment)
- Click parent species name → jump to it in phylo tree
- Eventually: embed the creature portrait here

**Phase 8: Events panel (new)**
- Buttons for all extinction/bloom events
- Cooldown timer shown visibly
- Save/Load buttons (currently F5 only)
- Takes pressure off users having to remember keybindings

#### Risks and open questions

- **Keyboard focus**: Every hotkey needs to check `!ctx.wants_keyboard_input()`. Easy to forget one.
- **Performance**: Egui is immediate-mode — the whole UI rebuilds every frame. For our UI (~6 panels, no huge tables), this is fine. If the phylo tree grows to thousands of nodes, need to cap display or virtualize.
- **Minimap integration**: Easiest to leave as a Bevy UI node overlay — egui panels dock around it. If we want to move it inside an egui panel, we'd convert the minimap Image to a `TextureId` and render via `egui::Image`. Not a blocker.
- **Save/Load**: Currently F5 only. Adding buttons is nice but watch for accidental clicks — confirm dialog for save overwrite?
- **Bevy 0.15 version lock**: If bevy_egui doesn't have a 0.15-compatible release, we'd either wait, fork, or bump Bevy (which has its own risks).
- **Settings panel (stretch)**: Live sliders for mutation rate, metabolism cost, etc. would be amazing for tuning but risk destabilising the sim mid-run. Gate behind a "dev mode" checkbox.

#### Not in scope for this phase

- Dockable/floating panel rearrangement (future — use `egui_dock` if wanted)
- Mobile/touch UI
- Theming beyond egui's defaults
- The creature portrait itself (that's a separate item that *integrates with* the egui inspect panel)

### 2. Symbiosis
Mutualism, parasitism, commensalism. Two organisms evolving to depend on each other. Research-level — may need new mechanics.

### 3. Performance Scaling

Three complementary approaches, roughly in order of bang-for-buck:

#### Rayon parallelization for brain evaluation
The `sensing_and_brain_system` iterates organisms sequentially, but brain evaluation is pure computation with no side effects — textbook `par_iter`. Could halve simulation cost on multi-core machines. Bevy already parallelizes independent *systems*, but this would parallelize *within* the most expensive system.

```rust
// Conceptual — collect inputs first, evaluate brains in parallel, write outputs back
inputs.par_iter().map(|i| brain.evaluate(i)).collect()
```

#### GPU instanced rendering
Currently each organism gets its own `ColorMaterial`. True instanced rendering would pack per-instance data (position, scale, colour) into a single buffer and draw all organisms in one draw call. The bitmask shader trick can handle body parts: each instance carries a feature bitmask, the shader scales absent parts to zero — no entity churn for LOD changes.

#### GPU compute for neural net batching
The big one. Pad all NEAT networks to a uniform max size, flatten into GPU buffers, evaluate all 2000+ brains in a single compute shader dispatch. Requires wgpu compute pipeline. Only worth it at 10k+ organisms — the other two approaches should come first.

### 4. WASM+WebGPU browser build
Accessibility — run in a browser without installing anything.

---

## Cool Ideas to Try

Small-to-medium features that aren't on the critical path but would be fun. Pick one when you're in the mood.

### Organism trails
Faint fading lines showing where organisms have been recently. Makes migration patterns, territorial behaviour, and foraging routes visible.

### Minimap legend
Small colour key on the heatmap so new viewers don't have to guess what green/red/white means.

### Symbiosis starter
Organisms near each other for extended periods develop energy transfer. Parasites drain, mutualists share. Just the foundation — evolution decides the rest.

---

## Creature Portrait — Detailed Inspect Visualization

A dedicated rendering area in the inspect panel that shows a large, detailed, visually pleasing portrait of the selected organism — its body and its brain. The map sprites stay simple (circles/blobs); this is only rendered when you click to inspect.

### Body Visualization: Modular Sprite Stacking

Build on the existing `BodyPlan` system but render at much larger scale with better art.

**Approach:**
- Library of body part shapes (SVG-style or procedural meshes), each with anchor points for attachment
- Parts snap onto predefined coordinates around the torso, respecting bilateral symmetry
- Proper z-ordering: armor plates behind torso, limbs to sides, eyes and mouth in front, claws at edges
- Only 10-20 distinct part shapes needed to create thousands of unique combinations

**Part representations:**
- **Torso** — smooth organic ellipse, size reflects genome body_size
- **PhotoSurface** — leaf-like structures growing from the back, subtle green glow/particle effect
- **Claws/Fangs** — sharp triangles at the mouth area, size reflects attack_power
- **Fins** — semi-transparent veined shapes on the sides
- **Eyes** — prominent circles with pupils, count reflects eye_count
- **Armor** — layered plate shapes behind the body, opacity reflects armor_value
- **Limbs** — jointed segments radiating from torso
- **Mouth** — opening on the front, size reflects foraging ability

### Trait-Based Color Coding

Colors communicate biology at a glance:
- **Energy source**: green tint = photosynthetic, red/dark tint = predator
- **Environment**: blue/sleek = aquatic adapted, tan/rough = land/desert adapted
- **Health**: brightness reflects current energy level
- **Species**: hue from species colour, so related organisms look related

### Brain Visualization: NEAT Topology Graph

Render the actual neural network as a node graph below or beside the body portrait.

- **Nodes as circles**: inputs on the left, outputs on the right, hidden neurons in the middle
- **Connections as lines**: thickness proportional to weight, colour = positive (blue) vs negative (red)
- **Disabled connections**: shown as faint dotted lines
- **Labels**: input nodes labelled with what they sense (food dir, energy, group size, etc.), output nodes with what they do (move, eat, attack, etc.)
- Shows brain complexity at a glance — a simple forager has few connections, a sophisticated predator has a dense web

### Procedural Generation Options (Future)

For even more organic-looking creatures:
- **Metaballs**: 2D organic blobs that merge into each other. Adding a "tail" means adding a new metaball at the rear. Looks like a living, squishy organism. Requires a custom shader.
- **L-Systems**: Branching fractal structures — particularly good for photosynthesizers. Genome parameters control branching angle, depth, and leaf density.

### Visual Polish

- Consistent line weight (2px) and limited colour palette for coherent aesthetic
- Simple "squash and stretch" animation — breathing/pulsing idle animation
- Winged creatures get slight bobbing motion
- Photosynthesizers sway gently
- Scale the portrait to fit the inspect panel regardless of organism body_size

### Implementation Notes

- Render as a separate Bevy camera/layer or an egui canvas (pairs well with the bevy_egui UI overhaul)
- Only rendered for the single selected organism — no performance concern
- Portrait updates live as the organism moves, eats, takes damage, etc.
- Could eventually support "compare two organisms" side-by-side

---

## Core Vision

- [x] **Visual simulation** — watch evolution happen in real time
- [x] **Emergent speciation** — species diverge from common ancestors
- [x] **Competition** — creatures compete for resources, territory, and each other
- [x] **No hard categories** — strategies emerge from evolution
- [x] **Biomes** — different environments with different selection pressures
- [x] **Niche construction** — species reshape biomes, biomes reshape species back
- [x] **Phenotype rendering** — creatures visually express evolved traits
- [x] **Spatial and temporal zoom** — pan/zoom + pause/speed controls

## Brains

- [x] **Per-organism neural network** — NEAT brains, inherited with mutation
- [x] **Emergent behaviour** — movement, feeding, attack, reproduction strategies emerge
- [x] **Recurrent memory** — 3 memory slots for learning-like behaviour
- [x] **Chemical signalling** — organisms emit and sense signals, evolution decides meaning
- [ ] **Cognitive speciation** — separated populations diverge cognitively
- [ ] **Sentience spectrum** — communication, deception, play

## Emergent Dynamics

- [x] **Phylogenetic tree** — a living family tree alongside the simulation
- [x] **Arms races** — predator/prey co-evolution (claws vs armor)
- [x] **Mass extinction events** — asteroid, ice age, volcanic eruption
- [x] **Convergent evolution** — detection of independent lineages evolving similar solutions
- [x] **Seasonal pressure** — changing environment forces ongoing adaptation
- [x] **Geographic isolation** — ocean/mountain barriers drive allopatric speciation
- [x] **Social behaviour** — group sensing and metabolic discount in place, signalling wired up. Pack hunting, herding, flocking can emerge.
- [ ] **Symbiosis** — mutualism, parasitism, commensalism
