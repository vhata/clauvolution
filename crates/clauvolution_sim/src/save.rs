use bevy::prelude::*;
use clauvolution_brain::Brain;
use clauvolution_core::*;
use clauvolution_genome::*;
use clauvolution_phylogeny::{
    ChronicleTarget, PhyloNode, PhyloTree, SpeciesStrategy, WorldChronicle,
};
use clauvolution_world::{Tile, TileMap};
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

/// The whole world as it is written to a save file.
///
/// Unlike `SaveGenome`, the fields fall into two deliberate groups. Fields
/// without a `serde(default)` are the ones the world cannot be rebuilt
/// without, and a file missing any of them is rejected: `tick` (every
/// phylogeny and chronicle tick is relative to it), `terrain_seed` (terrain
/// type, elevation and light are regenerated from it, so a guessed seed
/// would put the population on a different map) and `organisms` (the world
/// itself). Every other field has a neutral value and loads without it,
/// `terrain` included. A new field goes in one group or the other on
/// purpose. See "Save format: every field has a default unless the world
/// cannot be rebuilt without it" in `docs/DECISIONS.md`.
#[derive(Serialize, Deserialize)]
pub struct SaveState {
    pub tick: u64,
    /// Missing: the start of the seasonal cycle.
    #[serde(default)]
    pub season_tick: u64,
    pub terrain_seed: u64,
    /// Missing: every counter at zero. They feed the stats display only.
    #[serde(default)]
    pub stats: SaveStats,
    pub organisms: Vec<SaveOrganism>,
    /// Missing: no food. `food_regeneration_system` refills toward
    /// `max_food_density` from the first tick.
    #[serde(default)]
    pub food: Vec<SaveFood>,
    /// Missing: zero. `validate_save_state` then raises it above every
    /// innovation number in the loaded genomes, so new connections never
    /// reuse one.
    #[serde(default)]
    pub innovation_counter: u64,
    /// Missing: an empty tree. Lookups by species id already tolerate a
    /// species with no node, and classification records new species as
    /// they appear.
    #[serde(default)]
    pub phylo_nodes: Vec<SavePhyloNode>,
    /// Missing: an empty chronicle.
    #[serde(default)]
    pub chronicle_entries: Vec<SaveChronicleEntry>,
    /// The tile fields that change at runtime. Missing (a save from before
    /// terrain was persisted, or one whose record failed validation): the
    /// terrain as regenerated from `terrain_seed`, with a warning, so
    /// vegetation, moisture, nutrient and temperature changes since tick 0
    /// are lost.
    #[serde(default)]
    pub terrain: Option<SaveTerrain>,
}

/// The tile fields that change after generation, one value per tile in
/// `TileMap::tiles` order (row-major, `y * width + x`).
///
/// Terrain type, elevation and light level are never written after
/// `TileMap::generate`, so they are regenerated from `terrain_seed` and not
/// stored. The four fields here are written by `tile_dynamics_system`
/// (vegetation growth), niche construction (vegetation, moisture,
/// nutrients) and the ice age and volcano events (temperature, moisture,
/// nutrients). Each is stored as base64 of the little-endian bytes of its
/// `f32` values, which restores the exact bits in about half the space of a
/// JSON number array. A new tile field that changes at runtime belongs
/// here; `from_tile_map` destructures `Tile` exhaustively, so adding a
/// field does not compile until it is classified as stored or regenerated.
/// See "Save format: terrain persists only the tile fields that
/// change" in `docs/DECISIONS.md`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SaveTerrain {
    pub width: u32,
    pub height: u32,
    #[serde(with = "f32_base64")]
    pub temperature: Vec<f32>,
    #[serde(with = "f32_base64")]
    pub moisture: Vec<f32>,
    #[serde(with = "f32_base64")]
    pub nutrients: Vec<f32>,
    #[serde(with = "f32_base64")]
    pub vegetation_density: Vec<f32>,
}

impl SaveTerrain {
    /// Capture the mutable tile fields of `map`.
    ///
    /// The destructuring below names every `Tile` field and has no `..`, so
    /// adding a field to `Tile` stops this from compiling until the new
    /// field is classified: stored here (and restored in `apply_to`) if
    /// anything writes it after generation, or bound to `_` if
    /// `TileMap::generate` alone determines it.
    pub fn from_tile_map(map: &TileMap) -> Self {
        let n = map.tiles.len();
        let mut terrain = Self {
            width: map.width,
            height: map.height,
            temperature: Vec::with_capacity(n),
            moisture: Vec::with_capacity(n),
            nutrients: Vec::with_capacity(n),
            vegetation_density: Vec::with_capacity(n),
        };
        for tile in &map.tiles {
            let Tile {
                // Regenerated from `terrain_seed`; never written at runtime.
                terrain: _,
                elevation: _,
                light_level: _,
                // Written at runtime; stored.
                temperature,
                moisture,
                nutrients,
                vegetation_density,
            } = *tile;
            terrain.temperature.push(temperature);
            terrain.moisture.push(moisture);
            terrain.nutrients.push(nutrients);
            terrain.vegetation_density.push(vegetation_density);
        }
        terrain
    }

    /// Why this record does not describe a map of its own dimensions, or
    /// `None` when it does.
    fn problem(&self) -> Option<String> {
        let expected = self.width as usize * self.height as usize;
        for (name, values) in self.fields() {
            if values.len() != expected {
                return Some(format!(
                    "{} has {} values for a {}x{} map",
                    name,
                    values.len(),
                    self.width,
                    self.height
                ));
            }
            if let Some(v) = values.iter().find(|v| !v.is_finite()) {
                return Some(format!("{} holds a non-finite value ({})", name, v));
            }
        }
        None
    }

