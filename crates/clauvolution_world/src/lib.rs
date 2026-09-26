use bevy::prelude::*;
use clauvolution_core::{
    Food, FoodEnergy, GeographyStats, Organism, Position, Season, SimConfig, SimRng,
};
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

/// One cell of the world map.
///
/// Saves store only the fields that change after `TileMap::generate`; the
/// rest are regenerated from the terrain seed on load. A new field has to be
/// classified in `SaveTerrain::from_tile_map` (`clauvolution_sim::save`),
/// whose exhaustive destructuring of `Tile` fails to compile until it is. If
/// anything writes the field at runtime, add it to `SaveTerrain` and
/// `SaveTerrain::apply_to` as well, or it is lost on save and load.
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
    /// Land regions: the connected components of land tiles. Computed once
    /// in `generate` from the terrain alone, never saved (terrain type does
    /// not change at runtime, so a load regenerates the same labels).
    pub regions: Regions,
}

/// Components of land smaller than this many tiles pool into one "minor"
/// region. Set from the phase 2 step 1 baseline
/// (`docs/audits/2026-09-25-phase2-baseline/`): below it are islets that
/// hold a handful of organisms at most; above it the landmasses that hold
/// populations of their own.
pub const MINOR_REGION_MAX_TILES: u32 = 2048;

/// Label of a water tile in `Regions::labels`.
pub const REGION_WATER: u16 = u16::MAX;
/// Label shared by every land component under `MINOR_REGION_MAX_TILES`.
pub const REGION_MINOR: u16 = u16::MAX - 1;

/// Connected land components of a `TileMap`, found by flood fill over
/// non-water tiles with 4-neighbour adjacency and the torus wrap that
/// positions use (`rem_euclid` in `action_system`).
///
/// Major regions (at least `MINOR_REGION_MAX_TILES` tiles) are numbered
/// from 0 in descending order of size, ties broken by the lowest tile index,
/// so the numbering is a function of the terrain alone and region 0 is
/// always the largest landmass. Computed without any RNG.
#[derive(Clone, Debug, Default)]
pub struct Regions {
    /// One label per tile, in `TileMap::tiles` order: a major region index,
    /// `REGION_MINOR`, or `REGION_WATER`.
    pub labels: Vec<u16>,
    /// Tile count of each major region, indexed by label.
    pub sizes: Vec<u32>,
    /// How many components pooled into `REGION_MINOR`, and their tiles.
    pub minor_components: u32,
    pub minor_tiles: u32,
}

impl Regions {
    /// Label the land components of a `width` x `height` torus whose tiles
    /// are land where `is_land` is true.
    pub fn compute(width: u32, height: u32, is_land: &[bool], minor_max: u32) -> Self {
        let (w, h) = (width as usize, height as usize);
        debug_assert_eq!(is_land.len(), w * h);
        // First pass: raw component ids in scan order.
        let mut raw = vec![u32::MAX; w * h];
        let mut raw_sizes: Vec<u32> = Vec::new();
        let mut stack: Vec<usize> = Vec::new();
        for start in 0..w * h {
            if !is_land[start] || raw[start] != u32::MAX {
                continue;
            }
            let id = raw_sizes.len() as u32;
            let mut size = 0u32;
            raw[start] = id;
            stack.push(start);
            while let Some(i) = stack.pop() {
                size += 1;
                let (x, y) = (i % w, i / w);
                let neighbours = [
                    y * w + (x + 1) % w,
                    y * w + (x + w - 1) % w,
                    ((y + 1) % h) * w + x,
                    ((y + h - 1) % h) * w + x,
                ];
                for n in neighbours {
                    if is_land[n] && raw[n] == u32::MAX {
                        raw[n] = id;
                        stack.push(n);
                    }
                }
            }
            raw_sizes.push(size);
        }

        // Rank the major components by size; raw ids are already in order of
        // their lowest tile index, so a stable sort breaks ties by it.
        let mut major: Vec<u32> = (0..raw_sizes.len() as u32)
            .filter(|&id| raw_sizes[id as usize] >= minor_max)
            .collect();
        major.sort_by_key(|&id| std::cmp::Reverse(raw_sizes[id as usize]));
        assert!(
            major.len() < REGION_MINOR as usize,
            "too many major regions for a u16 label"
        );
        let mut relabel = vec![REGION_MINOR; raw_sizes.len()];
        for (rank, &id) in major.iter().enumerate() {
            relabel[id as usize] = rank as u16;
        }
        let labels = raw
            .iter()
            .map(|&id| {
                if id == u32::MAX {
                    REGION_WATER
                } else {
                    relabel[id as usize]
                }
            })
            .collect();
        let sizes = major.iter().map(|&id| raw_sizes[id as usize]).collect();
        let minor: Vec<u32> = raw_sizes
            .iter()
            .copied()
            .filter(|&n| n < minor_max)
            .collect();
        Regions {
            labels,
            sizes,
            minor_components: minor.len() as u32,
            minor_tiles: minor.iter().sum(),
        }
    }

