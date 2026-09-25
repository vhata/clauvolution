use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
            .add_event::<CameraFocusRequest>()
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

/// Shipped value of `SimConfig::strike_cost`. See `docs/DECISIONS.md`,
/// "Strike cost".
pub const DEFAULT_STRIKE_COST: f32 = 1.0;

#[derive(Resource, Clone, Serialize, Deserialize)]
pub struct SimConfig {
    pub world_width: u32,
    pub world_height: u32,
    pub initial_population: u32,
    /// Food items placed at tick 0, as a fraction of world tiles before the
    /// per-tile nutrient filter. Lowering it alone does not tame the founding
    /// boom (regeneration refills toward `max_food_density`); see
    /// `docs/DECISIONS.md`, "Per-biome seeding".
    pub initial_food_density: f32,
    /// Ceiling for `food_regeneration_system`, as a fraction of world tiles.
    /// Separate from `initial_food_density` so the starting stock and the
    /// regeneration ceiling can be tuned apart; equal today.
    pub max_food_density: f32,
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
    /// Fraction of a plant's current energy that one bite removes, before
    /// the mouth bonus. See `docs/DECISIONS.md`, "Grazing". Overridable with
    /// `--bite-fraction`.
    pub bite_fraction: f32,
    /// How far an eater can bite a living plant, as a multiple of its body
    /// size. 1.0: a bite needs contact, where a food item is taken from 3 ×
    /// body size; at 3.0 plants were grazed out on every seed. See
    /// `docs/DECISIONS.md`, "Grazing through eat". Overridable with
    /// `--bite-reach`.
    pub bite_reach: f32,
    /// Share of a plant bite an eater without a mouth segment takes (a
    /// mouthed eater takes all of it). Food items keep their own fixed
    /// mouthless share in `clauvolution_sim`. See `docs/DECISIONS.md`,
    /// "Grazing through eat". Overridable with `--mouthless-bite`.
    pub mouthless_bite_bonus: f32,
    /// Founders draw `diet` uniformly from `-spread..spread`. Overridable
    /// with `--founder-diet-spread`.
    pub founder_diet_spread: f32,
    /// Fraction of an animal victim's energy (a consumer, not a
    /// photosynthesiser) offered to its killer before digestion: the
    /// per-meal trophic share of a hunter's kill. See `docs/DECISIONS.md`,
    /// "Energy pyramid" and "Kill share by victim tissue". Overridable with
    /// `--kill-transfer-animal`, or with `--kill-transfer`, which sets both
    /// shares.
    pub kill_transfer_animal: f32,
    /// Fraction of a plant victim's energy (a photosynthesiser) offered to
    /// its killer before digestion. Overridable with `--kill-transfer-plant`,
    /// or with `--kill-transfer`, which sets both shares.
    pub kill_transfer_plant: f32,
    /// Energy an attacker pays per tick of a strike, per unit of strike
    /// force (`claw_power × body size`, the figure the damage gate reads).
    /// A strike is `attack` firing with a living organism within attack
    /// range; firing at nothing is free. Shipped at `DEFAULT_STRIKE_COST`,
    /// 1.0: below a hunter's typical kill, above a grazer's (about nothing).
    /// See `docs/DECISIONS.md`, "Strike cost". Overridable with
    /// `--strike-cost`.
    pub strike_cost: f32,
    /// Drag per unit of photosynthetic surface area in the speed formula:
    /// `speed × 1 / (1 + photo_area × photo_drag)`, beside armour's 0.3 per
    /// unit. A light-catching surface is broad and flat, so it is a sail.
    /// See `docs/DECISIONS.md`, "Photosynthetic surface drag". Overridable
    /// with `--photo-drag`.
    pub photo_drag: f32,
    /// Leaf area one tile can fully light. A photosynthesiser's light share is
    /// `min(1, window tiles × capacity / leaf area in the window)` over the
    /// canopy window around it, so where leaves exceed what the ground can
    /// light, everyone there is shaded in proportion. This is what sets how
    /// many fully lit plants the world holds. See `docs/DECISIONS.md`,
    /// "Canopy light sharing". Overridable with `--leaf-capacity`.
    pub leaf_capacity_per_tile: f32,
    /// Multiplier on every killer's animal digestion efficiency, 1.0 in the
    /// sim proper. `--animal-efficiency 0` is the interdependence test in
    /// `plans/2026-09-19-diet-axis.md`: nobody can live by hunting.
    pub animal_efficiency_multiplier: f32,
    /// Safety ceiling on the number of living organisms. Carrying capacity
    /// comes from energy: canopy light sharing (`leaf_capacity_per_tile`)
    /// shades crowded plants until they earn near their upkeep, and grazers
    /// eat them, so populations oscillate in the low thousands and only a
    /// seed whose consumers never take hold runs into this number. When it
    /// blocks births `reproduction_system` writes a chronicle entry per
    /// episode and counts it in `SimStats`, and births are admitted by
    /// probability rather than query order while it holds. Shipped at 2000
    /// as the historical cap until 2026-09-20; see DECISIONS.md "Emergent
    /// carrying capacity".
    pub population_ceiling: u32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            world_width: 512,
            world_height: 512,
            initial_population: 400,
            initial_food_density: 0.1,
            max_food_density: 0.1,
            food_regen_rate: 0.01,
            mutation_rate: 0.3,
            mutation_strength: 0.5,
            base_metabolism_cost: 0.08,
            movement_energy_cost: 0.04,
            reproduction_energy_threshold: 70.0,
            bite_fraction: 0.3,
            bite_reach: 1.0,
            mouthless_bite_bonus: 0.3,
            kill_transfer_animal: 0.1,
            kill_transfer_plant: 0.1,
            strike_cost: DEFAULT_STRIKE_COST,
            photo_drag: 1.0,
            leaf_capacity_per_tile: 0.02,
            founder_diet_spread: 1.0,
            animal_efficiency_multiplier: 1.0,
            reproduction_energy_cost: 40.0,
            max_organism_energy: 120.0,
            food_energy_value: 25.0,
            // Tuning sweep across 2.0, 1.5, 1.3, 1.0 at 15k ticks × 3 seeds
            // each. 1.0 preserves species count but over-speciates to the
            // point minority strategies can't find mates and collapse (2/3
            // seeds hit full plant monoculture). 1.3 is the sweet spot:
            // diversity holds longer, and one seed showed a healthy
            // 22-species / 80-predator ecosystem. See DECISIONS.md.
            species_compat_threshold: 1.0,
            terrain_seed: rand::random(),
            population_ceiling: 6000,
        }
    }
}