    fn fields(&self) -> [(&'static str, &[f32]); 4] {
        [
            ("temperature", &self.temperature),
            ("moisture", &self.moisture),
            ("nutrients", &self.nutrients),
            ("vegetation_density", &self.vegetation_density),
        ]
    }

    /// Overwrite the mutable fields of `map`, which should be the terrain
    /// regenerated from the save's seed. Leaves `map` untouched and returns
    /// the reason when the record does not fit it.
    pub fn apply_to(&self, map: &mut TileMap) -> Result<(), String> {
        if (self.width, self.height) != (map.width, map.height) {
            return Err(format!(
                "saved terrain is {}x{} but the world is {}x{}",
                self.width, self.height, map.width, map.height
            ));
        }
        if let Some(problem) = self.problem() {
            return Err(problem);
        }
        for (i, tile) in map.tiles.iter_mut().enumerate() {
            tile.temperature = self.temperature[i];
            tile.moisture = self.moisture[i];
            tile.nutrients = self.nutrients[i];
            tile.vegetation_density = self.vegetation_density[i];
        }
        Ok(())
    }
}

/// Apply the saved terrain state to `map`, the terrain regenerated from the
/// save's seed. When the save has no terrain state or it does not fit,
/// leaves `map` as regenerated and returns a warning for the caller to put
/// where the user will see it (headless runs have no log).
pub fn restore_terrain(map: &mut TileMap, terrain: Option<&SaveTerrain>) -> Result<(), String> {
    let reason = match terrain {
        None => "the save has no usable terrain state".to_string(),
        Some(t) => match t.apply_to(map) {
            Ok(()) => return Ok(()),
            Err(reason) => reason,
        },
    };
    Err(format!(
        "Terrain regenerated from the seed because {}; vegetation, moisture, nutrient and temperature changes since tick 0 are lost",
        reason
    ))
}

/// Serde adapter that stores a `Vec<f32>` as base64 of its little-endian
/// bytes, so every value round-trips bit for bit.
mod f32_base64 {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(values: &[f32], s: S) -> Result<S::Ok, S::Error> {
        let mut bytes = Vec::with_capacity(values.len() * 4);
        for v in values {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        s.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<f32>, D::Error> {
        let text = String::deserialize(d)?;
        let bytes = STANDARD.decode(text).map_err(D::Error::custom)?;
        if bytes.len() % 4 != 0 {
            return Err(D::Error::custom(format!(
                "{} bytes is not a whole number of f32 values",
                bytes.len()
            )));
        }
        let (chunks, _) = bytes.as_chunks::<4>();
        Ok(chunks.iter().map(|c| f32::from_le_bytes(*c)).collect())
    }
}

/// Lifetime counters. Every field defaults to zero.
#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
#[serde(default)]
pub struct SaveStats {
    pub total_births: u64,
    pub total_deaths: u64,
    pub max_generation: u32,
}

/// One organism as it is written to a save file.
///
/// Position, energy and genome are required: an organism has no neutral
/// place to stand, its energy is the ledger's starting balance, and without
/// a genome there is nothing to spawn. The rest is state an organism can
/// start over with, and a missing field takes the value a founder spawns
/// with.
#[derive(Serialize, Deserialize)]
pub struct SaveOrganism {
    pub x: f32,
    pub y: f32,
    pub energy: f32,
    /// Missing: full health.
    #[serde(default = "full_health")]
    pub health: f32,
    #[serde(default)]
    pub age: u64,
    #[serde(default)]
    pub generation: u32,
    /// Missing: 0, the unclassified id every founder spawns with. The next
    /// classification pass assigns a real species.
    #[serde(default)]
    pub species_id: u64,
    /// Missing: silent.
    #[serde(default)]
    pub signal: f32,
    /// Missing: cleared brain memory.
    #[serde(default)]
    pub memory: [f32; 3],
    pub genome: SaveGenome,
}

/// The health a founder spawns with, used when a saved organism has none.
fn full_health() -> f32 {
    1.0
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
#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaveNeuron {
    pub id: u64,
    pub neuron_type: u8, // 0=Input, 1=Hidden, 2=Output
    pub activation: u8,  // 0=Sigmoid, 1=Tanh, 2=Relu
    pub bias: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaveConnection {
    pub innovation: u64,
    pub from: u64,
    pub to: u64,
    pub weight: f32,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaveBodySegment {
    pub segment_type: u8,
    pub size: f32,
    pub attachment_angle: f32,
    pub attachment_slot: u8,
    pub symmetry: u8,
}

/// One food item. `x` and `y` are required, since a food item has no
/// neutral position. See "Save format: every field has a default unless the
/// world cannot be rebuilt without it" in `docs/DECISIONS.md`.
#[derive(Serialize, Deserialize)]
pub struct SaveFood {
    pub x: f32,
    pub y: f32,
    /// Missing: the energy regeneration gives new food,
    /// `SimConfig::food_energy_value`, filled in by `spawn_saved_food`.
    #[serde(default)]
    pub energy: Option<f32>,
}

/// One phylogeny node. Only `species_id` is required: it is the key every
/// organism, chronicle entry and child node refers to, and a guessed id would
/// attach the node to the wrong species. Every other field has a neutral
/// value. See "Save format: every field has a default unless the world cannot
/// be rebuilt without it" in `docs/DECISIONS.md`.
#[derive(Serialize, Deserialize)]
pub struct SavePhyloNode {
    pub species_id: u64,
    /// Missing: a root species.
    #[serde(default)]
    pub parent_id: Option<u64>,
    /// Missing: tick 0, the start of the run.
    #[serde(default)]
    pub born_tick: u64,
    /// Missing: living. The next population update marks it extinct if it
    /// has no members.
    #[serde(default)]
    pub extinct_tick: Option<u64>,
    /// Missing: 0. The next population update raises it to the living count.
    #[serde(default)]
    pub peak_population: u32,
    /// Missing: omnivore, the code any unknown value already maps to.
    #[serde(default = "omnivore_strategy")]
    pub strategy: u8,
    /// Missing (saves from before species naming) or empty: the placeholder
    /// `restore_phylo` fills in, `placeholder_species_name`. The generated
    /// names are built from the species' traits, which the node does not
    /// store, so the original name cannot be recomputed.
    #[serde(default)]
    pub name: String,
}

/// The `SavePhyloNode::strategy` code for an omnivore.
fn omnivore_strategy() -> u8 {
    3
}

#[derive(Serialize, Deserialize)]
pub struct SaveChronicleEntry {
    pub tick: u64,
    pub text: String,
    /// Added after the first saves were written; older files have no field
    /// and load as untargeted entries.
    #[serde(default)]
    pub target: Option<SaveChronicleTarget>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum SaveChronicleTarget {
    Species { id: u64 },
    Location { x: f32, y: f32 },
}

impl SaveChronicleTarget {
    fn from_target(target: ChronicleTarget) -> Self {
        match target {
            ChronicleTarget::Species(id) => SaveChronicleTarget::Species { id },
            ChronicleTarget::Location(pos) => SaveChronicleTarget::Location { x: pos.x, y: pos.y },
        }
    }

    fn into_target(self) -> ChronicleTarget {
        match self {
            SaveChronicleTarget::Species { id } => ChronicleTarget::Species(id),
            SaveChronicleTarget::Location { x, y } => ChronicleTarget::Location(Vec2::new(x, y)),
        }
    }
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

/// The genome as the sim uses it. A genome written with fewer brain
/// inputs than `NUM_INPUTS` (a save or creature file from before an input
/// was added) is migrated to the current layout with the new inputs
/// unconnected, so it behaves as it did; see
/// `Genome::migrate_input_layout`.
pub fn save_to_genome(s: &SaveGenome) -> Genome {
    let mut genome = save_to_genome_as_written(s);
    genome.migrate_input_layout();
    genome
}

/// The genome exactly as the file describes it, without input migration.
fn save_to_genome_as_written(s: &SaveGenome) -> Genome {
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
    terrain: Option<&TileMap>,
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
                energy: Some(*energy),
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
                target: e.target.map(SaveChronicleTarget::from_target),
            })
            .collect(),
        terrain: terrain.map(SaveTerrain::from_tile_map),
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
    let legacy = state
        .organisms
        .iter()
        .filter(|org| input_count(&org.genome) < NUM_INPUTS)
        .count();
    if legacy > 0 {
        info!(
            "Save file has {} organism(s) with an older brain layout; adding the missing inputs unconnected",
            legacy
        );
    }
    Some(state)
}

/// Drop any organisms that fail basic sanity checks; log a count if any
/// are removed. Non-fatal — the sim starts with the survivors.
fn validate_save_state(state: &mut SaveState) {
    raise_innovation_counter(state);

    if let Some(problem) = state.terrain.as_ref().and_then(SaveTerrain::problem) {
        warn!(
            "Save terrain state is unusable ({}); the terrain will be regenerated from the seed",
            problem
        );
        state.terrain = None;
    }

    let before = state.organisms.len();
    state
        .organisms
        .retain(|org| genome_problem(&org.genome).is_none());
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

/// Keep the innovation counter above every innovation number already in the
/// loaded genomes. A save missing `innovation_counter` loads it as zero, and
/// handing out numbers the population already carries would make unrelated
/// connections look homologous to crossover and to the species distance. A
/// save written by `save_world` already satisfies this and is unchanged.
fn raise_innovation_counter(state: &mut SaveState) {
    let Some(highest) = state
        .organisms
        .iter()
        .flat_map(|org| org.genome.connections.iter())
        .map(|c| c.innovation)
        .max()
    else {
        return;
    };
    if state.innovation_counter <= highest {
        warn!(
            "Save innovation counter {} is not above the highest innovation in its genomes ({}); raising it to {}",
            state.innovation_counter,
            highest,
            highest + 1
        );
        state.innovation_counter = highest + 1;
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

/// Spawn the saved food. An item saved without its energy gets
/// `default_energy`, the value regeneration gives new food.
pub fn spawn_saved_food(commands: &mut Commands, food: &[SaveFood], default_energy: f32) {
    for f in food {
        commands.spawn((
            Food,
            FoodEnergy(f.energy.unwrap_or(default_energy)),
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
                1 => SpeciesStrategy::Hunter,
                2 => SpeciesStrategy::Grazer,
                _ => SpeciesStrategy::Omnivore,
            },
            color: Color::WHITE, // will be reassigned by species classification
            name: if n.name.is_empty() {
                clauvolution_phylogeny::placeholder_species_name(n.species_id)
            } else {
                n.name.clone()
            },
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
                target: e.target.map(SaveChronicleTarget::into_target),
            });
    }
}

/// Number of input neurons in a serialised genome.
fn input_count(genome: &SaveGenome) -> usize {
    genome.neurons.iter().filter(|n| n.neuron_type == 0).count()
}

/// Why a serialised genome cannot be spawned, or `None` when it can.
/// Shared by save loading (which drops the organism) and creature import
/// (which refuses the file).
fn genome_problem(genome: &SaveGenome) -> Option<&'static str> {
    // Genome must have at least a torso body segment
    if genome.body_segments.is_empty() {
        return Some("genome has no body segments");
    }
    // Genome must have some neurons (otherwise the brain can't be built)
    if genome.neurons.is_empty() {
        return Some("genome has no neurons");
    }
    // Every connection must reference real neuron IDs
    let neuron_ids: std::collections::HashSet<u64> = genome.neurons.iter().map(|n| n.id).collect();
    if genome
        .connections
        .iter()
        .any(|c| !neuron_ids.contains(&c.from) || !neuron_ids.contains(&c.to))
    {
        return Some("a connection references a neuron the genome does not have");
    }
    None
}

/// Current creature file format version. Bumped when a field changes
/// meaning; new optional fields do not need a bump.
pub const CREATURE_FORMAT_VERSION: u32 = 1;

/// One organism's genome plus enough about where it came from to be useful
/// when it turns up in another world. Written by the Inspect panel's
/// export button, read by `--seed-with`. Only `genome` is required; every
/// metadata field defaults when absent, so a hand-written file needs no
/// more than a genome.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreatureFile {
    #[serde(default = "creature_format_version")]
    pub format_version: u32,
    /// Species name in the origin world, if the organism had one.
    #[serde(default)]
    pub species_name: Option<String>,
    /// Strategy label at export time (plant, grazer, hunter, omnivore).
    #[serde(default)]
    pub strategy: Option<String>,
    #[serde(default)]
    pub generation: u32,
    /// Tick of the origin world when the export happened.
    #[serde(default)]
    pub exported_tick: u64,
    /// Terrain seed of the origin world.
    #[serde(default)]
    pub origin_seed: Option<u64>,
    /// Session name of the origin world.
    #[serde(default)]
    pub origin_session: Option<String>,
    pub genome: SaveGenome,
}

fn creature_format_version() -> u32 {
    CREATURE_FORMAT_VERSION
}

impl CreatureFile {
    /// Describe `genome` and its provenance. `strategy` is the classified
    /// label so a reader (or a directory listing) can tell what the file
    /// holds without spawning it.
    pub fn new(
        genome: &Genome,
        species_name: Option<&str>,
        strategy: &str,
        generation: u32,
        exported_tick: u64,
        origin_seed: u64,
        origin_session: &str,
    ) -> Self {
        Self {
            format_version: CREATURE_FORMAT_VERSION,
            species_name: species_name.map(str::to_string),
            strategy: Some(strategy.to_string()),
            generation,
            exported_tick,
            origin_seed: Some(origin_seed),
            origin_session: Some(origin_session.to_string()),
            genome: genome_to_save(genome),
        }
    }

    /// The genome as the sim uses it.
    pub fn genome(&self) -> Genome {
        save_to_genome(&self.genome)
    }
}

/// File name for an exported creature: the species name slugified, or
/// `organism-<index>` when it has none, followed by the export tick so two
/// exports of the same species at different times do not overwrite each
/// other.
pub fn creature_file_name(species_name: Option<&str>, entity: Entity, tick: u64) -> String {
    let slug: String = species_name
        .map(|name| {
            let mut slug = String::with_capacity(name.len());
            let mut last_dash = true;
            for ch in name.chars() {
                if ch.is_ascii_alphanumeric() {
                    slug.push(ch.to_ascii_lowercase());
                    last_dash = false;
                } else if !last_dash {
                    slug.push('-');
                    last_dash = true;
                }
            }
            slug.trim_end_matches('-').to_string()
        })
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| format!("organism-{}", entity.index()));
    format!("{slug}-t{tick}.json")
}

/// Write `creature` to `path` as indented JSON, atomically, the same way
/// saves are written. Returns the error instead of panicking; the caller
/// decides how to surface it.
pub fn export_creature(path: &Path, creature: &CreatureFile) -> Result<(), SaveError> {
    let json = serde_json::to_string_pretty(creature).map_err(SaveError::Serialize)?;
    write_atomically(path, json.as_bytes()).map_err(|source| SaveError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Why a creature file could not be imported. Import happens at startup
/// from a command-line flag, so the caller reports it and exits.
#[derive(Debug)]
pub enum CreatureLoadError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    Invalid {
        path: PathBuf,
        reason: &'static str,
    },
}

impl fmt::Display for CreatureLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CreatureLoadError::Read { path, source } => {
                write!(f, "could not read {}: {}", path.display(), source)
            }
            CreatureLoadError::Parse { path, source } => {
                write!(f, "could not parse {}: {}", path.display(), source)
            }
            CreatureLoadError::Invalid { path, reason } => {
                write!(f, "{} is not a usable creature: {}", path.display(), reason)
            }
        }
    }
}

impl std::error::Error for CreatureLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CreatureLoadError::Read { source, .. } => Some(source),
            CreatureLoadError::Parse { source, .. } => Some(source),
            CreatureLoadError::Invalid { .. } => None,
        }
    }
}

