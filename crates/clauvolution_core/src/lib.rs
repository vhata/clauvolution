use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimConfig::default())
            .insert_resource(SimStats::default())
            .insert_resource(TickCounter(0));
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
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            world_width: 256,
            world_height: 256,
            initial_population: 200,
            initial_food_density: 0.05,
            food_regen_rate: 0.002,
            mutation_rate: 0.3,
            mutation_strength: 0.5,
            base_metabolism_cost: 0.1,
            movement_energy_cost: 0.05,
            reproduction_energy_threshold: 80.0,
            reproduction_energy_cost: 50.0,
            max_organism_energy: 100.0,
            food_energy_value: 30.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct SimStats {
    pub total_organisms: u32,
    pub total_food: u32,
    pub total_births: u64,
    pub total_deaths: u64,
    pub generation: u64,
}

#[derive(Resource)]
pub struct TickCounter(pub u64);

#[derive(Component)]
pub struct Organism;

#[derive(Component)]
pub struct Energy(pub f32);

#[derive(Component)]
pub struct Position(pub Vec2);

#[derive(Component)]
pub struct Velocity(pub Vec2);

#[derive(Component)]
pub struct BodySize(pub f32);

#[derive(Component)]
pub struct Age(pub u64);

#[derive(Component)]
pub struct SpeciesId(pub u64);

#[derive(Component)]
pub struct Food;

#[derive(Component)]
pub struct FoodEnergy(pub f32);
