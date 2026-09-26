use bevy::prelude::*;
use clauvolution_genome::{
    ActivationFn, ConnectionGene, Genome, NeuronGene, NeuronType, NUM_INPUTS, NUM_OUTPUTS,
};
use std::collections::HashMap;

pub struct BrainPlugin;

impl Plugin for BrainPlugin {
    fn build(&self, _app: &mut App) {}
}

/// A compiled neural network ready for evaluation.
/// Built from the genome's neuron and connection genes.
///
/// Every neuron id the network mentions gets a slot, and evaluation runs
/// over a flat `f32` array indexed by slot. Evaluation used to key a fresh
/// `HashMap<u64, f32>` by neuron id on every call, which was most of its
/// cost; the arithmetic is unchanged (same neurons in the same order, each
/// sum starting from the bias and adding incoming connections in genome
/// order), so the outputs are bit-identical. A slot that is never written
/// reads 0.0, as a missing map entry did. See DECISIONS.md, "Brains
/// evaluate over slots, not a map".
#[derive(Component, Clone, Debug)]
pub struct Brain {
    /// Neuron id for each slot, in slot order.
    slot_ids: Vec<u64>,
    /// `(neuron id, slot)` sorted by id, for `activation`.
    slot_by_id: Vec<(u64, u32)>,
    /// The slot each entry of `input_ids` writes.
    input_slots: Vec<u32>,
    /// Non-input neurons in evaluation (topological) order.
    steps: Vec<Step>,
    /// Incoming `(source slot, weight)` pairs, grouped per step.
    sources: Vec<(u32, f32)>,
    /// The slot each entry of `output_ids` reads.
    output_slots: Vec<u32>,
    /// Output neuron IDs in order
    output_ids: Vec<u64>,
    /// Input neuron IDs in order
    input_ids: Vec<u64>,
}

/// One non-input neuron's evaluation.
#[derive(Clone, Debug)]
struct Step {
    slot: u32,
    bias: f32,
    activation: ActivationFn,
    /// Range of `Brain::sources` feeding this neuron.
    sources_start: u32,
    sources_end: u32,
}

impl Brain {
    /// Build a Brain from a Genome
    pub fn from_genome(genome: &Genome) -> Self {
        let mut activations = HashMap::new();
        let mut biases = HashMap::new();
        let mut incoming: HashMap<u64, Vec<(u64, f32)>> = HashMap::new();
        let mut input_ids = Vec::new();
        let mut output_ids = Vec::new();

        for neuron in &genome.neurons {
            activations.insert(neuron.id, neuron.activation);
            biases.insert(neuron.id, neuron.bias);
            match neuron.neuron_type {
                NeuronType::Input => input_ids.push(neuron.id),
                NeuronType::Output => output_ids.push(neuron.id),
                NeuronType::Hidden => {}
            }
        }

        for conn in &genome.connections {
            if conn.enabled {
                incoming
                    .entry(conn.to)
                    .or_default()
                    .push((conn.from, conn.weight));
            }
        }

        // Topological sort for evaluation order
        let eval_order = topological_sort(&genome.neurons, &genome.connections);

        // Ensure input and output IDs are sorted
        input_ids.sort();
        output_ids.sort();

        // Assign slots in first-mention order.
        let mut slot_of: HashMap<u64, u32> = HashMap::new();
        let mut slot_ids: Vec<u64> = Vec::new();
        let mut slot = |id: u64| -> u32 {
            *slot_of.entry(id).or_insert_with(|| {
                slot_ids.push(id);
                (slot_ids.len() - 1) as u32
            })
        };

        let input_slots: Vec<u32> = input_ids.iter().map(|&id| slot(id)).collect();
        let mut steps = Vec::new();
        let mut sources = Vec::new();
        for &id in &eval_order {
            // Inputs are set directly, not evaluated.
            if input_ids.contains(&id) {
                continue;
            }
            let sources_start = sources.len() as u32;
            if let Some(conns) = incoming.get(&id) {
                for &(from_id, weight) in conns {
                    sources.push((slot(from_id), weight));
                }
            }
            steps.push(Step {
                slot: slot(id),
                bias: biases.get(&id).copied().unwrap_or(0.0),
                activation: activations
                    .get(&id)
                    .copied()
                    .unwrap_or(ActivationFn::Sigmoid),
                sources_start,
                sources_end: sources.len() as u32,
            });
        }
        let output_slots: Vec<u32> = output_ids.iter().map(|&id| slot(id)).collect();

        let mut slot_by_id: Vec<(u64, u32)> = slot_of.into_iter().collect();
        slot_by_id.sort_unstable();

        Brain {
            slot_ids,
            slot_by_id,
            input_slots,
            steps,
            sources,
            output_slots,
            output_ids,
            input_ids,
        }
    }

