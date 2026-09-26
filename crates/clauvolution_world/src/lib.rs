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

    /// Movement cost multiplier for a fully land-adapted organism
    /// (`aquatic_adaptation` 0). The sim interpolates between this table and
    /// `water_move_cost` by the organism's aquatic adaptation on every tile.
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

    /// Movement cost multiplier for a fully water-adapted organism
    /// (`aquatic_adaptation` 1). See `land_move_cost`.
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

/// Version of the terrain generator. Saves regenerate terrain from the seed
/// and record this number, so a save made by a different generator can be
/// recognised on load and the user warned that the map is not the one that
/// was saved. Raise it in any change after which `TileMap::generate`
/// produces a different map from the same seed and world size. Version 1
/// is every generator before the number was recorded; version 2 is the
/// phase 2 step 3 generator (sea level at `LAND_FRACTION`, Rock as an
/// elevation band, seamless noise); version 3 is phase 2 step 4
/// (continental centres and shelves).
pub const TERRAIN_GENERATOR_VERSION: u32 = 3;

/// Share of the map's tiles that are land. `TileMap::generate` puts sea
/// level at the elevation quantile that leaves this share above it, so land
/// area is the same on every seed while where it lies still varies.
pub const LAND_FRACTION: f32 = 0.40;
/// Number of continental centres. `TileMap::generate` raises the elevation
/// noise inside the Voronoi cell of each centre and lowers it along the
/// borders between cells, so the land gathers into up to this many
/// landmasses with ocean between them.
pub const CONTINENTS: usize = 4;
/// Least torus distance between two continental centres, as a share of the
/// world width. Centres are drawn by rejection until they are this far
/// apart, so no two cells are so close that one swallows the other.
pub const CONTINENT_MIN_SPACING: f32 = 0.3;
/// Height of the continent term added to the 0..1 elevation noise. Where it
/// is at full height (well inside a cell) land is likely; along a border it
/// is 0 and only the noise can lift land there.
pub const CONTINENT_WEIGHT: f32 = 0.8;
/// Distance in tiles from a cell border over which the continent term ramps
/// from 0 to its full height (smoothstep). The distance to the border is
/// taken as half the difference between the distances to the nearest two
/// centres.
pub const CONTINENT_RAMP_TILES: f32 = 48.0;
/// Largest displacement in tiles of the point at which the distances to the
/// continental centres are taken (domain warping by two seamless noise
/// maps), so cell borders, and the coasts that follow them, wander instead
/// of running straight.
pub const CONTINENT_WARP_TILES: f32 = 48.0;
/// Water within this many tiles (4-neighbour steps, with the torus wrap) of
/// land is ShallowWater, and all other water is DeepWater, whatever its
/// elevation: a shelf around every coast and open ocean beyond it.
pub const SHELF_WIDTH: u32 = 8;
/// Land at or above this elevation (on the 0..1 side) is Rock, whatever its
/// moisture, so the highest ground forms ranges.
pub const ROCK_ABOVE: f32 = 0.75;
/// Land below Rock is Sand under this moisture, Forest at or above
/// `FOREST_ABOVE_MOISTURE`, Grassland between.
pub const SAND_BELOW_MOISTURE: f32 = 0.25;
pub const FOREST_ABOVE_MOISTURE: f32 = 0.6;

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

/// Land biome for a tile at or above sea level (elevation 0..1).
fn land_terrain(elevation: f32, moisture: f32) -> TerrainType {
    if elevation >= ROCK_ABOVE {
        TerrainType::Rock
    } else if moisture < SAND_BELOW_MOISTURE {
        TerrainType::Sand
    } else if moisture >= FOREST_ABOVE_MOISTURE {
        TerrainType::Forest
    } else {
        TerrainType::Grassland
    }
}

impl Tile {
    fn new(terrain: TerrainType, elevation: f32, moisture: f32) -> Self {
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
/// (`docs/audits/2026-09-25-phase2-baseline/`). On the eight audit seeds the
/// land components are either 38 tiles (one, which held a median of 0
/// organisms and at most 3) or 990 tiles and up (each held a median of 14
/// or more). The cut sits in that gap.
pub const MINOR_REGION_MAX_TILES: u32 = 512;

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
        Self::compute_with_wrap(width, height, is_land, minor_max, true)
    }

