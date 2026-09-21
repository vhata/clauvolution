use bevy::prelude::*;
use clauvolution_core::{Food, FoodEnergy, Position, Season, SimConfig, SimRng};
use rand::Rng;
use std::collections::HashMap;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        // No systems are registered here. `update_spatial_hash`,
        // `tile_dynamics_system` and `food_regeneration_system` are scheduled
        // by `SimPlugin` inside the FixedUpdate chain so that every tick's
        // neighbour queries see that tick's positions and so that food
        // regeneration consumes `SimRng` at a fixed point in the tick. Left
        // unordered, Bevy's executor placed them wherever their data access
        // allowed, and that placement is not the same from run to run.
        app.insert_resource(SpatialHash::default());
    }
}

// --- Terrain ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainType {
    DeepWater,
    ShallowWater,
    Sand,
    Grassland,
    Forest,
    Rock,
}

impl TerrainType {
    pub fn base_color(&self) -> Color {
        match self {
            TerrainType::DeepWater => Color::srgb(0.1, 0.15, 0.5),
            TerrainType::ShallowWater => Color::srgb(0.2, 0.35, 0.6),
            TerrainType::Sand => Color::srgb(0.85, 0.78, 0.55),
            TerrainType::Grassland => Color::srgb(0.3, 0.55, 0.2),
            TerrainType::Forest => Color::srgb(0.15, 0.4, 0.1),
            TerrainType::Rock => Color::srgb(0.5, 0.48, 0.45),
        }
    }

    pub fn is_water(&self) -> bool {
        matches!(self, TerrainType::DeepWater | TerrainType::ShallowWater)
    }

    /// Movement cost multiplier for land-adapted organisms
    /// Deep water is nearly impassable — creates real geographic barriers
    pub fn land_move_cost(&self) -> f32 {
        match self {
            TerrainType::DeepWater => 10.0,
            TerrainType::ShallowWater => 3.0,
            TerrainType::Sand => 1.5,
            TerrainType::Grassland => 1.0,
            TerrainType::Forest => 1.3,
            TerrainType::Rock => 3.0,
        }
    }

