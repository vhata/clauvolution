use bevy::prelude::*;
use clauvolution_core::*;
use clauvolution_genome::*;
use clauvolution_brain::Brain;
use clauvolution_phylogeny::{PhyloTree, PhyloNode, WorldChronicle, SpeciesStrategy};
use serde::{Serialize, Deserialize};
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct SaveState {
    pub tick: u64,
    pub season_tick: u64,
    pub terrain_seed: u64,
    pub stats: SaveStats,
    pub organisms: Vec<SaveOrganism>,
    pub food: Vec<SaveFood>,
    pub innovation_counter: u64,
    pub phylo_nodes: Vec<SavePhyloNode>,
    pub chronicle_entries: Vec<SaveChronicleEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct SaveStats {
    pub total_births: u64,
    pub total_deaths: u64,
    pub max_generation: u32,
}

#[derive(Serialize, Deserialize)]
pub struct SaveOrganism {
    pub x: f32,
    pub y: f32,
    pub energy: f32,
    pub health: f32,
    pub age: u64,
    pub generation: u32,
    pub species_id: u64,
    pub signal: f32,
    pub memory: [f32; 3],
    pub genome: SaveGenome,
}

#[derive(Serialize, Deserialize)]
pub struct SaveGenome {
    pub neurons: Vec<SaveNeuron>,
    pub connections: Vec<SaveConnection>,
    pub body_segments: Vec<SaveBodySegment>,
    pub body_size: f32,
    pub speed_factor: f32,
    pub sense_range: f32,
    pub aquatic_adaptation: f32,
    pub photosynthesis_rate: f32,
    pub armor: f32,
    pub attack_power: f32,
    #[serde(default)]
    pub disease_resistance: f32,
    #[serde(default)]
    pub symbiosis_rate: f32,
}

#[derive(Serialize, Deserialize)]
pub struct SaveNeuron {
    pub id: u64,
    pub neuron_type: u8, // 0=Input, 1=Hidden, 2=Output
    pub activation: u8,  // 0=Sigmoid, 1=Tanh, 2=Relu
    pub bias: f32,
}

#[derive(Serialize, Deserialize)]
pub struct SaveConnection {
    pub innovation: u64,
    pub from: u64,
    pub to: u64,
    pub weight: f32,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize)]
pub struct SaveBodySegment {
    pub segment_type: u8,
    pub size: f32,
    pub attachment_angle: f32,
    pub attachment_slot: u8,
    pub symmetry: u8,
}

#[derive(Serialize, Deserialize)]
pub struct SaveFood {
    pub x: f32,
    pub y: f32,
    pub energy: f32,
}