    /// `compute`, with the torus wrap switchable so a test can compare
    /// against a flat map and see whether the seam joins components.
    fn compute_with_wrap(
        width: u32,
        height: u32,
        is_land: &[bool],
        minor_max: u32,
        wrap: bool,
    ) -> Self {
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
                    (wrap || x + 1 < w).then(|| y * w + (x + 1) % w),
                    (wrap || x > 0).then(|| y * w + (x + w - 1) % w),
                    (wrap || y + 1 < h).then(|| ((y + 1) % h) * w + x),
                    (wrap || y > 0).then(|| ((y + h - 1) % h) * w + x),
                ];
                for n in neighbours.into_iter().flatten() {
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

    /// Generate a world from two layered value-noise maps and a set of
    /// continental centres.
    ///
    /// Elevation is the 0..1 noise plus a continent term (see `CONTINENTS`),
    /// remapped to -1..1 with sea level at the quantile that leaves
    /// `LAND_FRACTION` of the tiles as land. Water within `SHELF_WIDTH` of
    /// land is shallow and the rest deep. Moisture stays in the noise map's
    /// native 0..1 because the biome thresholds in `land_terrain` and the
    /// vegetation carrying capacity in `tile_dynamics_system` both assume
    /// that range.
    pub fn generate(width: u32, height: u32, rng: &mut impl Rng) -> Self {
        Self::generate_with(width, height, rng, &TerrainParams::DEFAULT)
    }

    fn generate_with(width: u32, height: u32, rng: &mut impl Rng, p: &TerrainParams) -> Self {
        let mut elevation = generate_noise_map(width, height, 5, rng);
        let moisture = generate_noise_map(width, height, 4, rng);
        let centres = place_continent_centres(width, height, p.continents, p.min_spacing, rng);
        let warp_x = generate_noise_map(width, height, 4, rng);
        let warp_y = generate_noise_map(width, height, 4, rng);
        let warp: Vec<Vec2> = warp_x
            .iter()
            .zip(warp_y.iter())
            .map(|(&wx, &wy)| Vec2::new(wx * 2.0 - 1.0, wy * 2.0 - 1.0) * p.warp)
            .collect();
        add_continent_term(
            width,
            height,
            &mut elevation,
            &centres,
            &warp,
            p.weight,
            p.ramp,
        );
        set_sea_level(&mut elevation, LAND_FRACTION);

        let is_land: Vec<bool> = elevation.iter().map(|&e| e >= 0.0).collect();
        let to_land = distance_to_land(width, height, &is_land);
        let tiles: Vec<Tile> = elevation
            .iter()
            .zip(moisture.iter())
            .zip(to_land.iter())
            .map(|((&e, &m), &d)| {
                let terrain = if e >= 0.0 {
                    land_terrain(e, m)
                } else if d <= p.shelf {
                    TerrainType::ShallowWater
                } else {
                    TerrainType::DeepWater
                };
                Tile::new(terrain, e, m)
            })
            .collect();

        let regions = Regions::compute(width, height, &is_land, MINOR_REGION_MAX_TILES);

        TileMap {
            width,
            height,
            tiles,
            regions,
        }
    }
}

/// The generator's continent and shelf settings, so a test can sweep them.
/// `generate` always uses `DEFAULT`, built from the named constants.
#[derive(Clone, Copy, Debug)]
struct TerrainParams {
    continents: usize,
    min_spacing: f32,
    weight: f32,
    ramp: f32,
    warp: f32,
    shelf: u32,
}

impl TerrainParams {
    const DEFAULT: TerrainParams = TerrainParams {
        continents: CONTINENTS,
        min_spacing: CONTINENT_MIN_SPACING,
        weight: CONTINENT_WEIGHT,
        ramp: CONTINENT_RAMP_TILES,
        warp: CONTINENT_WARP_TILES,
        shelf: SHELF_WIDTH,
    };
}

/// Shortest distance between two points on a `width` x `height` torus.
fn torus_distance(a: Vec2, b: Vec2, width: f32, height: f32) -> f32 {
    let dx = (a.x - b.x).rem_euclid(width);
    let dy = (a.y - b.y).rem_euclid(height);
    Vec2::new(dx.min(width - dx), dy.min(height - dy)).length()
}

/// Draw `count` continental centres uniformly on the torus, rejecting a draw
/// closer than `min_spacing` x width to a centre already placed. After 1000
/// rejected draws for one centre the last draw is kept, so the function
/// always returns `count` centres and always uses a bounded number of draws
/// for a given seed.
fn place_continent_centres(
    width: u32,
    height: u32,
    count: usize,
    min_spacing: f32,
    rng: &mut impl Rng,
) -> Vec<Vec2> {
    let (w, h) = (width as f32, height as f32);
    let spacing = min_spacing * w;
    let mut centres: Vec<Vec2> = Vec::with_capacity(count);
    for _ in 0..count {
        let mut candidate = Vec2::ZERO;
        for _ in 0..1000 {
            candidate = Vec2::new(rng.gen_range(0.0..w), rng.gen_range(0.0..h));
            if centres
                .iter()
                .all(|&c| torus_distance(c, candidate, w, h) >= spacing)
            {
                break;
            }
        }
        centres.push(candidate);
    }
    centres
}

/// Add the continent term to a 0..1 elevation map: `weight` times a
/// smoothstep of the tile's distance to the nearest Voronoi border between
/// `centres`, reaching full height `ramp` tiles in from the border. The
/// distance to the border is taken as half the difference between the
/// distances to the nearest and second-nearest centre, on the torus, from
/// the tile's centre displaced by its entry in `warp`. With fewer than two
/// centres there is no border and the map is unchanged.
fn add_continent_term(
    width: u32,
    height: u32,
    elevation: &mut [f32],
    centres: &[Vec2],
    warp: &[Vec2],
    weight: f32,
    ramp: f32,
) {
    if centres.len() < 2 {
        return;
    }
    let (w, h) = (width as f32, height as f32);
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) + warp[i];
            let (mut d1, mut d2) = (f32::MAX, f32::MAX);
            for &c in centres {
                let d = torus_distance(p, c, w, h);
                if d < d1 {
                    d2 = d1;
                    d1 = d;
                } else if d < d2 {
                    d2 = d;
                }
            }
            let t = ((d2 - d1) * 0.5 / ramp).clamp(0.0, 1.0);
            let smooth = t * t * (3.0 - 2.0 * t);
            elevation[i] += weight * smooth;
        }
    }
}

