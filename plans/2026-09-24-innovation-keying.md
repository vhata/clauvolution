# Plan: innovation numbers keyed by structure, so that unrelated means different

**Status:** stopped after step 1 (#83, 2026-09-25): keying does not separate lineages, so steps 2 to 4 will not run. Crossover keeps the initiator-topology rule (decided 2026-09-26).
**Scope:** `species-distance-unrelated-ceiling` in `TODO.md`. The change makes a connection's identity its structure, as in NEAT proper, and carries saves, exported creatures and the species thresholds across it. It does not retune the species distance weights beyond what the threshold sweep needs.

## Why

Every connection gets a fresh number from `InnovationCounter::next` (`clauvolution_genome`), whether it is a founder's (`Genome::new_minimal_with_diet`), an added connection (`Genome::mutate_add_connection`) or the two halves of a split (`Genome::mutate_add_neuron`). Two genomes of separate descent therefore share no genes, even when both have wired `food dx` to `move_x`. `Genome::compatibility_distance` then gives them `0.5 × (a + b) / max(a, b)` from its excess and disjoint terms, and nothing from the weight term, because `avg_weight_diff` is 0 when no genes match. That is at most 1.0, against a join threshold of 1.0 (`SimConfig::species_compat_threshold`, `--species-threshold`) and a stay threshold of 1.3 (`SPECIES_HYSTERESIS_FACTOR`, in `species_classification_system`).

The measurements are in the "Species classification" entry of `docs/DECISIONS.md` and the TODO entry. On seeds 42, 3 and 7 at 5000 ticks, 93 to 100% of organisms sat within 1.0 of another species' representative on every pass. Of the organisms past 1.3 from their own representative (657, 1481 and 485 across the passes), only 0, 58 and 1 were also more than 1.0 from every other representative, which is what `choose_species` needs before it founds a species. Seed 42 forms only 3 species after its founders with the stay rule in place. Genomes do diverge (members reach 2.9 from their species' first representative), so mutation is not the constraint; the distance cannot say that two unrelated brains are far apart.

The quick fix was tried and set aside: normalising the structural terms so unrelated genomes always score 1.0 plus the body term founded 353 to 355 species from 400 founders at tick 151 and ran several times slower. Founders are unrelated by construction, so any rule that scores unrelated genomes as far apart shatters them. Keying innovations by structure changes what "unrelated" means before the normalisation is touched: two founders that both wired input 1 to output 26 carry the same gene.

One consequence has to be said up front, because it decides the order of the steps. Keying alone does not raise the ceiling. It lets unrelated genomes match, which lowers the excess and disjoint terms, and it switches on the weight term for them, which raises it: two independently drawn weights from `NEW_WEIGHT_RANGE` differ by more than one on average. Which effect wins is not predictable from the code, so step 1 measures it before anything changes behaviour.

## Decisions to take

Each becomes a `docs/DECISIONS.md` entry in the pull request that implements it.

### A global table, or a per-generation one

