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
            .insert_resource(SelectedOrganism::default());
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
            initial_food_density: 0.08,
            food_regen_rate: 0.005,
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