/// Distance from each tile to the nearest land tile, in 4-neighbour steps
/// with the torus wrap (0 on land). A multi-source breadth-first search
/// from every land tile; `u32::MAX` if the map has no land.
fn distance_to_land(width: u32, height: u32, is_land: &[bool]) -> Vec<u32> {
    let (w, h) = (width as usize, height as usize);
    let mut dist = vec![u32::MAX; w * h];
    let mut queue = std::collections::VecDeque::new();
    for (i, &land) in is_land.iter().enumerate() {
        if land {
            dist[i] = 0;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        let (x, y) = (i % w, i / w);
        let next = dist[i] + 1;
        for n in [
            y * w + (x + 1) % w,
            y * w + (x + w - 1) % w,
            ((y + 1) % h) * w + x,
            ((y + h - 1) % h) * w + x,
        ] {
            if dist[n] == u32::MAX {
                dist[n] = next;
                queue.push_back(n);
            }
        }
    }
    dist
}

/// Remap a 0..1 elevation map to -1..1 with sea level at 0, where sea level
/// is the elevation quantile that leaves `land_fraction` of the tiles above
/// it. Below sea level the map is stretched linearly onto -1..0 and above it
/// onto 0..1, so the order of tiles is kept, the deepest tile is -1 and the
/// highest peak is 1 on every seed.
///
/// Ties: a tile exactly at the sea value maps to 0 and becomes land, so if
/// several tiles shared that value the land share would come out above
/// `land_fraction`. On all eight standard audit seeds exactly one tile sits
/// at the sea value, and the land count is exactly the target.
fn set_sea_level(elevation: &mut [f32], land_fraction: f32) {
    let mut sorted = elevation.to_vec();
    sorted.sort_by(f32::total_cmp);
    let n = sorted.len();
    let water_tiles = ((1.0 - land_fraction) * n as f32).round() as usize;
    let sea = sorted[water_tiles.min(n - 1)];
    let (lo, hi) = (sorted[0], sorted[n - 1]);
    let below = (sea - lo).max(1e-6);
    let above = (hi - sea).max(1e-6);
    for e in elevation.iter_mut() {
        *e = if *e < sea {
            (*e - sea) / below
        } else {
            (*e - sea) / above
        };
    }
}

/// Multi-octave value noise for procedural terrain, normalised to 0..1.
///
/// The noise tiles seamlessly: each octave's random grid wraps, so the
/// value at `x = width - 1` runs smoothly into the value at `x = 0`, and the
/// same for `y`. Positions wrap with `rem_euclid`, so the world is a torus,
/// and a grid that did not wrap put a straight terrain seam along both
/// edges where uncorrelated values met; on six of the eight audit seeds
/// that seam joined landmasses by chance (see "Sea level at a fixed land
/// fraction" in `docs/DECISIONS.md`). Octave `k` has `2^k + 1` cells across
/// the map, the same interval count as the non-wrapping grid it replaced.
fn generate_noise_map(width: u32, height: u32, octaves: u32, rng: &mut impl Rng) -> Vec<f32> {
    let size = (width * height) as usize;
    let mut result = vec![0.0f32; size];

    for octave in 0..octaves {
        let freq = (1 << octave) as f32;
        let amplitude = 1.0 / freq;

        // A small random grid that wraps in both directions, interpolated.
        let grid_w = freq as u32 + 1;
        let grid_h = freq as u32 + 1;
        let grid: Vec<f32> = (0..grid_w * grid_h)
            .map(|_| rng.gen_range(-1.0..1.0))
            .collect();

        for y in 0..height {
            for x in 0..width {
                let gx = (x as f32 / width as f32) * grid_w as f32;
                let gy = (y as f32 / height as f32) * grid_h as f32;

                let x0 = (gx.floor() as u32).min(grid_w - 1);
                let y0 = (gy.floor() as u32).min(grid_h - 1);
                let x1 = (x0 + 1) % grid_w;
                let y1 = (y0 + 1) % grid_h;

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
    /// water sits below zero, and sea level must leave `LAND_FRACTION` of
    /// the map as land on every seed. Prints the per-biome counts on the
    /// eight audit seeds at the default world size, so a tuning pass can
    /// compare distributions (run with `--nocapture`), and asserts every
    /// land biome, Rock included, on seed 42, the seed the headless
    /// validation runs use.
    #[test]
    fn generated_map_has_unit_moisture_and_mixed_biomes() {
        const BIOMES: [TerrainType; 6] = [
            TerrainType::DeepWater,
            TerrainType::ShallowWater,
            TerrainType::Sand,
            TerrainType::Grassland,
            TerrainType::Forest,
            TerrainType::Rock,
        ];
        for seed in [1u64, 2, 3, 7, 42, 99, 314, 1000] {
            let mut rng = StdRng::seed_from_u64(seed);
            let map = TileMap::generate(512, 512, &mut rng);

            let mut counts: HashMap<TerrainType, usize> = HashMap::new();
            for tile in &map.tiles {
                *counts.entry(tile.terrain).or_default() += 1;
            }
            let n = |t: TerrainType| counts.get(&t).copied().unwrap_or(0);
            let land: usize = BIOMES[2..].iter().map(|&t| n(t)).sum();
            let mut line = format!(
                "seed {seed}: land {land} ({:.1}% of tiles);",
                100.0 * land as f64 / map.tiles.len() as f64
            );
            for &t in &BIOMES[..2] {
                line += &format!(" {t:?} {}", n(t));
            }
            line += "; of land:";
            for &t in &BIOMES[2..] {
                line += &format!(
                    " {t:?} {} ({:.1}%)",
                    n(t),
                    100.0 * n(t) as f64 / land as f64
                );
            }
            println!("{line}");

            let (mut min_m, mut max_m) = (f32::MAX, f32::MIN);
            let (mut min_e, mut max_e) = (f32::MAX, f32::MIN);
            for tile in &map.tiles {
                min_m = min_m.min(tile.moisture);
                max_m = max_m.max(tile.moisture);
                min_e = min_e.min(tile.elevation);
                max_e = max_e.max(tile.elevation);
            }
            assert!(
                min_m >= 0.0 && max_m <= 1.0,
                "seed {seed}: moisture outside 0..1: {min_m}..{max_m}"
            );
            assert!(
                (min_e + 1.0).abs() < 1e-5 && (max_e - 1.0).abs() < 1e-5,
                "seed {seed}: elevation should span -1..1 around sea level: {min_e}..{max_e}"
            );
            let expected = (LAND_FRACTION * map.tiles.len() as f32).round() as usize;
            assert!(
                land.abs_diff(expected) <= 1,
                "seed {seed}: {land} land tiles, expected {expected}"
            );
            if seed == 42 {
                for &t in &BIOMES[2..] {
                    assert!(n(t) > 0, "seed 42 has no {t:?} tiles");
                }
            }
        }
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

    /// Distance to land counts 4-neighbour steps and wraps the torus.
    #[test]
    fn distance_to_land_wraps_the_torus() {
        // 5 x 3 torus, land at (0, 0) only.
        let mut is_land = vec![false; 15];
        is_land[0] = true;
        let d = distance_to_land(5, 3, &is_land);
        assert_eq!(d[0], 0);
        assert_eq!(d[1], 1);
        assert_eq!(d[4], 1, "across the x edge");
        assert_eq!(d[10], 1, "across the y edge");
        assert_eq!(d[2], 2);
        assert_eq!(d[7], 3, "(2, 1): two steps in x, one in y");
    }

    /// Land sizes below which a major region does not count as a landmass
    /// for `regions_on_the_audit_seeds`' continent assertion.
    const LARGE_REGION_TILES: u32 = 10_000;

    /// How many major land regions share a component of land and
    /// ShallowWater with a larger region, i.e. are joined to it by a shelf
    /// with no deep water in the way.
    fn regions_bridged_by_shelves(map: &TileMap) -> usize {
        let not_deep: Vec<bool> = map
            .tiles
            .iter()
            .map(|t| t.terrain != TerrainType::DeepWater)
            .collect();
        let shelf = Regions::compute(map.width, map.height, &not_deep, 1);
        let mut first_region_on: HashMap<u16, u16> = HashMap::new();
        let mut seen = vec![false; map.regions.count()];
        let mut bridged = 0;
        for (i, &label) in map.regions.labels.iter().enumerate() {
            let region = label as usize;
            if region >= seen.len() || seen[region] {
                continue;
            }
            seen[region] = true;
            if first_region_on.insert(shelf.labels[i], label).is_some() {
                bridged += 1;
            }
        }
        bridged
    }

    /// Region counts and sizes on the eight audit seeds at the default world
    /// size, for the phase 2 audits, with shelf and deep-water tile counts
    /// and whether any shelf joins two major regions. Run with
    /// `--nocapture` to read them. Asserts at least two large landmasses on
    /// the default seed (42), and that no shelf bridges a strait on any of
    /// the eight seeds, which is what `SHELF_WIDTH` was chosen for.
    #[test]
    fn regions_on_the_audit_seeds() {
        for seed in [1u64, 2, 3, 7, 42, 99, 314, 1000] {
            let mut rng = StdRng::seed_from_u64(seed);
            let map = TileMap::generate(512, 512, &mut rng);
            let r = &map.regions;
            let land = map.tiles.iter().filter(|t| !t.terrain.is_water()).count();
            let count = |t: TerrainType| map.tiles.iter().filter(|x| x.terrain == t).count();
            let (deep, shallow) = (
                count(TerrainType::DeepWater),
                count(TerrainType::ShallowWater),
            );
            let bridged = regions_bridged_by_shelves(&map);
            println!(
                "seed {seed}: land {land}, {} major regions {:?}, {} minor components ({} tiles); \
                 shelf {shallow}, deep {deep} ({:.1}% of water); regions bridged by shelves {bridged}",
                r.count(),
                r.sizes,
                r.minor_components,
                r.minor_tiles,
                100.0 * deep as f64 / (deep + shallow) as f64
            );
            let is_land: Vec<bool> = map.tiles.iter().map(|t| !t.terrain.is_water()).collect();
            let all = Regions::compute(512, 512, &is_land, 1);
            println!("  every component: {:?}", all.sizes);
            // The same fill without the torus wrap. The noise tiles
            // seamlessly, so a landmass that crosses a map edge continues
            // on the other side and the flat fill cuts it in two; fewer
            // regions with the wrap is expected and no longer a seam
            // artefact (phase 2 step 3).
            let flat =
                Regions::compute_with_wrap(512, 512, &is_land, MINOR_REGION_MAX_TILES, false);
            println!(
                "  without the wrap: {} major regions {:?}, {} minor components ({} tiles)",
                flat.count(),
                flat.sizes,
                flat.minor_components,
                flat.minor_tiles
            );
            let flat_land: u32 = flat.sizes.iter().sum::<u32>() + flat.minor_tiles;
            assert_eq!(
                flat_land as usize, land,
                "the flat fill labels every land tile"
            );
            assert!(
                flat.count() + flat.minor_components as usize
                    >= r.count() + r.minor_components as usize,
                "the wrap can only join components"
            );
            let labelled: u32 = r.sizes.iter().sum::<u32>() + r.minor_tiles;
            assert_eq!(labelled as usize, land, "every land tile has a region");
            assert!(r.sizes.windows(2).all(|p| p[0] >= p[1]), "ranked by size");
            assert_eq!(bridged, 0, "seed {seed}: a shelf bridges a strait");
            if seed == 42 {
                let large = r.sizes.iter().filter(|&&n| n >= LARGE_REGION_TILES).count();
                assert!(
                    large >= 2,
                    "seed 42 has {large} landmasses of {LARGE_REGION_TILES}+ tiles"
                );
            }
        }
    }

    /// Sweep of the continent and shelf settings over the eight audit seeds,
    /// for a tuning pass. Ignored; run with
    /// `cargo test --release -p clauvolution_world sweep_continent_settings -- --ignored --nocapture`
    /// and comma-separated values in `CONTINENTS`, `WEIGHT`, `RAMP`, `WARP`
    /// and `SHELF` (each defaults to its constant). Prints, per setting, the
    /// seeds with two or more landmasses of `LARGE_REGION_TILES`, the seeds
    /// with a shelf bridge, and per seed the region sizes and deep share of
    /// the water.
    #[test]
    #[ignore]
    fn sweep_continent_settings() {
        let values = |key: &str, default: f32| -> Vec<f32> {
            std::env::var(key)
                .map(|v| v.split(',').map(|x| x.trim().parse().unwrap()).collect())
                .unwrap_or_else(|_| vec![default])
        };
        for &continents in &values("CONTINENTS", CONTINENTS as f32) {
            for &weight in &values("WEIGHT", CONTINENT_WEIGHT) {
                for &ramp in &values("RAMP", CONTINENT_RAMP_TILES) {
                    for &warp in &values("WARP", CONTINENT_WARP_TILES) {
                        for &shelf in &values("SHELF", SHELF_WIDTH as f32) {
                            let p = TerrainParams {
                                continents: continents as usize,
                                min_spacing: CONTINENT_MIN_SPACING,
                                weight,
                                ramp,
                                warp,
                                shelf: shelf as u32,
                            };
                            let (mut multi, mut bridged_seeds) = (0, 0);
                            let mut lines = String::new();
                            for seed in [1u64, 2, 3, 7, 42, 99, 314, 1000] {
                                let mut rng = StdRng::seed_from_u64(seed);
                                let map = TileMap::generate_with(512, 512, &mut rng, &p);
                                let large = map
                                    .regions
                                    .sizes
                                    .iter()
                                    .filter(|&&n| n >= LARGE_REGION_TILES)
                                    .count();
                                multi += usize::from(large >= 2);
                                let bridged = regions_bridged_by_shelves(&map);
                                bridged_seeds += usize::from(bridged > 0);
                                let water =
                                    map.tiles.iter().filter(|t| t.terrain.is_water()).count();
                                let deep = map
                                    .tiles
                                    .iter()
                                    .filter(|t| t.terrain == TerrainType::DeepWater)
                                    .count();
                                lines += &format!(
                                    "  seed {seed}: {:?} + {} minor tiles, bridged {bridged}, deep {:.0}% of water\n",
                                    map.regions.sizes,
                                    map.regions.minor_tiles,
                                    100.0 * deep as f64 / water as f64
                                );
                            }
                            println!(
                                "{p:?}: {multi}/8 seeds with 2+ large landmasses, \
                                 {bridged_seeds}/8 with a shelf bridge\n{lines}"
                            );
                        }
                    }
                }
            }
        }
    }
}
