use bevy::prelude::*;
use clauvolution_genome::{ActivationFn, ConnectionGene, Genome, NeuronGene, NeuronType, NUM_INPUTS, NUM_OUTPUTS};
use std::collections::HashMap;

pub struct BrainPlugin;

impl Plugin for BrainPlugin {
    fn build(&self, _app: &mut App) {}
}

/// A compiled neural network ready for evaluation.
/// Built from the genome's neuron and connection genes.
#[derive(Component, Clone, Debug)]
pub struct Brain {
    /// Neuron IDs in topological order (inputs first, then hidden, then outputs)
    eval_order: Vec<u64>,
    /// Activation function per neuron
    activations: HashMap<u64, ActivationFn>,
    /// Bias per neuron
    biases: HashMap<u64, f32>,
    /// Connections grouped by target neuron: target -> [(source, weight)]
    incoming: HashMap<u64, Vec<(u64, f32)>>,
    /// Output neuron IDs in order
    output_ids: Vec<u64>,
    /// Input neuron IDs in order
    input_ids: Vec<u64>,
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
                incoming.entry(conn.to).or_default().push((conn.from, conn.weight));
            }
        }

        // Topological sort for evaluation order
        let eval_order = topological_sort(&genome.neurons, &genome.connections);

        // Ensure input and output IDs are sorted
        input_ids.sort();
        output_ids.sort();

        Brain {
            eval_order,
            activations,
            biases,
            incoming,
            output_ids,
            input_ids,
        }
    }

    /// Evaluate the network given input values. Returns output values.
    pub fn evaluate(&self, inputs: &[f32; NUM_INPUTS]) -> [f32; NUM_OUTPUTS] {
        let mut values: HashMap<u64, f32> = HashMap::new();

        // Set input values
        for (i, &id) in self.input_ids.iter().enumerate() {
            if i < inputs.len() {
                values.insert(id, inputs[i]);
            } else {
                values.insert(id, 0.0);
            }
        }

        // Evaluate in topological order
        for &id in &self.eval_order {
            // Skip inputs, they're already set
            if self.input_ids.contains(&id) {
                continue;
            }

            let bias = self.biases.get(&id).copied().unwrap_or(0.0);
            let mut sum = bias;

            if let Some(conns) = self.incoming.get(&id) {
                for &(from_id, weight) in conns {
                    let from_val = values.get(&from_id).copied().unwrap_or(0.0);
                    sum += from_val * weight;
                }
            }

            let activation = self.activations.get(&id).copied().unwrap_or(ActivationFn::Sigmoid);
            values.insert(id, activation.apply(sum));
        }

        // Collect outputs
        let mut outputs = [0.0f32; NUM_OUTPUTS];
        for (i, &id) in self.output_ids.iter().enumerate() {
            if i < NUM_OUTPUTS {
                outputs[i] = values.get(&id).copied().unwrap_or(0.0);
            }
        }

        outputs
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
    let mut queue: Vec<u64> = all_ids.iter()
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