- **Global table.** One resource maps a connection key `(from, to)` to its innovation number for the life of the world. Any genome that ever adds `(from, to)` gets the same number. Convergent wiring in unrelated lineages becomes shared genes, which is the point.
- **Per-generation table (Stanley's NEAT).** The table resets each generation, so only identical mutations made in the same generation share a number. The sim has no generations: `reproduction_system` breeds continuously, so the nearest equivalent is a table cleared every tick or every classification pass. That dedupes simultaneous mutations only. It would not let two lineages founded apart share a gene later, so it does not touch the ceiling.

**Recommendation: global.** The per-generation table exists in NEAT to bound memory and to keep "same number" meaning "same event"; neither is the problem here. Its size must be measured in step 1 (distinct keys over a 15k-tick run); if it grows without bound, pruning to keys carried by living genomes is a later fix.

**Hidden neurons must be keyed too, or the table is wrong.** `mutate_add_neuron` gives the new neuron `max id + 1` within the genome, so the first hidden neuron in every lineage is id 35 (`NUM_INPUTS + NUM_OUTPUTS`), whatever connection it split. Keying `(from, to)` over per-genome hidden ids would match `3 -> 35` in two genomes where neuron 35 means different things, and `crossover` would then merge them, since it takes `self.neurons` and pulls the other parent's neurons in by id. So the table also maps a split's key (the `(from, to)` of the connection being split) to a world-wide hidden neuron id, and each hidden `NeuronGene` records the key it split. Recording it on the gene makes a genome self-describing: the table can be rebuilt from the living population, and a genome can be re-keyed in any world. A content-addressed id (a hash of the split key, no table at all) was considered: it is universal across worlds without a remap, but the ids are opaque in Inspect and collisions need their own handling. It is the clever option; the table is the plain one.

Two edge cases the step 2 code must handle. A genome can split the same key twice: the split disables the connection, `TOGGLE_CONNECTION_PROBABILITY` can re-enable it, and a second split would hand out a hidden id the genome already has. Skip the mutation in that case, or give a fresh id; decide in step 2 by whichever keeps the RNG draws in `mutate` unchanged. And `mutate_add_connection` draws `from` from every neuron, outputs included, so recurrent keys such as `26 -> 30` are ordinary table entries.

### How old saves and exported genomes get mapped

A save's innovation numbers today mean "the same mutation event", not "the same structure", and its hidden ids are per-genome. Both have to be rewritten on load. "Save format: every genome field has a default" says a versioned format is the right answer "once fields change meaning or type rather than merely appear"; this is the first time that happens.

- **Re-key on load from the genome's own history.** The legacy counter is monotonic within a world (`InnovationCounter::next`, raised on load by `raise_innovation_counter`), and `mutate_add_neuron` issues the split's incoming gene and outgoing gene as consecutive numbers `n` and `n + 1`. Connections are never deleted (mutation only disables them) and `crossover` keeps every gene of `self`, so a genome that carries a hidden neuron carries both of its split genes. The split key of a legacy hidden neuron is therefore `(from of gene n, to of gene n + 1)`, where `n` is the lowest innovation among its incoming genes. Resolving hidden neurons in ascending `n` resolves every split's endpoints before the split. The result goes through the world's table like any new gene. This runs after `Genome::migrate_input_layout` in `save_to_genome`, because the nearest-eater migration shifts output and hidden ids by `NUM_INPUTS - k` and keys must be taken in the current layout.
- **Keep legacy numbers and key only new genes.** Old genomes would never match new ones or each other across lineages, so a loaded world would carry the ceiling until its old genomes died out.
- **Refuse old saves.** Loses every save and export for a change with a mechanical rewrite, which "Brain inputs grow by migration" already rejected for the input change.

**Recommendation: re-key on load**, with a `SaveState` field marking the keying scheme (missing means legacy, per "Save format: every field has a default unless the world cannot be rebuilt without it") and `CREATURE_FORMAT_VERSION` raised to 2 in `clauvolution_sim::save`. A hidden neuron the recipe cannot place (a hand-written file, or a pair that is not consecutive) gets a fresh id and a warning, as `validate_save_state` does for other defects. The test is the one the input migration set: a re-keyed brain gives exactly the outputs it gave before (`migrated_legacy_brain_gives_the_same_outputs_for_the_old_inputs` is the model).

Exported creatures follow the same path, with one gain. Input and output ids are the same in every world, so an imported creature's input-to-output genes match the importing world's without any shared history, and its hidden neurons re-key through their recorded split keys. The counter jump in `spawn_initial_population` becomes unnecessary. The "Imported creatures are founders" DECISIONS entry rejected renumbering imports because it lost the match between two imports from one origin world; under keying, renumbering is what preserves it, so that entry is rewritten in step 2.

The table itself need not be saved. Rebuilt from the living genomes in save order, it loses only keys no living genome carries, and a re-created extinct key getting a new number is harmless. This follows "Species ids are issued once, from the phylo tree": derive from the record that already exists rather than keep a second copy in the save.

### Whether the founding population should share innovations

- **Share through the table, nothing more.** Founders draw 3 to 8 input-to-output connections (`FOUNDER_CONNECTION_COUNT`) from 234 possible pairs, and any two that pick the same pair share the gene. How much two random founders overlap, and so how far apart they score, must be measured in step 1.
- **Founders descend from a few ancestral templates**, for example one per founding biome, each founder a mutated copy. Founders would then be related, which matches the story of life starting somewhere, but it narrows founding diversity and adds a stage to `spawn_initial_population` that consumes `SimRng` differently, so every seed changes for a second reason at once.
- **Founders keep fresh numbers** while later mutations are keyed. Founders stay mutually unrelated and the 353-species result stays in reach of any normalisation fix.

**Recommendation: share through the table only**, and take templates up as a follow-up if step 3's sweep shows the founding pass shattering at every threshold that keeps later speciation alive.

### What happens to the join and stay thresholds

- **Keep 1.0 and 1.3.** Only defensible if step 2 happens to land in the 10 to 30 living species that DECISIONS treats as the healthy range; the distance has changed meaning, so an unchanged count would be luck.
- **Re-sweep the join threshold with the stay rule fixed at 1.3 times it.** The same table shape as the 2026-09-18 sweep, now that runs are deterministic and one run per cell is a real result.
- **Sweep the join threshold together with the structural normalisation** (`max(a, b)` against the mean of `a` and `b`, so unrelated genomes score 1.0 plus body), since keying is what may make the second one usable.

**Recommendation: the third, restricted by step 1.** Step 1 computes both normalisations under both identities on the same saves, which is cheap and needs no behaviour change; only the combinations that separate lineages in those distributions go into the sweep. The hysteresis factor stays at 1.3 unless the sweep shows members piling up in the stay band.

## The plan

Each step is a branch and a pull request. Every step that changes the run a seed produces records before and after numbers in `docs/DECISIONS.md`, and the determinism probe in `.github/workflows/probe.yml` must still match. The table is only touched from `reproduction_system` and the spawn and load paths, all serial, and std `HashMap` lookups are fine under "Headless runs are deterministic"; the rebuild on load must iterate genomes in save order, never map order.

### 1. Instruments and the re-key function, with no behaviour change

Build the re-keying in the genome crate as a pure function over genomes (legacy numbers in, keyed genomes and a table out), with the identical-outputs test. It is the save migration step 2 needs, and it is also the instrument: run `--headless 5000 --save-as` on seeds 42, 3 and 7, re-key each save offline, and compute the distance distributions under the current identity and the keyed one, each with both normalisations. Record, per seed: the share of organisms within 1.0 of another species' representative (the 93 to 100% number); how many are past 1.3 from their own and past 1.0 from all others (the 0 / 58 / 1 number); the typical founder-to-founder distance; how often unrelated genomes match at least one gene and what the weight term adds when they do; and the number of distinct keys.

Also add the two per-pass counts above as Graphs series and headless summary lines, computed in `species_classification_system` from the distances `choose_species` already evaluates, so step 2 is judged on the same instrument. No new RNG draws, so same-seed `--dump-history` CSVs must be byte-identical before and after.

Done when: the re-key test passes, a `docs/audits/` note has the table above per seed, the Graphs tab shows the two counts, and the note says which normalisation and threshold range step 3 should sweep, or that keying does not separate lineages and the plan stops here.

### 2. Keyed innovations

Replace `InnovationCounter` with a table resource that issues connection numbers by key and hidden ids by split key; the signature change reaches `Genome::new_minimal_with_diet`, `new_photosynthesizer_with_diet`, `mutate` and its two structural helpers, `spawn_initial_population`, `reproduction_system`, `save_world` and the resource inserts in `clauvolution_app` (`InnovationCounter(100)` in two places), plus the tests in the brain, render, phylogeny and sim crates that construct genomes. Add the split key to hidden neurons and to `SaveNeuron` with a default, the keying marker to `SaveState`, and the format bump to `CreatureFile`; wire step 1's re-key into `save_to_genome` after `migrate_input_layout`, and drop the counter jump from `spawn_initial_population`. Crossover's alignment in `Genome::crossover` still sorts and merges by innovation number, which stays one-to-one with the key, so it should need no change; its tests should add a pair of unrelated parents with shared keys, because today such a cross copies `self`'s brain unchanged (nothing matches) and after this it mixes weights.

Keep the join threshold at 1.0 in this step so the behaviour change is read on its own.

Done when: old saves and v1 creature files load and behave as before (the identical-outputs test, through both load paths), the determinism probe matches, and the step 1 instruments and species counts at 5000 ticks on seeds 42, 3 and 7 are recorded against step 1's baseline in DECISIONS, rewriting "Imported creatures are founders" and adding a line to "Brain inputs grow by migration" about the order of the two rewrites.

### 3. Threshold sweep

Sweep the join threshold, and the normalisation if step 1 kept both in play, on seeds 42, 3 and 7 at 5000 ticks, one run per cell. Record species formed after founding (per 1000 ticks), species alive at ticks 1000 to 5000, the founding pass count, the strategy mix, and wall time, since the rejected normalisation ran several times slower. Choose the cell that forms species through the run without shattering the founders, ship it, and replace the "Species classification" threshold tables in DECISIONS with the new ones, keeping the old ones as history.

Done when: the chosen setting is the default, the sweep table is in DECISIONS, and the fraction of drifting organisms that can found a species is well above the 0 / 58 / 1 of the Why section, with the new numbers stated.

### 4. Audit and outcome

The 8-seed, 15k-tick audit (`scripts/attractor_audit.sh`). Read for species formed per 1000 ticks across the whole run, whether seed 42 still stalls, table size at 15k, and whether any strategy was lost that the 2026-09-23 audit had. Resolve `species-distance-unrelated-ceiling` in `TODO.md`, or replace it with a narrower remainder entry per `docs/TODO_GUIDE.md`.

Done when: the audit directory has the runs, the TODO entry is resolved or narrowed, and the roadmap's species notes say what changed.

## Not in scope

- Changing the body term or the weight of any distance term beyond the normalisation choice above.
- Pruning the table. Only if step 4 shows it growing without bound.
- Dropping innovation numbers in favour of the key itself. The excess and disjoint weights are equal (`c1 = c2 = 0.5`), so the excess/disjoint split, the one thing historical ordering buys, has no effect on the distance, and the key alone would do. It is a larger save change for no behaviour change.
- Ancestral founder templates, unless step 3 asks for them; then it is its own plan.
- Making `crossover`'s `self` the fitter parent. The doc comment says it is; `reproduction_system` passes whichever parent initiated. Separate TODO if it matters.
- Speciation by geography or mate choice. Phase 2 of `docs/design/simulation-rules.md`.

## Open questions

- **Does keying separate lineages at all?** It lowers the structural terms for convergent genomes and turns on the weight term. Step 1 answers this; if the answer is no, the plan stops after step 1 and the TODO entry records why.
- **The weight term for coincidental matches.** `avg_weight_diff` is a mean over matching genes, so one shared gene between otherwise unrelated brains turns on the whole term. Scaling it by the matching fraction is a possible follow-up if step 1 shows it dominating.
- **Should a species representative be re-keyed mid-run?** No, under the plan: re-keying happens only at load and import. Listed because a loaded world's first classification pass sees re-keyed genomes against re-keyed representatives, which is consistent, but its species may split or merge on that pass; step 2 should measure it on one save.
- **Imports from two different origin worlds.** The re-key recipe uses only a genome's own numbers, so it should not care, but the step 2 test should import from two worlds whose counters overlap.
- **Table growth.** Keys are pairs over every neuron id ever issued. Unknown until step 1 counts them.

## References

- `TODO.md`: `species-distance-unrelated-ceiling`.
- `docs/DECISIONS.md`: "Species classification", "Species ids are issued once, from the phylo tree", "Imported creatures are founders, and the innovation counter jumps past them", "Headless runs are deterministic", the three save-format entries, "Brain inputs grow by migration".
- `crates/clauvolution_genome/src/lib.rs`: `InnovationCounter`, `new_minimal_with_diet`, `mutate_add_connection`, `mutate_add_neuron`, `crossover`, `compatibility_distance`, `migrate_input_layout`.
- `crates/clauvolution_sim/src/lib.rs`: `reproduction_system`, `choose_species`, `species_classification_system`, `spawn_initial_population`.
- `crates/clauvolution_sim/src/save.rs`: `save_to_genome`, `raise_innovation_counter`, `CreatureFile`.
