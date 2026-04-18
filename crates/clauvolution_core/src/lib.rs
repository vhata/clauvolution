use bevy::prelude::*;
use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        let session = Session::new();
        info!("Session: {} ({})", session.name, session.dir.display());
        app.add_event::<WorldEventRequest>()
            .insert_resource(session)
            .insert_resource(SimConfig::default())
            .insert_resource(SimStats::default())
            .insert_resource(TickCounter(0))
            .insert_resource(SimSpeed::default())
            .insert_resource(SpeciesColors::default())
            .insert_resource(SelectedOrganism::default())
            .insert_resource(Season::default())
            .insert_resource(FitnessTracker::default())
            .insert_resource(PopulationHistory::default())
            .insert_resource(BloomEffects::default())
            .insert_resource(UiInputState::default())
            .insert_resource(TrailsVisible::default())
            .insert_resource(FoodSnapshot::default());
    }
}

/// A named session with a storage directory for logs and screenshots
#[derive(Resource)]
pub struct Session {
    pub name: String,
    pub dir: PathBuf,
}

impl Session {
    pub fn new() -> Self {
        let mut name = Self::generate_name();
        let mut dir = PathBuf::from("sessions").join(&name);
        // If name collides, append a number
        if dir.exists() {
            for i in 2..100 {
                let candidate = format!("{}-{}", name, i);
                let candidate_dir = PathBuf::from("sessions").join(&candidate);
                if !candidate_dir.exists() {
                    name = candidate;
                    dir = candidate_dir;
                    break;
                }
            }
        }
        std::fs::create_dir_all(&dir).expect("Failed to create session directory");
        Self { name, dir }
    }

    fn generate_name() -> String {
        let mut rng = rand::thread_rng();

        let adjectives1 = [
            "ancient", "astral", "barren", "bright", "cerulean", "cosmic",
            "dark", "distant", "eternal", "ethereal", "feral", "frozen",
            "gilded", "glacial", "golden", "hidden", "infinite", "iridescent",
            "jade", "keen", "kindred", "luminous", "lunar", "midnight",
            "molten", "nascent", "nebular", "obsidian", "pale", "primal",
            "quiet", "radiant", "scarlet", "silent", "spectral", "stellar",
            "tethered", "twisted", "vast", "veiled", "violet", "wandering",
            "young", "zealous",
        ];

        let adjectives2 = [
            "arcing", "blazing", "burning", "collapsing", "crystalline",
            "dormant", "drifting", "echoing", "eroding", "fading", "fractal",
            "glowing", "grinding", "hollow", "humming", "ignited", "iron",
            "jagged", "jeweled", "kindled", "latticed", "living", "massive",
            "migrating", "nameless", "orbital", "ossified", "petrified",
            "pulsing", "quaking", "restless", "roiling", "shattered",
            "spiraling", "tidal", "tumbling", "unbound", "undying",
            "volatile", "withering", "woven",
        ];

        let nouns = [
            "abyss", "apex", "aurora", "bloom", "caldera", "canyon",
            "cinder", "comet", "corona", "crater", "crown", "delta",
            "drift", "dusk", "eclipse", "ember", "flare", "flux",
            "forge", "genesis", "geyser", "glacier", "haven", "helix",
            "horizon", "lagoon", "mantle", "nebula", "nova", "pinnacle",
            "plume", "pulsar", "quasar", "remnant", "rift", "shard",
            "solstice", "spire", "storm", "summit", "tide", "void",
            "vortex", "zenith",
        ];

        let a1 = adjectives1[rng.gen_range(0..adjectives1.len())];
        let a2 = adjectives2[rng.gen_range(0..adjectives2.len())];
        let n = nouns[rng.gen_range(0..nouns.len())];

        format!("{}-{}-{}", a1, a2, n)
    }

    pub fn log_path(&self) -> PathBuf {
        self.dir.join("chronicle.log")
    }

    pub fn screenshot_path(&self, label: &str) -> PathBuf {
        self.dir.join(format!("{}.png", label))
    }
}

