# Plan: plant physics, so that plants sit down of their own accord

**Date:** 2026-09-20
**Status:** proposed
**Scope:** two physical couplings that the sim is missing, then the resumption of step 3 of `plans/2026-09-19-diet-axis.md`. Not a rule that names a strategy. Not nutrients as a consumed resource (phase 2 of `docs/design/simulation-rules.md`).

## Why

Step 3 of the diet-axis plan (#20) swept every knob it was given and none of them kept the consumers alive. The uncapped run said why: plants doubled every 50 ticks from 2.3k to 98k while the grazers were still growing, and under any ceiling the replacement lottery drifts to the level that is always ready to breed. Two facts about the current rules make that inevitable, and both are omissions rather than decisions:

- **Light is not shared.** Photosynthesis income is `rate × photo surface area × tile light`, and the only competition is a penalty for plants sharing the same one-unit tile, which never engages on a 262k-tile map. A saturated plant earns about twelve times its upkeep and is permanently ready to reproduce.
- **Leaves have no physical consequences for motion.** Movement speed is `speed_factor × 2 / sqrt(body size) × armour drag`, and armour drag is the only body part that slows anything. A plant with a brain that outputs movement walks like anything else, at a cost its surplus income hides.

The principle, from the discussion of 2026-09-20: nothing in nature forbids a photosynthesiser from moving or from eating meat. What roots an oak is that light is a thin income, so a light-eater has to spread a large flat surface, and a large flat surface is a sail; and the payoff of moving is nearly nothing because the sun shines everywhere. The small ones swim, and the carnivorous ones ambush. The sim should say the same thing through the same two mechanisms, and then we watch whether the big leafy ones sit down by selection rather than by decree.

## The two couplings

**Drag from photosynthetic surface.** The speed formula gains a term for photo surface area beside the existing armour term: `speed × 1 / (1 + photo_area × photo_drag)`. Armour uses 0.3 per unit; a sail should drag more than a plate, so `photo_drag` starts at 1.0 and is a `SimConfig` knob with an override. A small-surface photosynthesiser is barely slowed; a broad one moves at a third of its speed or less. Nothing is forbidden: a leafy organism with claws can still attack, it just has to ambush.

**Light shared over a canopy.** Each tile lights a fixed amount of leaf area; a plant's leaves claim light from the tiles around it; where the leaves within a neighbourhood exceed what its tiles can light, everyone there is shaded in proportion. Concretely: build a per-tile sum of leaf area (`photo surface area × body size`) each tick, take box sums over a window of `CANOPY_RADIUS` tiles with a summed-area table so the cost is one pass over the grid plus four lookups per plant, and set each plant's light share to `min(1, window tiles × leaf_capacity_per_tile / leaf area in window)`. This replaces the per-tile penalty (`PLANT_DENSITY_PENALTY`) rather than adding to it. `leaf_capacity_per_tile` is the constant that sets how many fully lit plants the world holds, and it is a knob with an override because that is what the tuning step sweeps.

The point of the second coupling is not the plant count on its own. It is that a shaded plant earns near its upkeep and is not ready to breed, which removes the lottery advantage under a ceiling and the doubling without one; and it is that a plant's surplus becomes scarce enough that spending it on locomotion has a price, which is what makes the first coupling bite by selection.

## The plan

Four steps. Steps 1 to 3 are each one branch and one pull request; step 4 is the resumption of the diet-axis plan and may be one or two. Each step's pull request carries its `docs/DECISIONS.md` entries, runs `scripts/check.sh`, and updates `docs/FEATURES.md` where the sim's visible behaviour changes. The rule is the same as always: instrument first, then the dynamic, then tune.

### 1. Instruments, with no behaviour change

Make both couplings observable before either exists.

- **Movement by strategy.** `action_system` writes each organism's movement vector into the existing but unused `Velocity` component each tick. `record_population_history` averages its length over plants and over eaters: `avg_speed_plants` and `avg_speed_eaters` in `PopSnapshot`, the CSV, the headless summary, and a Graphs series.
- **Light share.** A `LightShare(f32)` component that `photosynthesis_system` writes for every photosynthesiser (today it is the per-tile density factor, so the instrument is live from step 1). `avg_light_share` over plants in the same places.
- **Readiness.** The share of plants and of eaters whose energy is above their reproduction threshold: `ready_share_plants` and `ready_share_eaters`. This is the lottery metric from step 3 of the diet plan, and the one number that says whether shading is doing its job.
- **Leaf area.** `avg_photo_area` over plants, so drag and shading can be read against the trait they act on.

Done when: the new columns are populated on a 5000-tick run, the ledger residual is unchanged, and the strategy counts match main within same-seed noise.

### 2. Photosynthetic surface drag

- `photo_drag` on `SimConfig`, default 1.0, `--photo-drag` override, applied in the speed formula beside armour drag, with the DECISIONS entry that says why a leaf is a sail.
- Sweep 0.3, 1.0 and 3.0 on seeds 1, 3 and 42 at 5000 ticks, reading `avg_speed_plants` against `avg_speed_eaters` and the strategy counts.

Done when: plants' mean speed falls well below the eaters' at the chosen value, small-surface photosynthesisers still move (the distribution of plant speed against leaf area shows it), and the DECISIONS entry records the sweep. No claim is made about the population outcome yet; that is step 3's job.

### 3. Canopy light sharing

- The summed-area table over leaf area per tile, `CANOPY_RADIUS` as a named constant, `leaf_capacity_per_tile` on `SimConfig` with `--leaf-capacity` as the override, and the per-tile penalty removed. The DECISIONS entry for plant density competition is superseded and says so.
- The first measurement is the run that exposed the problem: seed 42 with the ceiling at 100000 and the diet-axis settings (founder spread 1.0, bite 0.3). Sweep `leaf_capacity_per_tile` until the plant count levels off on its own well below 10k and `ready_share_plants` falls well below one. Record the tick cost at the plateau, because the ceiling raise in the diet plan's step 4 depends on it.
- Then seeds 1, 3 and 42 at 5000 ticks under the normal ceiling, watching whether grazers persist past the point where every previous configuration lost them.

Done when: an uncapped seed-42 run plateaus below 10k plants with the ledger at rounding, plant readiness is well below one at the plateau, and the DECISIONS entry records the capacity constant, the radius, the sweep, and the tick cost.

### 4. Resume the diet axis

With both couplings in, pick up `plans/2026-09-19-diet-axis.md` step 3 where it paused: consumer persistence at 5000 ticks on the three seeds, the interdependence test (`--animal-efficiency 0`), the kill share revisited (the per-meal pyramid share double-counts what metabolism already charges once digestion is explicit), the species threshold re-swept, the 8-seed audit with a dated roadmap block and a "Phase 1 outcome" section in the design doc, and then the diet plan's step 4, the ceiling measurement.

Done when the diet plan's own done-whens for steps 3 and 4 are met or their failure is recorded with the reason.

## Not in scope

- Nutrients as a resource consumed by growth. It couples plant capacity to soil and biomes and belongs to phase 2 of the design doc.
- Any rule of the form "photosynthesisers cannot X". Every limit here is a physical cost on a body, applied to every body.
- Plant-on-plant grazing. It exists (a plant with an attack output and a claw bites its neighbours and keeps a quarter) and at high density it is the largest flow in the sim. Observe it through the flows; do not gate it.
- Plant predators. Allowed; drag makes them ambushers. Observe.
- Performance work beyond measuring the tick cost at the plateau.

## Open questions

- **Leaf upkeep.** Metabolism charges body segments by count (`0.015 × size` each), not by area, so a large leaf costs the same to keep as a small one. Shading may not bite unless leaves cost by area; if step 3's sweep cannot bring readiness down without an absurd capacity constant, area-based segment upkeep is the next physical coupling, and it is a small change.
- **Canopy radius.** Start at 4 tiles (a 9 by 9 window). A plant's own footprint is under a tile, so the radius is really a statement about how far light competition reaches; it should be swept once alongside the capacity constant and then left alone.
- **Whether drag should also shorten attack range.** Probably not: attack range is about reach, drag is about locomotion. Left alone unless plant ambushers turn out to dominate.
- **Food items under the canopy.** Terrain food does not photosynthesise and takes no light. Left alone.

## References

- `plans/2026-09-19-diet-axis.md`: the plan this one interrupts, its step 3 outcome, and its step 4.
- `docs/DECISIONS.md`: "Diet axis tuning pass" (the sweeps and the reading), "Plant density competition" (superseded by step 3 here), "Grazing".
- `docs/design/simulation-rules.md`: the carrying-capacity decision ("emergent from energy") that this plan finally gives a mechanism.
- Code: `action_system` (speed and armour drag) and `photosynthesis_system` (`PLANT_DENSITY_PENALTY`, `PHOTO_OUTPUT_MULTIPLIER`) in `crates/clauvolution_sim/src/lib.rs`; `metabolism_system` for segment upkeep; `Velocity` and `PopSnapshot` in `crates/clauvolution_core/src/lib.rs`.
