use bevy::prelude::*;
use rand::Rng;
use rand_distr::{Distribution, Normal};

pub struct GenomePlugin;

impl Plugin for GenomePlugin {
    fn build(&self, _app: &mut App) {}
}

/// Global innovation counter for NEAT
#[derive(Resource)]
pub struct InnovationCounter(pub u64);

impl InnovationCounter {
    pub fn next(&mut self) -> u64 {
        let n = self.0;
        self.0 += 1;
        n
    }
}

/// A neuron gene in the NEAT network
#[derive(Clone, Debug)]
pub struct NeuronGene {
    pub id: u64,
    pub neuron_type: NeuronType,
    pub activation: ActivationFn,
    pub bias: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeuronType {
    Input,
    Hidden,
    Output,
}

#[derive(Clone, Copy, Debug)]
pub enum ActivationFn {
    Sigmoid,
    Tanh,
    Relu,
}

impl ActivationFn {
    pub fn apply(&self, x: f32) -> f32 {
        match self {
            ActivationFn::Sigmoid => 1.0 / (1.0 + (-x).exp()),
            ActivationFn::Tanh => x.tanh(),
            ActivationFn::Relu => x.max(0.0),
        }
    }

    pub fn random(rng: &mut impl Rng) -> Self {
        match rng.gen_range(0..3) {
            0 => ActivationFn::Sigmoid,
            1 => ActivationFn::Tanh,
            _ => ActivationFn::Relu,
        }
    }
}

/// A connection gene in the NEAT network
#[derive(Clone, Debug)]
pub struct ConnectionGene {
    pub innovation: u64,
    pub from: u64,
    pub to: u64,
    pub weight: f32,
    pub enabled: bool,
}

/// The full genome of an organism
#[derive(Component, Clone, Debug)]
pub struct Genome {
    pub neurons: Vec<NeuronGene>,
    pub connections: Vec<ConnectionGene>,
    pub body_size: f32,
    pub speed_factor: f32,
    pub sense_range: f32,
}

// Phase 1 brain I/O layout
pub const NUM_INPUTS: usize = 9;
pub const NUM_OUTPUTS: usize = 4;

// Inputs:
//  0: energy_level (0-1)
//  1: nearest_food_dir_x (-1 to 1)
//  2: nearest_food_dir_y (-1 to 1)
//  3: nearest_food_dist (0-1, normalized)
//  4: nearest_organism_dir_x
//  5: nearest_organism_dir_y
//  6: nearest_organism_dist
//  7: nearest_organism_size_ratio
//  8: bias (always 1.0)

// Outputs:
//  0: move_x (-1 to 1)
//  1: move_y (-1 to 1)
//  2: eat (> 0.5 = attempt eat)
//  3: reproduce (> 0.5 = attempt reproduce)

impl Genome {
    /// Create a minimal starting genome with direct input-output connections
    pub fn new_minimal(innovation: &mut InnovationCounter, rng: &mut impl Rng) -> Self {
        let mut neurons = Vec::new();

        // Input neurons (ids 0..NUM_INPUTS)
        for i in 0..NUM_INPUTS {
            neurons.push(NeuronGene {
                id: i as u64,
                neuron_type: NeuronType::Input,
                activation: ActivationFn::Sigmoid,
                bias: 0.0,
            });
        }

        // Output neurons (ids NUM_INPUTS..NUM_INPUTS+NUM_OUTPUTS)
        for i in 0..NUM_OUTPUTS {
            neurons.push(NeuronGene {
                id: (NUM_INPUTS + i) as u64,
                neuron_type: NeuronType::Output,
                activation: ActivationFn::Tanh,
                bias: rng.gen_range(-1.0..1.0),
            });
        }

        // Start with a few random connections from inputs to outputs
        let mut connections = Vec::new();
        let num_initial_connections = rng.gen_range(3..=8);
        for _ in 0..num_initial_connections {
            let from = rng.gen_range(0..NUM_INPUTS) as u64;
            let to = (NUM_INPUTS + rng.gen_range(0..NUM_OUTPUTS)) as u64;

            // Skip if this connection already exists
            if connections.iter().any(|c: &ConnectionGene| c.from == from && c.to == to) {
                continue;
            }

            connections.push(ConnectionGene {
                innovation: innovation.next(),
                from,
                to,
                weight: rng.gen_range(-2.0..2.0),
                enabled: true,
            });
        }

        Self {
            neurons,
            connections,
            body_size: rng.gen_range(0.5..1.5),
            speed_factor: rng.gen_range(0.5..1.5),
            sense_range: rng.gen_range(30.0..80.0),
        }
    }