#[derive(Resource, Clone, Serialize, Deserialize)]
pub struct SimConfig {
    pub world_width: u32,
    pub world_height: u32,
    pub initial_population: u32,
    pub initial_food_density: f32,
    pub food_regen_rate: f32,
    pub mutation_rate: f32,
    pub mutation_strength: f32,
    pub base_metabolism_cost: f32,
    pub movement_energy_cost: f32,
    pub reproduction_energy_threshold: f32,
    pub reproduction_energy_cost: f32,
    pub max_organism_energy: f32,
    pub food_energy_value: f32,
    pub species_compat_threshold: f32,
    pub terrain_seed: u64,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            world_width: 512,
            world_height: 512,
            initial_population: 400,
            initial_food_density: 0.1,
            food_regen_rate: 0.01,
            mutation_rate: 0.3,
            mutation_strength: 0.5,
            base_metabolism_cost: 0.08,
            movement_energy_cost: 0.04,
            reproduction_energy_threshold: 70.0,
            reproduction_energy_cost: 40.0,
            max_organism_energy: 120.0,
            food_energy_value: 25.0,
            species_compat_threshold: 2.0,
            terrain_seed: rand::random(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathCause {
    Starvation = 0,
    Predation = 1,
    OldAge = 2,
    Disease = 3,
}

#[derive(Resource, Default)]
pub struct SimStats {
    pub total_organisms: u32,
    pub total_food: u32,
    pub total_births: u64,
    pub total_deaths: u64,
    pub max_generation: u32,
    pub species_count: u32,
    /// Deaths categorised by cause, indexed by DeathCause as usize
    pub deaths_by_cause: [u64; 4],
}

#[derive(Resource)]
pub struct TickCounter(pub u64);

/// Seasonal cycle — affects light, temperature, food regen
#[derive(Resource)]
pub struct Season {
    pub cycle_ticks: u64,     // ticks per full year
    pub current_tick: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SeasonName {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Default for Season {
    fn default() -> Self {
        Self {
            cycle_ticks: 1800, // 60 seconds at 30 ticks/sec
            current_tick: 0,
        }
    }
}

impl Season {
    pub fn advance(&mut self) {
        self.current_tick += 1;
        if self.current_tick >= self.cycle_ticks {
            self.current_tick = 0;
        }
    }

    /// 0.0 = start of year, 1.0 = end of year
    pub fn phase(&self) -> f32 {
        self.current_tick as f32 / self.cycle_ticks as f32
    }

    pub fn name(&self) -> SeasonName {
        let phase = self.phase();
        if phase < 0.25 { SeasonName::Spring }
        else if phase < 0.5 { SeasonName::Summer }
        else if phase < 0.75 { SeasonName::Autumn }
        else { SeasonName::Winter }
    }

    /// Light multiplier: high in summer, low in winter
    pub fn light_multiplier(&self) -> f32 {
        let phase = self.phase();
        // Sinusoidal: peaks at summer (0.375), troughs at winter (0.875)
        let seasonal = (phase * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2).sin();
        0.7 + seasonal * 0.3 // ranges 0.4 to 1.0
    }

    /// Food regen multiplier: high in spring/summer, low in winter
    pub fn food_regen_multiplier(&self) -> f32 {
        let phase = self.phase();
        let seasonal = (phase * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2).sin();
        0.6 + seasonal * 0.4 // ranges 0.2 to 1.0
    }

    /// Temperature modifier added to tile temperature
    pub fn temperature_modifier(&self) -> f32 {
        let phase = self.phase();
        let seasonal = (phase * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2).sin();
        seasonal * 0.3 // -0.3 in winter, +0.3 in summer
    }
}

/// A snapshot of population metrics at a point in time
#[derive(Clone, Default)]
pub struct PopSnapshot {
    pub organisms: u32,
    pub food: u32,
    pub species: u32,
    pub births_per_sec: u32,
    pub deaths_per_sec: u32,
    pub max_generation: u32,
    pub plants: u32,
    pub predators: u32,
    pub foragers: u32,
    pub avg_lifespan: f32,
    // Disease / health metrics
    pub infected: u32,
    pub avg_disease_resistance: f32,
    // Per-cause deaths for this one-second interval
    pub deaths_starvation: u32,
    pub deaths_predation: u32,
    pub deaths_old_age: u32,
    pub deaths_disease: u32,
    // Average evolved traits (for tuning)
    pub avg_body_size: f32,
    pub avg_speed: f32,
    pub avg_armor: f32,
    pub avg_attack: f32,
    pub avg_photo: f32,
}

/// Tracks organism lifespans for fitness measurement
#[derive(Resource, Default)]
pub struct FitnessTracker {
    pub recent_lifespans: Vec<u64>,
    pub avg_lifespan: f32,
}

/// Ring buffer of population history for graphing
#[derive(Resource)]
pub struct PopulationHistory {
    pub snapshots: Vec<PopSnapshot>,
    pub max_entries: usize,
    pub visible: bool,
    prev_births: u64,
    prev_deaths: u64,
    prev_deaths_by_cause: [u64; 4],
}

impl Default for PopulationHistory {
    fn default() -> Self {
        Self {
            snapshots: Vec::new(),
            max_entries: 300, // 5 minutes at 1 snapshot/sec
            visible: true,
            prev_births: 0,
            prev_deaths: 0,
            prev_deaths_by_cause: [0; 4],
        }
    }
}

impl PopulationHistory {
    #[allow(clippy::too_many_arguments)]
    pub fn record(&mut self, stats: &SimStats, snapshot: PopSnapshotInput) {
        let births_per_sec = (stats.total_births - self.prev_births) as u32;
        let deaths_per_sec = (stats.total_deaths - self.prev_deaths) as u32;
        self.prev_births = stats.total_births;
        self.prev_deaths = stats.total_deaths;

        let ds = (stats.deaths_by_cause[0] - self.prev_deaths_by_cause[0]) as u32;
        let dp = (stats.deaths_by_cause[1] - self.prev_deaths_by_cause[1]) as u32;
        let da = (stats.deaths_by_cause[2] - self.prev_deaths_by_cause[2]) as u32;
        let dd = (stats.deaths_by_cause[3] - self.prev_deaths_by_cause[3]) as u32;
        self.prev_deaths_by_cause = stats.deaths_by_cause;

        self.snapshots.push(PopSnapshot {
            organisms: snapshot.organisms,
            food: snapshot.food,
            species: stats.species_count,
            births_per_sec,
            deaths_per_sec,
            max_generation: stats.max_generation,
            plants: snapshot.plants,
            predators: snapshot.predators,
            foragers: snapshot.foragers,
            avg_lifespan: snapshot.avg_lifespan,
            infected: snapshot.infected,
            avg_disease_resistance: snapshot.avg_disease_resistance,
            deaths_starvation: ds,
            deaths_predation: dp,
            deaths_old_age: da,
            deaths_disease: dd,
            avg_body_size: snapshot.avg_body_size,
            avg_speed: snapshot.avg_speed,
            avg_armor: snapshot.avg_armor,
            avg_attack: snapshot.avg_attack,
            avg_photo: snapshot.avg_photo,
        });

        if self.snapshots.len() > self.max_entries {
            self.snapshots.remove(0);
        }
    }
}

/// Helper struct for passing many values into PopulationHistory::record
#[derive(Default)]
pub struct PopSnapshotInput {
    pub organisms: u32,
    pub food: u32,
    pub plants: u32,
    pub predators: u32,
    pub foragers: u32,
    pub avg_lifespan: f32,
    pub infected: u32,
    pub avg_disease_resistance: f32,
    pub avg_body_size: f32,
    pub avg_speed: f32,
    pub avg_armor: f32,
    pub avg_attack: f32,
    pub avg_photo: f32,
}

/// Tracks whether egui is currently capturing mouse/keyboard input
/// so world-view systems can gate their handlers
#[derive(Resource, Default)]
pub struct UiInputState {
    pub wants_keyboard: bool,
    pub pointer_over_ui: bool,
}

/// World event requests — fired by UI buttons or keyboard shortcuts,
/// consumed by the mass_extinction_input_system
#[derive(Event, Clone, Copy, Debug)]
pub enum WorldEventRequest {
    Asteroid,
    IceAge,
    Volcano,
    SolarBloom,
    NutrientRain,
    CambrianSpark,
    Save,
}

/// Active temporary bloom effects — decay over time
#[derive(Resource, Default)]
pub struct BloomEffects {
    /// Light multiplier boost (decays to 0)
    pub solar_bloom: f32,
    /// Mutation rate multiplier (decays to 1)
    pub mutation_boost: f32,
    /// Remaining ticks for each effect
    pub solar_ticks: u64,
    pub mutation_ticks: u64,
}

impl BloomEffects {
    pub fn light_multiplier(&self) -> f32 {
        if self.solar_ticks > 0 { self.solar_bloom } else { 1.0 }
    }

    pub fn mutation_multiplier(&self) -> f32 {
        if self.mutation_ticks > 0 { self.mutation_boost } else { 1.0 }
    }

    pub fn tick(&mut self) {
        if self.solar_ticks > 0 { self.solar_ticks -= 1; }
        if self.mutation_ticks > 0 { self.mutation_ticks -= 1; }
    }
}

/// Simulation speed: 0 = paused, 1 = normal, 2+ = fast
#[derive(Resource)]
pub struct SimSpeed {
    pub paused: bool,
    pub multiplier: f32,
}

impl Default for SimSpeed {
    fn default() -> Self {
        Self {
            paused: false,
            multiplier: 1.0,
        }
    }
}

/// Map from species ID to display colour
#[derive(Resource, Default)]
pub struct SpeciesColors {
    pub colors: std::collections::HashMap<u64, Color>,
    next_hue: f32,
}

impl SpeciesColors {
    pub fn get_or_create(&mut self, species_id: u64) -> Color {
        *self.colors.entry(species_id).or_insert_with(|| {
            let hue = self.next_hue;
            self.next_hue = (self.next_hue + 0.618033988) % 1.0; // golden ratio for good spread
            Color::hsl(hue * 360.0, 0.7, 0.6)
        })
    }
}

/// Currently selected organism for inspection
#[derive(Resource, Default)]
pub struct SelectedOrganism {
    pub entity: Option<Entity>,
}

#[derive(Component)]
pub struct Organism;

#[derive(Component)]
pub struct Energy(pub f32);

#[derive(Component)]
pub struct Health(pub f32);

#[derive(Component)]
pub struct Position(pub Vec2);

#[derive(Component)]
pub struct Velocity(pub Vec2);

/// Memory slots for recurrent brain connections
#[derive(Component, Clone)]
pub struct BrainMemory(pub [f32; 3]);

/// Chemical signal emitted by an organism — sensed by nearby organisms
#[derive(Component, Clone, Default)]
pub struct Signal(pub f32);

/// Tracks the last notable action for visual feedback
#[derive(Component, Clone, Default)]
pub struct ActionFlash {
    pub action: ActionType,
    pub timer: f32, // counts down from 0.3 to 0
}

#[derive(Clone, Default, PartialEq)]
pub enum ActionType {
    #[default]
    None,
    Eating,
    Attacking,
    Reproducing,
}

#[derive(Component)]
pub struct BodySize(pub f32);

#[derive(Component)]
pub struct Age(pub u64);

#[derive(Component)]
pub struct Generation(pub u32);

#[derive(Component)]
pub struct SpeciesId(pub u64);

/// Number of same-species organisms nearby — computed by sensing, used by metabolism
#[derive(Component, Default)]
pub struct GroupSize(pub u32);

/// Brief visual marker spawned where an organism dies
#[derive(Component)]
pub struct DeathMarker {
    pub timer: f32,
    pub was_predated: bool, // true = killed by predator, false = starvation/old age
}

/// Tracks parentage for lineage display
#[derive(Component, Default)]
pub struct ParentInfo {
    pub parent_species_id: Option<u64>,
}

/// An organism's current infection state. Absent = healthy.
/// Severity and ticks_remaining are sampled on infection and decrement over time.
#[derive(Component, Default, Clone)]
pub struct Infection {
    pub severity: f32,       // 0.0-1.0, scales energy drain and transmission
    pub ticks_remaining: u32, // counts down to 0 = recovered
}

/// Ring buffer of recent positions, used for drawing trails behind organisms.
/// Sampled every N ticks by the sim, consumed by the render trail system.
#[derive(Component, Default)]
pub struct TrailHistory {
    pub positions: std::collections::VecDeque<Vec2>,
}

impl TrailHistory {
    pub const MAX_LEN: usize = 20;

    pub fn push(&mut self, pos: Vec2) {
        if self.positions.len() >= Self::MAX_LEN {
            self.positions.pop_front();
        }
        self.positions.push_back(pos);
    }
}

/// The master RNG for all simulation randomness. Seeded at startup from
/// `SimConfig::terrain_seed`. Same seed → same sim — used for
/// organism placement, food regen, mutation, disease rolls, reproduction.
///
/// Safe to take as `ResMut<SimRng>` in any FixedUpdate system because
/// the sim schedule is strictly `.chain()`-ed (no parallel access).
/// Interactive randomness (keyboard triggers, R-key random select) uses
/// thread_rng — not part of the reproducible sim stream.
#[derive(Resource)]
pub struct SimRng(pub StdRng);

impl SimRng {
    pub fn from_seed(seed: u64) -> Self {
        Self(StdRng::seed_from_u64(seed))
    }
}

/// Per-tick snapshot of all food items (entity, position, energy).
/// Populated once at the start of FixedUpdate, read by sensing and action systems.
/// Avoids rebuilding the same Vec twice per tick.
#[derive(Resource, Default)]
pub struct FoodSnapshot {
    pub entries: Vec<(Entity, Vec2, f32)>,
}

/// Whether organism trails are rendered
#[derive(Resource)]
pub struct TrailsVisible(pub bool);

impl Default for TrailsVisible {
    fn default() -> Self {
        Self(false) // off by default — opt in with T
    }
}

#[derive(Component)]
pub struct Food;

#[derive(Component)]
pub struct FoodEnergy(pub f32);
