use bevy::prelude::*;
use clauvolution_brain::Brain;
use clauvolution_core::*;
use clauvolution_genome::{Genome, InnovationCounter, NUM_INPUTS};
use clauvolution_world::{SpatialHash, TileMap};
use rand::Rng;

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                sensing_and_brain_system,
                action_system,
                photosynthesis_system,
                metabolism_system,
                death_system,
                reproduction_system,
            )
                .chain(),
        )
        .insert_resource(Time::<Fixed>::from_hz(30.0));
    }
}

#[derive(Component)]
pub struct BrainOutput {
    pub move_x: f32,
    pub move_y: f32,
    pub eat: f32,
    pub reproduce: f32,
}

impl Default for BrainOutput {
    fn default() -> Self {
        Self {
            move_x: 0.0,
            move_y: 0.0,
            eat: 0.0,
            reproduce: 0.0,
        }
    }
}

fn sensing_and_brain_system(
    config: Res<SimConfig>,
    spatial_hash: Res<SpatialHash>,
    tile_map: Res<TileMap>,
    mut organisms: Query<
        (Entity, &Position, &Energy, &Genome, &Brain, &BodySize, &mut BrainOutput),
        With<Organism>,
    >,
    food_query: Query<(Entity, &Position), (With<Food>, Without<Organism>)>,
    all_positions: Query<(&Position, &BodySize), (With<Organism>, Without<Food>)>,
) {
    let food_positions: Vec<(Entity, Vec2)> = food_query.iter().map(|(e, p)| (e, p.0)).collect();

    for (entity, pos, energy, genome, brain, body_size, mut output) in &mut organisms {
        let mut inputs = [0.0f32; NUM_INPUTS];

        // Input 0: energy level
        inputs[0] = energy.0 / config.max_organism_energy;

        // Find nearest food
        let sense_range = genome.effective_sense_range();
        let mut nearest_food_dist = f32::MAX;
        let mut nearest_food_dir = Vec2::ZERO;

        for &(_food_entity, food_pos) in &food_positions {
            let diff = food_pos - pos.0;
            let dist = diff.length();
            if dist < nearest_food_dist && dist < sense_range {
                nearest_food_dist = dist;
                nearest_food_dir = if dist > 0.001 { diff / dist } else { Vec2::ZERO };
            }
        }

        if nearest_food_dist < f32::MAX {
            inputs[1] = nearest_food_dir.x;
            inputs[2] = nearest_food_dir.y;
            inputs[3] = 1.0 - (nearest_food_dist / sense_range).min(1.0);
        }

        // Find nearest organism
        let nearby_entities = spatial_hash.query_radius(pos.0, sense_range);
        let mut nearest_org_dist = f32::MAX;
        let mut nearest_org_dir = Vec2::ZERO;
        let mut nearest_org_size_ratio = 1.0f32;

        for &nearby_entity in &nearby_entities {
            if nearby_entity == entity {
                continue;
            }
            if let Ok((other_pos, other_size)) = all_positions.get(nearby_entity) {
                let diff = other_pos.0 - pos.0;
                let dist = diff.length();
                if dist < nearest_org_dist && dist < sense_range {
                    nearest_org_dist = dist;
                    nearest_org_dir = if dist > 0.001 { diff / dist } else { Vec2::ZERO };
                    nearest_org_size_ratio = other_size.0 / body_size.0;
                }
            }
        }

        if nearest_org_dist < f32::MAX {
            inputs[4] = nearest_org_dir.x;
            inputs[5] = nearest_org_dir.y;
            inputs[6] = 1.0 - (nearest_org_dist / sense_range).min(1.0);
            inputs[7] = nearest_org_size_ratio.min(2.0) / 2.0;
        }

        // Terrain inputs
        let tile = tile_map.tile_at_pos(pos.0);
        inputs[8] = if tile.terrain.is_water() { 1.0 } else { 0.0 };
        inputs[9] = tile.nutrients;
        inputs[10] = tile.light_level;
        inputs[11] = genome.aquatic_adaptation;

        // Bias
        inputs[12] = 1.0;

        let brain_out = brain.evaluate(&inputs);
        output.move_x = brain_out[0];
        output.move_y = brain_out[1];
        output.eat = brain_out[2];
        output.reproduce = brain_out[3];
    }
}