/// Read a creature file and check its genome can be spawned. Unlike
/// `load_world`, an unusable genome is an error rather than a silent skip:
/// the user named this file on the command line and should hear about it.
pub fn load_creature(path: &Path) -> Result<CreatureFile, CreatureLoadError> {
    let json = std::fs::read_to_string(path).map_err(|source| CreatureLoadError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let creature: CreatureFile =
        serde_json::from_str(&json).map_err(|source| CreatureLoadError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
    if let Some(reason) = genome_problem(&creature.genome) {
        return Err(CreatureLoadError::Invalid {
            path: path.to_path_buf(),
            reason,
        });
    }
    Ok(creature)
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
            None,
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

    /// A complete save with one spawnable organism whose single connection
    /// carries innovation 7, as a JSON value so tests can remove fields.
    fn complete_save_json() -> serde_json::Value {
        serde_json::json!({
            "tick": 500, "season_tick": 120, "terrain_seed": 42,
            "stats": {"total_births": 9, "total_deaths": 4, "max_generation": 3},
            "organisms": [{
                "x": 1.0, "y": 2.0, "energy": 50.0, "health": 0.4, "age": 80,
                "generation": 3, "species_id": 5, "signal": 0.6, "memory": [0.1, 0.2, 0.3],
                "genome": {
                    "neurons": [
                        {"id": 0, "neuron_type": 0, "activation": 0, "bias": 0.0},
                        {"id": 1, "neuron_type": 2, "activation": 1, "bias": 0.0}
                    ],
                    "connections": [
                        {"innovation": 7, "from": 0, "to": 1, "weight": 0.5, "enabled": true}
                    ],
                    "body_segments": [{"segment_type": 0, "size": 1.0,
                        "attachment_angle": 0.0, "attachment_slot": 0, "symmetry": 1}]
                }
            }],
            "food": [{"x": 3.0, "y": 4.0, "energy": 10.0}],
            "innovation_counter": 100,
            "phylo_nodes": [{"species_id": 5, "parent_id": null, "born_tick": 10,
                "extinct_tick": null, "peak_population": 12, "strategy": 2, "name": "Test"}],
            "chronicle_entries": [{"tick": 10, "text": "A species appeared"}]
        })
    }

    fn load_json(tag: &str, value: &serde_json::Value) -> Option<SaveState> {
        let scratch = ScratchDir::new(tag);
        let path = scratch.0.join("save.json");
        std::fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        load_world(&path)
    }

    fn remove_field(value: &mut serde_json::Value, field: &str) {
        value
            .as_object_mut()
            .unwrap()
            .remove(field)
            .unwrap_or_else(|| panic!("fixture has no field {}", field));
    }

    #[test]
    fn complete_save_fixture_loads_unchanged() {
        let state = load_json("complete", &complete_save_json()).expect("the fixture loads");
        assert_eq!(state.tick, 500);
        assert_eq!(state.season_tick, 120);
        assert_eq!(state.innovation_counter, 100);
        assert_eq!(state.organisms[0].species_id, 5);
        assert_eq!(state.food.len(), 1);
        assert_eq!(state.phylo_nodes.len(), 1);
        assert_eq!(state.chronicle_entries.len(), 1);
    }

    /// A save carrying only the fields the world cannot be rebuilt without
    /// loads, keeps its organism, and fills everything else with the values
    /// documented on `SaveState` and `SaveOrganism`.
    #[test]
    fn save_with_only_required_fields_loads_with_defaults() {
        let mut value = complete_save_json();
        for field in [
            "season_tick",
            "stats",
            "food",
            "innovation_counter",
            "phylo_nodes",
            "chronicle_entries",
        ] {
            remove_field(&mut value, field);
        }
        let organism = &mut value["organisms"][0];
        for field in [
            "health",
            "age",
            "generation",
            "species_id",
            "signal",
            "memory",
        ] {
            remove_field(organism, field);
        }

        let state =
            load_json("required-only", &value).expect("a save missing optional fields loads");
        assert_eq!(state.tick, 500);
        assert_eq!(state.terrain_seed, 42);
        assert_eq!(state.season_tick, 0);
        assert_eq!(state.stats, SaveStats::default());
        assert!(state.food.is_empty());
        assert!(state.phylo_nodes.is_empty());
        assert!(state.chronicle_entries.is_empty());
        assert!(state.terrain.is_none());

        assert_eq!(state.organisms.len(), 1, "the organism must not be dropped");
        let org = &state.organisms[0];
        assert_eq!((org.x, org.y, org.energy), (1.0, 2.0, 50.0));
        assert_eq!(org.health, full_health());
        assert_eq!(org.age, 0);
        assert_eq!(org.generation, 0);
        assert_eq!(org.species_id, 0);
        assert_eq!(org.signal, 0.0);
        assert_eq!(org.memory, [0.0; 3]);
    }

    /// With the counter missing it loads as zero, below the innovation the
    /// organism already carries; validation must lift it clear.
    #[test]
    fn missing_innovation_counter_is_raised_above_loaded_genomes() {
        let mut value = complete_save_json();
        remove_field(&mut value, "innovation_counter");
        let state = load_json("innovation", &value).expect("loads");
        assert_eq!(state.innovation_counter, 8);
    }

    #[test]
    fn missing_innovation_counter_with_no_connections_stays_zero() {
        let mut value = complete_save_json();
        remove_field(&mut value, "innovation_counter");
        value["organisms"] = serde_json::json!([]);
        let state = load_json("innovation-empty", &value).expect("loads");
        assert_eq!(state.innovation_counter, 0);
    }

    /// The fields with no neutral value still reject the whole file.
    #[test]
    fn save_missing_a_required_field_is_rejected() {
        for field in ["tick", "terrain_seed", "organisms"] {
            let mut value = complete_save_json();
            remove_field(&mut value, field);
            assert!(
                load_json(&format!("missing-{}", field), &value).is_none(),
                "a save without `{}` must not load",
                field
            );
        }
        for field in ["x", "y", "energy", "genome"] {
            let mut value = complete_save_json();
            remove_field(&mut value["organisms"][0], field);
            assert!(
                load_json(&format!("missing-organism-{}", field), &value).is_none(),
                "a save with an organism without `{}` must not load",
                field
            );
        }
    }

    /// A phylogeny node from before species naming has no `name`. The file
    /// loads, and the restored node carries the placeholder name rather
    /// than a blank one.
    #[test]
    fn phylo_node_without_a_name_loads_with_the_placeholder() {
        let mut value = complete_save_json();
        remove_field(&mut value["phylo_nodes"][0], "name");
        let state = load_json("phylo-no-name", &value).expect("a node without a name loads");
        assert_eq!(state.phylo_nodes.len(), 1, "the node must not be dropped");
        assert_eq!(state.phylo_nodes[0].name, "");

        let mut phylo = PhyloTree::default();
        restore_phylo(&mut phylo, &state.phylo_nodes);
        assert_eq!(phylo.nodes[&5].name, "Species 5");
    }

    /// A saved name is kept as written.
    #[test]
    fn restore_phylo_keeps_a_saved_name() {
        let state = load_json("phylo-named", &complete_save_json()).expect("loads");
        let mut phylo = PhyloTree::default();
        restore_phylo(&mut phylo, &state.phylo_nodes);
        assert_eq!(phylo.nodes[&5].name, "Test");
    }

    /// Sub-records carrying only their required fields load with the values
    /// documented on `SavePhyloNode` and `SaveFood`.
    #[test]
    fn sub_records_with_only_required_fields_load_with_defaults() {
        let mut value = complete_save_json();
        for field in [
            "parent_id",
            "born_tick",
            "extinct_tick",
            "peak_population",
            "strategy",
            "name",
        ] {
            remove_field(&mut value["phylo_nodes"][0], field);
        }
        remove_field(&mut value["food"][0], "energy");

        let state = load_json("sub-records", &value).expect("loads");
        let node = &state.phylo_nodes[0];
        assert_eq!(node.species_id, 5);
        assert_eq!(node.parent_id, None);
        assert_eq!(node.born_tick, 0);
        assert_eq!(node.extinct_tick, None);
        assert_eq!(node.peak_population, 0);
        assert_eq!(node.strategy, omnivore_strategy());

        let mut phylo = PhyloTree::default();
        restore_phylo(&mut phylo, &state.phylo_nodes);
        assert_eq!(phylo.nodes[&5].strategy, SpeciesStrategy::Omnivore);
        assert_eq!(phylo.root_species, vec![5]);

        let food = &state.food[0];
        assert_eq!((food.x, food.y, food.energy), (3.0, 4.0, None));
    }

    /// Food saved without its energy spawns with the default it is given;
    /// food saved with energy keeps it.
    #[test]
    fn food_without_energy_spawns_with_the_default() {
        let mut world = World::new();
        let food = [
            SaveFood {
                x: 1.0,
                y: 2.0,
                energy: None,
            },
            SaveFood {
                x: 3.0,
                y: 4.0,
                energy: Some(7.5),
            },
        ];
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        spawn_saved_food(&mut commands, &food, 25.0);
        queue.apply(&mut world);

        let mut energies: Vec<f32> = world
            .query::<&FoodEnergy>()
            .iter(&world)
            .map(|e| e.0)
            .collect();
        energies.sort_by(f32::total_cmp);
        assert_eq!(energies, vec![7.5, 25.0]);
    }

    /// The sub-record fields with no neutral value still reject the file.
    #[test]
    fn sub_record_missing_a_required_field_is_rejected() {
        let mut value = complete_save_json();
        remove_field(&mut value["phylo_nodes"][0], "species_id");
        assert!(
            load_json("missing-phylo-species-id", &value).is_none(),
            "a phylo node without `species_id` must not load"
        );
        for field in ["x", "y"] {
            let mut value = complete_save_json();
            remove_field(&mut value["food"][0], field);
            assert!(
                load_json(&format!("missing-food-{}", field), &value).is_none(),
                "a food item without `{}` must not load",
                field
            );
        }
    }

    /// A small map generated from `seed`, as `load_saved_world` rebuilds it.
    fn small_map(seed: u64) -> TileMap {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        TileMap::generate(24, 16, &mut rng)
    }

    /// Every tile field, so a comparison covers the regenerated fields too.
    fn tile_fields(map: &TileMap) -> Vec<(clauvolution_world::TerrainType, [u32; 6])> {
        map.tiles
            .iter()
            .map(|t| {
                (
                    t.terrain,
                    [
                        t.elevation.to_bits(),
                        t.temperature.to_bits(),
                        t.moisture.to_bits(),
                        t.light_level.to_bits(),
                        t.nutrients.to_bits(),
                        t.vegetation_density.to_bits(),
                    ],
                )
            })
            .collect()
    }

    /// Change the map the ways the sim does: an ice age everywhere, a
    /// volcano's nutrient boost and niche construction on a few tiles, and
    /// some vegetation growth, with values that are not round decimals.
    fn modify_like_a_run(map: &mut TileMap) {
        for tile in &mut map.tiles {
            tile.temperature *= 0.5;
            tile.moisture *= 0.7;
        }
        for i in (0..map.tiles.len()).step_by(7) {
            let tile = &mut map.tiles[i];
            tile.nutrients = (tile.nutrients + 0.5).min(1.0);
            tile.vegetation_density = (tile.vegetation_density + 0.0123).min(1.0);
            tile.moisture = (tile.moisture + 0.004_56).min(1.0);
        }
        for tile in &mut map.tiles {
            tile.vegetation_density +=
                (tile.nutrients * tile.moisture - tile.vegetation_density) * 0.001;
        }
    }

    #[test]
    fn modified_terrain_survives_a_save_and_load() {
        let scratch = ScratchDir::new("terrain-round-trip");
        let path = scratch.0.join("save.json");
        let config = SimConfig {
            terrain_seed: 42,
            ..SimConfig::default()
        };
        let mut map = small_map(config.terrain_seed);
        let regenerated = tile_fields(&map);
        modify_like_a_run(&mut map);
        let at_save = tile_fields(&map);
        assert_ne!(at_save, regenerated, "the fixture must change the terrain");

        save_world(
            &path,
            &TickCounter(7),
            &Season::default(),
            &SimStats::default(),
            &InnovationCounter(100),
            &config,
            &[],
            &[],
            &PhyloTree::default(),
            &WorldChronicle::default(),
            Some(&map),
        )
        .expect("save");
        let state = load_world(&path).expect("the written file loads");
        let terrain = state.terrain.as_ref().expect("the save carries terrain");
        assert_eq!(terrain, &SaveTerrain::from_tile_map(&map));

        let mut loaded = small_map(state.terrain_seed);
        restore_terrain(&mut loaded, state.terrain.as_ref()).expect("the terrain fits");
        assert_eq!(tile_fields(&loaded), at_save);
    }

    /// Every save written before terrain was persisted has no `terrain`
    /// field. It loads, and the terrain is the one regenerated from the seed.
    #[test]
    fn save_without_terrain_loads_with_regenerated_terrain() {
        let value = complete_save_json();
        assert!(value.get("terrain").is_none());
        let state = load_json("no-terrain", &value).expect("a save without terrain loads");
        assert!(state.terrain.is_none());
        assert_eq!(state.organisms.len(), 1);

        let mut map = small_map(state.terrain_seed);
        let regenerated = tile_fields(&map);
        let warning = restore_terrain(&mut map, state.terrain.as_ref())
            .expect_err("a missing terrain is reported");
        assert!(warning.contains("no usable terrain state"), "{warning}");
        assert_eq!(tile_fields(&map), regenerated);
    }

    /// A terrain record that does not describe its own map is dropped at
    /// load time; the rest of the save still loads.
    #[test]
    fn terrain_with_the_wrong_tile_count_is_dropped_but_the_save_loads() {
        let mut terrain = SaveTerrain::from_tile_map(&small_map(42));
        terrain.nutrients.pop();
        let mut value = complete_save_json();
        value["terrain"] = serde_json::to_value(&terrain).unwrap();
        let state = load_json("short-terrain", &value).expect("the save still loads");
        assert!(state.terrain.is_none());
        assert_eq!(state.organisms.len(), 1);
    }

    #[test]
    fn terrain_for_a_different_world_size_leaves_the_map_as_regenerated() {
        let mut other = small_map(42);
        modify_like_a_run(&mut other);
        let terrain = SaveTerrain::from_tile_map(&other);

        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut map = TileMap::generate(16, 24, &mut rng);
        let regenerated = tile_fields(&map);
        let err = terrain.apply_to(&mut map).expect_err("dimensions differ");
        assert!(err.contains("24x16"), "{err}");
        assert_eq!(tile_fields(&map), regenerated);
    }

    #[test]
    fn terrain_fields_are_stored_as_base64_strings() {
        let value = serde_json::to_value(SaveTerrain::from_tile_map(&small_map(1))).unwrap();
        let encoded = value["vegetation_density"].as_str().expect("a string");
        // 24 x 16 tiles, four bytes each, base64 without line breaks.
        assert_eq!(encoded.len(), (24 * 16 * 4usize).div_ceil(3) * 4);
    }

    /// A genome in the 22-input layout that preceded the nearest-eater
    /// inputs, as a file from before that change holds it: inputs 0..22,
    /// outputs 22..31, hidden neurons 31 and 32, and connections through
    /// them including a self-loop.
    fn legacy_22_input_save_genome() -> SaveGenome {
        let mut neurons = Vec::new();
        for id in 0..22 {
            neurons.push(SaveNeuron {
                id,
                neuron_type: 0,
                activation: 0,
                bias: 0.0,
            });
        }
        for id in 22..31 {
            neurons.push(SaveNeuron {
                id,
                neuron_type: 2,
                activation: 1,
                bias: 0.05 * id as f32 - 1.0,
            });
        }
        neurons.push(SaveNeuron {
            id: 31,
            neuron_type: 1,
            activation: 2,
            bias: 0.2,
        });
        neurons.push(SaveNeuron {
            id: 32,
            neuron_type: 1,
            activation: 0,
            bias: -0.3,
        });
        let edges: [(u64, u64, f32); 8] = [
            (1, 22, 1.2),
            (2, 23, -0.7),
            (4, 31, 0.9),
            (31, 26, 1.5),
            (31, 32, -1.1),
            (32, 24, 0.8),
            (32, 32, 0.4),
            (21, 30, -0.6),
        ];
        let connections = edges
            .iter()
            .enumerate()
            .map(|(i, &(from, to, weight))| SaveConnection {
                innovation: 50 + i as u64,
                from,
                to,
                weight,
                enabled: true,
            })
            .collect();
        SaveGenome {
            neurons,
            connections,
            body_segments: vec![SaveBodySegment {
                segment_type: 0,
                size: 1.0,
                attachment_angle: 0.0,
                attachment_slot: 0,
                symmetry: 1,
            }],
            ..SaveGenome::default()
        }
    }

    /// A save or creature file from before the nearest-eater inputs loads
    /// through both entry points, gains the new inputs, and its brain gives
    /// the same outputs for the same old inputs.
    #[test]
    fn genome_with_the_old_input_count_migrates_on_load() {
        let legacy = legacy_22_input_save_genome();
        let scratch = ScratchDir::new("legacy-inputs");

        let creature_path = scratch.0.join("legacy.json");
        let genome_json = serde_json::to_string(&legacy).unwrap();
        std::fs::write(&creature_path, format!("{{\"genome\": {genome_json}}}")).unwrap();
        let from_creature = load_creature(&creature_path)
            .expect("an old-layout creature file loads")
            .genome();

        let world_path = scratch.0.join("save.json");
        let org_json = serde_json::to_string(&SaveOrganism {
            x: 1.0,
            y: 2.0,
            energy: 50.0,
            health: 1.0,
            age: 0,
            generation: 0,
            species_id: 1,
            signal: 0.0,
            memory: [0.0; 3],
            genome: legacy.clone(),
        })
        .unwrap();
        std::fs::write(
            &world_path,
            format!(
                r#"{{"tick": 3, "season_tick": 0, "terrain_seed": 42,
                "stats": {{"total_births": 0, "total_deaths": 0, "max_generation": 0}},
                "organisms": [{org_json}], "food": [], "innovation_counter": 60,
                "phylo_nodes": [], "chronicle_entries": []}}"#
            ),
        )
        .unwrap();
        let state = load_world(&world_path).expect("an old-layout save loads");
        assert_eq!(state.organisms.len(), 1);
        let from_save = save_to_genome(&state.organisms[0].genome);

        let as_written = save_to_genome_as_written(&legacy);
        let old_brain = Brain::from_genome(&as_written);
        assert_eq!(old_brain.input_ids().len(), 22);

        for migrated in [&from_creature, &from_save] {
            let inputs = migrated
                .neurons
                .iter()
                .filter(|n| n.neuron_type == NeuronType::Input)
                .count();
            assert_eq!(inputs, NUM_INPUTS);
            assert_eq!(
                migrated.neurons.len(),
                as_written.neurons.len() + NUM_INPUTS - 22
            );
            assert_eq!(migrated.connections.len(), as_written.connections.len());

            let new_brain = Brain::from_genome(migrated);
            let expected_outputs: Vec<u64> =
                (NUM_INPUTS as u64..(NUM_INPUTS + NUM_OUTPUTS) as u64).collect();
            assert_eq!(new_brain.output_ids(), &expected_outputs[..]);
            for step in 0..40 {
                let mut inputs = [0.0f32; NUM_INPUTS];
                for (i, value) in inputs.iter_mut().enumerate() {
                    *value = ((step * 7 + i * 13) % 17) as f32 / 8.0 - 1.0;
                }
                assert_eq!(old_brain.evaluate(&inputs), new_brain.evaluate(&inputs));
            }
        }
    }

    #[test]
    fn chronicle_targets_survive_a_save_round_trip() {
        let mut chronicle = WorldChronicle::default();
        chronicle.log(1, "plain".to_string());
        chronicle.log_species(2, "species".to_string(), 42);
        chronicle.log_location(3, "place".to_string(), Vec2::new(12.5, -3.0));

        let saved: Vec<SaveChronicleEntry> = chronicle
            .entries
            .iter()
            .map(|e| SaveChronicleEntry {
                tick: e.tick,
                text: e.text.clone(),
                target: e.target.map(SaveChronicleTarget::from_target),
            })
            .collect();
        let json = serde_json::to_string(&saved).unwrap();
        let loaded: Vec<SaveChronicleEntry> = serde_json::from_str(&json).unwrap();

        let mut restored = WorldChronicle::default();
        restore_chronicle(&mut restored, &loaded);
        assert_eq!(restored.entries, chronicle.entries);
    }

    #[test]
    fn chronicle_entries_without_a_target_field_still_load() {
        let legacy = r#"[{"tick": 5, "text": "World loaded from save"}]"#;
        let loaded: Vec<SaveChronicleEntry> = serde_json::from_str(legacy).unwrap();
        let mut restored = WorldChronicle::default();
        restore_chronicle(&mut restored, &loaded);
        assert_eq!(restored.entries.len(), 1);
        assert_eq!(restored.entries[0].tick, 5);
        assert_eq!(restored.entries[0].target, None);
    }

    /// A founder genome with one hidden neuron, so the round trip covers
    /// every gene kind the file can hold.
    fn sample_genome() -> Genome {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(3);
        let mut innovation = InnovationCounter(100);
        let mut genome = Genome::new_minimal_with_diet(&mut innovation, &mut rng, 1.0);
        for _ in 0..20 {
            genome.mutate(&mut innovation, &mut rng, 1.0, 1.0);
        }
        genome
    }

    #[test]
    fn creature_export_round_trips_through_import() {
        let scratch = ScratchDir::new("creature-round-trip");
        let genome = sample_genome();
        let creature = CreatureFile::new(
            &genome,
            Some("Vorax Lumen"),
            "hunter",
            12,
            4567,
            42,
            "pale-fading-shard",
        );
        let path = scratch.0.join(creature_file_name(
            creature.species_name.as_deref(),
            Entity::from_raw(9),
            4567,
        ));
        assert_eq!(path.file_name().unwrap(), "vorax-lumen-t4567.json");

        export_creature(&path, &creature).expect("export into a writable directory");
        assert!(!scratch.0.join("vorax-lumen-t4567.json.tmp").exists());

        let loaded = load_creature(&path).expect("the written file loads");
        assert_eq!(loaded.format_version, CREATURE_FORMAT_VERSION);
        assert_eq!(loaded.species_name.as_deref(), Some("Vorax Lumen"));
        assert_eq!(loaded.strategy.as_deref(), Some("hunter"));
        assert_eq!(loaded.generation, 12);
        assert_eq!(loaded.exported_tick, 4567);
        assert_eq!(loaded.origin_seed, Some(42));
        assert_eq!(loaded.origin_session.as_deref(), Some("pale-fading-shard"));

        let back = loaded.genome();
        assert_eq!(back.neurons.len(), genome.neurons.len());
        assert_eq!(back.connections.len(), genome.connections.len());
        assert_eq!(back.body_segments.len(), genome.body_segments.len());
        for (a, b) in back.connections.iter().zip(&genome.connections) {
            assert_eq!(
                (a.innovation, a.from, a.to, a.enabled),
                (b.innovation, b.from, b.to, b.enabled)
            );
            assert_eq!(a.weight, b.weight);
        }
        for (a, b) in back.neurons.iter().zip(&genome.neurons) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.bias, b.bias);
        }
        assert_eq!(back.body_size, genome.body_size);
        assert_eq!(back.diet, genome.diet);
        assert_eq!(back.photosynthesis_rate, genome.photosynthesis_rate);
        // The genome must build a brain, which is what spawning needs.
        let _ = Brain::from_genome(&back);
    }

    #[test]
    fn creature_file_name_falls_back_to_the_entity() {
        let e = Entity::from_raw(17);
        assert_eq!(creature_file_name(None, e, 5), "organism-17-t5.json");
        assert_eq!(creature_file_name(Some("  "), e, 5), "organism-17-t5.json");
        assert_eq!(
            creature_file_name(Some("Zy'ra  Kel!"), e, 5),
            "zy-ra-kel-t5.json"
        );
    }

    #[test]
    fn creature_file_needs_only_a_genome() {
        // A hand-written file with no metadata still imports.
        let scratch = ScratchDir::new("creature-minimal");
        let path = scratch.0.join("minimal.json");
        let genome_json = serde_json::to_string(&genome_to_save(&sample_genome())).unwrap();
        std::fs::write(&path, format!("{{\"genome\": {genome_json}}}")).unwrap();
        let loaded = load_creature(&path).expect("a genome-only file loads");
        assert_eq!(loaded.format_version, CREATURE_FORMAT_VERSION);
        assert!(loaded.species_name.is_none());
        assert_eq!(loaded.generation, 0);
    }

    #[test]
    fn creature_import_rejects_unusable_files() {
        let scratch = ScratchDir::new("creature-invalid");

        let missing = scratch.0.join("missing.json");
        assert!(matches!(
            load_creature(&missing),
            Err(CreatureLoadError::Read { .. })
        ));

        let garbage = scratch.0.join("garbage.json");
        std::fs::write(&garbage, "not json").unwrap();
        assert!(matches!(
            load_creature(&garbage),
            Err(CreatureLoadError::Parse { .. })
        ));

        // A connection to a neuron the genome does not have cannot be
        // compiled into a brain.
        let mut broken = genome_to_save(&sample_genome());
        broken.connections.push(SaveConnection {
            innovation: 9999,
            from: 0,
            to: 100_000,
            weight: 1.0,
            enabled: true,
        });
        let bad = scratch.0.join("broken.json");
        std::fs::write(
            &bad,
            serde_json::to_string(&CreatureFile {
                format_version: CREATURE_FORMAT_VERSION,
                species_name: None,
                strategy: None,
                generation: 0,
                exported_tick: 0,
                origin_seed: None,
                origin_session: None,
                genome: broken,
            })
            .unwrap(),
        )
        .unwrap();
        let err = load_creature(&bad).expect_err("a dangling connection is rejected");
        assert!(matches!(err, CreatureLoadError::Invalid { .. }), "{err}");
        assert!(err.to_string().contains("broken.json"));
    }
}
