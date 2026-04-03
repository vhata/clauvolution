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

// --- Brain I/O ---

pub const NUM_INPUTS: usize = 19;
pub const NUM_OUTPUTS: usize = 9;
pub const NUM_MEMORY: usize = 3;

// Inputs:
//  0: energy_level (0-1)
//  1: nearest_food_dir_x (-1 to 1)
//  2: nearest_food_dir_y (-1 to 1)
//  3: nearest_food_dist (0-1, normalized)
//  4: nearest_organism_dir_x
//  5: nearest_organism_dir_y
//  6: nearest_organism_dist
//  7: nearest_organism_size_ratio
//  8: terrain_is_water (0 or 1)
//  9: terrain_nutrients (0-1)
// 10: light_level (0-1)
// 11: own_aquatic_adaptation (0-1)
// 12: own_health (0-1) — damage taken
// 13: nearest_organism_is_same_species (0 or 1)
// 14: memory_0 (from previous tick)
// 15: memory_1
// 16: memory_2
// 17: nearest_organism_photo_hint (0=predator-like, 1=plant-like)
// 18: bias (always 1.0)

// Outputs:
//  0: move_x (-1 to 1)
//  1: move_y (-1 to 1)
//  2: eat (> 0.5 = attempt eat food)
//  3: reproduce (> 0.5 = attempt reproduce)
//  4: attack (> 0.5 = attempt attack nearest organism)
//  5: signal_0 — chemical signal emission
//  6: memory_out_0
//  7: memory_out_1
//  8: memory_out_2

// --- Neuron / Connection genes ---

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

#[derive(Clone, Debug)]
pub struct ConnectionGene {
    pub innovation: u64,
    pub from: u64,
    pub to: u64,
    pub weight: f32,
    pub enabled: bool,
}

// --- Body segment genes ---

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentType {
    Torso,
    Limb,
    Fin,
    Eye,
    Mouth,
    PhotoSurface,
    Claw,
    ArmorPlate,
}