impl SimConfig {
    /// The share of a victim's energy offered to its killer before
    /// digestion, by the victim's tissue: `kill_transfer_plant` for a
    /// photosynthesiser, `kill_transfer_animal` for anything else.
    pub fn kill_share(&self, victim_is_plant: bool) -> f32 {
        if victim_is_plant {
            self.kill_transfer_plant
        } else {
            self.kill_transfer_animal
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
    /// Tick at which the current population-ceiling episode began, or
    /// `None` while the ceiling is not engaged. `reproduction_system` uses
    /// it to write one chronicle entry per episode rather than one per tick.
    pub ceiling_engaged_since: Option<u64>,
    /// Number of times `SimConfig::population_ceiling` has engaged.
    pub ceiling_episodes: u64,
    /// Births blocked by the population ceiling over the whole run.
    pub ceiling_blocked_births: u64,
    /// Founders (generation 0) labelled hunters (`diet >= 1/3`, not a
    /// photosynthesiser) that have died, the sum of their ages at death in
    /// ticks, and the oldest of them. `hunter-emergence` reads how long a
    /// founding hunter lives from these.
    pub founder_hunter_deaths: u64,
    pub founder_hunter_age_sum: u64,
    pub founder_hunter_age_max: u64,
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
    /// Attack intents with a living organism within attack range, each
    /// charged `SimConfig::strike_cost` per unit of strike force.
    pub strikes: u64,
    /// Energy attackers paid for those strikes (booked to the ledger's
    /// `movement` flow).
    pub strike_energy: f64,
    /// Who fed on whom, and through which output. Counted over the whole
    /// run; `PopulationHistory` diffs it into per-second values for the
    /// Graphs tab and the history CSV.
    pub feeding: FeedingCounts,
    /// Gate outcomes for consumer attackers with `diet >= 0` (hunters
    /// included). The hunter band is in `feeding.hunter_gates`, so it reaches
    /// the history CSV; this wider band is a run total only.
    pub diet_nonneg_gates: GateOutcomes,
    /// Gate outcomes for founding hunters (generation 0, labelled hunter).
    pub founder_hunter_gates: GateOutcomes,
    /// Hunter attack intents by the attacker's age, in the buckets of
    /// `AGE_BUCKETS`.
    pub hunter_intent_ages: [u64; AGE_BUCKET_COUNT],
    /// Hunter attacks with a consumer in reach, by the attacker's age.
    pub hunter_reach_ages: [u64; AGE_BUCKET_COUNT],
    /// Founding hunters that fired `attack` at least once, and the sum of
    /// their ages at the first intent.
    pub founder_hunters_fired: HashSet<Entity>,
    pub founder_hunter_first_intent_age_sum: u64,
    /// Founding hunters that fired with a consumer in reach at least once,
    /// and the sum of their ages the first time.
    pub founder_hunters_reached: HashSet<Entity>,
    pub founder_hunter_first_reach_age_sum: u64,
    /// Founding hunters that killed at least one consumer.
    pub founder_hunters_killed: HashSet<Entity>,
    /// Plants killed by hunters (the strategy label, `diet >= 1/3` and not a
    /// photosynthesiser), and the energy those hunters kept from them after
    /// digestion. A run total only, so the history CSV is unchanged.
    pub hunter_plant_kills: u64,
    pub hunter_plant_kill_energy: f64,
}

/// Upper bounds (exclusive, in ticks of age) of the attacker-age buckets the
/// hunter gate counters use; the last bucket is everything older.
pub const AGE_BUCKETS: [u64; 5] = [100, 200, 300, 500, 1000];
pub const AGE_BUCKET_COUNT: usize = AGE_BUCKETS.len() + 1;

/// The bucket of `AGE_BUCKETS` an age falls in.
pub fn age_bucket(age: u64) -> usize {
    AGE_BUCKETS
        .iter()
        .position(|&upper| age < upper)
        .unwrap_or(AGE_BUCKETS.len())
}

/// What happened to the attack intents of one band of attackers
/// (`plans/2026-09-21-pyramid-top.md`, step 5). The partition that matters
/// is over `consumer_in_reach`: attacks with at least one living,
/// unclaimed non-photosynthesiser within attack range. Each such attack is
/// counted in exactly one of the six outcome fields.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct GateOutcomes {
    /// `attack > 0.5` fired.
    pub intents: u64,
    /// Intents with any living organism within attack range (a paid strike).
    pub strikes: u64,
    /// Intents with at least one living, unclaimed consumer within range.
    pub consumer_in_reach: u64,
    /// The strike killed a consumer.
    pub kills_consumer: u64,
    /// A consumer passed both gates but a nearer plant was struck.
    pub kills_plant_instead: u64,
    /// No consumer passed both gates; at least one passed the damage gate
    /// and none passed the size gate. Lifting the size gate alone kills.
    pub rejected_size: u64,
    /// No consumer passed the damage gate; at least one passed the size
    /// gate. Lifting the damage gate alone kills.
    pub rejected_damage: u64,
    /// Every consumer in reach failed both gates.
    pub rejected_both: u64,
    /// No consumer passed both, but some passed each: different targets
    /// failed different gates.
    pub rejected_mixed: u64,
}

impl GateOutcomes {
    /// Record one attack's outcome against the consumers in reach. `any_size`
    /// / `any_damage` / `any_both` say whether some consumer passed the size
    /// gate, the damage gate, or both; `struck_consumer` whether the strike
    /// landed on a consumer.
    pub fn record_consumer_attack(
        &mut self,
        any_size: bool,
        any_damage: bool,
        any_both: bool,
        struck_consumer: bool,
    ) {
        self.consumer_in_reach += 1;
        if any_both {
            if struck_consumer {
                self.kills_consumer += 1;
            } else {
                self.kills_plant_instead += 1;
            }
        } else {
            match (any_size, any_damage) {
                (false, true) => self.rejected_size += 1,
                (true, false) => self.rejected_damage += 1,
                (false, false) => self.rejected_both += 1,
                (true, true) => self.rejected_mixed += 1,
            }
        }
    }