    /// Movement cost multiplier for water-adapted organisms
    /// Land is hard for aquatic organisms — they stay in water
    pub fn water_move_cost(&self) -> f32 {
        match self {
            TerrainType::DeepWater => 1.0,
            TerrainType::ShallowWater => 1.0,
            TerrainType::Sand => 5.0,
            TerrainType::Grassland => 4.0,
            TerrainType::Forest => 5.0,
            TerrainType::Rock => 8.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub terrain: TerrainType,
    pub elevation: f32,
    pub temperature: f32,
    pub moisture: f32,
    pub light_level: f32,
    pub nutrients: f32,
    pub vegetation_density: f32,
}

impl Tile {
    fn from_elevation_moisture(elevation: f32, moisture: f32) -> Self {
        let terrain = if elevation < -0.3 {
            TerrainType::DeepWater
        } else if elevation < -0.05 {
            TerrainType::ShallowWater
        } else if moisture < 0.25 {
            if elevation > 0.6 {
                TerrainType::Rock
            } else {
                TerrainType::Sand
            }
        } else if moisture > 0.6 {
            TerrainType::Forest
        } else {
            TerrainType::Grassland
        };

        let temperature = (1.0 - elevation.max(0.0) * 0.5).clamp(0.2, 1.0);
        let light_level = if terrain.is_water() { 0.6 } else { 1.0 };
        let nutrients = match terrain {
            TerrainType::Forest => 0.8,
            TerrainType::Grassland => 0.6,
            TerrainType::ShallowWater => 0.5,
            TerrainType::Sand => 0.15,
            TerrainType::DeepWater => 0.3,
            TerrainType::Rock => 0.1,
        };

        Tile {
            terrain,
            elevation,
            temperature,
            moisture,
            light_level,
            nutrients,
            vegetation_density: if terrain.is_water() {
                0.0
            } else {
                nutrients * 0.5
            },
        }
    }
}

/// The tile grid resource
#[derive(Resource)]
pub struct TileMap {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<Tile>,
}

impl TileMap {
    pub fn get(&self, x: u32, y: u32) -> &Tile {
        &self.tiles[(y * self.width + x) as usize]
    }

    pub fn get_mut(&mut self, x: u32, y: u32) -> &mut Tile {
        &mut self.tiles[(y * self.width + x) as usize]
    }

    pub fn tile_at_pos(&self, pos: Vec2) -> &Tile {
        let x = (pos.x as u32).min(self.width - 1);
        let y = (pos.y as u32).min(self.height - 1);
        self.get(x, y)
    }

    /// Generate a world from two layered value-noise maps.
    ///
    /// Elevation is remapped to -1..1 so that water sits below zero; moisture
    /// stays in the noise map's native 0..1 because the biome thresholds in
    /// `Tile::from_elevation_moisture` and the vegetation carrying capacity in
    /// `tile_dynamics_system` both assume that range.
    pub fn generate(width: u32, height: u32, rng: &mut impl Rng) -> Self {
        let mut elevation = generate_noise_map(width, height, 5, rng);
        for e in &mut elevation {
            *e = *e * 2.0 - 1.0;
        }
        let moisture = generate_noise_map(width, height, 4, rng);

        let tiles: Vec<Tile> = elevation
            .iter()
            .zip(moisture.iter())
            .map(|(&e, &m)| Tile::from_elevation_moisture(e, m))
            .collect();

        TileMap {
            width,
            height,
            tiles,
        }
    }
}

/// Simple multi-octave value noise for procedural terrain, normalised to 0..1.
fn generate_noise_map(width: u32, height: u32, octaves: u32, rng: &mut impl Rng) -> Vec<f32> {
    let size = (width * height) as usize;
    let mut result = vec![0.0f32; size];

    for octave in 0..octaves {
        let freq = (1 << octave) as f32;
        let amplitude = 1.0 / freq;

        // Generate a small random grid and interpolate
        let grid_w = (freq as u32 + 2).max(2);
        let grid_h = (freq as u32 + 2).max(2);
        let grid: Vec<f32> = (0..grid_w * grid_h)
            .map(|_| rng.gen_range(-1.0..1.0))
            .collect();

        for y in 0..height {
            for x in 0..width {
                let gx = (x as f32 / width as f32) * (grid_w - 1) as f32;
                let gy = (y as f32 / height as f32) * (grid_h - 1) as f32;

                let x0 = gx.floor() as u32;
                let y0 = gy.floor() as u32;
                let x1 = (x0 + 1).min(grid_w - 1);
                let y1 = (y0 + 1).min(grid_h - 1);

                let fx = gx - gx.floor();
                let fy = gy - gy.floor();

                // Smoothstep
                let sx = fx * fx * (3.0 - 2.0 * fx);
                let sy = fy * fy * (3.0 - 2.0 * fy);

                let v00 = grid[(y0 * grid_w + x0) as usize];
                let v10 = grid[(y0 * grid_w + x1) as usize];
                let v01 = grid[(y1 * grid_w + x0) as usize];
                let v11 = grid[(y1 * grid_w + x1) as usize];

                let top = v00 + sx * (v10 - v00);
                let bottom = v01 + sx * (v11 - v01);
                let value = top + sy * (bottom - top);

                result[(y * width + x) as usize] += value * amplitude;
            }
        }
    }

    // Normalize to 0..1
    let min = result.iter().cloned().fold(f32::MAX, f32::min);
    let max = result.iter().cloned().fold(f32::MIN, f32::max);
    let range = (max - min).max(0.001);
    for v in &mut result {
        *v = (*v - min) / range;
    }

    result
}

// --- Tile dynamics: vegetation growth, nutrient cycling ---

pub fn tile_dynamics_system(mut tile_map: ResMut<TileMap>) {
    for tile in &mut tile_map.tiles {
        if !tile.terrain.is_water() {
            // Vegetation grows toward nutrient-determined carrying capacity
            let capacity = tile.nutrients * tile.moisture;
            let growth_rate = 0.001;
            tile.vegetation_density += (capacity - tile.vegetation_density) * growth_rate;
            tile.vegetation_density = tile.vegetation_density.clamp(0.0, 1.0);
        }
    }
}

// --- Spatial Hash ---

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

/// Rebuild the spatial hash from every `Position` in the world.
///
/// Runs once per FixedUpdate tick, ordered by `SimPlugin` directly after
/// `tick_counter_system` and before any system that queries neighbours.
/// Positions change once per tick, so rebuilding per frame (the previous
/// `PreUpdate` placement) left every tick after the first in a frame reading
/// stale cells whenever a frame ran several ticks.
pub fn update_spatial_hash(
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

// --- Food spawning (now biome-aware) ---

pub fn spawn_initial_food(
    commands: &mut Commands,
    config: &SimConfig,
    tile_map: &TileMap,
    rng: &mut impl Rng,
) {
    let total_tiles = config.world_width as f32 * config.world_height as f32;
    let food_count = (total_tiles * config.initial_food_density) as u32;

    for _ in 0..food_count {
        let x = rng.gen_range(0.0..config.world_width as f32);
        let y = rng.gen_range(0.0..config.world_height as f32);

        let tile = tile_map.tile_at_pos(Vec2::new(x, y));

        // Food spawns proportional to nutrients — less food in deserts and deep water
        if rng.gen::<f32>() < tile.nutrients {
            commands.spawn((
                Food,
                FoodEnergy(config.food_energy_value),
                Position(Vec2::new(x, y)),
            ));
        }
    }
}

pub fn food_regeneration_system(
    mut commands: Commands,
    config: Res<SimConfig>,
    food_query: Query<&Food>,
    tile_map: Res<TileMap>,
    season: Res<Season>,
    mut sim_rng: ResMut<SimRng>,
) {
    let current_food = food_query.iter().len() as f32;
    let max_food = config.world_width as f32 * config.world_height as f32 * config.max_food_density;

    let deficit_ratio = ((max_food - current_food) / max_food).max(0.0);
    let seasonal_regen = config.food_regen_rate * season.food_regen_multiplier();
    let to_spawn = (deficit_ratio * seasonal_regen * max_food).ceil() as u32;

    let rng = &mut sim_rng.0;
    for _ in 0..to_spawn {
        let x = rng.gen_range(0.0..config.world_width as f32);
        let y = rng.gen_range(0.0..config.world_height as f32);

        let tile = tile_map.tile_at_pos(Vec2::new(x, y));

        // Food spawns proportional to vegetation density + nutrients
        if rng.gen::<f32>() < (tile.vegetation_density + tile.nutrients) * 0.5 {
            commands.spawn((
                Food,
                FoodEnergy(config.food_energy_value),
                Position(Vec2::new(x, y)),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    /// Moisture must span 0..1 (the biome thresholds and the vegetation
    /// carrying capacity assume it) while elevation stays signed so that
    /// water sits below zero. Uses the default world size and the seed the
    /// headless validation runs use, and prints the biome counts so a tuning
    /// pass can compare distributions.
    #[test]
    fn generated_map_has_unit_moisture_and_mixed_biomes() {
        let mut rng = StdRng::seed_from_u64(42);
        let map = TileMap::generate(512, 512, &mut rng);

        let mut counts: HashMap<TerrainType, usize> = HashMap::new();
        for tile in &map.tiles {
            *counts.entry(tile.terrain).or_default() += 1;
        }
        let mut sorted: Vec<_> = counts.iter().collect();
        sorted.sort_by_key(|(t, _)| format!("{t:?}"));
        for (terrain, n) in sorted {
            println!("{terrain:?}: {n}");
        }

        let (mut min_m, mut max_m) = (f32::MAX, f32::MIN);
        let (mut min_e, mut max_e) = (f32::MAX, f32::MIN);
        for tile in &map.tiles {
            min_m = min_m.min(tile.moisture);
            max_m = max_m.max(tile.moisture);
            min_e = min_e.min(tile.elevation);
            max_e = max_e.max(tile.elevation);
        }
        println!("moisture {min_m}..{max_m}, elevation {min_e}..{max_e}");

        assert!(
            min_m >= 0.0 && max_m <= 1.0,
            "moisture outside 0..1: {min_m}..{max_m}"
        );
        assert!(
            min_e < 0.0 && max_e > 0.0,
            "elevation should straddle zero: {min_e}..{max_e}"
        );
        assert!(
            counts.len() > 1,
            "expected more than one biome type, got {counts:?}"
        );
        assert!(
            counts.contains_key(&TerrainType::Forest),
            "no Forest tiles generated"
        );
    }
}