    /// Mutate this genome in place
    pub fn mutate(&mut self, innovation: &mut InnovationCounter, rng: &mut impl Rng, rate: f32, strength: f32) {
        let normal = Normal::new(0.0, strength as f64).unwrap();

        // Mutate connection weights
        for conn in &mut self.connections {
            if rng.gen::<f32>() < rate {
                if rng.gen::<f32>() < 0.1 {
                    // 10% chance: completely new random weight
                    conn.weight = rng.gen_range(-2.0..2.0);
                } else {
                    // 90% chance: perturb existing weight
                    conn.weight += normal.sample(rng) as f32;
                    conn.weight = conn.weight.clamp(-4.0, 4.0);
                }
            }
        }

        // Mutate neuron biases
        for neuron in &mut self.neurons {
            if neuron.neuron_type != NeuronType::Input && rng.gen::<f32>() < rate {
                neuron.bias += normal.sample(rng) as f32;
                neuron.bias = neuron.bias.clamp(-4.0, 4.0);
            }
        }

        // Add new connection (probability 0.05)
        if rng.gen::<f32>() < 0.05 {
            self.mutate_add_connection(innovation, rng);
        }

        // Add new neuron by splitting a connection (probability 0.03)
        if rng.gen::<f32>() < 0.03 {
            self.mutate_add_neuron(innovation, rng);
        }

        // Toggle a connection enabled/disabled (probability 0.02)
        if !self.connections.is_empty() && rng.gen::<f32>() < 0.02 {
            let idx = rng.gen_range(0..self.connections.len());
            self.connections[idx].enabled = !self.connections[idx].enabled;
        }

        // Mutate body traits
        if rng.gen::<f32>() < rate {
            self.body_size += normal.sample(rng) as f32 * 0.2;
            self.body_size = self.body_size.clamp(0.2, 3.0);
        }
        if rng.gen::<f32>() < rate {
            self.speed_factor += normal.sample(rng) as f32 * 0.2;
            self.speed_factor = self.speed_factor.clamp(0.2, 3.0);
        }
        if rng.gen::<f32>() < rate {
            self.sense_range += normal.sample(rng) as f32 * 5.0;
            self.sense_range = self.sense_range.clamp(10.0, 150.0);
        }
    }

    fn mutate_add_connection(&mut self, innovation: &mut InnovationCounter, rng: &mut impl Rng) {
        let non_input: Vec<u64> = self.neurons.iter()
            .filter(|n| n.neuron_type != NeuronType::Input)
            .map(|n| n.id)
            .collect();

        if non_input.is_empty() {
            return;
        }

        let all_ids: Vec<u64> = self.neurons.iter().map(|n| n.id).collect();
        let from = all_ids[rng.gen_range(0..all_ids.len())];
        let to = non_input[rng.gen_range(0..non_input.len())];

        // No self-connections, no duplicates
        if from == to {
            return;
        }
        if self.connections.iter().any(|c| c.from == from && c.to == to) {
            return;
        }

        self.connections.push(ConnectionGene {
            innovation: innovation.next(),
            from,
            to,
            weight: rng.gen_range(-2.0..2.0),
            enabled: true,
        });
    }

    fn mutate_add_neuron(&mut self, innovation: &mut InnovationCounter, rng: &mut impl Rng) {
        let enabled: Vec<usize> = self.connections.iter()
            .enumerate()
            .filter(|(_, c)| c.enabled)
            .map(|(i, _)| i)
            .collect();

        if enabled.is_empty() {
            return;
        }

        let conn_idx = enabled[rng.gen_range(0..enabled.len())];
        self.connections[conn_idx].enabled = false;

        let old_from = self.connections[conn_idx].from;
        let old_to = self.connections[conn_idx].to;
        let old_weight = self.connections[conn_idx].weight;

        let new_id = self.neurons.iter().map(|n| n.id).max().unwrap_or(0) + 1;

        self.neurons.push(NeuronGene {
            id: new_id,
            neuron_type: NeuronType::Hidden,
            activation: ActivationFn::random(rng),
            bias: 0.0,
        });

        // Connection from old source to new neuron with weight 1.0
        self.connections.push(ConnectionGene {
            innovation: innovation.next(),
            from: old_from,
            to: new_id,
            weight: 1.0,
            enabled: true,
        });

        // Connection from new neuron to old target with original weight
        self.connections.push(ConnectionGene {
            innovation: innovation.next(),
            from: new_id,
            to: old_to,
            weight: old_weight,
            enabled: true,
        });
    }