    pub fn minus(&self, other: &GateOutcomes) -> GateOutcomes {
        GateOutcomes {
            intents: self.intents - other.intents,
            strikes: self.strikes - other.strikes,
            consumer_in_reach: self.consumer_in_reach - other.consumer_in_reach,
            kills_consumer: self.kills_consumer - other.kills_consumer,
            kills_plant_instead: self.kills_plant_instead - other.kills_plant_instead,
            rejected_size: self.rejected_size - other.rejected_size,
            rejected_damage: self.rejected_damage - other.rejected_damage,
            rejected_both: self.rejected_both - other.rejected_both,
            rejected_mixed: self.rejected_mixed - other.rejected_mixed,
        }
    }
}

/// Trophic counters for the graze/attack split
/// (`plans/2026-09-21-pyramid-top.md`, step 1). They separate the two ways
/// a consumer can take plant tissue and say who is killing whom, so that
/// moving grazing from `attack` to `eat` can be judged by numbers.
///
/// "Grazer" here means a killer whose `diet` is below 0 (plant-leaning),
/// a wider band than the grazer strategy label (`diet <= -1/3`), so that
/// omnivores on the plant side are counted as well. "Consumer" means a
/// victim that is not a photosynthesiser (`Genome::is_photosynthesiser`).
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct FeedingCounts {
    /// Bites taken from a living plant through the `eat` output
    /// (`grazing_system`, since step 2 of the plan).
    pub grazes_eat: u64,
    /// Bites taken from a living plant through the `attack` output. Always
    /// 0 since step 2, where an attack on a plant is a kill attempt; kept so
    /// history CSVs line up with the step 1 baseline.
    pub grazes_attack: u64,
    /// The part of `grazes_eat` whose eater is itself a photosynthesiser.
    pub grazes_eat_by_plant: u64,
    /// Kills whose victim is a consumer.
    pub kills_consumer: u64,
    /// Kills whose victim is a photosynthesiser.
    pub kills_plant: u64,
    /// The part of `kills_plant` whose killer is not a photosynthesiser.
    pub kills_plant_by_consumer: u64,
    /// Energy kept (after digestion) by non-photosynthesiser killers from
    /// the plants they killed: how much plant kills feed consumers.
    pub plant_kill_energy_consumer: f64,
    /// Kills by a killer with `diet < 0`, any victim.
    pub grazer_kills: u64,
    /// Kills by a killer with `diet < 0` whose victim is a consumer: the
    /// bystander kills the split is meant to remove.
    pub grazer_kills_consumer: u64,
    /// Attack intents (`attack > 0.5`) with no living photosynthesiser
    /// within attack range, whether or not another attacker had claimed it.
    pub attacks_no_plant_in_reach: u64,
    /// Gate outcomes for hunter attackers (`diet >= 1/3`, not a
    /// photosynthesiser). See `GateOutcomes`.
    pub hunter_gates: GateOutcomes,
}

impl FeedingCounts {
    /// Count one kill by who made it and what it killed. `kept` is the
    /// energy the killer kept after digestion.
    pub fn record_kill(
        &mut self,
        killer_diet: f32,
        killer_is_plant: bool,
        victim_is_plant: bool,
        kept: f32,
    ) {
        if victim_is_plant {
            self.kills_plant += 1;
            if !killer_is_plant {
                self.kills_plant_by_consumer += 1;
                self.plant_kill_energy_consumer += kept as f64;
            }
        } else {
            self.kills_consumer += 1;
        }
        if killer_diet < 0.0 {
            self.grazer_kills += 1;
            if !victim_is_plant {
                self.grazer_kills_consumer += 1;
            }
        }
    }