fn action_system(
    config: Res<SimConfig>,
    tile_map: Res<TileMap>,
    mut organisms: Query<
        (&mut Position, &mut Energy, &BrainOutput, &Genome, &BodySize),
        (With<Organism>, Without<Food>),
    >,
    food_query: Query<(Entity, &Position, &FoodEnergy), (With<Food>, Without<Organism>)>,
    mut commands: Commands,
) {
    let foods: Vec<(Entity, Vec2, f32)> = food_query
        .iter()
        .map(|(e, p, fe)| (e, p.0, fe.0))
        .collect();

    let mut eaten_food: Vec<Entity> = Vec::new();

    for (mut pos, mut energy, output, genome, body_size) in &mut organisms {
        // Movement with terrain-dependent cost
        let move_dir = Vec2::new(output.move_x, output.move_y);
        let speed = genome.speed_factor * 2.0 / body_size.0.sqrt();
        let movement = move_dir * speed;

        let tile = tile_map.tile_at_pos(pos.0);

        // Terrain movement cost: interpolate between land and water cost based on aquatic adaptation
        let aqua = genome.aquatic_adaptation;
        let fin_bonus = genome.fin_area() * 0.3;
        let limb_bonus = genome.limb_count() as f32 * 0.15;

        let terrain_cost = if tile.terrain.is_water() {
            let base = tile.terrain.water_move_cost();
            // Fins help in water, aquatic adaptation helps
            (base * (1.0 - aqua * 0.5) * (1.0 - fin_bonus.min(0.5))).max(0.5)
        } else {
            let base = tile.terrain.land_move_cost();
            // Limbs help on land, low aquatic adaptation helps
            (base * (1.0 + aqua * 0.5) * (1.0 - limb_bonus.min(0.4))).max(0.5)
        };

        pos.0 += movement;
        pos.0.x = pos.0.x.rem_euclid(config.world_width as f32);
        pos.0.y = pos.0.y.rem_euclid(config.world_height as f32);

        let move_cost = movement.length() * config.movement_energy_cost * body_size.0 * terrain_cost;
        energy.0 -= move_cost;

        // Eating — need a mouth to eat effectively
        if output.eat > 0.0 {
            let mouth_bonus = if genome.has_mouth() { 1.0 } else { 0.3 };
            let eat_range = body_size.0 * 3.0;
            for &(food_entity, food_pos, food_energy) in &foods {
                if eaten_food.contains(&food_entity) {
                    continue;
                }
                let dist = (pos.0 - food_pos).length();
                if dist < eat_range {
                    energy.0 = (energy.0 + food_energy * mouth_bonus).min(config.max_organism_energy);
                    eaten_food.push(food_entity);
                    break;
                }
            }
        }
    }

    for food_entity in eaten_food {
        commands.entity(food_entity).despawn();
    }
}

/// Photosynthesis: organisms with photo surfaces gain energy from light
fn photosynthesis_system(
    tile_map: Res<TileMap>,
    mut organisms: Query<(&Position, &mut Energy, &Genome), With<Organism>>,
    config: Res<SimConfig>,
) {
    for (pos, mut energy, genome) in &mut organisms {
        if genome.photosynthesis_rate > 0.01 && genome.has_photo_surface() {
            let tile = tile_map.tile_at_pos(pos.0);
            let photo_area = genome.total_photo_surface_area();
            let gained = genome.photosynthesis_rate * photo_area * tile.light_level * 2.0;
            energy.0 = (energy.0 + gained).min(config.max_organism_energy);
        }
    }
}

fn metabolism_system(
    config: Res<SimConfig>,
    mut organisms: Query<(&mut Energy, &BodySize, &Genome), With<Organism>>,
) {
    for (mut energy, body_size, genome) in &mut organisms {
        // Base cost scales with body size and speed
        let mut cost = config.base_metabolism_cost * body_size.0 * (1.0 + genome.speed_factor * 0.2);
        // Extra body parts have maintenance cost
        cost += genome.body_segments.len() as f32 * 0.005;
        // Brain complexity has a cost
        cost += genome.neurons.len() as f32 * 0.001;
        energy.0 -= cost;
    }
}

fn death_system(
    mut commands: Commands,
    organisms: Query<(Entity, &Energy), With<Organism>>,
    mut stats: ResMut<SimStats>,
) {
    for (entity, energy) in &organisms {
        if energy.0 <= 0.0 {
            commands.entity(entity).despawn_recursive();
            stats.total_deaths += 1;
        }
    }
}

fn reproduction_system(
    mut commands: Commands,
    config: Res<SimConfig>,
    mut innovation: ResMut<InnovationCounter>,
    mut organisms: Query<
        (Entity, &Position, &mut Energy, &Genome, &BrainOutput, &BodySize),
        With<Organism>,
    >,
    mut stats: ResMut<SimStats>,
) {
    let mut rng = rand::thread_rng();
    let mut new_organisms: Vec<(Vec2, Genome)> = Vec::new();

    for (_entity, pos, mut energy, genome, output, _body_size) in &mut organisms {
        if output.reproduce > 0.5 && energy.0 > config.reproduction_energy_threshold {
            energy.0 -= config.reproduction_energy_cost;

            let mut child_genome = genome.clone();
            child_genome.mutate(&mut innovation, &mut rng, config.mutation_rate, config.mutation_strength);

            let offset = Vec2::new(
                rng.gen_range(-5.0..5.0),
                rng.gen_range(-5.0..5.0),
            );
            let child_pos = Vec2::new(
                (pos.0.x + offset.x).rem_euclid(config.world_width as f32),
                (pos.0.y + offset.y).rem_euclid(config.world_height as f32),
            );

            new_organisms.push((child_pos, child_genome));
        }
    }

    for (child_pos, child_genome) in new_organisms {
        let brain = Brain::from_genome(&child_genome);
        let body_size = child_genome.body_size;

        commands.spawn((
            Organism,
            Energy(config.reproduction_energy_cost * 0.8),
            Position(child_pos),
            Velocity(Vec2::ZERO),
            BodySize(body_size),
            Age(0),
            SpeciesId(0),
            BrainOutput::default(),
            brain,
            child_genome,
        ));

        stats.total_births += 1;
    }
}

pub fn spawn_initial_population(
    commands: &mut Commands,
    config: &SimConfig,
    innovation: &mut InnovationCounter,
    rng: &mut impl Rng,
) {
    for _ in 0..config.initial_population {
        let x = rng.gen_range(0.0..config.world_width as f32);
        let y = rng.gen_range(0.0..config.world_height as f32);
        let genome = Genome::new_minimal(innovation, rng);
        let brain = Brain::from_genome(&genome);
        let body_size = genome.body_size;

        commands.spawn((
            Organism,
            Energy(config.max_organism_energy * 0.5),
            Position(Vec2::new(x, y)),
            Velocity(Vec2::ZERO),
            BodySize(body_size),
            Age(0),
            SpeciesId(0),
            BrainOutput::default(),
            brain,
            genome,
        ));
    }
}
