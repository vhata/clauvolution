use bevy::prelude::*;
use clauvolution_brain::Brain;
use clauvolution_core::*;
use clauvolution_genome::*;
use clauvolution_phylogeny::{PhyloNode, PhyloTree, SpeciesStrategy, WorldChronicle};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// Why a save could not be written. Surfaced to the log and the chronicle
/// rather than panicking, so a full disk or a permissions problem costs the
/// user one save, not the running world.
#[derive(Debug)]
pub enum SaveError {
    /// The world state could not be serialised to JSON.
    Serialize(serde_json::Error),
    /// The save file could not be written to `path`.
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::Serialize(e) => write!(f, "could not serialise world state: {}", e),
            SaveError::Write { path, source } => {
                write!(f, "could not write {}: {}", path.display(), source)
            }
        }
    }
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SaveError::Serialize(e) => Some(e),
            SaveError::Write { source, .. } => Some(source),
        }
    }
}

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

/// The genome as it is written to a save file.
///
/// Every field is optional on load: the container-level `serde(default)`
/// fills a missing field from `Default for SaveGenome`, so a save written
/// before a trait existed still loads, with that trait at a neutral value
/// rather than failing the whole file. A new field must be added to the
/// `Default` impl below (the compiler insists), which is where its neutral
/// value is chosen and documented. See "Save format: every genome field has
/// a default" in `docs/DECISIONS.md`.
#[derive(Serialize, Deserialize)]
#[serde(default)]
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
    pub disease_resistance: f32,
    pub symbiosis_rate: f32,
    pub diet: f32,
}

impl Default for SaveGenome {
    /// The value a field takes when a save file does not carry it.
    ///
    /// Traits whose zero is a valid "trait absent" reading (no armour, no
    /// claws, a land-dweller, a neutral symbiont, a generalist diet) default
    /// to zero. The three whose zero lies outside `*_BOUNDS` or describes a
    /// degenerate body take the midpoint of the founder range in
    /// `Genome::new_minimal_with_diet`, so a loaded organism is an ordinary
    /// founder rather than a point with no size, speed or senses. An empty
    /// brain or body is not a usable genome; `validate_save_state` drops
    /// such an organism with a warning instead of aborting the load.
    fn default() -> Self {
        Self {
            neurons: Vec::new(),
            connections: Vec::new(),
            body_segments: Vec::new(),
            // Founders draw 0.5..1.5.
            body_size: 1.0,
            // Founders draw 0.5..1.5.
            speed_factor: 1.0,
            // Founders draw 30.0..80.0, in world units.
            sense_range: 55.0,
            aquatic_adaptation: 0.0,
            photosynthesis_rate: 0.0,
            armor: 0.0,
            attack_power: 0.0,
            disease_resistance: 0.0,
            symbiosis_rate: 0.0,
            // Absent in saves from before the diet axis; 0.0 is the generalist.
            diet: 0.0,
        }
    }
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
        neurons: g
            .neurons
            .iter()
            .map(|n| SaveNeuron {
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
            })
            .collect(),
        connections: g
            .connections
            .iter()
            .map(|c| SaveConnection {
                innovation: c.innovation,
                from: c.from,
                to: c.to,
                weight: c.weight,
                enabled: c.enabled,
            })
            .collect(),
        body_segments: g
            .body_segments
            .iter()
            .map(|s| SaveBodySegment {
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
                symmetry: match s.symmetry {
                    Symmetry::None => 0,
                    Symmetry::Bilateral => 1,
                },
            })
            .collect(),
        body_size: g.body_size,
        speed_factor: g.speed_factor,
        sense_range: g.sense_range,
        aquatic_adaptation: g.aquatic_adaptation,
        photosynthesis_rate: g.photosynthesis_rate,
        armor: g.armor,
        attack_power: g.attack_power,
        disease_resistance: g.disease_resistance,
        symbiosis_rate: g.symbiosis_rate,
        diet: g.diet,
    }
}

fn save_to_genome(s: &SaveGenome) -> Genome {
    Genome {
        neurons: s
            .neurons
            .iter()
            .map(|n| NeuronGene {
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
            })
            .collect(),
        connections: s
            .connections
            .iter()
            .map(|c| ConnectionGene {
                innovation: c.innovation,
                from: c.from,
                to: c.to,
                weight: c.weight,
                enabled: c.enabled,
            })
            .collect(),
        body_segments: s
            .body_segments
            .iter()
            .map(|seg| BodySegmentGene {
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
                symmetry: match seg.symmetry {
                    0 => Symmetry::None,
                    _ => Symmetry::Bilateral,
                },
            })
            .collect(),
        body_size: s.body_size,
        speed_factor: s.speed_factor,
        sense_range: s.sense_range,
        aquatic_adaptation: s.aquatic_adaptation,
        photosynthesis_rate: s.photosynthesis_rate,
        armor: s.armor,
        attack_power: s.attack_power,
        disease_resistance: s.disease_resistance,
        symbiosis_rate: s.symbiosis_rate,
        diet: s.diet,
    }
}

