use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        // Only create a normal Session if the app hasn't already inserted one.
        // Headless mode pre-inserts a `Session::new_ephemeral()` so we don't
        // litter sessions/ with tiny throwaway directories.
        if !app.world().contains_resource::<Session>() {
            let session = Session::new();
            info!("Session: {} ({})", session.name, session.dir.display());
            app.insert_resource(session);
        }
        app.add_event::<WorldEventRequest>()
            .insert_resource(SimConfig::default())
            .insert_resource(SimStats::default())
            .insert_resource(PredationStats::default())
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
            .insert_resource(FoodSnapshot::default())
            .insert_resource(EnergyLedger::default());
    }
}

/// A named session with a storage directory for logs and screenshots
#[derive(Resource)]
pub struct Session {
    pub name: String,
    pub dir: PathBuf,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
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

    /// A session that doesn't create an on-disk directory. For headless
    /// runs, tests, or any mode where the cosmic-named sessions/<name>/
    /// folder would be noise.
    /// Create or reuse a session directory with the given name. Intended
    /// for headless `--save-as <name>` — same name on rerun overwrites
    /// the previous save in-place (no `-2`, `-3` collision suffixes).
    pub fn with_name(name: &str) -> Self {
        let dir = PathBuf::from("sessions").join(name);
        std::fs::create_dir_all(&dir).expect("Failed to create session directory");
        Self {
            name: name.to_string(),
            dir,
        }
    }

    pub fn new_ephemeral() -> Self {
        Self {
            name: format!("ephemeral-{}", Self::generate_name()),
            dir: PathBuf::from("/dev/null"),
        }
    }

