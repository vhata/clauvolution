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
    // Not an iterator: this hands out fresh NEAT innovation numbers and never ends.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u64 {
        let n = self.0;
        self.0 += 1;
        n
    }
}

// --- Brain I/O ---

pub const NUM_INPUTS: usize = 26;
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
// 18: nearest_organism_signal (-1 to 1) — chemical signal emitted by nearest organism
// 19: nearby_same_species_count (0-1, normalized: 0=alone, 1=10+ nearby)
// 20: avg_nearby_same_species_signal (-1 to 1) — average signal of nearby kin
// 21: bias (always 1.0)
// 22: nearest_eater_dir_x (-1 to 1) — nearest non-photosynthesiser in sense range
// 23: nearest_eater_dir_y
// 24: nearest_eater_dist (0-1, 1 = touching, 0 = none in range)
// 25: nearest_eater_size_ratio (its size / own size, capped at 2, halved)
//
// Input neurons carry ids 0..NUM_INPUTS and output neurons the next
// NUM_OUTPUTS ids; hidden neurons take ids above those. Genomes written
// with fewer inputs are brought up to this layout by
// `Genome::migrate_input_layout`.

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
            symmetry: if rng.gen_bool(0.6) {
                Symmetry::Bilateral
            } else {
                Symmetry::None
            },
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

// --- Scalar trait bounds ---

/// Inclusive clamp range of one scalar trait. Mutation clamps to it and the
/// body term of `compatibility_distance` divides by its span, so the two
/// never disagree about how wide a trait is.
#[derive(Clone, Copy, Debug)]
pub struct TraitBounds {
    pub min: f32,
    pub max: f32,
}

impl TraitBounds {
    pub const fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    pub fn span(&self) -> f32 {
        self.max - self.min
    }

    pub fn clamp(&self, value: f32) -> f32 {
        value.clamp(self.min, self.max)
    }
}

pub const BODY_SIZE_BOUNDS: TraitBounds = TraitBounds::new(0.3, 2.0);
pub const SPEED_FACTOR_BOUNDS: TraitBounds = TraitBounds::new(0.2, 3.0);
pub const SENSE_RANGE_BOUNDS: TraitBounds = TraitBounds::new(10.0, 150.0);
pub const AQUATIC_ADAPTATION_BOUNDS: TraitBounds = TraitBounds::new(0.0, 1.0);
pub const PHOTOSYNTHESIS_RATE_BOUNDS: TraitBounds = TraitBounds::new(0.0, 1.0);
pub const ARMOR_BOUNDS: TraitBounds = TraitBounds::new(0.0, 1.0);
pub const ATTACK_POWER_BOUNDS: TraitBounds = TraitBounds::new(0.0, 1.0);
pub const DISEASE_RESISTANCE_BOUNDS: TraitBounds = TraitBounds::new(0.0, 1.0);
pub const SYMBIOSIS_RATE_BOUNDS: TraitBounds = TraitBounds::new(-1.0, 1.0);
pub const DIET_BOUNDS: TraitBounds = TraitBounds::new(-1.0, 1.0);

/// Photosynthesis rate above which an organism with a photo surface counts
/// as a photosynthesiser for classification, rendering, density competition
/// and niche construction. See `Genome::is_photosynthesiser`.
pub const PHOTOSYNTHESISER_RATE_THRESHOLD: f32 = 0.2;

/// Photosynthesis rate above which an organism with a photo surface earns
/// any sun at all. Deliberately far below `PHOTOSYNTHESISER_RATE_THRESHOLD`
/// so a lineage drifting toward plant-hood is paid for the first steps.
/// See `Genome::can_photosynthesise`.
pub const PHOTOSYNTHESIS_YIELD_THRESHOLD: f32 = 0.01;

/// Half-width of the band founders draw `diet` from when no spread is given:
/// the same near-neutral start as the other traits.
pub const DEFAULT_FOUNDER_DIET_SPREAD: f32 = 0.2;

/// Draw a founder's `diet` uniformly from `-spread..spread`, clamped to the
/// trait bounds; a zero spread is exactly neutral.
fn founder_diet(rng: &mut impl Rng, spread: f32) -> f32 {
    let s = spread.abs().min(DIET_BOUNDS.max);
    if s <= f32::EPSILON {
        0.0
    } else {
        rng.gen_range(-s..s)
    }
}

/// Number of scalar traits on the genome (every field after `body_segments`).
pub const SCALAR_TRAIT_COUNT: usize = 10;

/// Bounds of each scalar trait, in the same order as `Genome::scalar_traits`.
pub const SCALAR_TRAIT_BOUNDS: [TraitBounds; SCALAR_TRAIT_COUNT] = [
    BODY_SIZE_BOUNDS,
    SPEED_FACTOR_BOUNDS,
    SENSE_RANGE_BOUNDS,
    AQUATIC_ADAPTATION_BOUNDS,
    PHOTOSYNTHESIS_RATE_BOUNDS,
    ARMOR_BOUNDS,
    ATTACK_POWER_BOUNDS,
    DISEASE_RESISTANCE_BOUNDS,
    SYMBIOSIS_RATE_BOUNDS,
    DIET_BOUNDS,
];

// -----------------------------------------------------------------------------
// NEAT founder and mutation tuning constants
//
// Grouped here so the network-side tuning does not require hunting for
// literals scattered through `Genome::new_minimal_with_diet`, `Genome::mutate`,
// `mutate_add_connection` and `mutate_add_neuron`. Body-segment mutation
// lives on `BodySegmentGene::mutate`; trait clamps are the `*_BOUNDS` above.
// -----------------------------------------------------------------------------