    /// Crossover two genomes. `self` is the fitter parent.
    pub fn crossover(&self, other: &Genome, rng: &mut impl Rng) -> Genome {
        let mut child_neurons = self.neurons.clone();
        let mut child_connections = Vec::new();

        // Align connections by innovation number
        let mut i = 0;
        let mut j = 0;
        let self_conns = &self.connections;
        let other_conns = &other.connections;

        // Sort by innovation for alignment
        let mut s_sorted: Vec<&ConnectionGene> = self_conns.iter().collect();
        let mut o_sorted: Vec<&ConnectionGene> = other_conns.iter().collect();
        s_sorted.sort_by_key(|c| c.innovation);
        o_sorted.sort_by_key(|c| c.innovation);

        while i < s_sorted.len() && j < o_sorted.len() {
            let s = s_sorted[i];
            let o = o_sorted[j];

            if s.innovation == o.innovation {
                // Matching gene — random parent
                if rng.gen_bool(0.5) {
                    child_connections.push(s.clone());
                } else {
                    child_connections.push(o.clone());
                }
                i += 1;
                j += 1;
            } else if s.innovation < o.innovation {
                // Disjoint from fitter parent (self) — include
                child_connections.push(s.clone());
                i += 1;
            } else {
                // Disjoint from less fit parent — skip
                j += 1;
            }
        }

        // Excess genes from fitter parent
        while i < s_sorted.len() {
            child_connections.push(s_sorted[i].clone());
            i += 1;
        }

        // Include any hidden neurons from other parent that are referenced by child connections
        let child_neuron_ids: std::collections::HashSet<u64> = child_neurons.iter().map(|n| n.id).collect();
        for conn in &child_connections {
            for id in [conn.from, conn.to] {
                if !child_neuron_ids.contains(&id) {
                    if let Some(neuron) = other.neurons.iter().find(|n| n.id == id) {
                        child_neurons.push(neuron.clone());
                    }
                }
            }
        }

        // Interpolate body traits
        let t = rng.gen::<f32>();
        Genome {
            neurons: child_neurons,
            connections: child_connections,
            body_size: self.body_size * t + other.body_size * (1.0 - t),
            speed_factor: self.speed_factor * t + other.speed_factor * (1.0 - t),
            sense_range: self.sense_range * t + other.sense_range * (1.0 - t),
        }
    }

    /// Compute compatibility distance between two genomes (for speciation)
    pub fn compatibility_distance(&self, other: &Genome) -> f32 {
        let c1 = 1.0; // excess coefficient
        let c2 = 1.0; // disjoint coefficient
        let c3 = 0.4; // weight difference coefficient

        let mut s_sorted: Vec<&ConnectionGene> = self.connections.iter().collect();
        let mut o_sorted: Vec<&ConnectionGene> = other.connections.iter().collect();
        s_sorted.sort_by_key(|c| c.innovation);
        o_sorted.sort_by_key(|c| c.innovation);

        let mut matching = 0;
        let mut disjoint = 0;
        let mut weight_diff_sum = 0.0f32;
        let mut i = 0;
        let mut j = 0;

        while i < s_sorted.len() && j < o_sorted.len() {
            if s_sorted[i].innovation == o_sorted[j].innovation {
                matching += 1;
                weight_diff_sum += (s_sorted[i].weight - o_sorted[j].weight).abs();
                i += 1;
                j += 1;
            } else if s_sorted[i].innovation < o_sorted[j].innovation {
                disjoint += 1;
                i += 1;
            } else {
                disjoint += 1;
                j += 1;
            }
        }

        let excess = (s_sorted.len() - i) + (o_sorted.len() - j);
        let n = s_sorted.len().max(o_sorted.len()).max(1) as f32;
        let avg_weight_diff = if matching > 0 { weight_diff_sum / matching as f32 } else { 0.0 };

        // Also factor in body trait differences
        let body_diff = (self.body_size - other.body_size).abs()
            + (self.speed_factor - other.speed_factor).abs()
            + (self.sense_range - other.sense_range).abs() * 0.01;

        (c1 * excess as f32 / n) + (c2 * disjoint as f32 / n) + (c3 * avg_weight_diff) + body_diff * 0.5
    }
}