    /// Evaluate the network given input values. Returns output values.
    pub fn evaluate(&self, inputs: &[f32; NUM_INPUTS]) -> [f32; NUM_OUTPUTS] {
        self.evaluate_into(inputs, &mut Vec::new())
    }

    /// Evaluate and leave every neuron's value in `values`, indexed by slot.
    /// The buffer is resized to fit and reused across calls, so a caller
    /// that keeps it (as `BrainActivations` does) allocates once. The values
    /// are what the brain-activation heatmap renders; read one with
    /// `activation`.
    pub fn evaluate_into(
        &self,
        inputs: &[f32; NUM_INPUTS],
        values: &mut Vec<f32>,
    ) -> [f32; NUM_OUTPUTS] {
        values.clear();
        values.resize(self.slot_ids.len(), 0.0);

        for (i, &slot) in self.input_slots.iter().enumerate() {
            values[slot as usize] = if i < inputs.len() { inputs[i] } else { 0.0 };
        }

        for step in &self.steps {
            let mut sum = step.bias;
            for &(from, weight) in
                &self.sources[step.sources_start as usize..step.sources_end as usize]
            {
                sum += values[from as usize] * weight;
            }
            values[step.slot as usize] = step.activation.apply(sum);
        }

        let mut outputs = [0.0f32; NUM_OUTPUTS];
        for (i, &slot) in self.output_slots.iter().enumerate() {
            if i < NUM_OUTPUTS {
                outputs[i] = values[slot as usize];
            }
        }
        outputs
    }

    /// Neuron `id`'s value in a buffer `evaluate_into` filled for this
    /// brain; 0.0 for an id the brain does not have or a buffer too short.
    pub fn activation(&self, values: &[f32], id: u64) -> f32 {
        self.slot_by_id
            .binary_search_by_key(&id, |&(neuron, _)| neuron)
            .ok()
            .and_then(|i| values.get(self.slot_by_id[i].1 as usize))
            .copied()
            .unwrap_or(0.0)
    }

    /// Sorted input neuron IDs (used by UI to align labels with indices)
    pub fn input_ids(&self) -> &[u64] {
        &self.input_ids
    }

    /// Sorted output neuron IDs
    pub fn output_ids(&self) -> &[u64] {
        &self.output_ids
    }
}