    /// All bites of living plants, through either output.
    pub fn grazes(&self) -> u64 {
        self.grazes_eat + self.grazes_attack
    }

    /// Per-field difference, for per-interval values from two cumulative
    /// readings.
    pub fn minus(&self, other: &FeedingCounts) -> FeedingCounts {
        FeedingCounts {
            grazes_eat: self.grazes_eat - other.grazes_eat,
            grazes_attack: self.grazes_attack - other.grazes_attack,
            grazes_eat_by_plant: self.grazes_eat_by_plant - other.grazes_eat_by_plant,
            kills_consumer: self.kills_consumer - other.kills_consumer,
            kills_plant: self.kills_plant - other.kills_plant,
            kills_plant_by_consumer: self.kills_plant_by_consumer - other.kills_plant_by_consumer,
            plant_kill_energy_consumer: self.plant_kill_energy_consumer
                - other.plant_kill_energy_consumer,
            grazer_kills: self.grazer_kills - other.grazer_kills,
            grazer_kills_consumer: self.grazer_kills_consumer - other.grazer_kills_consumer,
            attacks_no_plant_in_reach: self.attacks_no_plant_in_reach
                - other.attacks_no_plant_in_reach,
            hunter_gates: self.hunter_gates.minus(&other.hunter_gates),
        }
    }
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
    // Strategy counts; see `classify_strategy` in `clauvolution_phylogeny`.
    pub plants: u32,
    pub grazers: u32,
    pub hunters: u32,
    pub omnivores: u32,
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
    /// Mean `Genome::diet` over non-plants, -1 herbivore to +1 carnivore.
    /// Plants carry the trait but never eat, and at 95% of the population
    /// they swamped the mean, so they are left out. 0.0 when no consumer
    /// is alive.
    pub avg_diet: f32,
    // Plant physics instruments (`plans/2026-09-20-plant-physics.md`): how
    // plants and eaters differ in motion, light, and readiness to breed.
    /// Mean photo surface area over plants.
    pub avg_photo_area: f32,
    /// Mean movement per tick (world units) over plants and over eaters.
    pub avg_speed_plants: f32,
    pub avg_speed_eaters: f32,
    /// Mean `LightShare` over plants, 0..1.
    pub avg_light_share: f32,
    /// Share of plants and of eaters whose energy is above their own
    /// reproduction threshold: who is ready when a birth slot opens.
    pub ready_share_plants: f32,
    pub ready_share_eaters: f32,
    // Symbiosis metrics
    pub symbiotic_pairs: u32,
    pub avg_symbiosis_rate: f32,
    // Energy ledger: total live energy at the sample, the flows moved during
    // this one-second interval, and the largest per-tick residual in it.
    pub energy_total: f32,
    pub energy_flows: EnergyFlows,
    pub ledger_max_residual: f32,
    pub ledger_cumulative_residual: f32,
    /// Grazes and kills during this one-second interval, by output and by
    /// who killed whom. See `FeedingCounts`.
    pub feeding: FeedingCounts,
    /// Mean body size and armour value over labelled grazers
    /// (`diet <= -1/3`, not a photosynthesiser); 0.0 when none is alive.
    /// Read by the headless summary's grazer timeline, not written to the
    /// history CSV.
    pub avg_grazer_body_size: f32,
    pub avg_grazer_armor: f32,
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
    prev_feeding: FeedingCounts,
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
            prev_feeding: FeedingCounts::default(),
        }
    }
}