    fn generate_name() -> String {
        let mut rng = rand::thread_rng();

        let adjectives1 = [
            "ancient",
            "astral",
            "barren",
            "bright",
            "cerulean",
            "cosmic",
            "dark",
            "distant",
            "eternal",
            "ethereal",
            "feral",
            "frozen",
            "gilded",
            "glacial",
            "golden",
            "hidden",
            "infinite",
            "iridescent",
            "jade",
            "keen",
            "kindred",
            "luminous",
            "lunar",
            "midnight",
            "molten",
            "nascent",
            "nebular",
            "obsidian",
            "pale",
            "primal",
            "quiet",
            "radiant",
            "scarlet",
            "silent",
            "spectral",
            "stellar",
            "tethered",
            "twisted",
            "vast",
            "veiled",
            "violet",
            "wandering",
            "young",
            "zealous",
        ];

        let adjectives2 = [
            "arcing",
            "blazing",
            "burning",
            "collapsing",
            "crystalline",
            "dormant",
            "drifting",
            "echoing",
            "eroding",
            "fading",
            "fractal",
            "glowing",
            "grinding",
            "hollow",
            "humming",
            "ignited",
            "iron",
            "jagged",
            "jeweled",
            "kindled",
            "latticed",
            "living",
            "massive",
            "migrating",
            "nameless",
            "orbital",
            "ossified",
            "petrified",
            "pulsing",
            "quaking",
            "restless",
            "roiling",
            "shattered",
            "spiraling",
            "tidal",
            "tumbling",
            "unbound",
            "undying",
            "volatile",
            "withering",
            "woven",
        ];

        let nouns = [
            "abyss", "apex", "aurora", "bloom", "caldera", "canyon", "cinder", "comet", "corona",
            "crater", "crown", "delta", "drift", "dusk", "eclipse", "ember", "flare", "flux",
            "forge", "genesis", "geyser", "glacier", "haven", "helix", "horizon", "lagoon",
            "mantle", "nebula", "nova", "pinnacle", "plume", "pulsar", "quasar", "remnant", "rift",
            "shard", "solstice", "spire", "storm", "summit", "tide", "void", "vortex", "zenith",
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
            // Tuning sweep across 2.0, 1.5, 1.3, 1.0 at 15k ticks × 3 seeds
            // each. 1.0 preserves species count but over-speciates to the
            // point minority strategies can't find mates and collapse (2/3
            // seeds hit full plant monoculture). 1.3 is the sweet spot:
            // diversity holds longer, and one seed showed a healthy
            // 22-species / 80-predator ecosystem. See DECISIONS.md.
            species_compat_threshold: 1.3,
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
    /// Killed by a world event (asteroid impact, volcanic eruption).
    Event = 4,
}

/// Number of `DeathCause` variants; the length of every per-cause array.
pub const DEATH_CAUSE_COUNT: usize = 5;

/// Run-wide counters. Live population and food counts are not kept here;
/// `record_population_history` recounts them once per second into
/// `PopulationHistory`, and readers take the latest snapshot from there.
#[derive(Resource, Default)]
pub struct SimStats {
    pub total_births: u64,
    pub total_deaths: u64,
    pub max_generation: u32,
    pub species_count: u32,
    /// Deaths categorised by cause, indexed by DeathCause as usize.
    /// The entries sum to `total_deaths`.
    pub deaths_by_cause: [u64; DEATH_CAUSE_COUNT],
}

/// Instrumentation counters for the attack path. Rolled up over a whole run
/// so we can diagnose why "Predator"-classified genomes produce zero
/// predation deaths — is the brain never firing the attack output, are
/// targets out of range, is the size gate rejecting, or is damage too low
/// after armor?
#[derive(Resource, Default)]
pub struct PredationStats {
    /// `output.attack > 0.5` fired this tick
    pub attacks_attempted: u64,
    /// Attacker found at least one neighbour entity within attack range
    pub targets_considered: u64,
    /// Candidate rejected: attacker body size <= prey body size * 0.6
    pub rejected_size_gate: u64,
    /// Candidate rejected: damage (claw vs armor) <= 0.1
    pub rejected_damage: u64,
    /// Successful kills
    pub kills: u64,
}

#[derive(Resource)]
pub struct TickCounter(pub u64);

/// Seasonal cycle — affects light, temperature, food regen
#[derive(Resource)]
pub struct Season {
    pub cycle_ticks: u64, // ticks per full year
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
        if phase < 0.25 {
            SeasonName::Spring
        } else if phase < 0.5 {
            SeasonName::Summer
        } else if phase < 0.75 {
            SeasonName::Autumn
        } else {
            SeasonName::Winter
        }
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
    /// Simulation tick at which the snapshot was taken.
    pub tick: u64,
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
    pub deaths_event: u32,
    // Average evolved traits (for tuning)
    pub avg_body_size: f32,
    pub avg_speed: f32,
    pub avg_armor: f32,
    pub avg_attack: f32,
    pub avg_photo: f32,
    // Symbiosis metrics
    pub symbiotic_pairs: u32,
    pub avg_symbiosis_rate: f32,
    // Energy ledger: total live energy at the sample, the flows moved during
    // this one-second interval, and the largest per-tick residual in it.
    pub energy_total: f32,
    pub energy_flows: EnergyFlows,
    pub ledger_max_residual: f32,
    pub ledger_cumulative_residual: f32,
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
    /// Ring-buffer cap, sized for the Graphs tab. Headless mode raises it to
    /// `usize::MAX` so `--dump-history` writes the whole run.
    pub max_entries: usize,
    pub visible: bool,
    prev_births: u64,
    prev_deaths: u64,
    prev_deaths_by_cause: [u64; DEATH_CAUSE_COUNT],
    prev_flows: EnergyFlows,
}

impl Default for PopulationHistory {
    fn default() -> Self {
        Self {
            snapshots: Vec::new(),
            max_entries: 300, // 5 minutes at 1 snapshot/sec
            visible: true,
            prev_births: 0,
            prev_deaths: 0,
            prev_deaths_by_cause: [0; DEATH_CAUSE_COUNT],
            prev_flows: EnergyFlows::default(),
        }
    }
}

impl PopulationHistory {
    #[allow(clippy::too_many_arguments)]
    pub fn record(
        &mut self,
        stats: &SimStats,
        ledger: &mut EnergyLedger,
        snapshot: PopSnapshotInput,
    ) {
        let births_per_sec = (stats.total_births - self.prev_births) as u32;
        let deaths_per_sec = (stats.total_deaths - self.prev_deaths) as u32;
        self.prev_births = stats.total_births;
        self.prev_deaths = stats.total_deaths;

        // Flows since the previous sample; the ledger's interval maximum is
        // read and reset here so the snapshot owns the peak for its second.
        let energy_flows = ledger.cumulative.minus(&self.prev_flows);
        self.prev_flows = ledger.cumulative;
        let ledger_max_residual = ledger.take_interval_max_residual() as f32;

        let ds = (stats.deaths_by_cause[0] - self.prev_deaths_by_cause[0]) as u32;
        let dp = (stats.deaths_by_cause[1] - self.prev_deaths_by_cause[1]) as u32;
        let da = (stats.deaths_by_cause[2] - self.prev_deaths_by_cause[2]) as u32;
        let dd = (stats.deaths_by_cause[3] - self.prev_deaths_by_cause[3]) as u32;
        let de = (stats.deaths_by_cause[4] - self.prev_deaths_by_cause[4]) as u32;
        self.prev_deaths_by_cause = stats.deaths_by_cause;

        self.snapshots.push(PopSnapshot {
            tick: snapshot.tick,
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
            deaths_event: de,
            avg_body_size: snapshot.avg_body_size,
            avg_speed: snapshot.avg_speed,
            avg_armor: snapshot.avg_armor,
            avg_attack: snapshot.avg_attack,
            avg_photo: snapshot.avg_photo,
            symbiotic_pairs: snapshot.symbiotic_pairs,
            avg_symbiosis_rate: snapshot.avg_symbiosis_rate,
            energy_total: ledger.total as f32,
            energy_flows,
            ledger_max_residual,
            ledger_cumulative_residual: ledger.cumulative_residual as f32,
        });

        if self.snapshots.len() > self.max_entries {
            self.snapshots.remove(0);
        }
    }
}

/// Helper struct for passing many values into PopulationHistory::record
#[derive(Default)]
pub struct PopSnapshotInput {
    pub tick: u64,
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
    pub symbiotic_pairs: u32,
    pub avg_symbiosis_rate: f32,
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
        if self.solar_ticks > 0 {
            self.solar_bloom
        } else {
            1.0
        }
    }

    pub fn mutation_multiplier(&self) -> f32 {
        if self.mutation_ticks > 0 {
            self.mutation_boost
        } else {
            1.0
        }
    }

    pub fn tick(&mut self) {
        if self.solar_ticks > 0 {
            self.solar_ticks -= 1;
        }
        if self.mutation_ticks > 0 {
            self.mutation_ticks -= 1;
        }
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
            self.next_hue = (self.next_hue + 0.618_034) % 1.0; // golden ratio for good spread
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

/// Per-organism record of the last-tick neural activations, keyed by
/// neuron ID. Populated by `sensing_and_brain_system` for live UI
/// visualisation (brain activation heatmap in the Inspect tab).
#[derive(Component, Clone, Default, Debug)]
pub struct BrainActivations {
    pub values: std::collections::HashMap<u64, f32>,
}

/// Tracks a potential or confirmed symbiotic link to another organism.
/// Updated every tick by the symbiosis tracker — when `link_target` has
/// been the same entity for at least `SYMBIOSIS_LINK_THRESHOLD` consecutive
/// ticks AND the target reciprocates, the pair is considered "linked" and
/// exchanges energy per the `symbiosis_rate` genome trait.
#[derive(Component, Clone, Default, Debug)]
pub struct Symbiosis {
    pub link_target: Option<Entity>,
    pub link_ticks: u32,
}

/// Shared between sim (link activation) and UI (display): number of
/// consecutive ticks a mutual-nearest pair must hold before the link is
/// considered active and energy transfer kicks in.
/// Tuning history:
///   30 (~1s) — initial. Too strict; orgs move enough that 30 straight
///              ticks as each other's nearest is rare (3–35 pairs max).
///   10 (~0.3s) — (current) short enough that incidental closeness
///              forms a link; long enough that pure flybys don't.
pub const SYMBIOSIS_LINK_THRESHOLD: u32 = 10;

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

/// An organism that died this tick and is waiting for `death_system` to
/// despawn it. Carries the cause so attribution is recorded at the kill, not
/// inferred from health afterwards. Systems that run between the kill and
/// `death_system` (photosynthesis, symbiosis transfer) filter on
/// `Without<Killed>` so a corpse earns nothing.
#[derive(Component, Clone, Copy, Debug)]
pub struct Killed(pub DeathCause);

/// Brief visual marker spawned where an organism dies
#[derive(Component)]
pub struct DeathMarker {
    pub timer: f32,
    pub was_predated: bool, // true = killed by predator, false = starvation/old age/disease/event
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
    pub severity: f32,        // 0.0-1.0, scales energy drain and transmission
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
/// `SimConfig::terrain_seed`. Same seed → same early-sim trajectory.
///
/// Used for: organism placement, food regen, mutation, disease rolls,
/// reproduction, asteroid targets etc.
///
/// Safe to take as `ResMut<SimRng>` in any FixedUpdate system because
/// the sim schedule is strictly `.chain()`-ed (no parallel access).
/// Interactive randomness (keyboard triggers, R-key random select) uses
/// thread_rng — not part of the reproducible sim stream.
///
/// **Reproducibility limits:** runs with the same seed produce identical
/// state for the first ~50 ticks, then gradually diverge. Root cause is
/// Bevy's parallel task pool and archetype-based Query iteration, which
/// aren't themselves deterministic. Forcing a single-threaded task pool
/// would recover full determinism at a perf cost — see ROADMAP.
/// For most "same seed, similar outcome" use cases, what we have is
/// sufficient.
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

/// Whether organism trails are rendered. Off by default; opt in with T.
#[derive(Resource, Default)]
pub struct TrailsVisible(pub bool);

#[derive(Component)]
pub struct Food;

#[derive(Component)]
pub struct FoodEnergy(pub f32);

/// Energy moved by each simulation rule, as magnitudes. Every field is
/// non-negative except `death`, which is the signed energy an organism held
/// when it was removed and can be slightly negative when metabolism
/// overdrew it on its final tick.
///
/// Used two ways. As a component it is the per-organism scratch record that
/// `photosynthesis_system` and `metabolism_system` write from inside
/// `par_iter_mut`, where a shared resource cannot be touched; `ledger_system`
/// sums those records serially and zeroes them each tick, and `death_system`
/// folds in a dying organism's record before the despawn removes it. As the
/// `EnergyLedger` totals it is the same set of flows summed over the tick and
/// over the run.
#[derive(Component, Clone, Copy, Default, Debug, PartialEq)]
pub struct EnergyFlows {
    /// Sun energy credited to photosynthesisers.
    pub photosynthesis: f64,
    /// Energy from food items eaten.
    pub food: f64,
    /// Energy paid to a killer at a kill (the 10% pyramid share).
    pub predation: f64,
    /// Gross energy moved between symbiotic partners. A transfer, so it does
    /// not change the total and is not part of `net()`.
    pub symbiosis: f64,
    /// Per-tick body maintenance cost.
    pub metabolism: f64,
    /// Movement cost paid in `action_system`.
    pub movement: f64,
    /// Infection drain.
    pub disease: f64,
    /// Energy parents paid to reproduce.
    pub reproduction_spent: f64,
    /// Starting energy handed to children.
    pub reproduction_received: f64,
    /// Energy removed from the world with organisms that died, including the
    /// share of a victim's energy that does not reach its killer and the
    /// energy zeroed on a kill, a disease death, or an old-age death.
    pub death: f64,
    /// Income discarded by the `max_organism_energy` clamp.
    pub clamp: f64,
}

impl EnergyFlows {
    /// Signed change in total live energy these flows account for.
    pub fn net(&self) -> f64 {
        self.photosynthesis + self.food + self.predation + self.reproduction_received
            - self.metabolism
            - self.movement
            - self.disease
            - self.reproduction_spent
            - self.death
            - self.clamp
    }

    pub fn add(&mut self, other: &EnergyFlows) {
        self.photosynthesis += other.photosynthesis;
        self.food += other.food;
        self.predation += other.predation;
        self.symbiosis += other.symbiosis;
        self.metabolism += other.metabolism;
        self.movement += other.movement;
        self.disease += other.disease;
        self.reproduction_spent += other.reproduction_spent;
        self.reproduction_received += other.reproduction_received;
        self.death += other.death;
        self.clamp += other.clamp;
    }

    pub fn minus(&self, other: &EnergyFlows) -> EnergyFlows {
        EnergyFlows {
            photosynthesis: self.photosynthesis - other.photosynthesis,
            food: self.food - other.food,
            predation: self.predation - other.predation,
            symbiosis: self.symbiosis - other.symbiosis,
            metabolism: self.metabolism - other.metabolism,
            movement: self.movement - other.movement,
            disease: self.disease - other.disease,
            reproduction_spent: self.reproduction_spent - other.reproduction_spent,
            reproduction_received: self.reproduction_received - other.reproduction_received,
            death: self.death - other.death,
            clamp: self.clamp - other.clamp,
        }
    }

    pub fn clear(&mut self) {
        *self = EnergyFlows::default();
    }

    /// `(label, value)` pairs in display order, for summaries and CSV.
    pub fn entries(&self) -> [(&'static str, f64); 11] {
        [
            ("photosynthesis", self.photosynthesis),
            ("food", self.food),
            ("predation", self.predation),
            ("symbiosis", self.symbiosis),
            ("metabolism", self.metabolism),
            ("movement", self.movement),
            ("disease", self.disease),
            ("reproduction_spent", self.reproduction_spent),
            ("reproduction_received", self.reproduction_received),
            ("death", self.death),
            ("clamp", self.clamp),
        ]
    }
}

/// Run-wide energy accounting. Serial systems add to `tick` directly at the
/// point they move energy; parallel systems write per-organism `EnergyFlows`
/// components that `ledger_system` sums into `tick`. At the end of each tick
/// `ledger_system` compares the change in total live energy against
/// `tick.net()`; the difference is the residual, which is zero up to f32
/// rounding when every energy movement has been recorded.
#[derive(Resource, Clone, Debug)]
pub struct EnergyLedger {
    /// Flows recorded so far in the current tick.
    pub tick: EnergyFlows,
    /// Flows summed over the whole run.
    pub cumulative: EnergyFlows,
    /// Total live organism energy at the end of the last closed tick.
    pub total: f64,
    /// Total live energy when the world was spawned or loaded.
    pub baseline: f64,
    /// Residual of the last closed tick.
    pub last_residual: f64,
    /// Largest absolute per-tick residual seen in the run.
    pub max_abs_residual: f64,
    /// Sum of every per-tick residual: the drift between the baseline plus
    /// the cumulative net flows and the live total.
    pub cumulative_residual: f64,
    /// Ticks whose residual exceeded `TOLERANCE`.
    pub breaches: u64,
    /// Tick of the last chronicle warning, for rate limiting.
    pub last_warning_tick: Option<u64>,
    /// Largest absolute residual since the last population snapshot.
    interval_max_residual: f64,
    /// Total at the end of the previous tick; `None` until the baseline is
    /// set by the spawn or load path.
    prev_total: Option<f64>,
}

impl Default for EnergyLedger {
    fn default() -> Self {
        Self {
            tick: EnergyFlows::default(),
            cumulative: EnergyFlows::default(),
            total: 0.0,
            baseline: 0.0,
            last_residual: 0.0,
            max_abs_residual: 0.0,
            cumulative_residual: 0.0,
            breaches: 0,
            last_warning_tick: None,
            interval_max_residual: 0.0,
            prev_total: None,
        }
    }
}

impl EnergyLedger {
    /// Per-tick residual above which the books are considered out of
    /// balance. Energy is stored as f32, so each write to an `Energy`
    /// component rounds by up to half an ulp: about 4e-6 at the 120 cap.
    /// With 2000 organisms and three to four energy writes each per tick, a
    /// fully aligned worst case is around 0.03 and the typical random-walk
    /// value is under 1e-3. 0.1 sits above the worst case and far below any
    /// rule-sized amount (the smallest is the 0.08 base metabolism cost of a
    /// single organism), so a breach is a missing or double-counted flow,
    /// not rounding.
    pub const TOLERANCE: f64 = 0.1;

    /// Minimum ticks between chronicle warnings about the residual.
    pub const WARNING_INTERVAL_TICKS: u64 = 300;

    /// Start the books from the energy of a freshly spawned or loaded
    /// population, so the first tick compares against a real total instead
    /// of zero.
    pub fn reset_baseline(&mut self, total: f64) {
        *self = EnergyLedger {
            total,
            baseline: total,
            prev_total: Some(total),
            ..EnergyLedger::default()
        };
    }

    pub fn has_baseline(&self) -> bool {
        self.prev_total.is_some()
    }

    /// Close the tick: record the residual against `total`, roll the tick's
    /// flows into the cumulative totals, and clear them. Returns the residual.
    pub fn close_tick(&mut self, total: f64) -> f64 {
        let prev = self.prev_total.unwrap_or(total);
        let residual = (total - prev) - self.tick.net();
        self.last_residual = residual;
        self.max_abs_residual = self.max_abs_residual.max(residual.abs());
        self.interval_max_residual = self.interval_max_residual.max(residual.abs());
        self.cumulative_residual += residual;
        if residual.abs() > Self::TOLERANCE {
            self.breaches += 1;
        }
        self.cumulative.add(&self.tick);
        self.tick.clear();
        self.total = total;
        self.prev_total = Some(total);
        residual
    }

    /// Whether a chronicle warning is due for this tick, and if so record it.
    pub fn should_warn(&mut self, tick: u64) -> bool {
        let due = match self.last_warning_tick {
            None => true,
            Some(last) => tick.saturating_sub(last) >= Self::WARNING_INTERVAL_TICKS,
        };
        if due {
            self.last_warning_tick = Some(tick);
        }
        due
    }

    /// The largest absolute residual since the last call, then reset.
    pub fn take_interval_max_residual(&mut self) -> f64 {
        std::mem::take(&mut self.interval_max_residual)
    }
}