    /// Number of major regions.
    pub fn count(&self) -> usize {
        self.sizes.len()
    }
}

impl TileMap {
    pub fn get(&self, x: u32, y: u32) -> &Tile {
        &self.tiles[(y * self.width + x) as usize]
    }

    pub fn get_mut(&mut self, x: u32, y: u32) -> &mut Tile {
        &mut self.tiles[(y * self.width + x) as usize]
    }

    pub fn tile_at_pos(&self, pos: Vec2) -> &Tile {
        &self.tiles[self.index_at_pos(pos)]
    }

    /// Index into `tiles` (and `regions.labels`) of the tile under `pos`,
    /// clamped the same way `tile_at_pos` is.
    pub fn index_at_pos(&self, pos: Vec2) -> usize {
        let x = (pos.x as u32).min(self.width - 1);
        let y = (pos.y as u32).min(self.height - 1);
        (y * self.width + x) as usize
    }

    /// Region label of the tile under `pos`: a major region index,
    /// `REGION_MINOR` or `REGION_WATER`.
    pub fn region_at_pos(&self, pos: Vec2) -> u16 {
        self.regions.labels[self.index_at_pos(pos)]
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

        let is_land: Vec<bool> = tiles.iter().map(|t| !t.terrain.is_water()).collect();
        let regions = Regions::compute(width, height, &is_land, MINOR_REGION_MAX_TILES);

        TileMap {
            width,
            height,
            tiles,
            regions,
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

    /// How many cells either side of the centre cell a query of `radius`
    /// visits. `query_radius` and sensing's copy of the hash both use it, so
    /// they visit the same cells in the same order.
    pub fn cell_range(&self, radius: f32) -> i32 {
        (radius / self.cell_size).ceil() as i32 + 1
    }

    pub fn query_radius(&self, pos: Vec2, radius: f32) -> Vec<Entity> {
        let mut result = Vec::new();
        let cells_range = self.cell_range(radius);
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

/// Rebuild the spatial hash from every organism `Position` in the world.
///
/// Runs once per FixedUpdate tick, ordered by `SimPlugin` directly after
/// `tick_counter_system` and before any system that queries neighbours.
/// Positions change once per tick, so rebuilding per frame (the previous
/// `PreUpdate` placement) left every tick after the first in a frame reading
/// stale cells whenever a frame ran several ticks.
///
/// Food is not indexed. Every reader (sensing, predation, disease
/// transmission, symbiosis tracking, mate search) resolves the returned
/// entities through an organism-only query and would discard food anyway,
/// and food is sensed through `FoodSnapshot` instead. Indexing it only made
/// every neighbour query walk and reject the food in the surrounding cells.
pub fn update_spatial_hash(
    mut spatial_hash: ResMut<SpatialHash>,
    query: Query<(Entity, &Position), With<Organism>>,
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
    mut geo: ResMut<GeographyStats>,
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
            // Counted for the phase 2 instruments: deep, shallow, land.
            let landing = match tile.terrain {
                TerrainType::DeepWater => 0,
                TerrainType::ShallowWater => 1,
                _ => 2,
            };
            geo.food_spawned[landing] += 1;
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

    /// The hash indexes organisms only. Food has its own per-tick snapshot,
    /// and every neighbour reader resolves hits through an organism query, so
    /// a food entity in the hash is pure cost for each query that walks it.
    #[test]
    fn spatial_hash_indexes_organisms_and_skips_food() {
        let mut world = World::new();
        world.insert_resource(SpatialHash::new(16.0));
        let origin = Vec2::new(100.0, 100.0);
        let organism = world.spawn((Organism, Position(origin))).id();
        let food = world
            .spawn((
                Food,
                FoodEnergy(10.0),
                Position(origin + Vec2::new(1.0, 1.0)),
            ))
            .id();
        let far_organism = world
            .spawn((Organism, Position(origin + Vec2::new(500.0, 0.0))))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(update_spatial_hash);
        schedule.run(&mut world);

        let hash = world.resource::<SpatialHash>();
        let nearby = hash.query_radius(origin, 8.0);
        assert!(
            nearby.contains(&organism),
            "organism at the origin is missing"
        );
        assert!(!nearby.contains(&food), "food must not be indexed");
        assert!(
            !nearby.contains(&far_organism),
            "distant organism leaked in"
        );

        let indexed: usize = hash.cells.values().map(Vec::len).sum();
        assert_eq!(indexed, 2, "only the two organisms should be indexed");
    }

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

    /// Flood fill joins components across the torus edges, ranks major
    /// regions by size and pools small ones.
    #[test]
    fn regions_wrap_the_torus_and_pool_minor_components() {
        // 6 x 4 torus. `#` is land.
        //   #....#   <- joined across the x edge: 4 tiles with row 3
        //   ......
        //   ..##..   <- 2 tiles, minor at minor_max 3
        //   #....#
        let rows = ["#....#", "......", "..##..", "#....#"];
        let is_land: Vec<bool> = rows
            .iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect();
        let regions = Regions::compute(6, 4, &is_land, 3);
        assert_eq!(regions.sizes, vec![4]);
        assert_eq!(regions.minor_components, 1);
        assert_eq!(regions.minor_tiles, 2);
        assert_eq!(regions.labels[0], 0);
        assert_eq!(regions.labels[5], 0);
        assert_eq!(regions.labels[18], 0, "joined across the y edge");
        assert_eq!(regions.labels[14], REGION_MINOR);
        assert_eq!(regions.labels[1], REGION_WATER);
    }

    /// Region counts and sizes on the eight audit seeds at the default world
    /// size, for the phase 2 audits. Run with `--nocapture` to read them.
    #[test]
    fn regions_on_the_audit_seeds() {
        for seed in [1u64, 2, 3, 7, 42, 99, 314, 1000] {
            let mut rng = StdRng::seed_from_u64(seed);
            let map = TileMap::generate(512, 512, &mut rng);
            let r = &map.regions;
            let land = map.tiles.iter().filter(|t| !t.terrain.is_water()).count();
            println!(
                "seed {seed}: land {land}, {} major regions {:?}, {} minor components ({} tiles)",
                r.count(),
                r.sizes,
                r.minor_components,
                r.minor_tiles
            );
            let is_land: Vec<bool> = map.tiles.iter().map(|t| !t.terrain.is_water()).collect();
            let all = Regions::compute(512, 512, &is_land, 1);
            println!("  every component: {:?}", all.sizes);
            let labelled: u32 = r.sizes.iter().sum::<u32>() + r.minor_tiles;
            assert_eq!(labelled as usize, land, "every land tile has a region");
            assert!(r.sizes.windows(2).all(|p| p[0] >= p[1]), "ranked by size");
        }
    }
}