/// Number of input-to-output connections a founder brain starts with, before
/// duplicates are dropped.
const FOUNDER_CONNECTION_COUNT: std::ops::RangeInclusive<usize> = 3..=8;
/// Bias range \[min, max) drawn for each founder output neuron.
const FOUNDER_OUTPUT_BIAS_RANGE: std::ops::Range<f32> = -1.0..1.0;
/// Weight range \[min, max) for a brand-new connection: founder connections,
/// `mutate_add_connection`, and a weight reset in `mutate`.
const NEW_WEIGHT_RANGE: std::ops::Range<f32> = -2.0..2.0;
/// Magnitude a connection weight or neuron bias is clamped to after a
/// perturbation. The reset range above is deliberately narrower.
const WEIGHT_CLAMP: f32 = 4.0;

/// Given that a connection weight mutates, the chance it is re-rolled from
/// `NEW_WEIGHT_RANGE` instead of perturbed by the normal step.
const WEIGHT_RESET_PROBABILITY: f32 = 0.1;
/// Per-mutation chance of a NEAT add-connection structural mutation.
const ADD_CONNECTION_PROBABILITY: f32 = 0.05;
/// Per-mutation chance of a NEAT add-neuron structural mutation (splits an
/// enabled connection).
const ADD_NEURON_PROBABILITY: f32 = 0.03;
/// Per-mutation chance of toggling one connection's enabled flag.
const TOGGLE_CONNECTION_PROBABILITY: f32 = 0.02;
/// Weight of the incoming half when `mutate_add_neuron` splits a connection;
/// the outgoing half keeps the old weight, so the split starts out neutral.
const SPLIT_INCOMING_WEIGHT: f32 = 1.0;

// Each `*_MUTATION_STEP` scales the normal sample added to that scalar trait
// when it mutates; the per-trait chance is the caller's `rate`, and the
// result is clamped to the matching `*_BOUNDS`.

/// Step scale for `body_size` (bounds 0.3..2.0).
const BODY_SIZE_MUTATION_STEP: f32 = 0.2;
/// Step scale for `speed_factor` (bounds 0.2..3.0).
const SPEED_FACTOR_MUTATION_STEP: f32 = 0.2;
/// Step scale for `sense_range`, in world units (bounds 10..150).
const SENSE_RANGE_MUTATION_STEP: f32 = 5.0;
/// Step scale for `aquatic_adaptation` (bounds 0..1).
const AQUATIC_ADAPTATION_MUTATION_STEP: f32 = 0.1;
/// Step scale for `photosynthesis_rate` (bounds 0..1).
const PHOTOSYNTHESIS_RATE_MUTATION_STEP: f32 = 0.05;
/// Step scale for `armor` (bounds 0..1).
const ARMOR_MUTATION_STEP: f32 = 0.05;
/// Step scale for `attack_power` (bounds 0..1).
const ATTACK_POWER_MUTATION_STEP: f32 = 0.05;
/// Step scale for `disease_resistance` (bounds 0..1).
const DISEASE_RESISTANCE_MUTATION_STEP: f32 = 0.05;
/// Step scale for `symbiosis_rate` (bounds -1..1).
const SYMBIOSIS_RATE_MUTATION_STEP: f32 = 0.1;
/// Step scale for `diet` (bounds -1..1).
const DIET_MUTATION_STEP: f32 = 0.1;

/// Multiplier on `rate` for each existing body segment's own mutation roll.
const SEGMENT_MUTATION_RATE_FACTOR: f32 = 0.5;
/// Per-mutation chance of growing a new random body segment.
const ADD_SEGMENT_PROBABILITY: f32 = 0.03;
/// Body segments (torso included) beyond which no new segment is grown.
const MAX_BODY_SEGMENTS: usize = 8;
/// Per-mutation chance of dropping a non-torso body segment.
const REMOVE_SEGMENT_PROBABILITY: f32 = 0.02;
/// Body segments (torso included) at or below which none is removed.
const MIN_BODY_SEGMENTS: usize = 2;

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
    pub disease_resistance: f32, // 0.0 = vulnerable, 1.0 = fully resistant
    /// Behaviour toward a stable symbiotic partner. -1.0 = full parasite
    /// (drain energy from partner), 0.0 = neutral (no transfer), +1.0 =
    /// full donor (gift energy to partner). Evolution decides what works.
    pub symbiosis_rate: f32,
    /// Digestive specialisation. -1.0 = pure herbivore (digests plant tissue
    /// fully, animal tissue not at all), +1.0 = pure carnivore, 0.0 = a
    /// generalist that digests a quarter of each. See `plant_efficiency`
    /// and `animal_efficiency`.
    pub diet: f32,
}

impl Genome {
    /// Create a minimal starting genome with the default founder diet spread.
    pub fn new_minimal(innovation: &mut InnovationCounter, rng: &mut impl Rng) -> Self {
        Self::new_minimal_with_diet(innovation, rng, DEFAULT_FOUNDER_DIET_SPREAD)
    }