impl PopulationHistory {
    #[allow(clippy::too_many_arguments)]
    pub fn record(
        &mut self,
        stats: &SimStats,
        predation: &PredationStats,
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

        let feeding = predation.feeding.minus(&self.prev_feeding);
        self.prev_feeding = predation.feeding;

        self.snapshots.push(PopSnapshot {
            tick: snapshot.tick,
            organisms: snapshot.organisms,
            food: snapshot.food,
            species: stats.species_count,
            births_per_sec,
            deaths_per_sec,
            max_generation: stats.max_generation,
            plants: snapshot.plants,
            grazers: snapshot.grazers,
            hunters: snapshot.hunters,
            omnivores: snapshot.omnivores,
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
            avg_diet: snapshot.avg_diet,
            avg_photo_area: snapshot.avg_photo_area,
            avg_speed_plants: snapshot.avg_speed_plants,
            avg_speed_eaters: snapshot.avg_speed_eaters,
            avg_light_share: snapshot.avg_light_share,
            ready_share_plants: snapshot.ready_share_plants,
            ready_share_eaters: snapshot.ready_share_eaters,
            symbiotic_pairs: snapshot.symbiotic_pairs,
            avg_symbiosis_rate: snapshot.avg_symbiosis_rate,
            energy_total: ledger.total as f32,
            energy_flows,
            ledger_max_residual,
            ledger_cumulative_residual: ledger.cumulative_residual as f32,
            feeding,
            avg_grazer_body_size: snapshot.avg_grazer_body_size,
            avg_grazer_armor: snapshot.avg_grazer_armor,
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
    pub grazers: u32,
    pub hunters: u32,
    pub omnivores: u32,
    pub avg_lifespan: f32,
    pub infected: u32,
    pub avg_disease_resistance: f32,
    pub avg_body_size: f32,
    pub avg_speed: f32,
    pub avg_armor: f32,
    pub avg_attack: f32,
    pub avg_photo: f32,
    pub avg_diet: f32,
    pub avg_photo_area: f32,
    pub avg_speed_plants: f32,
    pub avg_speed_eaters: f32,
    pub avg_light_share: f32,
    pub ready_share_plants: f32,
    pub ready_share_eaters: f32,
    pub symbiotic_pairs: u32,
    pub avg_symbiosis_rate: f32,
    pub avg_grazer_body_size: f32,
    pub avg_grazer_armor: f32,
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
    /// Write one organism's genome and metadata to a creature file in the
    /// session directory, for `--seed-with` in another world.
    ExportOrganism(Entity),
}

/// Outcome of the most recent organism export, for the Inspect panel and
/// any other reader outside the sim crate. `None` until an export has
/// been attempted this run.
#[derive(Resource, Default)]
pub struct OrganismExportReport {
    /// The path written on success, or the error text on failure.
    pub last: Option<Result<PathBuf, String>>,
}

/// Ask the camera to centre on a world position. Fired by UI elements that
/// name a place (a chronicle entry for a volcano, for instance) and consumed
/// by the render crate's camera system, which is the only thing that moves
/// the camera. The UI crate cannot reach the camera directly because it does
/// not depend on the render crate.
#[derive(Event, Clone, Copy, Debug, PartialEq)]
pub struct CameraFocusRequest(pub Vec2);

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

/// The share of full light a photosynthesiser received on its last tick,
/// 0..1, written by `photosynthesis_system`. Today it is the per-tile density
/// factor; `plans/2026-09-20-plant-physics.md` makes it canopy shading. An
/// instrument first: the Graphs tab and the history average it over plants.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct LightShare(pub f32);

/// Memory slots for recurrent brain connections
#[derive(Component, Clone)]
pub struct BrainMemory(pub [f32; 3]);

/// Per-organism record of the last-tick neural activations, indexed by the
/// organism's `Brain` slots; read one neuron with `Brain::activation`.
/// Populated by `sensing_and_brain_system` for live UI visualisation
/// (brain activation heatmap in the Inspect tab). The buffer is reused
/// every tick.
#[derive(Component, Clone, Default, Debug)]
pub struct BrainActivations {
    pub values: Vec<f32>,
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
    /// Took a bite out of a living plant.
    Grazing,
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

/// Seconds of virtual time a `DeathMarker` lives before the sim despawns it.
pub const DEATH_MARKER_SECS: f32 = 0.5;

/// Brief visual marker spawned where an organism dies. The sim owns its
/// lifetime (`death_marker_expiry_system` counts `timer` down each tick and
/// despawns it at zero) so markers expire in headless runs too; the render
/// crate only reads `timer` to animate the fade.
#[derive(Component)]
pub struct DeathMarker {
    /// Seconds left before expiry, starting at `DEATH_MARKER_SECS`.
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
/// **Reproducibility:** same-seed headless runs are bit-identical at any
/// compute pool size, because headless frames advance the clock by a fixed
/// step and every consumer of this RNG runs in the serial FixedUpdate chain.
/// The parallel systems (`par_iter_mut`) never touch it. GUI runs are paced
/// by the wall clock and are not reproducible; see "Headless runs are
/// deterministic" in docs/DECISIONS.md.
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
    /// Energy moved from living plants to grazers by bites: each bite times
    /// the grazer's plant efficiency. A transfer between organisms, like
    /// `symbiosis`, so it does not change the total and is not part of
    /// `net()`; the undigested rest of each bite is the loss, in `digestion`.
    pub grazing: f64,
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
    /// share of a victim's energy that is never offered to its killer (the
    /// trophic pyramid) and the energy zeroed on a kill, a disease death, or
    /// an old-age death.
    pub death: f64,
    /// Income discarded by the `max_organism_energy` clamp.
    pub clamp: f64,
    /// Energy lost between a meal and its eater: the undigested share of a
    /// bite or of a killer's pyramid share. Food items are not organism
    /// energy, so their undigested share never enters the ledger.
    pub digestion: f64,
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
            - self.digestion
    }

    pub fn add(&mut self, other: &EnergyFlows) {
        self.photosynthesis += other.photosynthesis;
        self.food += other.food;
        self.predation += other.predation;
        self.grazing += other.grazing;
        self.symbiosis += other.symbiosis;
        self.metabolism += other.metabolism;
        self.movement += other.movement;
        self.disease += other.disease;
        self.reproduction_spent += other.reproduction_spent;
        self.reproduction_received += other.reproduction_received;
        self.death += other.death;
        self.clamp += other.clamp;
        self.digestion += other.digestion;
    }

    pub fn minus(&self, other: &EnergyFlows) -> EnergyFlows {
        EnergyFlows {
            photosynthesis: self.photosynthesis - other.photosynthesis,
            food: self.food - other.food,
            predation: self.predation - other.predation,
            grazing: self.grazing - other.grazing,
            symbiosis: self.symbiosis - other.symbiosis,
            metabolism: self.metabolism - other.metabolism,
            movement: self.movement - other.movement,
            disease: self.disease - other.disease,
            reproduction_spent: self.reproduction_spent - other.reproduction_spent,
            reproduction_received: self.reproduction_received - other.reproduction_received,
            death: self.death - other.death,
            clamp: self.clamp - other.clamp,
            digestion: self.digestion - other.digestion,
        }
    }

    pub fn clear(&mut self) {
        *self = EnergyFlows::default();
    }

    /// `(label, value)` pairs in display order, for summaries and CSV.
    pub fn entries(&self) -> [(&'static str, f64); 13] {
        [
            ("photosynthesis", self.photosynthesis),
            ("food", self.food),
            ("predation", self.predation),
            ("grazing", self.grazing),
            ("symbiosis", self.symbiosis),
            ("metabolism", self.metabolism),
            ("movement", self.movement),
            ("disease", self.disease),
            ("reproduction_spent", self.reproduction_spent),
            ("reproduction_received", self.reproduction_received),
            ("death", self.death),
            ("clamp", self.clamp),
            ("digestion", self.digestion),
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

#[cfg(test)]
mod kill_share_tests {
    use super::*;

    #[test]
    fn kill_share_follows_victim_tissue() {
        let config = SimConfig {
            kill_transfer_animal: 0.6,
            kill_transfer_plant: 0.1,
            ..Default::default()
        };
        assert_eq!(config.kill_share(false), 0.6);
        assert_eq!(config.kill_share(true), 0.1);
    }

    #[test]
    fn default_shares_are_equal() {
        let config = SimConfig::default();
        assert_eq!(config.kill_share(false), config.kill_share(true));
    }
}

#[cfg(test)]
mod energy_flow_tests {
    use super::*;

    /// A graze moves `kept` from plant to grazer and destroys `wasted`, so
    /// the flows must net to `-wasted` alone: the transfer is not income.
    #[test]
    fn graze_nets_to_the_digestion_loss_only() {
        let flows = EnergyFlows {
            grazing: 8.0,
            digestion: 2.0,
            ..EnergyFlows::default()
        };
        assert_eq!(flows.net(), -2.0);
    }

    /// A kill: the killer keeps `kept` of a `PREDATION_TRANSFER_FRACTION`
    /// share, the undigested part of the share goes to digestion, and the
    /// rest of the victim's energy to death. The total drops by the victim's
    /// energy less what the killer kept.
    #[test]
    fn kill_nets_to_victim_energy_less_kept() {
        let victim = 50.0;
        let offered = victim * 0.1;
        let kept = offered * 0.25;
        let wasted = offered - kept;
        let flows = EnergyFlows {
            predation: kept,
            digestion: wasted,
            death: victim - wasted,
            ..EnergyFlows::default()
        };
        assert!((flows.net() - (kept - victim)).abs() < 1e-9);
    }

    #[test]
    fn entries_cover_every_flow() {
        let flows = EnergyFlows {
            photosynthesis: 1.0,
            food: 2.0,
            predation: 3.0,
            grazing: 4.0,
            symbiosis: 5.0,
            metabolism: 6.0,
            movement: 7.0,
            disease: 8.0,
            reproduction_spent: 9.0,
            reproduction_received: 10.0,
            death: 11.0,
            clamp: 12.0,
            digestion: 13.0,
        };
        let sum: f64 = flows.entries().iter().map(|(_, v)| v).sum();
        assert_eq!(sum, (1..=13).sum::<i32>() as f64);
    }
}

#[cfg(test)]
mod feeding_count_tests {
    use super::*;

    /// A kill lands in exactly one victim bucket, and in the grazer buckets
    /// only when the killer's diet is below zero.
    #[test]
    fn record_kill_sorts_by_killer_diet_and_victim() {
        let mut c = FeedingCounts::default();
        c.record_kill(-0.8, false, false, 1.0); // grazer kills a consumer
        c.record_kill(-0.1, false, true, 2.0); // plant-leaning killer kills a plant
        c.record_kill(0.0, false, false, 1.0); // neutral diet is not a grazer
        c.record_kill(0.9, false, true, 0.5); // hunter kills a plant
        c.record_kill(-0.5, true, true, 4.0); // a plant kills a plant
        assert_eq!(c.kills_consumer, 2);
        assert_eq!(c.kills_plant, 3);
        assert_eq!(c.kills_plant_by_consumer, 2);
        assert!((c.plant_kill_energy_consumer - 2.5).abs() < 1e-9);
        assert_eq!(c.grazer_kills, 3);
        assert_eq!(c.grazer_kills_consumer, 1);
        assert_eq!(c.grazes(), 0);
    }

    /// Per-second values come from two cumulative readings.
    #[test]
    fn minus_is_per_field() {
        let a = FeedingCounts {
            grazes_eat: 5,
            grazes_attack: 7,
            grazes_eat_by_plant: 2,
            kills_consumer: 3,
            kills_plant: 1,
            kills_plant_by_consumer: 1,
            plant_kill_energy_consumer: 3.5,
            grazer_kills: 2,
            grazer_kills_consumer: 2,
            attacks_no_plant_in_reach: 9,
            ..Default::default()
        };
        let b = FeedingCounts {
            grazes_eat: 1,
            grazes_attack: 2,
            grazes_eat_by_plant: 1,
            kills_consumer: 3,
            kills_plant: 0,
            kills_plant_by_consumer: 0,
            plant_kill_energy_consumer: 1.0,
            grazer_kills: 1,
            grazer_kills_consumer: 1,
            attacks_no_plant_in_reach: 4,
            ..Default::default()
        };
        let d = a.minus(&b);
        assert_eq!(d.grazes_eat, 4);
        assert_eq!(d.grazes_attack, 5);
        assert_eq!(d.kills_consumer, 0);
        assert_eq!(d.kills_plant, 1);
        assert_eq!(d.grazer_kills, 1);
        assert_eq!(d.grazer_kills_consumer, 1);
        assert_eq!(d.attacks_no_plant_in_reach, 5);
        assert_eq!(d.grazes_eat_by_plant, 1);
        assert_eq!(d.kills_plant_by_consumer, 1);
        assert!((d.plant_kill_energy_consumer - 2.5).abs() < 1e-9);
        assert_eq!(a.grazes(), 12);
    }

    /// Each attack with a consumer in reach lands in exactly one outcome.
    #[test]
    fn gate_outcomes_partition_consumer_attacks() {
        let mut g = GateOutcomes::default();
        g.record_consumer_attack(true, true, true, true); // killed a consumer
        g.record_consumer_attack(true, true, true, false); // struck a nearer plant
        g.record_consumer_attack(false, true, false, false); // size gate
        g.record_consumer_attack(true, false, false, false); // damage gate
        g.record_consumer_attack(false, false, false, false); // both
        g.record_consumer_attack(true, true, false, false); // mixed
        assert_eq!(g.consumer_in_reach, 6);
        assert_eq!(
            (
                g.kills_consumer,
                g.kills_plant_instead,
                g.rejected_size,
                g.rejected_damage,
                g.rejected_both,
                g.rejected_mixed
            ),
            (1, 1, 1, 1, 1, 1)
        );
        assert_eq!(g.minus(&GateOutcomes::default()), g);
    }

    #[test]
    fn age_buckets_are_half_open() {
        assert_eq!(age_bucket(0), 0);
        assert_eq!(age_bucket(99), 0);
        assert_eq!(age_bucket(100), 1);
        assert_eq!(age_bucket(999), 4);
        assert_eq!(age_bucket(1000), 5);
        assert_eq!(age_bucket(50_000), 5);
    }
}