/// Save the current simulation state to a file.
///
/// The JSON is written to a sibling temporary file and renamed into place,
/// so a write that fails part way (disk full, permissions) leaves any
/// previous save at `path` intact. Returns the error instead of panicking;
/// the caller decides how to surface it.
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
) -> Result<(), SaveError> {
    let state = SaveState {
        tick: tick.0,
        season_tick: season.current_tick,
        terrain_seed: config.terrain_seed,
        stats: SaveStats {
            total_births: stats.total_births,
            total_deaths: stats.total_deaths,
            max_generation: stats.max_generation,
        },
        organisms: organisms
            .iter()
            .map(
                |(pos, energy, health, age, gen, species, signal, memory, genome)| SaveOrganism {
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
                },
            )
            .collect(),
        food: food
            .iter()
            .map(|(pos, energy)| SaveFood {
                x: pos.x,
                y: pos.y,
                energy: *energy,
            })
            .collect(),
        innovation_counter: innovation.0,
        phylo_nodes: phylo
            .nodes
            .values()
            .map(|n| SavePhyloNode {
                species_id: n.species_id,
                parent_id: n.parent_id,
                born_tick: n.born_tick,
                extinct_tick: n.extinct_tick,
                peak_population: n.peak_population,
                // 0 plant, 1 hunter (was predator), 2 grazer (was forager),
                // 3 omnivore. Kept so pre-diet-axis saves map naturally.
                strategy: match n.strategy {
                    SpeciesStrategy::Photosynthesizer => 0,
                    SpeciesStrategy::Hunter => 1,
                    SpeciesStrategy::Grazer => 2,
                    SpeciesStrategy::Omnivore => 3,
                },
                name: n.name.clone(),
            })
            .collect(),
        chronicle_entries: chronicle
            .entries
            .iter()
            .map(|e| SaveChronicleEntry {
                tick: e.tick,
                text: e.text.clone(),
            })
            .collect(),
    };

    let json = serde_json::to_string(&state).map_err(SaveError::Serialize)?;
    write_atomically(path, json.as_bytes()).map_err(|source| SaveError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Write `bytes` to `path` via a temporary file in the same directory and a
/// rename, so `path` only ever holds a complete file. The temporary file is
/// removed on failure as far as the filesystem allows.
fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let result = std::fs::write(&tmp, bytes).and_then(|_| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
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
        warn!(
            "Save file had {} organism(s) with invalid genomes — skipped",
            removed
        );
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

/// Reconstruct organisms from save data. Returns the total energy spawned
/// so the caller can set the `EnergyLedger` baseline.
pub fn spawn_saved_organisms(commands: &mut Commands, organisms: &[SaveOrganism]) -> f64 {
    let mut total_energy = 0.0f64;
    for org in organisms {
        let genome = save_to_genome(&org.genome);
        let brain = Brain::from_genome(&genome);
        let body_size = genome.body_size;
        total_energy += org.energy as f64;

        commands
            .spawn((
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
            ))
            .insert((
                brain,
                LightShare::default(),
                genome,
                TrailHistory::default(),
                BrainActivations::default(),
                Symbiosis::default(),
                EnergyFlows::default(),
            ));
    }
    total_energy
}

pub fn spawn_saved_food(commands: &mut Commands, food: &[SaveFood]) {
    for f in food {
        commands.spawn((Food, FoodEnergy(f.energy), Position(Vec2::new(f.x, f.y))));
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
                1 => SpeciesStrategy::Hunter,
                2 => SpeciesStrategy::Grazer,
                _ => SpeciesStrategy::Omnivore,
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
        chronicle
            .entries
            .push(clauvolution_phylogeny::ChronicleEntry {
                tick: e.tick,
                text: e.text.clone(),
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique scratch directory under the system temp dir, removed on drop.
    struct ScratchDir(PathBuf);

    impl ScratchDir {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "clauvolution-save-test-{}-{}",
                tag,
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn save_empty_world(path: &Path) -> Result<(), SaveError> {
        save_world(
            path,
            &TickCounter(7),
            &Season::default(),
            &SimStats::default(),
            &InnovationCounter(100),
            &SimConfig::default(),
            &[],
            &[],
            &PhyloTree::default(),
            &WorldChronicle::default(),
        )
    }

    #[test]
    fn save_to_missing_directory_returns_error_instead_of_panicking() {
        let scratch = ScratchDir::new("missing-dir");
        let path = scratch.0.join("does-not-exist").join("save.json");
        let err = save_empty_world(&path).expect_err("write into a missing directory must fail");
        match &err {
            SaveError::Write { path: p, .. } => assert_eq!(p, &path),
            other => panic!("expected a write error, got {:?}", other),
        }
        assert!(err.to_string().contains("save.json"));
        assert!(!path.exists());
    }

    #[test]
    fn save_to_writable_directory_round_trips_through_load() {
        let scratch = ScratchDir::new("round-trip");
        let path = scratch.0.join("save.json");
        save_empty_world(&path).expect("save into a writable directory");
        assert!(path.exists());
        assert!(!scratch.0.join("save.json.tmp").exists());
        let state = load_world(&path).expect("the written file loads");
        assert_eq!(state.tick, 7);
        assert!(state.organisms.is_empty());
    }

    #[test]
    fn failed_save_leaves_the_previous_file_untouched() {
        let scratch = ScratchDir::new("keep-previous");
        // Occupy the temp-file path with a directory so the write fails
        // before the rename can replace the existing save.
        let path = scratch.0.join("save.json");
        std::fs::write(&path, b"previous").unwrap();
        std::fs::create_dir(scratch.0.join("save.json.tmp")).unwrap();
        save_empty_world(&path).expect_err("writing over a directory must fail");
        assert_eq!(std::fs::read(&path).unwrap(), b"previous");
    }

    /// A genome with no fields at all parses: every field has a default, so a
    /// save written before any given trait existed cannot fail on that trait.
    #[test]
    fn genome_with_no_fields_loads_with_the_documented_defaults() {
        let g: SaveGenome = serde_json::from_str("{}").expect("an empty genome object parses");
        let d = SaveGenome::default();
        assert!(g.neurons.is_empty());
        assert!(g.connections.is_empty());
        assert!(g.body_segments.is_empty());
        assert_eq!(g.body_size, d.body_size);
        assert_eq!(g.speed_factor, d.speed_factor);
        assert_eq!(g.sense_range, d.sense_range);
        assert_eq!(g.aquatic_adaptation, d.aquatic_adaptation);
        assert_eq!(g.photosynthesis_rate, d.photosynthesis_rate);
        assert_eq!(g.armor, d.armor);
        assert_eq!(g.attack_power, d.attack_power);
        assert_eq!(g.disease_resistance, d.disease_resistance);
        assert_eq!(g.symbiosis_rate, d.symbiosis_rate);
        assert_eq!(g.diet, d.diet);
    }

    /// The defaults for the three traits whose zero is out of bounds must be
    /// values a founder could have, or a loaded organism starts degenerate.
    #[test]
    fn genome_defaults_lie_within_trait_bounds() {
        let d = SaveGenome::default();
        assert_eq!(BODY_SIZE_BOUNDS.clamp(d.body_size), d.body_size);
        assert_eq!(SPEED_FACTOR_BOUNDS.clamp(d.speed_factor), d.speed_factor);
        assert_eq!(SENSE_RANGE_BOUNDS.clamp(d.sense_range), d.sense_range);
        assert_ne!(d.body_size, 0.0);
        assert_ne!(d.speed_factor, 0.0);
        assert_ne!(d.sense_range, 0.0);
    }

    /// A save whose genomes predate several traits loads through the real
    /// `load_world` path, keeps the organism, and fills the missing traits.
    #[test]
    fn save_with_older_genome_loads_and_keeps_the_organism() {
        let scratch = ScratchDir::new("older-genome");
        let path = scratch.0.join("save.json");
        // One organism with a brain and a torso, but a genome written before
        // `sense_range`, `armor`, `disease_resistance` and `diet` existed.
        let json = r#"{
            "tick": 3, "season_tick": 0, "terrain_seed": 42,
            "stats": {"total_births": 0, "total_deaths": 0, "max_generation": 0},
            "organisms": [{
                "x": 1.0, "y": 2.0, "energy": 50.0, "health": 100.0, "age": 0,
                "generation": 0, "species_id": 1, "signal": 0.0, "memory": [0.0, 0.0, 0.0],
                "genome": {
                    "neurons": [{"id": 0, "neuron_type": 0, "activation": 0, "bias": 0.0}],
                    "connections": [],
                    "body_segments": [{"segment_type": 0, "size": 1.0,
                        "attachment_angle": 0.0, "attachment_slot": 0, "symmetry": 1}],
                    "body_size": 0.7,
                    "speed_factor": 1.3
                }
            }],
            "food": [], "innovation_counter": 1, "phylo_nodes": [], "chronicle_entries": []
        }"#;
        std::fs::write(&path, json).unwrap();
        let state = load_world(&path).expect("a save missing newer genome fields loads");
        assert_eq!(state.organisms.len(), 1, "the organism must not be dropped");
        let g = &state.organisms[0].genome;
        assert_eq!(g.body_size, 0.7);
        assert_eq!(g.speed_factor, 1.3);
        let d = SaveGenome::default();
        assert_eq!(g.sense_range, d.sense_range);
        assert_eq!(g.armor, d.armor);
        assert_eq!(g.disease_resistance, d.disease_resistance);
        assert_eq!(g.diet, d.diet);
    }
}
