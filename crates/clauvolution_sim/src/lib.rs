use bevy::prelude::*;
use clauvolution_brain::Brain;
use clauvolution_core::*;
use clauvolution_genome::{Genome, InnovationCounter, NUM_INPUTS};
use clauvolution_world::SpatialHash;
use rand::Rng;

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                sensing_and_brain_system,
                action_system,
                metabolism_system,
                death_system,
                reproduction_system,
            )
                .chain(),
        )
        .insert_resource(Time::<Fixed>::from_hz(30.0));
    }
}

/// Sensory input + brain output stored per organism per tick
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

/// Sensing + brain evaluation combined
fn sensing_and_brain_system(
    config: Res<SimConfig>,
    spatial_hash: Res<SpatialHash>,
    mut organisms: Query<
        (Entity, &Position, &Energy, &Genome, &Brain, &BodySize, &mut BrainOutput),
        With<Organism>,
    >,
    food_query: Query<(Entity, &Position), (With<Food>, Without<Organism>)>,
    all_positions: Query<(&Position, &BodySize), (With<Organism>, Without<Food>)>,
) {
    // Build a quick list of food positions for sensing
    let food_positions: Vec<(Entity, Vec2)> = food_query.iter().map(|(e, p)| (e, p.0)).collect();

    // For each organism, build sensory input and evaluate brain
    for (entity, pos, energy, genome, brain, body_size, mut output) in &mut organisms {
        let mut inputs = [0.0f32; NUM_INPUTS];

        // Input 0: energy level (normalized 0-1)
        inputs[0] = energy.0 / config.max_organism_energy;

        // Find nearest food within sense range
        let sense_range = genome.sense_range;
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

        // Find nearest organism within sense range
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

        // Input 8: bias
        inputs[8] = 1.0;

        // Evaluate brain
        let brain_out = brain.evaluate(&inputs);

        output.move_x = brain_out[0];
        output.move_y = brain_out[1];
        output.eat = brain_out[2];
        output.reproduce = brain_out[3];
    }
}

/// Execute organism actions based on brain outputs
fn action_system(
    config: Res<SimConfig>,
    mut organisms: Query<
        (&mut Position, &mut Energy, &BrainOutput, &Genome, &BodySize),
        (With<Organism>, Without<Food>),
    >,
    food_query: Query<(Entity, &Position, &FoodEnergy), (With<Food>, Without<Organism>)>,
    mut commands: Commands,
) {
    // Collect food entities for eating
    let foods: Vec<(Entity, Vec2, f32)> = food_query
        .iter()
        .map(|(e, p, fe)| (e, p.0, fe.0))
        .collect();

    let mut eaten_food: Vec<Entity> = Vec::new();

    for (mut pos, mut energy, output, genome, body_size) in &mut organisms {
        // Movement
        let move_dir = Vec2::new(output.move_x, output.move_y);
        let speed = genome.speed_factor * 2.0 / body_size.0.sqrt();
        let movement = move_dir * speed;
        pos.0 += movement;

        // Wrap around world edges
        pos.0.x = pos.0.x.rem_euclid(config.world_width as f32);
        pos.0.y = pos.0.y.rem_euclid(config.world_height as f32);

        // Movement energy cost
        let move_cost = movement.length() * config.movement_energy_cost * body_size.0;
        energy.0 -= move_cost;

        // Eating
        if output.eat > 0.0 {
            let eat_range = body_size.0 * 3.0;
            for &(food_entity, food_pos, food_energy) in &foods {
                if eaten_food.contains(&food_entity) {
                    continue;
                }
                let dist = (pos.0 - food_pos).length();
                if dist < eat_range {
                    energy.0 = (energy.0 + food_energy).min(config.max_organism_energy);
                    eaten_food.push(food_entity);
                    break;
                }
            }
        }
    }

    // Despawn eaten food
    for food_entity in eaten_food {
        commands.entity(food_entity).despawn();
    }
}

/// Subtract base metabolism cost each tick
fn metabolism_system(
    config: Res<SimConfig>,
    mut organisms: Query<(&mut Energy, &BodySize, &Genome), With<Organism>>,
) {
    for (mut energy, body_size, genome) in &mut organisms {
        // Larger organisms and faster ones cost more to maintain
        let cost = config.base_metabolism_cost * body_size.0 * (1.0 + genome.speed_factor * 0.2);
        energy.0 -= cost;
    }
}

/// Remove dead organisms
fn death_system(
    mut commands: Commands,
    organisms: Query<(Entity, &Energy), With<Organism>>,
    mut stats: ResMut<SimStats>,
) {
    for (entity, energy) in &organisms {
        if energy.0 <= 0.0 {
            commands.entity(entity).despawn();
            stats.total_deaths += 1;
        }
    }
}

/// Reproduction: organisms with enough energy can reproduce
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
        // Must want to reproduce AND have enough energy
        if output.reproduce > 0.5 && energy.0 > config.reproduction_energy_threshold {
            energy.0 -= config.reproduction_energy_cost;

            // Asexual reproduction with mutation (Phase 1)
            let mut child_genome = genome.clone();
            child_genome.mutate(&mut innovation, &mut rng, config.mutation_rate, config.mutation_strength);

            // Offset spawn position slightly
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

    // Spawn children
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

/// Spawn the initial population
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