    /// Create a minimal starting genome whose `diet` is drawn uniformly from
    /// `-diet_spread..diet_spread` (`SimConfig::founder_diet_spread`).
    pub fn new_minimal_with_diet(
        innovation: &mut InnovationCounter,
        rng: &mut impl Rng,
        diet_spread: f32,
    ) -> Self {
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
                bias: rng.gen_range(FOUNDER_OUTPUT_BIAS_RANGE),
            });
        }

        let mut connections = Vec::new();
        let num_initial_connections = rng.gen_range(FOUNDER_CONNECTION_COUNT);
        for _ in 0..num_initial_connections {
            let from = rng.gen_range(0..NUM_INPUTS) as u64;
            let to = (NUM_INPUTS + rng.gen_range(0..NUM_OUTPUTS)) as u64;

            if connections
                .iter()
                .any(|c: &ConnectionGene| c.from == from && c.to == to)
            {
                continue;
            }

            connections.push(ConnectionGene {
                innovation: innovation.next(),
                from,
                to,
                weight: rng.gen_range(NEW_WEIGHT_RANGE),
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
            disease_resistance: rng.gen_range(0.0..0.2),
            // Start near-neutral; selection decides whether parasitism or
            // mutualism pays off in this world.
            symbiosis_rate: rng.gen_range(-0.2..0.2),
            // The squared efficiency curve makes the middle a real cost, so
            // the spread decides whether founders start as generalists or
            // already span the axis; see `docs/DECISIONS.md`, "Grazing".
            diet: founder_diet(rng, diet_spread),
        }
    }

    /// Create a dedicated photosynthesizer genome with the default founder
    /// diet spread.
    pub fn new_photosynthesizer(innovation: &mut InnovationCounter, rng: &mut impl Rng) -> Self {
        Self::new_photosynthesizer_with_diet(innovation, rng, DEFAULT_FOUNDER_DIET_SPREAD)
    }

    /// Create a dedicated photosynthesizer genome; see `new_minimal_with_diet`.
    pub fn new_photosynthesizer_with_diet(
        innovation: &mut InnovationCounter,
        rng: &mut impl Rng,
        diet_spread: f32,
    ) -> Self {
        let mut genome = Self::new_minimal_with_diet(innovation, rng, diet_spread);

        // High photosynthesis rate
        genome.photosynthesis_rate = rng.gen_range(0.4..0.8);

        // Guarantee photo surfaces — replace some body parts
        genome.body_segments = vec![
            BodySegmentGene {
                segment_type: SegmentType::Torso,
                size: rng.gen_range(0.5..1.0),
                attachment_angle: 0.0,
                attachment_slot: 0,
                symmetry: Symmetry::Bilateral,
            },
            BodySegmentGene {
                segment_type: SegmentType::PhotoSurface,
                size: rng.gen_range(0.5..1.2),
                attachment_angle: rng.gen_range(-1.0..1.0),
                attachment_slot: 1,
                symmetry: Symmetry::Bilateral,
            },
            BodySegmentGene {
                segment_type: SegmentType::PhotoSurface,
                size: rng.gen_range(0.4..1.0),
                attachment_angle: rng.gen_range(-1.0..1.0),
                attachment_slot: 2,
                symmetry: Symmetry::Bilateral,
            },
        ];

        // Plants are slower, smaller, less aggressive
        genome.body_size = rng.gen_range(0.3..0.8);
        genome.speed_factor = rng.gen_range(0.2..0.6);
        genome.attack_power = 0.0;
        genome.armor = rng.gen_range(0.0..0.2);

        genome
    }

    /// Derived traits from body segments
    pub fn has_fins(&self) -> bool {
        self.body_segments
            .iter()
            .any(|s| s.segment_type == SegmentType::Fin)
    }

    pub fn has_claws(&self) -> bool {
        self.body_segments
            .iter()
            .any(|s| s.segment_type == SegmentType::Claw)
    }

    pub fn has_armor(&self) -> bool {
        self.body_segments
            .iter()
            .any(|s| s.segment_type == SegmentType::ArmorPlate)
    }

    pub fn claw_power(&self) -> f32 {
        self.body_segments
            .iter()
            .filter(|s| s.segment_type == SegmentType::Claw)
            .map(|s| s.size)
            .sum::<f32>()
            + self.attack_power
    }

    pub fn armor_value(&self) -> f32 {
        self.body_segments
            .iter()
            .filter(|s| s.segment_type == SegmentType::ArmorPlate)
            .map(|s| s.size)
            .sum::<f32>()
            + self.armor
    }

    pub fn has_limbs(&self) -> bool {
        self.body_segments
            .iter()
            .any(|s| s.segment_type == SegmentType::Limb)
    }

    pub fn has_eyes(&self) -> bool {
        self.body_segments
            .iter()
            .any(|s| s.segment_type == SegmentType::Eye)
    }

    pub fn has_mouth(&self) -> bool {
        self.body_segments
            .iter()
            .any(|s| s.segment_type == SegmentType::Mouth)
    }

    pub fn has_photo_surface(&self) -> bool {
        self.body_segments
            .iter()
            .any(|s| s.segment_type == SegmentType::PhotoSurface)
    }

    /// Whether this organism is a plant: a photo surface and a photosynthesis
    /// rate above `PHOTOSYNTHESISER_RATE_THRESHOLD`. The one identity rule
    /// used by strategy classification, rendering, plant density competition
    /// and niche construction.
    pub fn is_photosynthesiser(&self) -> bool {
        self.photosynthesis_rate > PHOTOSYNTHESISER_RATE_THRESHOLD && self.has_photo_surface()
    }

    /// Whether this organism earns any sun: a photo surface and a rate above
    /// `PHOTOSYNTHESIS_YIELD_THRESHOLD`. Looser than `is_photosynthesiser`
    /// on purpose; see that constant.
    pub fn can_photosynthesise(&self) -> bool {
        self.photosynthesis_rate > PHOTOSYNTHESIS_YIELD_THRESHOLD && self.has_photo_surface()
    }

    /// Fraction of plant tissue this organism can digest, from `diet`:
    /// `((1 - diet) / 2)^2`. 1.0 for a pure herbivore, 0.25 for a
    /// generalist, 0.0 for a pure carnivore.
    pub fn plant_efficiency(&self) -> f32 {
        let h = (1.0 - DIET_BOUNDS.clamp(self.diet)) / 2.0;
        h * h
    }

    /// Fraction of animal tissue this organism can digest, from `diet`:
    /// `((1 + diet) / 2)^2`. 0.0 for a pure herbivore, 0.25 for a
    /// generalist, 1.0 for a pure carnivore.
    pub fn animal_efficiency(&self) -> f32 {
        let c = (1.0 + DIET_BOUNDS.clamp(self.diet)) / 2.0;
        c * c
    }

    pub fn total_photo_surface_area(&self) -> f32 {
        self.body_segments
            .iter()
            .filter(|s| s.segment_type == SegmentType::PhotoSurface)
            .map(|s| s.size)
            .sum()
    }

    pub fn fin_area(&self) -> f32 {
        self.body_segments
            .iter()
            .filter(|s| s.segment_type == SegmentType::Fin)
            .map(|s| s.size)
            .sum()
    }

    pub fn limb_count(&self) -> usize {
        self.body_segments
            .iter()
            .filter(|s| s.segment_type == SegmentType::Limb)
            .count()
    }

    pub fn eye_count(&self) -> usize {
        self.body_segments
            .iter()
            .filter(|s| s.segment_type == SegmentType::Eye)
            .count()
    }

    /// Effective sense range, boosted by eyes
    pub fn effective_sense_range(&self) -> f32 {
        let eye_bonus = self.eye_count() as f32 * 10.0;
        self.sense_range + eye_bonus
    }

    /// The scalar traits in `SCALAR_TRAIT_BOUNDS` order.
    pub fn scalar_traits(&self) -> [f32; SCALAR_TRAIT_COUNT] {
        [
            self.body_size,
            self.speed_factor,
            self.sense_range,
            self.aquatic_adaptation,
            self.photosynthesis_rate,
            self.armor,
            self.attack_power,
            self.disease_resistance,
            self.symbiosis_rate,
            self.diet,
        ]
    }

    /// Mutate this genome in place
    pub fn mutate(
        &mut self,
        innovation: &mut InnovationCounter,
        rng: &mut impl Rng,
        rate: f32,
        strength: f32,
    ) {
        let normal = Normal::new(0.0, strength as f64).unwrap();

        // Mutate connection weights
        for conn in &mut self.connections {
            if rng.gen::<f32>() < rate {
                if rng.gen::<f32>() < WEIGHT_RESET_PROBABILITY {
                    conn.weight = rng.gen_range(NEW_WEIGHT_RANGE);
                } else {
                    conn.weight += normal.sample(rng) as f32;
                    conn.weight = conn.weight.clamp(-WEIGHT_CLAMP, WEIGHT_CLAMP);
                }
            }
        }

        // Mutate neuron biases
        for neuron in &mut self.neurons {
            if neuron.neuron_type != NeuronType::Input && rng.gen::<f32>() < rate {
                neuron.bias += normal.sample(rng) as f32;
                neuron.bias = neuron.bias.clamp(-WEIGHT_CLAMP, WEIGHT_CLAMP);
            }
        }

        // Structural mutations
        if rng.gen::<f32>() < ADD_CONNECTION_PROBABILITY {
            self.mutate_add_connection(innovation, rng);
        }
        if rng.gen::<f32>() < ADD_NEURON_PROBABILITY {
            self.mutate_add_neuron(innovation, rng);
        }
        if !self.connections.is_empty() && rng.gen::<f32>() < TOGGLE_CONNECTION_PROBABILITY {
            let idx = rng.gen_range(0..self.connections.len());
            self.connections[idx].enabled = !self.connections[idx].enabled;
        }

        // Mutate body traits
        if rng.gen::<f32>() < rate {
            self.body_size += normal.sample(rng) as f32 * BODY_SIZE_MUTATION_STEP;
            self.body_size = BODY_SIZE_BOUNDS.clamp(self.body_size);
        }
        if rng.gen::<f32>() < rate {
            self.speed_factor += normal.sample(rng) as f32 * SPEED_FACTOR_MUTATION_STEP;
            self.speed_factor = SPEED_FACTOR_BOUNDS.clamp(self.speed_factor);
        }
        if rng.gen::<f32>() < rate {
            self.sense_range += normal.sample(rng) as f32 * SENSE_RANGE_MUTATION_STEP;
            self.sense_range = SENSE_RANGE_BOUNDS.clamp(self.sense_range);
        }
        if rng.gen::<f32>() < rate {
            self.aquatic_adaptation += normal.sample(rng) as f32 * AQUATIC_ADAPTATION_MUTATION_STEP;
            self.aquatic_adaptation = AQUATIC_ADAPTATION_BOUNDS.clamp(self.aquatic_adaptation);
        }
        if rng.gen::<f32>() < rate {
            self.photosynthesis_rate +=
                normal.sample(rng) as f32 * PHOTOSYNTHESIS_RATE_MUTATION_STEP;
            self.photosynthesis_rate = PHOTOSYNTHESIS_RATE_BOUNDS.clamp(self.photosynthesis_rate);
        }
        if rng.gen::<f32>() < rate {
            self.armor += normal.sample(rng) as f32 * ARMOR_MUTATION_STEP;
            self.armor = ARMOR_BOUNDS.clamp(self.armor);
        }
        if rng.gen::<f32>() < rate {
            self.attack_power += normal.sample(rng) as f32 * ATTACK_POWER_MUTATION_STEP;
            self.attack_power = ATTACK_POWER_BOUNDS.clamp(self.attack_power);
        }
        if rng.gen::<f32>() < rate {
            self.disease_resistance += normal.sample(rng) as f32 * DISEASE_RESISTANCE_MUTATION_STEP;
            self.disease_resistance = DISEASE_RESISTANCE_BOUNDS.clamp(self.disease_resistance);
        }
        if rng.gen::<f32>() < rate {
            self.symbiosis_rate += normal.sample(rng) as f32 * SYMBIOSIS_RATE_MUTATION_STEP;
            self.symbiosis_rate = SYMBIOSIS_RATE_BOUNDS.clamp(self.symbiosis_rate);
        }
        if rng.gen::<f32>() < rate {
            self.diet += normal.sample(rng) as f32 * DIET_MUTATION_STEP;
            self.diet = DIET_BOUNDS.clamp(self.diet);
        }

        // Mutate existing body segments
        for seg in &mut self.body_segments {
            if rng.gen::<f32>() < rate * SEGMENT_MUTATION_RATE_FACTOR {
                seg.mutate(rng, strength);
            }
        }

        // Add a body segment
        if rng.gen::<f32>() < ADD_SEGMENT_PROBABILITY
            && self.body_segments.len() < MAX_BODY_SEGMENTS
        {
            self.body_segments.push(BodySegmentGene::random(rng));
        }

        // Remove a body segment (never the torso)
        if rng.gen::<f32>() < REMOVE_SEGMENT_PROBABILITY
            && self.body_segments.len() > MIN_BODY_SEGMENTS
        {
            let idx = rng.gen_range(1..self.body_segments.len());
            self.body_segments.remove(idx);
        }
    }

    fn mutate_add_connection(&mut self, innovation: &mut InnovationCounter, rng: &mut impl Rng) {
        let non_input: Vec<u64> = self
            .neurons
            .iter()
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
        if self
            .connections
            .iter()
            .any(|c| c.from == from && c.to == to)
        {
            return;
        }

        self.connections.push(ConnectionGene {
            innovation: innovation.next(),
            from,
            to,
            weight: rng.gen_range(NEW_WEIGHT_RANGE),
            enabled: true,
        });
    }

    fn mutate_add_neuron(&mut self, innovation: &mut InnovationCounter, rng: &mut impl Rng) {
        let enabled: Vec<usize> = self
            .connections
            .iter()
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
            weight: SPLIT_INCOMING_WEIGHT,
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

    /// Crossover two genomes. `self` supplies the child's structure and
    /// `other` only varies it.
    ///
    /// The sim has no fitness score, so `self` is not the fitter parent in
    /// the NEAT sense: `reproduction_system` passes the parent that initiated
    /// the mating and pays for the child, which is whichever of the pair it
    /// visits first in query order.
    ///
    /// - Connections: every gene of `self`. Where both parents carry the same
    ///   innovation, the copy (weight and enabled flag) is picked from either
    ///   parent at random. `other`'s disjoint and excess genes are dropped, so
    ///   the child's topology is exactly `self`'s.
    /// - Neurons: all of `self`'s. `other`'s are added only if a child
    ///   connection names a neuron `self` lacks, which matching genes from
    ///   shared descent do not.
    /// - Body segments: `self`'s list 70% of the time; otherwise `self`'s
    ///   torso followed by a per-slot pick from either parent.
    /// - Scalar traits, including diet: each blended with its own factor,
    ///   symmetric in the two parents.
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

        let child_neuron_ids: std::collections::HashSet<u64> =
            child_neurons.iter().map(|n| n.id).collect();
        for conn in &child_connections {
            for id in [conn.from, conn.to] {
                if !child_neuron_ids.contains(&id) {
                    if let Some(neuron) = other.neurons.iter().find(|n| n.id == id) {
                        child_neurons.push(neuron.clone());
                    }
                }
            }
        }

        // Crossover body segments: mostly `self`'s, with some mixing
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

        // Each scalar trait draws its own blend factor, so a child can carry
        // one parent's speed and the other's armour instead of lying on the
        // straight line between them in trait space.
        let mut blend = |a: f32, b: f32| {
            let t = rng.gen::<f32>();
            a * t + b * (1.0 - t)
        };
        Genome {
            neurons: child_neurons,
            connections: child_connections,
            body_segments: child_segments,
            body_size: blend(self.body_size, other.body_size),
            speed_factor: blend(self.speed_factor, other.speed_factor),
            sense_range: blend(self.sense_range, other.sense_range),
            aquatic_adaptation: blend(self.aquatic_adaptation, other.aquatic_adaptation),
            photosynthesis_rate: blend(self.photosynthesis_rate, other.photosynthesis_rate),
            armor: blend(self.armor, other.armor),
            attack_power: blend(self.attack_power, other.attack_power),
            disease_resistance: blend(self.disease_resistance, other.disease_resistance),
            symbiosis_rate: blend(self.symbiosis_rate, other.symbiosis_rate),
            diet: blend(self.diet, other.diet),
        }
    }

    /// Compatibility distance between two genomes, used for speciation.
    ///
    /// Four terms, each roughly 0..1 before weighting: excess and disjoint
    /// connection counts over the larger gene count, the mean weight
    /// difference of matching connections, and the body term from
    /// `body_trait_distance`. The body term carries weight 1.0 and the three
    /// NEAT terms 0.5 each, so species are trait-led. See
    /// `docs/DECISIONS.md`, "Species classification".
    pub fn compatibility_distance(&self, other: &Genome) -> f32 {
        let c1 = 0.5;
        let c2 = 0.5;
        let c3 = 0.5;
        let c_body = 1.0;

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
        let avg_weight_diff = if matching > 0 {
            weight_diff_sum / matching as f32
        } else {
            0.0
        };

        (c1 * excess as f32 / n)
            + (c2 * disjoint as f32 / n)
            + (c3 * avg_weight_diff)
            + c_body * self.body_trait_distance(other)
    }

    /// Mean over the scalar traits of the absolute difference divided by the
    /// trait's clamp span, so one trait at opposite ends of its range
    /// contributes `1 / SCALAR_TRAIT_COUNT` and the whole term lies in 0..1.
    /// Each per-trait share is capped at 1.0 so values outside the bounds
    /// (founders, old saves) cannot push the term above 1.0.
    pub fn body_trait_distance(&self, other: &Genome) -> f32 {
        let a = self.scalar_traits();
        let b = other.scalar_traits();
        let sum: f32 = a
            .iter()
            .zip(b.iter())
            .zip(SCALAR_TRAIT_BOUNDS.iter())
            .map(|((x, y), bounds)| ((x - y).abs() / bounds.span()).min(1.0))
            .sum();
        sum / SCALAR_TRAIT_COUNT as f32
    }

    /// Bring a genome written with fewer brain inputs up to the current
    /// layout, so it behaves exactly as it did. Returns whether anything
    /// changed.
    ///
    /// Inputs occupy ids `0..NUM_INPUTS` and outputs follow them, so a
    /// genome from an older layout with `k` inputs has its outputs at
    /// `k..`. Every id at or above `k` (outputs and hidden neurons) moves up
    /// by `NUM_INPUTS - k`, connections are rewritten to match, and the
    /// missing inputs are added with ids `k..NUM_INPUTS` and no
    /// connections. Innovation numbers are untouched: the connections are
    /// the same genes between the same neurons. The brain reads inputs and
    /// outputs by sorted id, which the shift preserves, and a new input with
    /// no connections contributes nothing, so the brain's outputs for the
    /// old inputs are unchanged. A genome whose inputs are not exactly
    /// `0..k` is not a layout this knows and is left alone. See "Brain
    /// inputs grow by migration" in `docs/DECISIONS.md`.
    pub fn migrate_input_layout(&mut self) -> bool {
        let mut input_ids: Vec<u64> = self
            .neurons
            .iter()
            .filter(|n| n.neuron_type == NeuronType::Input)
            .map(|n| n.id)
            .collect();
        if input_ids.len() >= NUM_INPUTS {
            return false;
        }
        input_ids.sort_unstable();
        if input_ids.iter().enumerate().any(|(i, &id)| id != i as u64) {
            return false;
        }

        let old_count = input_ids.len() as u64;
        let shift = NUM_INPUTS as u64 - old_count;
        let remap = |id: u64| if id >= old_count { id + shift } else { id };
        for neuron in &mut self.neurons {
            neuron.id = remap(neuron.id);
        }
        for conn in &mut self.connections {
            conn.from = remap(conn.from);
            conn.to = remap(conn.to);
        }

        // New inputs go directly after the last existing one, so a founder's
        // neuron list keeps its inputs-then-outputs order.
        let insert_at = self
            .neurons
            .iter()
            .rposition(|n| n.neuron_type == NeuronType::Input)
            .map_or(0, |i| i + 1);
        let new_inputs = (old_count..NUM_INPUTS as u64).map(|id| NeuronGene {
            id,
            neuron_type: NeuronType::Input,
            activation: ActivationFn::Sigmoid,
            bias: 0.0,
        });
        self.neurons.splice(insert_at..insert_at, new_inputs);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn base_genome(seed: u64) -> Genome {
        let mut innovation = InnovationCounter(0);
        let mut rng = StdRng::seed_from_u64(seed);
        Genome::new_minimal(&mut innovation, &mut rng)
    }

    /// Set trait `index` (in `SCALAR_TRAIT_BOUNDS` order) to `value`.
    fn set_trait(genome: &mut Genome, index: usize, value: f32) {
        match index {
            0 => genome.body_size = value,
            1 => genome.speed_factor = value,
            2 => genome.sense_range = value,
            3 => genome.aquatic_adaptation = value,
            4 => genome.photosynthesis_rate = value,
            5 => genome.armor = value,
            6 => genome.attack_power = value,
            7 => genome.disease_resistance = value,
            8 => genome.symbiosis_rate = value,
            9 => genome.diet = value,
            _ => panic!("no scalar trait at index {index}"),
        }
    }

    #[test]
    fn scalar_traits_match_bounds_order() {
        let mut genome = base_genome(1);
        for (index, bounds) in SCALAR_TRAIT_BOUNDS.iter().enumerate() {
            set_trait(&mut genome, index, bounds.max);
            assert_eq!(genome.scalar_traits()[index], bounds.max, "trait {index}");
        }
    }

    #[test]
    fn identical_genomes_have_zero_distance() {
        let a = base_genome(7);
        let b = a.clone();
        assert_eq!(a.compatibility_distance(&b), 0.0);
        assert_eq!(a.body_trait_distance(&b), 0.0);
    }

    #[test]
    fn one_trait_across_its_full_range_contributes_one_over_n() {
        let expected = 1.0 / SCALAR_TRAIT_COUNT as f32;
        for (index, bounds) in SCALAR_TRAIT_BOUNDS.iter().enumerate() {
            let mut a = base_genome(3);
            let mut b = a.clone();
            set_trait(&mut a, index, bounds.min);
            set_trait(&mut b, index, bounds.max);
            let body = a.body_trait_distance(&b);
            assert!(
                (body - expected).abs() < 1e-6,
                "trait {index}: body term {body}"
            );
            // Brains are identical, so the NEAT terms are zero and the total
            // is the body term alone.
            let total = a.compatibility_distance(&b);
            assert!(
                (total - expected).abs() < 1e-6,
                "trait {index}: total {total}"
            );
        }
    }

    #[test]
    fn body_term_never_exceeds_one() {
        let mut a = base_genome(5);
        let mut b = a.clone();
        for (index, bounds) in SCALAR_TRAIT_BOUNDS.iter().enumerate() {
            set_trait(&mut a, index, bounds.min);
            set_trait(&mut b, index, bounds.max);
        }
        assert!((a.body_trait_distance(&b) - 1.0).abs() < 1e-6);

        // Values outside the clamp bounds (founders, old saves) are capped
        // per trait rather than pushing the term past 1.0.
        for (index, bounds) in SCALAR_TRAIT_BOUNDS.iter().enumerate() {
            set_trait(&mut a, index, bounds.min - 10.0 * bounds.span());
            set_trait(&mut b, index, bounds.max + 10.0 * bounds.span());
        }
        assert!(a.body_trait_distance(&b) <= 1.0);
    }

    #[test]
    fn diet_efficiencies_follow_the_squared_curve() {
        let mut g = base_genome(2);
        g.diet = -1.0;
        assert!((g.plant_efficiency() - 1.0).abs() < 1e-6);
        assert!(g.animal_efficiency().abs() < 1e-6);
        g.diet = 1.0;
        assert!(g.plant_efficiency().abs() < 1e-6);
        assert!((g.animal_efficiency() - 1.0).abs() < 1e-6);
        g.diet = 0.0;
        assert!((g.plant_efficiency() - 0.25).abs() < 1e-6);
        assert!((g.animal_efficiency() - 0.25).abs() < 1e-6);
        // Out-of-range values (old saves) are clamped, not extrapolated.
        g.diet = 3.0;
        assert!((g.animal_efficiency() - 1.0).abs() < 1e-6);
        assert!(g.plant_efficiency().abs() < 1e-6);
    }

    #[test]
    fn founder_diet_spread_is_honoured() {
        let mut innovation = InnovationCounter(0);
        let mut rng = StdRng::seed_from_u64(8);
        for _ in 0..50 {
            let g = Genome::new_minimal_with_diet(&mut innovation, &mut rng, 0.0);
            assert_eq!(g.diet, 0.0);
            let g = Genome::new_minimal_with_diet(&mut innovation, &mut rng, 1.0);
            assert!(g.diet > -1.0 && g.diet < 1.0);
            let g = Genome::new_minimal(&mut innovation, &mut rng);
            assert!(g.diet.abs() < DEFAULT_FOUNDER_DIET_SPREAD);
        }
        // Over many draws at full spread, both specialist thirds are reached.
        let mut lo = false;
        let mut hi = false;
        for _ in 0..200 {
            let g = Genome::new_minimal_with_diet(&mut innovation, &mut rng, 1.0);
            lo |= g.diet < -1.0 / 3.0;
            hi |= g.diet > 1.0 / 3.0;
        }
        assert!(lo && hi);
    }

    #[test]
    fn diet_mutates_within_bounds() {
        let mut innovation = InnovationCounter(0);
        let mut rng = StdRng::seed_from_u64(4);
        let mut g = Genome::new_minimal(&mut innovation, &mut rng);
        let start = g.diet;
        let mut moved = false;
        for _ in 0..2000 {
            g.mutate(&mut innovation, &mut rng, 1.0, 1.0);
            assert!(g.diet >= DIET_BOUNDS.min && g.diet <= DIET_BOUNDS.max);
            moved |= g.diet != start;
        }
        assert!(moved, "diet never mutated in 2000 rounds at rate 1.0");
    }

    #[test]
    fn photosynthesiser_predicates_use_their_thresholds() {
        let mut g = base_genome(6);
        g.body_segments.push(BodySegmentGene {
            segment_type: SegmentType::PhotoSurface,
            size: 1.0,
            attachment_angle: 0.0,
            attachment_slot: 0,
            symmetry: Symmetry::None,
        });
        g.photosynthesis_rate = 0.1;
        assert!(g.can_photosynthesise());
        assert!(!g.is_photosynthesiser());
        g.photosynthesis_rate = 0.5;
        assert!(g.is_photosynthesiser());
        // No photo surface: neither, whatever the rate.
        g.body_segments
            .retain(|s| s.segment_type != SegmentType::PhotoSurface);
        assert!(!g.can_photosynthesise());
        assert!(!g.is_photosynthesiser());
    }

    #[test]
    fn crossover_blends_each_trait_independently() {
        let mut fast_soft = base_genome(11);
        fast_soft.speed_factor = SPEED_FACTOR_BOUNDS.max;
        fast_soft.armor = ARMOR_BOUNDS.min;
        let mut slow_hard = fast_soft.clone();
        slow_hard.speed_factor = SPEED_FACTOR_BOUNDS.min;
        slow_hard.armor = ARMOR_BOUNDS.max;

        let mut rng = StdRng::seed_from_u64(99);
        let mut off_line = 0;
        for _ in 0..500 {
            let child = fast_soft.crossover(&slow_hard, &mut rng);
            // With a single shared blend factor t, the child's normalised
            // position along the speed axis and the armour axis would be
            // equal (t and 1 - t respectively, summing to 1). Independent
            // factors let the child sit near parent A's speed and parent
            // B's armour at the same time.
            let speed_pos =
                (child.speed_factor - SPEED_FACTOR_BOUNDS.min) / SPEED_FACTOR_BOUNDS.span();
            let armor_pos = (child.armor - ARMOR_BOUNDS.min) / ARMOR_BOUNDS.span();
            if speed_pos > 0.8 && armor_pos > 0.8 {
                off_line += 1;
            }
        }
        assert!(
            off_line > 0,
            "no offspring combined A's speed with B's armour in 500 trials"
        );
    }

    /// A genome in the 22-input layout that preceded the nearest-eater
    /// inputs: inputs 0..22, outputs 22..31, one hidden neuron at 31 with a
    /// self-loop.
    fn legacy_22_input_genome() -> Genome {
        let mut genome = base_genome(5);
        genome.neurons.clear();
        genome.connections.clear();
        for id in 0..22u64 {
            genome.neurons.push(NeuronGene {
                id,
                neuron_type: NeuronType::Input,
                activation: ActivationFn::Sigmoid,
                bias: 0.0,
            });
        }
        for id in 22..31u64 {
            genome.neurons.push(NeuronGene {
                id,
                neuron_type: NeuronType::Output,
                activation: ActivationFn::Tanh,
                bias: 0.1,
            });
        }
        genome.neurons.push(NeuronGene {
            id: 31,
            neuron_type: NeuronType::Hidden,
            activation: ActivationFn::Relu,
            bias: 0.0,
        });
        for (innovation, (from, to)) in [(0, 22), (21, 26), (4, 31), (31, 26), (31, 31)]
            .into_iter()
            .enumerate()
        {
            genome.connections.push(ConnectionGene {
                innovation: innovation as u64 + 100,
                from,
                to,
                weight: 0.5,
                enabled: true,
            });
        }
        genome
    }

    #[test]
    fn legacy_genome_migrates_to_the_current_input_layout() {
        let mut genome = legacy_22_input_genome();
        assert!(genome.migrate_input_layout());

        let ids_of = |kind: NeuronType| -> Vec<u64> {
            let mut ids: Vec<u64> = genome
                .neurons
                .iter()
                .filter(|n| n.neuron_type == kind)
                .map(|n| n.id)
                .collect();
            ids.sort_unstable();
            ids
        };
        let shift = (NUM_INPUTS - 22) as u64;
        assert_eq!(
            ids_of(NeuronType::Input),
            (0..NUM_INPUTS as u64).collect::<Vec<_>>()
        );
        assert_eq!(
            ids_of(NeuronType::Output),
            (NUM_INPUTS as u64..(NUM_INPUTS + NUM_OUTPUTS) as u64).collect::<Vec<_>>()
        );
        assert_eq!(ids_of(NeuronType::Hidden), vec![31 + shift]);

        // Same genes, same innovations, endpoints moved with their neurons.
        let edges: Vec<(u64, u64, u64)> = genome
            .connections
            .iter()
            .map(|c| (c.innovation, c.from, c.to))
            .collect();
        assert_eq!(
            edges,
            vec![
                (100, 0, 22 + shift),
                (101, 21, 26 + shift),
                (102, 4, 31 + shift),
                (103, 31 + shift, 26 + shift),
                (104, 31 + shift, 31 + shift),
            ]
        );
        // The new inputs are unconnected.
        for id in 22..NUM_INPUTS as u64 {
            assert!(genome
                .connections
                .iter()
                .all(|c| c.from != id && c.to != id));
        }
        // Inputs stay ahead of the outputs in the neuron list.
        let first_output = genome
            .neurons
            .iter()
            .position(|n| n.neuron_type == NeuronType::Output)
            .unwrap();
        assert_eq!(first_output, NUM_INPUTS);

        assert!(!genome.migrate_input_layout(), "migration is idempotent");
    }

    #[test]
    fn current_and_unrecognised_layouts_are_left_alone() {
        let mut current = base_genome(8);
        let before: Vec<u64> = current.neurons.iter().map(|n| n.id).collect();
        assert!(!current.migrate_input_layout());
        assert_eq!(
            current.neurons.iter().map(|n| n.id).collect::<Vec<_>>(),
            before
        );

        // Inputs that are not 0..k are not a known older layout.
        let mut odd = legacy_22_input_genome();
        odd.neurons[3].id = 500;
        assert!(!odd.migrate_input_layout());
    }
}