#[derive(Serialize, Deserialize)]
pub struct SavePhyloNode {
    pub species_id: u64,
    pub parent_id: Option<u64>,
    pub born_tick: u64,
    pub extinct_tick: Option<u64>,
    pub peak_population: u32,
    pub strategy: u8,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct SaveChronicleEntry {
    pub tick: u64,
    pub text: String,
}

// --- Conversion helpers ---

fn genome_to_save(g: &Genome) -> SaveGenome {
    SaveGenome {
        neurons: g.neurons.iter().map(|n| SaveNeuron {
            id: n.id,
            neuron_type: match n.neuron_type {
                NeuronType::Input => 0,
                NeuronType::Hidden => 1,
                NeuronType::Output => 2,
            },
            activation: match n.activation {
                ActivationFn::Sigmoid => 0,
                ActivationFn::Tanh => 1,
                ActivationFn::Relu => 2,
            },
            bias: n.bias,
        }).collect(),
        connections: g.connections.iter().map(|c| SaveConnection {
            innovation: c.innovation,
            from: c.from,
            to: c.to,
            weight: c.weight,
            enabled: c.enabled,
        }).collect(),
        body_segments: g.body_segments.iter().map(|s| SaveBodySegment {
            segment_type: match s.segment_type {
                SegmentType::Torso => 0,
                SegmentType::Limb => 1,
                SegmentType::Fin => 2,
                SegmentType::Eye => 3,
                SegmentType::Mouth => 4,
                SegmentType::PhotoSurface => 5,
                SegmentType::Claw => 6,
                SegmentType::ArmorPlate => 7,
            },
            size: s.size,
            attachment_angle: s.attachment_angle,
            attachment_slot: s.attachment_slot,
            symmetry: match s.symmetry { Symmetry::None => 0, Symmetry::Bilateral => 1 },
        }).collect(),
        body_size: g.body_size,
        speed_factor: g.speed_factor,
        sense_range: g.sense_range,
        aquatic_adaptation: g.aquatic_adaptation,
        photosynthesis_rate: g.photosynthesis_rate,
        armor: g.armor,
        attack_power: g.attack_power,
        disease_resistance: g.disease_resistance,
        symbiosis_rate: g.symbiosis_rate,
    }
}

fn save_to_genome(s: &SaveGenome) -> Genome {
    Genome {
        neurons: s.neurons.iter().map(|n| NeuronGene {
            id: n.id,
            neuron_type: match n.neuron_type {
                0 => NeuronType::Input,
                1 => NeuronType::Hidden,
                _ => NeuronType::Output,
            },
            activation: match n.activation {
                0 => ActivationFn::Sigmoid,
                1 => ActivationFn::Tanh,
                2 => ActivationFn::Relu,
                _ => ActivationFn::Relu,
            },
            bias: n.bias,
        }).collect(),
        connections: s.connections.iter().map(|c| ConnectionGene {
            innovation: c.innovation,
            from: c.from,
            to: c.to,
            weight: c.weight,
            enabled: c.enabled,
        }).collect(),
        body_segments: s.body_segments.iter().map(|seg| BodySegmentGene {
            segment_type: match seg.segment_type {
                0 => SegmentType::Torso,
                1 => SegmentType::Limb,
                2 => SegmentType::Fin,
                3 => SegmentType::Eye,
                4 => SegmentType::Mouth,
                5 => SegmentType::PhotoSurface,
                6 => SegmentType::Claw,
                _ => SegmentType::ArmorPlate,
            },
            size: seg.size,
            attachment_angle: seg.attachment_angle,
            attachment_slot: seg.attachment_slot,
            symmetry: match seg.symmetry { 0 => Symmetry::None, _ => Symmetry::Bilateral },
        }).collect(),
        body_size: s.body_size,
        speed_factor: s.speed_factor,
        sense_range: s.sense_range,
        aquatic_adaptation: s.aquatic_adaptation,
        photosynthesis_rate: s.photosynthesis_rate,
        armor: s.armor,
        attack_power: s.attack_power,
        disease_resistance: s.disease_resistance,
        symbiosis_rate: s.symbiosis_rate,
    }
}

/// Save the current simulation state to a file
pub fn save_world(
    path: &Path,
    tick: &TickCounter,
    season: &Season,
    stats: &SimStats,
    innovation: &InnovationCounter,
    config: &SimConfig,
    organisms: &[(Vec2, f32, f32, u64, u32, u64, f32, [f32; 3], Genome)],
    food: &[(Vec2, f32)],
    phylo: &PhyloTree,
    chronicle: &WorldChronicle,
) {
    let state = SaveState {
        tick: tick.0,
        season_tick: season.current_tick,
        terrain_seed: config.terrain_seed,
        stats: SaveStats {
            total_births: stats.total_births,
            total_deaths: stats.total_deaths,
            max_generation: stats.max_generation,
        },
        organisms: organisms.iter().map(|(pos, energy, health, age, gen, species, signal, memory, genome)| {
            SaveOrganism {
                x: pos.x,
                y: pos.y,
                energy: *energy,
                health: *health,
                age: *age,
                generation: *gen,
                species_id: *species,
                signal: *signal,
                memory: *memory,
                genome: genome_to_save(genome),
            }
        }).collect(),
        food: food.iter().map(|(pos, energy)| SaveFood {
            x: pos.x,
            y: pos.y,
            energy: *energy,
        }).collect(),
        innovation_counter: innovation.0,
        phylo_nodes: phylo.nodes.values().map(|n| SavePhyloNode {
            species_id: n.species_id,
            parent_id: n.parent_id,
            born_tick: n.born_tick,
            extinct_tick: n.extinct_tick,
            peak_population: n.peak_population,
            strategy: match n.strategy {
                SpeciesStrategy::Photosynthesizer => 0,
                SpeciesStrategy::Predator => 1,
                SpeciesStrategy::Forager => 2,
            },
            name: n.name.clone(),
        }).collect(),
        chronicle_entries: chronicle.entries.iter().map(|e| SaveChronicleEntry {
            tick: e.tick,
            text: e.text.clone(),
        }).collect(),
    };

    let json = serde_json::to_string(&state).expect("Failed to serialize save state");
    std::fs::write(path, json).expect("Failed to write save file");
    info!("World saved to {}", path.display());
}

/// Load simulation state from a file. Performs a basic structural sanity
/// check and logs warnings for any anomalies found — the caller still gets
/// a valid SaveState, but individual broken organisms are filtered out.
pub fn load_world(path: &Path) -> Option<SaveState> {
    let json = std::fs::read_to_string(path).ok()?;
    let mut state: SaveState = match serde_json::from_str(&json) {
        Ok(s) => s,
        Err(e) => {
            warn!("Save file {} could not be parsed: {}", path.display(), e);
            return None;
        }
    };

    validate_save_state(&mut state);
    Some(state)
}

/// Drop any organisms that fail basic sanity checks; log a count if any
/// are removed. Non-fatal — the sim starts with the survivors.
fn validate_save_state(state: &mut SaveState) {
    let before = state.organisms.len();
    state.organisms.retain(|org| {
        // Genome must have at least a torso body segment
        if org.genome.body_segments.is_empty() {
            return false;
        }
        // Genome must have some neurons (otherwise the brain can't be built)
        if org.genome.neurons.is_empty() {
            return false;
        }
        // Every enabled connection must reference real neuron IDs
        let neuron_ids: std::collections::HashSet<u64> =
            org.genome.neurons.iter().map(|n| n.id).collect();
        for conn in &org.genome.connections {
            if !neuron_ids.contains(&conn.from) || !neuron_ids.contains(&conn.to) {
                return false;
            }
        }
        true
    });
    let removed = before - state.organisms.len();
    if removed > 0 {
        warn!("Save file had {} organism(s) with invalid genomes — skipped", removed);
    }

    // Clamp position components into finite numbers — NaN/inf would crash the spatial hash
    for org in &mut state.organisms {
        if !org.x.is_finite() {
            warn!("Save organism x was non-finite ({}); snapping to 0", org.x);
            org.x = 0.0;
        }
        if !org.y.is_finite() {
            warn!("Save organism y was non-finite ({}); snapping to 0", org.y);
            org.y = 0.0;
        }
    }
}

/// Reconstruct organisms from save data
pub fn spawn_saved_organisms(
    commands: &mut Commands,
    organisms: &[SaveOrganism],
) {
    for org in organisms {
        let genome = save_to_genome(&org.genome);
        let brain = Brain::from_genome(&genome);
        let body_size = genome.body_size;

        commands.spawn((
            Organism,
            Energy(org.energy),
            Health(org.health),
            Position(Vec2::new(org.x, org.y)),
            Velocity(Vec2::ZERO),
            BodySize(body_size),
            Age(org.age),
            Generation(org.generation),
            SpeciesId(org.species_id),
            crate::BrainOutput::default(),
            BrainMemory(org.memory),
            ActionFlash::default(),
            Signal(org.signal),
            GroupSize::default(),
            ParentInfo::default(), // parent info not preserved in saves
        )).insert((brain, genome, TrailHistory::default(), BrainActivations::default(), Symbiosis::default()));
    }
}

pub fn spawn_saved_food(commands: &mut Commands, food: &[SaveFood]) {
    for f in food {
        commands.spawn((
            Food,
            FoodEnergy(f.energy),
            Position(Vec2::new(f.x, f.y)),
        ));
    }
}

pub fn restore_phylo(phylo: &mut PhyloTree, nodes: &[SavePhyloNode]) {
    for n in nodes {
        let node = PhyloNode {
            species_id: n.species_id,
            parent_id: n.parent_id,
            born_tick: n.born_tick,
            extinct_tick: n.extinct_tick,
            peak_population: n.peak_population,
            current_population: 0,
            strategy: match n.strategy {
                0 => SpeciesStrategy::Photosynthesizer,
                1 => SpeciesStrategy::Predator,
                _ => SpeciesStrategy::Forager,
            },
            color: Color::WHITE, // will be reassigned by species classification
            name: n.name.clone(),
        };
        if n.parent_id.is_none() {
            phylo.root_species.push(n.species_id);
        }
        phylo.nodes.insert(n.species_id, node);
    }
}

pub fn restore_chronicle(chronicle: &mut WorldChronicle, entries: &[SaveChronicleEntry]) {
    for e in entries {
        chronicle.entries.push(clauvolution_phylogeny::ChronicleEntry {
            tick: e.tick,
            text: e.text.clone(),
        });
    }
}
