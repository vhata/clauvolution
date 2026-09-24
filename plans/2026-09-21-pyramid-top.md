# Plan: the top of the pyramid, so that attack means attack and a hunter can live

**Status:** done 2026-09-24. All six steps ran; no hunter level formed. Outcome in `docs/design/simulation-rules.md` ("Pyramid-top outcome" under "Phase 1 outcome") and `docs/audits/2026-09-24-pyramid-step6/`; the remaining work is `hunter-emergence` in `TODO.md`.
**Scope:** the two design questions phase 1 left open, `graze-attack-output-split` and `hunter-emergence` in `TODO.md`, taken in that order because the second cannot be measured while the first is confounding it. Not phase 2 of `docs/design/simulation-rules.md`; see "Decisions taken while planning" for the one phase 2 question settled here so it stops gating.

## Why

Phase 1 (`plans/2026-09-19-diet-axis.md`, `plans/2026-09-20-plant-physics.md`) ended with plants and grazers persisting and cycling on all sixteen audit runs, and with two findings that are not tuning problems.

First, grazers kill each other. In the 2026-09-20 phase 1 audit, with no hunters alive on any run, predation was 43% of all deaths, about equal to starvation. The cause is structural. The only way to bite a living plant is the `attack` brain output, and an attack is resolved in `predation_system` against whatever passing neighbour is nearest: a plant is grazed, a consumer is killed. In a grazer-dense world the nearest passing neighbour is often another grazer, so a grazer that fires `attack` to feed kills a bystander and keeps almost nothing of it at its plant-leaning animal efficiency. #32 tested and disproved the hypothesis that hash-bucket order was to blame; striking the nearest passing target did not lower predation deaths. There is a second consequence hidden in the same path: a graze must pass the damage gate (`claw_power × size − armour × 0.5 > 0.1`), so a grazer needs claws to eat. Claws, which should be the predator's tool, are a grazing prerequisite, and every grazer is armed.

Second, no hunter level exists. Organisms with diet at or above +1/3 are gone by tick 600 to 1000 on every seed and every configuration tried (kill share 0.1 to 1.0, food items 0.1 and 0.02). Payoff is not what binds: a founding hunter digests 11% or less of plant tissue, has no food-item bridge while its random brain learns to chase, and starves in a few hundred ticks. The interdependence test the diet plan promised (hunting off shows grazers overrunning plants) cannot run until a top level exists, and phase 2's biomes have only two levels to sort until it does.

The second question cannot be read while the first is open: any measure of hunter kills, prey deaths or hunter persistence is buried under grazer-on-grazer kills. So the order is fixed.

## Decisions taken while planning

Recorded here because the planning conversation is otherwise lost. Each becomes a `docs/DECISIONS.md` entry in the pull request that implements it.

- **Grazing moves to the `eat` output.** The `eat` output, which today only takes food items, also bites the nearest living photosynthesiser in reach, with the mouth bonus, and `attack` is reserved for a kill attempt. Chosen over a separate `graze` output neuron (which changes the brain interface, grows every existing brain by a node, and asks a random founder brain to discover a second feeding output) and over a diet-weighted target preference (a programmed behaviour, against the project's principle). A mouth becomes the grazer's tool and claws the hunter's, which is the physical reading and needs no new gene.
- **Digestion follows the tissue, not the act.** Whatever is eaten is digested at the efficiency for what it is: plant tissue at `plant_efficiency`, animal tissue at `animal_efficiency`. Today a kill is always digested at animal efficiency even when the victim is a plant. Making the rule tissue-based means a hunter that kills a plant gains almost nothing, which is what keeps the apex from grazing (the "how species come to need each other" row of the decisions table).
- **Ocean vegetation becomes a config field, not a decision.** The phase 2 question `oceans-as-habitat` (water as habitat or only barrier) is a two-way door if built as a knob: water tiles already carry nutrients and light and are barren only because their vegetation regrowth is zero. The phase 2 plan will add a config field for water-tile vegetation, default to one setting, and cost both against the same seeds. The one thing decided now is that the terrain generator work in phase 2 should leave shallow shelves around landmasses so the habitat setting has somewhere to start. This lifts the gate on phase 2 without pre-empting the measurement.

## The plan

Each step is a branch and a pull request. Steps 1 to 3 resolve `graze-attack-output-split`; steps 4 to 6 work `hunter-emergence`. Every step that changes a rule gets a `docs/DECISIONS.md` entry with the before and after numbers, and the Graphs tab shows the effect before tuning starts.

### 1. Instruments, with no behaviour change

Add to `PredationStats` (or a sibling) what the split needs to be judged by: grazes taken through `eat` and through `attack` separately, kills where the killer's diet is below 0 (a grazer killing), kills where the victim is a consumer versus a plant, and attackers that fired with no plant in reach. Surface the graze and grazer-kill counts as Graphs series and in the headless summary, alongside the existing predation-deaths line. Record the baseline on seeds 42, 3 and 7 at 5000 ticks with the current rules.

Done when: the Graphs tab shows grazer-on-consumer kills as its own line, the headless summary prints the new counters, and a `docs/audits/` note records the baseline numbers per seed.

### 2. Bite with the mouth

In `action_system`, when `eat` fires and a living photosynthesiser is within eat range (`body_size × 3.0`, the food-item reach), take a bite of it: `bite = plant energy × bite_fraction × mouth_bonus`, digested at the eater's `plant_efficiency`, one bite per plant per tick under the same claim rule the predation path uses today, with the plant's energy reduced and the `Grazing` flash set. Food items in reach are still eaten first, since they are free tissue. The plant branch leaves `predation_system`: an attack on a photosynthesiser is a kill attempt like any other, subject to the size gate, and its energy is digested at the killer's `plant_efficiency` because the tissue is plant. Grazing needs a mouth and no claws; killing needs claws and no mouth.

The spatial hash is already available to `action_system`'s neighbours; this step may need it as a resource there, or a small grazing system placed between action and predation. Prefer whichever keeps `action_system`'s query set small; record the choice.

Done when: on seeds 42, 3 and 7 at 5000 ticks, grazer-on-consumer kills are a small fraction of what step 1 recorded, predation deaths fall accordingly, grazes are taken through `eat`, and plants and grazers still persist and cycle. If grazers starve instead, the bite reach or the mouth bonus is the first knob, and the decision entry says what was swept.

### 3. Resolve the TODO and re-read the pyramid

Remove `graze-attack-output-split` from `TODO.md`, update the phase 1 outcome in `docs/design/simulation-rules.md` and the 2026-09-20 roadmap block, and run the 8-seed, 15k-tick audit once (one run per seed now that headless runs are deterministic), reading specifically for hunters: whether any organism with diet at or above +1/3 is alive at 5000 or 15000 on any seed, and how long founding hunters now survive. This is the cheapest `hunter-emergence` candidate (time, and a world where attack means attack) and it must be read before anything is built for step 4.

Done when: the audit directory has the run, the roadmap block records hunters per seed at 5000 and 15000, and the reading says whether steps 4 to 6 are still needed and which first.

### 4. Let a hunter see prey

If step 3 still shows no hunter level, add a `nearest eater` brain input group: direction, distance and size ratio of the nearest non-photosynthesiser, the analogue of the existing nearest-food and nearest-organism inputs. Today the nearest-organism input is usually a plant, since plants are half the population, so a hunter's brain has nothing to steer at. This changes `NUM_INPUTS` in the genome crate; the pull request states the save-compatibility consequence and either migrates old genomes (add the input nodes unconnected) or documents the break.

Done when: the input is wired and shown in Inspect, and on the three seeds at 5000 ticks the mean lifetime of founders with diet at or above +1/3 is longer than in step 3.

### 5. The size gate on consumer prey

The attacker must be above 0.6 of the prey's size to kill a consumer. Founding hunters are not large. Measure how many attacks by diet-positive attackers the gate rejects (step 1's counters can be extended for this), and if it is most of them, make the gate a function of the damage already computed rather than a second threshold, or scale it by diet. Whichever is tried, it is one knob, swept once, recorded.