/// Topological sort of neurons based on connections.
/// Returns neuron IDs in evaluation order.
fn topological_sort(neurons: &[NeuronGene], connections: &[ConnectionGene]) -> Vec<u64> {
    let all_ids: Vec<u64> = neurons.iter().map(|n| n.id).collect();
    let mut in_degree: HashMap<u64, usize> = HashMap::new();
    let mut adj: HashMap<u64, Vec<u64>> = HashMap::new();

    for &id in &all_ids {
        in_degree.insert(id, 0);
    }

    for conn in connections {
        if conn.enabled {
            // Only count edges to nodes that exist
            if in_degree.contains_key(&conn.to) && in_degree.contains_key(&conn.from) {
                *in_degree.entry(conn.to).or_default() += 1;
                adj.entry(conn.from).or_default().push(conn.to);
            }
        }
    }

    // Kahn's algorithm
    let mut queue: Vec<u64> = all_ids
        .iter()
        .filter(|id| in_degree[id] == 0)
        .copied()
        .collect();
    let mut order = Vec::new();

    while let Some(id) = queue.pop() {
        order.push(id);
        if let Some(neighbors) = adj.get(&id) {
            for &next in neighbors {
                let deg = in_degree.get_mut(&next).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push(next);
                }
            }
        }
    }

    // If there are cycles (recurrent connections), append remaining neurons
    // This handles the case where NEAT creates recurrent connections
    for &id in &all_ids {
        if !order.contains(&id) {
            order.push(id);
        }
    }

    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use clauvolution_genome::InnovationCounter;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    const LEGACY_INPUTS: usize = 22;

    /// A genome in the 22-input layout that preceded the nearest-eater
    /// inputs, grown by ordinary mutation so it carries hidden neurons and
    /// whatever recurrent links mutation produces.
    fn evolved_legacy_genome(seed: u64) -> Genome {
        let mut innovation = InnovationCounter(0);
        let mut rng = StdRng::seed_from_u64(seed);
        let mut genome = Genome::new_minimal(&mut innovation, &mut rng);
        genome.neurons.clear();
        genome.connections.clear();
        for id in 0..LEGACY_INPUTS as u64 {
            genome.neurons.push(NeuronGene {
                id,
                neuron_type: NeuronType::Input,
                activation: ActivationFn::Sigmoid,
                bias: 0.0,
            });
        }
        for i in 0..NUM_OUTPUTS {
            genome.neurons.push(NeuronGene {
                id: (LEGACY_INPUTS + i) as u64,
                neuron_type: NeuronType::Output,
                activation: ActivationFn::Tanh,
                bias: rng.gen_range(-0.5..0.5),
            });
        }
        for _ in 0..8 {
            genome.connections.push(ConnectionGene {
                innovation: innovation.next(),
                from: rng.gen_range(0..LEGACY_INPUTS) as u64,
                to: (LEGACY_INPUTS + rng.gen_range(0..NUM_OUTPUTS)) as u64,
                weight: rng.gen_range(-2.0..2.0),
                enabled: true,
            });
        }
        for _ in 0..60 {
            genome.mutate(&mut innovation, &mut rng, 0.5, 0.5);
        }
        genome
    }

    /// The evaluator `Brain` replaced, kept verbatim as the reference: a
    /// fresh `HashMap` keyed by neuron id on every call.
    fn reference_evaluate(
        genome: &Genome,
        inputs: &[f32; NUM_INPUTS],
    ) -> ([f32; NUM_OUTPUTS], HashMap<u64, f32>) {
        let mut activations = HashMap::new();
        let mut biases = HashMap::new();
        let mut incoming: HashMap<u64, Vec<(u64, f32)>> = HashMap::new();
        let mut input_ids = Vec::new();
        let mut output_ids = Vec::new();
        for neuron in &genome.neurons {
            activations.insert(neuron.id, neuron.activation);
            biases.insert(neuron.id, neuron.bias);
            match neuron.neuron_type {
                NeuronType::Input => input_ids.push(neuron.id),
                NeuronType::Output => output_ids.push(neuron.id),
                NeuronType::Hidden => {}
            }
        }
        for conn in &genome.connections {
            if conn.enabled {
                incoming
                    .entry(conn.to)
                    .or_default()
                    .push((conn.from, conn.weight));
            }
        }
        let eval_order = topological_sort(&genome.neurons, &genome.connections);
        input_ids.sort();
        output_ids.sort();

        let mut values: HashMap<u64, f32> = HashMap::new();
        for (i, &id) in input_ids.iter().enumerate() {
            if i < inputs.len() {
                values.insert(id, inputs[i]);
            } else {
                values.insert(id, 0.0);
            }
        }
        for &id in &eval_order {
            if input_ids.contains(&id) {
                continue;
            }
            let bias = biases.get(&id).copied().unwrap_or(0.0);
            let mut sum = bias;
            if let Some(conns) = incoming.get(&id) {
                for &(from_id, weight) in conns {
                    let from_val = values.get(&from_id).copied().unwrap_or(0.0);
                    sum += from_val * weight;
                }
            }
            let activation = activations
                .get(&id)
                .copied()
                .unwrap_or(ActivationFn::Sigmoid);
            values.insert(id, activation.apply(sum));
        }
        let mut outputs = [0.0f32; NUM_OUTPUTS];
        for (i, &id) in output_ids.iter().enumerate() {
            if i < NUM_OUTPUTS {
                outputs[i] = values.get(&id).copied().unwrap_or(0.0);
            }
        }
        (outputs, values)
    }

    #[test]
    fn slot_evaluation_matches_the_map_evaluator_bit_for_bit() {
        let mut innovation = InnovationCounter(0);
        let mut rng = StdRng::seed_from_u64(7);
        let mut values = Vec::new();
        let mut with_hidden = 0;
        for genome_index in 0..40 {
            let mut genome = Genome::new_minimal(&mut innovation, &mut rng);
            for _ in 0..(genome_index * 3) {
                genome.mutate(&mut innovation, &mut rng, 0.5, 0.5);
            }
            if genome
                .neurons
                .iter()
                .any(|n| n.neuron_type == NeuronType::Hidden)
            {
                with_hidden += 1;
            }
            // A connection from a neuron the genome does not list reads 0.
            genome.connections.push(ConnectionGene {
                innovation: innovation.next(),
                from: 1_000_000,
                to: NUM_INPUTS as u64,
                weight: 1.5,
                enabled: true,
            });
            let brain = Brain::from_genome(&genome);
            for _ in 0..20 {
                let mut inputs = [0.0f32; NUM_INPUTS];
                for value in inputs.iter_mut() {
                    *value = rng.gen_range(-2.0..2.0);
                }
                let (expected, trace) = reference_evaluate(&genome, &inputs);
                let got = brain.evaluate_into(&inputs, &mut values);
                assert_eq!(
                    expected.map(f32::to_bits),
                    got.map(f32::to_bits),
                    "genome {genome_index}: outputs differ"
                );
                for neuron in &genome.neurons {
                    let want = trace.get(&neuron.id).copied().unwrap_or(0.0);
                    assert_eq!(
                        want.to_bits(),
                        brain.activation(&values, neuron.id).to_bits(),
                        "genome {genome_index}: neuron {} activation differs",
                        neuron.id
                    );
                }
            }
        }
        assert!(
            with_hidden > 20,
            "the fixture should exercise hidden neurons, got {with_hidden} of 40"
        );
        let brain = Brain::from_genome(&Genome::new_minimal(&mut innovation, &mut rng));
        assert_eq!(brain.activation(&values, u64::MAX), 0.0);
        assert_eq!(brain.activation(&[], 0), 0.0);
    }

    #[test]
    fn migrated_legacy_brain_gives_the_same_outputs_for_the_old_inputs() {
        let mut seeds_with_hidden = 0;
        for seed in 0..20 {
            let legacy = evolved_legacy_genome(seed);
            let hidden = legacy
                .neurons
                .iter()
                .filter(|n| n.neuron_type == NeuronType::Hidden)
                .count();
            if hidden > 0 {
                seeds_with_hidden += 1;
            }
            let mut migrated = legacy.clone();
            assert!(migrated.migrate_input_layout());

            let old_brain = Brain::from_genome(&legacy);
            let new_brain = Brain::from_genome(&migrated);
            assert_eq!(old_brain.input_ids().len(), LEGACY_INPUTS);
            assert_eq!(new_brain.input_ids().len(), NUM_INPUTS);

            let mut rng = StdRng::seed_from_u64(1000 + seed);
            for _ in 0..50 {
                // The old brain reads only the first 22 slots. The migrated
                // one also reads the new slots, which must make no
                // difference because nothing is wired to them.
                let mut inputs = [0.0f32; NUM_INPUTS];
                for value in inputs.iter_mut() {
                    *value = rng.gen_range(-1.0..1.0);
                }
                let old_out = old_brain.evaluate(&inputs);
                let new_out = new_brain.evaluate(&inputs);
                assert_eq!(
                    old_out, new_out,
                    "seed {seed} ({hidden} hidden neurons): outputs differ after migration"
                );
            }
        }
        assert!(
            seeds_with_hidden > 10,
            "the fixture should exercise hidden neurons, got {seeds_with_hidden} of 20"
        );
    }

    /// A small evolving population: founders, then rounds of crossover and
    /// mutation between random members, so the genomes share descent the
    /// way a saved world's do, carry hidden neurons, and include whatever
    /// recurrent links and re-enabled splits mutation produces.
    fn evolved_population(seed: u64) -> Vec<Genome> {
        let mut innovation = InnovationCounter(0);
        let mut rng = StdRng::seed_from_u64(seed);
        let mut population: Vec<Genome> = (0..12)
            .map(|_| Genome::new_minimal(&mut innovation, &mut rng))
            .collect();
        for _ in 0..400 {
            let a = rng.gen_range(0..population.len());
            let b = rng.gen_range(0..population.len());
            let mut child = population[a].crossover(&population[b], &mut rng);
            child.mutate(&mut innovation, &mut rng, 0.5, 0.5);
            let slot = rng.gen_range(0..population.len());
            population[slot] = child;
        }
        population
    }

    /// Re-keying is a safe renaming: the brain built from a re-keyed genome
    /// gives the same outputs, bit for bit. Brains resolve ids by lookup, so
    /// any consistent renaming passes this, including a wrong split key; the
    /// genome crate's `nested_splits_are_keyed_on_re_keyed_endpoints` checks
    /// that the keys themselves are right.
    #[test]
    fn re_keyed_brain_gives_exactly_the_same_outputs() {
        let mut hidden_neurons = 0;
        let mut keyed_hidden = 0;
        for seed in 0..10 {
            let population = evolved_population(seed);
            let (keyed, _, report) = clauvolution_genome::rekey_population(&population);
            keyed_hidden += report.hidden_keyed;
            assert_eq!(report.hidden_unplaced, 0, "seed {seed}: {report:?}");
            assert_eq!(report.dangling_ids, 0, "seed {seed}: {report:?}");

            let mut rng = StdRng::seed_from_u64(2000 + seed);
            for (legacy, keyed) in population.iter().zip(&keyed) {
                hidden_neurons += legacy
                    .neurons
                    .iter()
                    .filter(|n| n.neuron_type == NeuronType::Hidden)
                    .count();
                let old_brain = Brain::from_genome(legacy);
                let new_brain = Brain::from_genome(keyed);
                assert_eq!(old_brain.input_ids(), new_brain.input_ids());
                assert_eq!(old_brain.output_ids(), new_brain.output_ids());
                for _ in 0..30 {
                    let mut inputs = [0.0f32; NUM_INPUTS];
                    for value in inputs.iter_mut() {
                        *value = rng.gen_range(-1.0..1.0);
                    }
                    assert_eq!(
                        old_brain.evaluate(&inputs).map(f32::to_bits),
                        new_brain.evaluate(&inputs).map(f32::to_bits),
                        "seed {seed}: outputs differ after re-keying"
                    );
                }
            }
        }
        assert!(
            hidden_neurons > 100 && keyed_hidden > 100,
            "the fixture should exercise hidden neurons, got {hidden_neurons} ({keyed_hidden} keyed)"
        );
    }
}
