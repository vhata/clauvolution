use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimConfig::default())
            .insert_resource(SimStats::default())
            .insert_resource(TickCounter(0))
            .insert_resource(SimSpeed::default())
            .insert_resource(SpeciesColors::default())
            .insert_resource(SelectedOrganism::default())
            .insert_resource(Season::default())
            .insert_resource(FitnessTracker::default())
            .insert_resource(PopulationHistory::default());
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
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            world_width: 256,
            world_height: 256,
            initial_population: 200,
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
            species_compat_threshold: 1.5,
        }
    }
}

#[derive(Resource, Default)]
pub struct SimStats {
    pub total_organisms: u32,
    pub total_food: u32,
    pub total_births: u64,
    pub total_deaths: u64,
    pub max_generation: u32,
    pub species_count: u32,
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
}

impl Default for PopulationHistory {
    fn default() -> Self {
        Self {
            snapshots: Vec::new(),
            max_entries: 300, // 5 minutes at 1 snapshot/sec
            visible: true,
            prev_births: 0,
            prev_deaths: 0,
        }
    }
}

impl PopulationHistory {
    pub fn record(&mut self, stats: &SimStats, organism_count: u32, food_count: u32, plants: u32, predators: u32, foragers: u32, avg_lifespan: f32) {
        let births_per_sec = (stats.total_births - self.prev_births) as u32;
        let deaths_per_sec = (stats.total_deaths - self.prev_deaths) as u32;
        self.prev_births = stats.total_births;
        self.prev_deaths = stats.total_deaths;

        self.snapshots.push(PopSnapshot {
            organisms: organism_count,
            food: food_count,
            species: stats.species_count,
            births_per_sec,
            deaths_per_sec,
            max_generation: stats.max_generation,
            plants,
            predators,
            foragers,
            avg_lifespan,
        });

        if self.snapshots.len() > self.max_entries {
            self.snapshots.remove(0);
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

#[derive(Component)]
pub struct Food;

#[derive(Component)]
pub struct FoodEnergy(pub f32);
