use bevy::prelude::*;
use clauvolution_core::{Food, FoodEnergy, Position, SimConfig};
use rand::Rng;
use std::collections::HashMap;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SpatialHash::default())
            .add_systems(PreUpdate, update_spatial_hash)
            .add_systems(FixedUpdate, food_regeneration_system);
    }
}

/// Spatial hash for efficient neighbor queries.
/// Maps grid cells to lists of entity IDs.
#[derive(Resource, Default)]
pub struct SpatialHash {
    pub cell_size: f32,
    pub cells: HashMap<(i32, i32), Vec<Entity>>,
}

impl SpatialHash {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    pub fn cell_key(&self, pos: Vec2) -> (i32, i32) {
        (
            (pos.x / self.cell_size).floor() as i32,
            (pos.y / self.cell_size).floor() as i32,
        )
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    pub fn insert(&mut self, entity: Entity, pos: Vec2) {
        let key = self.cell_key(pos);
        self.cells.entry(key).or_default().push(entity);
    }

    /// Get all entities in cells within range of a position
    pub fn query_radius(&self, pos: Vec2, radius: f32) -> Vec<Entity> {
        let mut result = Vec::new();
        let cells_range = (radius / self.cell_size).ceil() as i32 + 1;
        let center = self.cell_key(pos);

        for dx in -cells_range..=cells_range {
            for dy in -cells_range..=cells_range {
                let key = (center.0 + dx, center.1 + dy);
                if let Some(entities) = self.cells.get(&key) {
                    result.extend(entities);
                }
            }
        }

        result
    }
}

fn update_spatial_hash(
    mut spatial_hash: ResMut<SpatialHash>,
    query: Query<(Entity, &Position)>,
) {
    spatial_hash.clear();
    if spatial_hash.cell_size < 1.0 {
        spatial_hash.cell_size = 16.0;
    }
    for (entity, pos) in &query {
        spatial_hash.insert(entity, pos.0);
    }
}

/// Spawn food randomly across the world
pub fn spawn_initial_food(commands: &mut Commands, config: &SimConfig, rng: &mut impl Rng) {
    let total_tiles = config.world_width as f32 * config.world_height as f32;
    let food_count = (total_tiles * config.initial_food_density) as u32;

    for _ in 0..food_count {
        let x = rng.gen_range(0.0..config.world_width as f32);
        let y = rng.gen_range(0.0..config.world_height as f32);

        commands.spawn((
            Food,
            FoodEnergy(config.food_energy_value),
            Position(Vec2::new(x, y)),
        ));
    }
}

/// System to regenerate food over time
pub fn food_regeneration_system(
    mut commands: Commands,
    config: Res<SimConfig>,
    food_query: Query<&Food>,
) {
    let current_food = food_query.iter().len() as f32;
    let max_food = config.world_width as f32 * config.world_height as f32 * config.initial_food_density;

    // Regenerate food proportional to how far below max we are
    let deficit_ratio = ((max_food - current_food) / max_food).max(0.0);
    let to_spawn = (deficit_ratio * config.food_regen_rate * max_food).ceil() as u32;

    let mut rng = rand::thread_rng();
    for _ in 0..to_spawn {
        let x = rng.gen_range(0.0..config.world_width as f32);
        let y = rng.gen_range(0.0..config.world_height as f32);

        commands.spawn((
            Food,
            FoodEnergy(config.food_energy_value),
            Position(Vec2::new(x, y)),
        ));
    }
}