impl SegmentType {
    pub fn random(rng: &mut impl Rng) -> Self {
        match rng.gen_range(0..8) {
            0 => SegmentType::Torso,
            1 => SegmentType::Limb,
            2 => SegmentType::Fin,
            3 => SegmentType::Eye,
            4 => SegmentType::Mouth,
            5 => SegmentType::PhotoSurface,
            6 => SegmentType::Claw,
            _ => SegmentType::ArmorPlate,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Symmetry {
    None,
    Bilateral,
}

#[derive(Clone, Debug)]
pub struct BodySegmentGene {
    pub segment_type: SegmentType,
    pub size: f32,
    pub attachment_angle: f32,
    pub attachment_slot: u8,
    pub symmetry: Symmetry,
}

impl BodySegmentGene {
    pub fn random(rng: &mut impl Rng) -> Self {
        Self {
            segment_type: SegmentType::random(rng),
            size: rng.gen_range(0.3..1.5),
            attachment_angle: rng.gen_range(-std::f32::consts::PI..std::f32::consts::PI),
            attachment_slot: rng.gen_range(0..4),
            symmetry: if rng.gen_bool(0.6) { Symmetry::Bilateral } else { Symmetry::None },
        }
    }

    pub fn mutate(&mut self, rng: &mut impl Rng, strength: f32) {
        let normal = Normal::new(0.0, strength as f64).unwrap();

        if rng.gen::<f32>() < 0.05 {
            self.segment_type = SegmentType::random(rng);
        }
        self.size += normal.sample(rng) as f32 * 0.2;
        self.size = self.size.clamp(0.1, 2.5);
        self.attachment_angle += normal.sample(rng) as f32 * 0.3;
        if rng.gen::<f32>() < 0.02 {
            self.symmetry = if self.symmetry == Symmetry::Bilateral {
                Symmetry::None
            } else {
                Symmetry::Bilateral
            };
        }
    }
}

// --- Full genome ---

#[derive(Component, Clone, Debug)]
pub struct Genome {
    pub neurons: Vec<NeuronGene>,
    pub connections: Vec<ConnectionGene>,
    pub body_segments: Vec<BodySegmentGene>,
    pub body_size: f32,
    pub speed_factor: f32,
    pub sense_range: f32,
    pub aquatic_adaptation: f32,
    pub photosynthesis_rate: f32,
    pub armor: f32,
    pub attack_power: f32,
}

impl Genome {
    /// Create a minimal starting genome
    pub fn new_minimal(innovation: &mut InnovationCounter, rng: &mut impl Rng) -> Self {
        let mut neurons = Vec::new();

        for i in 0..NUM_INPUTS {
            neurons.push(NeuronGene {
                id: i as u64,
                neuron_type: NeuronType::Input,
                activation: ActivationFn::Sigmoid,
                bias: 0.0,
            });
        }

        for i in 0..NUM_OUTPUTS {
            neurons.push(NeuronGene {
                id: (NUM_INPUTS + i) as u64,
                neuron_type: NeuronType::Output,
                activation: ActivationFn::Tanh,
                bias: rng.gen_range(-1.0..1.0),
            });
        }

        let mut connections = Vec::new();
        let num_initial_connections = rng.gen_range(3..=8);
        for _ in 0..num_initial_connections {
            let from = rng.gen_range(0..NUM_INPUTS) as u64;
            let to = (NUM_INPUTS + rng.gen_range(0..NUM_OUTPUTS)) as u64;

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

        // Start with a torso + 1-2 random body parts
        let mut body_segments = vec![BodySegmentGene {
            segment_type: SegmentType::Torso,
            size: rng.gen_range(0.6..1.2),
            attachment_angle: 0.0,
            attachment_slot: 0,
            symmetry: Symmetry::Bilateral,
        }];

        let extra_parts = rng.gen_range(1..=3);
        for _ in 0..extra_parts {
            body_segments.push(BodySegmentGene::random(rng));
        }

        Self {
            neurons,
            connections,
            body_segments,
            body_size: rng.gen_range(0.5..1.5),
            speed_factor: rng.gen_range(0.5..1.5),
            sense_range: rng.gen_range(30.0..80.0),
            aquatic_adaptation: rng.gen_range(0.0..0.5),
            photosynthesis_rate: rng.gen_range(0.0..0.1),
            armor: rng.gen_range(0.0..0.1),
            attack_power: rng.gen_range(0.0..0.1),
        }
    }

    /// Derived traits from body segments
    pub fn has_fins(&self) -> bool {
        self.body_segments.iter().any(|s| s.segment_type == SegmentType::Fin)
    }

    pub fn has_claws(&self) -> bool {
        self.body_segments.iter().any(|s| s.segment_type == SegmentType::Claw)
    }

    pub fn has_armor(&self) -> bool {
        self.body_segments.iter().any(|s| s.segment_type == SegmentType::ArmorPlate)
    }

    pub fn claw_power(&self) -> f32 {
        self.body_segments.iter()
            .filter(|s| s.segment_type == SegmentType::Claw)
            .map(|s| s.size)
            .sum::<f32>()
            + self.attack_power
    }

    pub fn armor_value(&self) -> f32 {
        self.body_segments.iter()
            .filter(|s| s.segment_type == SegmentType::ArmorPlate)
            .map(|s| s.size)
            .sum::<f32>()
            + self.armor
    }

    pub fn has_limbs(&self) -> bool {
        self.body_segments.iter().any(|s| s.segment_type == SegmentType::Limb)
    }

    pub fn has_eyes(&self) -> bool {
        self.body_segments.iter().any(|s| s.segment_type == SegmentType::Eye)
    }

    pub fn has_mouth(&self) -> bool {
        self.body_segments.iter().any(|s| s.segment_type == SegmentType::Mouth)
    }

    pub fn has_photo_surface(&self) -> bool {
        self.body_segments.iter().any(|s| s.segment_type == SegmentType::PhotoSurface)
    }

    pub fn total_photo_surface_area(&self) -> f32 {
        self.body_segments.iter()
            .filter(|s| s.segment_type == SegmentType::PhotoSurface)
            .map(|s| s.size)
            .sum()
    }

    pub fn fin_area(&self) -> f32 {
        self.body_segments.iter()
            .filter(|s| s.segment_type == SegmentType::Fin)
            .map(|s| s.size)
            .sum()
    }

    pub fn limb_count(&self) -> usize {
        self.body_segments.iter()
            .filter(|s| s.segment_type == SegmentType::Limb)
            .count()
    }

    pub fn eye_count(&self) -> usize {
        self.body_segments.iter()
            .filter(|s| s.segment_type == SegmentType::Eye)
            .count()
    }

    /// Effective sense range, boosted by eyes
    pub fn effective_sense_range(&self) -> f32 {
        let eye_bonus = self.eye_count() as f32 * 10.0;
        self.sense_range + eye_bonus
    }

    /// Mutate this genome in place
    pub fn mutate(&mut self, innovation: &mut InnovationCounter, rng: &mut impl Rng, rate: f32, strength: f32) {
        let normal = Normal::new(0.0, strength as f64).unwrap();

        // Mutate connection weights
        for conn in &mut self.connections {
            if rng.gen::<f32>() < rate {
                if rng.gen::<f32>() < 0.1 {
                    conn.weight = rng.gen_range(-2.0..2.0);
                } else {
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

        // Structural mutations
        if rng.gen::<f32>() < 0.05 {
            self.mutate_add_connection(innovation, rng);
        }
        if rng.gen::<f32>() < 0.03 {
            self.mutate_add_neuron(innovation, rng);
        }
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
        if rng.gen::<f32>() < rate {
            self.aquatic_adaptation += normal.sample(rng) as f32 * 0.1;
            self.aquatic_adaptation = self.aquatic_adaptation.clamp(0.0, 1.0);
        }
        if rng.gen::<f32>() < rate {
            self.photosynthesis_rate += normal.sample(rng) as f32 * 0.05;
            self.photosynthesis_rate = self.photosynthesis_rate.clamp(0.0, 1.0);
        }
        if rng.gen::<f32>() < rate {
            self.armor += normal.sample(rng) as f32 * 0.05;
            self.armor = self.armor.clamp(0.0, 1.0);
        }
        if rng.gen::<f32>() < rate {
            self.attack_power += normal.sample(rng) as f32 * 0.05;
            self.attack_power = self.attack_power.clamp(0.0, 1.0);
        }

        // Mutate existing body segments
        for seg in &mut self.body_segments {
            if rng.gen::<f32>() < rate * 0.5 {
                seg.mutate(rng, strength);
            }
        }

        // Add a body segment (probability 0.03)
        if rng.gen::<f32>() < 0.03 && self.body_segments.len() < 8 {
            self.body_segments.push(BodySegmentGene::random(rng));
        }

        // Remove a body segment (probability 0.02, never remove torso)
        if rng.gen::<f32>() < 0.02 && self.body_segments.len() > 2 {
            let idx = rng.gen_range(1..self.body_segments.len());
            self.body_segments.remove(idx);
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

        self.connections.push(ConnectionGene {
            innovation: innovation.next(),
            from: old_from,
            to: new_id,
            weight: 1.0,
            enabled: true,
        });

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

        let mut s_sorted: Vec<&ConnectionGene> = self.connections.iter().collect();
        let mut o_sorted: Vec<&ConnectionGene> = other.connections.iter().collect();
        s_sorted.sort_by_key(|c| c.innovation);
        o_sorted.sort_by_key(|c| c.innovation);

        let mut i = 0;
        let mut j = 0;

        while i < s_sorted.len() && j < o_sorted.len() {
            let s = s_sorted[i];
            let o = o_sorted[j];

            if s.innovation == o.innovation {
                if rng.gen_bool(0.5) {
                    child_connections.push(s.clone());
                } else {
                    child_connections.push(o.clone());
                }
                i += 1;
                j += 1;
            } else if s.innovation < o.innovation {
                child_connections.push(s.clone());
                i += 1;
            } else {
                j += 1;
            }
        }

        while i < s_sorted.len() {
            child_connections.push(s_sorted[i].clone());
            i += 1;
        }

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

        // Crossover body segments: take from fitter parent with some mixing
        let child_segments = if rng.gen_bool(0.7) {
            self.body_segments.clone()
        } else {
            // Mix: take torso from self, then randomly pick from either parent
            let mut segs = vec![self.body_segments[0].clone()];
            let max_len = self.body_segments.len().max(other.body_segments.len());
            for idx in 1..max_len {
                if rng.gen_bool(0.5) {
                    if idx < self.body_segments.len() {
                        segs.push(self.body_segments[idx].clone());
                    }
                } else if idx < other.body_segments.len() {
                    segs.push(other.body_segments[idx].clone());
                }
            }
            segs
        };

        let t = rng.gen::<f32>();
        Genome {
            neurons: child_neurons,
            connections: child_connections,
            body_segments: child_segments,
            body_size: self.body_size * t + other.body_size * (1.0 - t),
            speed_factor: self.speed_factor * t + other.speed_factor * (1.0 - t),
            sense_range: self.sense_range * t + other.sense_range * (1.0 - t),
            aquatic_adaptation: self.aquatic_adaptation * t + other.aquatic_adaptation * (1.0 - t),
            photosynthesis_rate: self.photosynthesis_rate * t + other.photosynthesis_rate * (1.0 - t),
            armor: self.armor * t + other.armor * (1.0 - t),
            attack_power: self.attack_power * t + other.attack_power * (1.0 - t),
        }
    }

    /// Compute compatibility distance between two genomes (for speciation)
    pub fn compatibility_distance(&self, other: &Genome) -> f32 {
        let c1 = 1.0;
        let c2 = 1.0;
        let c3 = 0.4;

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

        let body_diff = (self.body_size - other.body_size).abs()
            + (self.speed_factor - other.speed_factor).abs()
            + (self.sense_range - other.sense_range).abs() * 0.01
            + (self.aquatic_adaptation - other.aquatic_adaptation).abs()
            + (self.photosynthesis_rate - other.photosynthesis_rate).abs()
            + (self.armor - other.armor).abs()
            + (self.attack_power - other.attack_power).abs();

        (c1 * excess as f32 / n) + (c2 * disjoint as f32 / n) + (c3 * avg_weight_diff) + body_diff * 0.5
    }
}