Done when: the gate's rejection share for hunters is recorded before and after, and the change either helps hunter lifetime or is reverted with the numbers.

### 6. Audit and outcome

The 8-seed, 15k audit again. Update `hunter-emergence` (resolve it, or replace it with a narrower remainder entry per `docs/TODO_GUIDE.md`), write the outcome into the design doc, and, if a hunter level exists, run the interdependence test from the diet plan (`--animal-efficiency 0`) and record it.

Done when: hunters (diet at or above +1/3) are alive at 15k ticks on most of the eight seeds, or the design doc says why not and what the next candidate is.

## What done looks like for the whole plan

Attack means a kill attempt and nothing else; grazers feed with mouths; predation deaths with no hunters alive are a small fraction of what they were; a hunter level persists on most seeds, or the reason it cannot is written down with measurements. The interdependence test has either run or is blocked on one named thing. Phase 2 can then place a three-level pyramid on its biomes.

## Not in scope

- Phase 2 (biomes as pressure and barrier). It follows this plan; its one gating question is settled above.
- A separate `graze` brain output. Rejected in the decision above.
- Diet-weighted target selection or any rule that reads the diet gene to pick a victim. Programmed behaviour.
- Plant defences (toxins, thorns beyond armour). The first follow-on once grazing is watchable through `eat`, per the decisions table.
- Founder staging by trophic level. Rejected in the decisions table.

## Open questions

- **Should attack on a plant be allowed at all?** Step 2 keeps it as a kill attempt digested at plant efficiency. The alternative is that `predation_system` skips photosynthesisers entirely, which is simpler but is a rule about who may be attacked. Decide in step 2 with the numbers: if plant kills by consumers are a meaningful energy path after the change, revisit.
- **Bite reach.** Eat range is 3× body size; attack range is 4×. Step 2 uses eat range. If grazers starve, this is the first knob.
- **`NUM_INPUTS` and saves.** Step 4 changes the brain interface. Whether to migrate old genomes or accept a save break is decided there; `genome-serde-default-consistency` (in flight) makes field-level defaults the rule but cannot add input nodes.
- **Hunter bridge.** If steps 4 and 5 both fail, the remaining candidate is that a founding hunter needs an omnivore bridge: the diet axis is continuous, so intermediate diets exist, and the question becomes why they do not persist long enough for carnivory to emerge from them. That is a new plan, not a step here.

## References

- `TODO.md`: `graze-attack-output-split`, `hunter-emergence`, `oceans-as-habitat`.
- `docs/design/simulation-rules.md`: decisions table, "Phase 1 outcome", "Phase 2".
- `docs/audits/2026-09-20-phase1-audit/`: the baseline this plan reads against.
- #32: the disproved ordering hypothesis and the nearest-target rule it leaves in place.
- `plans/2026-09-19-diet-axis.md`: the interdependence test and the ceiling measurement.
