use bevy::prelude::*;
use clauvolution_brain::Brain;
use clauvolution_core::*;
use clauvolution_genome::{Genome, InnovationCounter, NUM_INPUTS, NUM_MEMORY};
use clauvolution_world::{SpatialHash, TileMap};
use rand::Rng;
use std::collections::HashMap;

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (sim_speed_system, mass_extinction_input_system))
            .add_systems(
                FixedUpdate,
                (
                    sensing_and_brain_system,
                    action_system,
                    predation_system,
                    photosynthesis_system,
                    niche_construction_system,
                    metabolism_system,
                    death_system,
                    reproduction_system,
                    species_classification_system,
                    record_population_history,
                )
                    .chain(),
            )
            .insert_resource(Time::<Fixed>::from_hz(30.0))
            .insert_resource(SpeciesClassificationTimer(Timer::from_seconds(
                3.0,
                TimerMode::Repeating,
            )))
            .insert_resource(ExtinctionCooldown(Timer::from_seconds(2.0, TimerMode::Once)))
            .insert_resource(PopHistoryTimer(Timer::from_seconds(1.0, TimerMode::Repeating)));
    }
}

#[derive(Resource)]
struct SpeciesClassificationTimer(Timer);

#[derive(Resource)]
struct PopHistoryTimer(Timer);

#[derive(Resource)]
struct ExtinctionCooldown(Timer);

#[derive(Component)]
pub struct BrainOutput {
    pub move_x: f32,
    pub move_y: f32,
    pub eat: f32,
    pub reproduce: f32,
    pub attack: f32,
    pub signal: f32,
    pub memory_out: [f32; NUM_MEMORY],
}

impl Default for BrainOutput {
    fn default() -> Self {
        Self {
            move_x: 0.0,
            move_y: 0.0,
            eat: 0.0,
            reproduce: 0.0,
            attack: 0.0,
            signal: 0.0,
            memory_out: [0.0; NUM_MEMORY],
        }
    }
}

fn sim_speed_system(
    speed: Res<SimSpeed>,
    mut fixed_time: ResMut<Time<Fixed>>,
) {
    if speed.paused {
        fixed_time.set_timestep_hz(0.001);
    } else {
        fixed_time.set_timestep_hz(30.0 * speed.multiplier as f64);
    }
}

/// Player-triggered mass extinction events
fn mass_extinction_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut cooldown: ResMut<ExtinctionCooldown>,
    time: Res<Time>,
    organisms: Query<(Entity, &Position, &Energy), With<Organism>>,
    mut tile_map: Option<ResMut<TileMap>>,
    mut stats: ResMut<SimStats>,
) {
    cooldown.0.tick(time.delta());
    if !cooldown.0.finished() {
        return;
    }

    let mut triggered = false;
    let mut rng = rand::thread_rng();

    // X = asteroid (kill 70% randomly)
    if keys.just_pressed(KeyCode::KeyX) {
        info!("MASS EXTINCTION: Asteroid impact!");
        let mut killed = 0u32;
        for (entity, _, _) in &organisms {
            if rng.gen::<f32>() < 0.7 {
                commands.entity(entity).despawn_recursive();
                killed += 1;
            }
        }
        stats.total_deaths += killed as u64;
        triggered = true;
    }

    // I = ice age (reduce temperature globally)
    if keys.just_pressed(KeyCode::KeyI) {
        if let Some(ref mut tm) = tile_map {
            info!("MASS EXTINCTION: Ice age!");
            for tile in &mut tm.tiles {
                tile.temperature *= 0.5;
                tile.moisture *= 0.7;
            }
            triggered = true;
        }
    }

    // V = volcanic eruption (random kill zone + nutrient boost)
    if keys.just_pressed(KeyCode::KeyV) {
        info!("MASS EXTINCTION: Volcanic eruption!");
        let center_x = rng.gen_range(0.0..256.0f32);
        let center_y = rng.gen_range(0.0..256.0f32);
        let radius = 40.0;

        let mut killed = 0u32;
        for (entity, pos, _) in &organisms {
            let dist = ((pos.0.x - center_x).powi(2) + (pos.0.y - center_y).powi(2)).sqrt();
            if dist < radius {
                commands.entity(entity).despawn_recursive();
                killed += 1;
            }
        }
        stats.total_deaths += killed as u64;

        // Boost nutrients in affected area
        if let Some(ref mut tm) = tile_map {
            for y in 0..tm.height {
                for x in 0..tm.width {
                    let dist = ((x as f32 - center_x).powi(2) + (y as f32 - center_y).powi(2)).sqrt();
                    if dist < radius {
                        let tile = tm.get_mut(x, y);
                        tile.nutrients = (tile.nutrients + 0.5).min(1.0);
                    }
                }
            }
        }
        triggered = true;
    }

    if triggered {
        cooldown.0.reset();
    }
}

fn sensing_and_brain_system(
    config: Res<SimConfig>,
    spatial_hash: Res<SpatialHash>,
    tile_map: Res<TileMap>,
    mut organisms: Query<
        (Entity, &Position, &Energy, &Health, &Genome, &Brain, &BodySize, &SpeciesId, &BrainMemory, &mut BrainOutput),
        With<Organism>,
    >,
    food_query: Query<(Entity, &Position), (With<Food>, Without<Organism>)>,
    all_org_data: Query<(&Position, &BodySize, &SpeciesId, &Genome), (With<Organism>, Without<Food>)>,
) {
    let food_positions: Vec<(Entity, Vec2)> = food_query.iter().map(|(e, p)| (e, p.0)).collect();

    for (entity, pos, energy, health, genome, brain, body_size, species_id, memory, mut output) in &mut organisms {
        let mut inputs = [0.0f32; NUM_INPUTS];

        inputs[0] = energy.0 / config.max_organism_energy;

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

        let nearby_entities = spatial_hash.query_radius(pos.0, sense_range);
        let mut nearest_org_dist = f32::MAX;
        let mut nearest_org_dir = Vec2::ZERO;
        let mut nearest_org_size_ratio = 1.0f32;
        let mut nearest_org_same_species = 0.0f32;
        let mut nearest_org_photo_hint = 0.5f32;

        for &nearby_entity in &nearby_entities {
            if nearby_entity == entity {
                continue;
            }
            if let Ok((other_pos, other_size, other_species, other_genome)) = all_org_data.get(nearby_entity) {
                let diff = other_pos.0 - pos.0;
                let dist = diff.length();
                if dist < nearest_org_dist && dist < sense_range {
                    nearest_org_dist = dist;
                    nearest_org_dir = if dist > 0.001 { diff / dist } else { Vec2::ZERO };
                    nearest_org_size_ratio = other_size.0 / body_size.0;
                    nearest_org_same_species = if other_species.0 == species_id.0 { 1.0 } else { 0.0 };
                    nearest_org_photo_hint = other_genome.photosynthesis_rate.min(1.0);
                }
            }
        }

        if nearest_org_dist < f32::MAX {
            inputs[4] = nearest_org_dir.x;
            inputs[5] = nearest_org_dir.y;
            inputs[6] = 1.0 - (nearest_org_dist / sense_range).min(1.0);
            inputs[7] = nearest_org_size_ratio.min(2.0) / 2.0;
        }

        let tile = tile_map.tile_at_pos(pos.0);
        inputs[8] = if tile.terrain.is_water() { 1.0 } else { 0.0 };
        inputs[9] = tile.nutrients;
        inputs[10] = tile.light_level;
        inputs[11] = genome.aquatic_adaptation;
        inputs[12] = health.0;
        inputs[13] = nearest_org_same_species;
        inputs[14] = memory.0[0];
        inputs[15] = memory.0[1];
        inputs[16] = memory.0[2];
        inputs[17] = nearest_org_photo_hint;
        inputs[18] = 1.0;

        let brain_out = brain.evaluate(&inputs);
        output.move_x = brain_out[0];
        output.move_y = brain_out[1];
        output.eat = brain_out[2];
        output.reproduce = brain_out[3];
        output.attack = brain_out[4];
        output.signal = brain_out[5];
        output.memory_out = [brain_out[6], brain_out[7], brain_out[8]];
    }
}

fn action_system(
    config: Res<SimConfig>,
    tile_map: Res<TileMap>,
    mut organisms: Query<
        (&mut Position, &mut Energy, &mut BrainMemory, &BrainOutput, &Genome, &BodySize),
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

    for (mut pos, mut energy, mut memory, output, genome, body_size) in &mut organisms {
        // Update memory
        memory.0 = output.memory_out;

        let move_dir = Vec2::new(output.move_x, output.move_y);
        let speed = genome.speed_factor * 2.0 / body_size.0.sqrt();
        let movement = move_dir * speed;

        let tile = tile_map.tile_at_pos(pos.0);

        let aqua = genome.aquatic_adaptation;
        let fin_bonus = genome.fin_area() * 0.3;
        let limb_bonus = genome.limb_count() as f32 * 0.15;

        let terrain_cost = if tile.terrain.is_water() {
            let base = tile.terrain.water_move_cost();
            (base * (1.0 - aqua * 0.5) * (1.0 - fin_bonus.min(0.5))).max(0.5)
        } else {
            let base = tile.terrain.land_move_cost();
            (base * (1.0 + aqua * 0.5) * (1.0 - limb_bonus.min(0.4))).max(0.5)
        };

        pos.0 += movement;
        pos.0.x = pos.0.x.rem_euclid(config.world_width as f32);
        pos.0.y = pos.0.y.rem_euclid(config.world_height as f32);

        let move_cost = movement.length() * config.movement_energy_cost * body_size.0 * terrain_cost;
        energy.0 -= move_cost;

        // Eating food
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

/// Predation: organisms can attack and eat each other
fn predation_system(
    spatial_hash: Res<SpatialHash>,
    config: Res<SimConfig>,
    mut organisms: Query<
        (Entity, &Position, &mut Energy, &mut Health, &Genome, &BodySize, &BrainOutput),
        With<Organism>,
    >,
    _commands: Commands,
    _stats: ResMut<SimStats>,
) {
    // Collect attack intents
    let attackers: Vec<(Entity, Vec2, f32, f32, f32)> = organisms
        .iter()
        .filter(|(_, _, _, _, _, _, output)| output.attack > 0.5)
        .map(|(e, pos, _, _, genome, body_size, _)| {
            let attack_str = genome.claw_power() * body_size.0;
            let attack_range = body_size.0 * 4.0;
            (e, pos.0, attack_str, attack_range, body_size.0)
        })
        .collect();

    let mut kills: Vec<(Entity, Entity, f32)> = Vec::new(); // (killer, victim, energy_gained)

    for (attacker_entity, attacker_pos, attack_str, attack_range, attacker_size) in &attackers {
        let nearby = spatial_hash.query_radius(*attacker_pos, *attack_range);

        for &target_entity in &nearby {
            if target_entity == *attacker_entity {
                continue;
            }

            if let Ok((_, target_pos, _, _, target_genome, target_body_size, _)) =
                organisms.get(target_entity)
            {
                let dist = (target_pos.0 - *attacker_pos).length();
                if dist > *attack_range {
                    continue;
                }

                // Attack succeeds if attacker is strong enough relative to target defense
                let defense = target_genome.armor_value() * target_body_size.0;
                let damage = (attack_str - defense * 0.5).max(0.0);

                if damage > 0.1 {
                    // Size advantage matters — can't easily eat things bigger than you
                    if *attacker_size > target_body_size.0 * 0.6 {
                        let energy_gained = target_body_size.0 * 15.0;
                        kills.push((*attacker_entity, target_entity, energy_gained));
                        break; // Only one kill per tick
                    }
                }
            }
        }
    }

    // Apply kills — set victim energy to 0, death_system handles despawn
    for (killer, victim, energy_gained) in kills {
        if let Ok((_, _, mut killer_energy, _, _, _, _)) = organisms.get_mut(killer) {
            killer_energy.0 = (killer_energy.0 + energy_gained).min(config.max_organism_energy);
        }
        if let Ok((_, _, mut victim_energy, mut victim_health, _, _, _)) = organisms.get_mut(victim) {
            victim_energy.0 = 0.0;
            victim_health.0 = 0.0;
        }
    }
}

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

/// Niche construction: organisms modify the tiles they're on
fn niche_construction_system(
    mut tile_map: ResMut<TileMap>,
    organisms: Query<(&Position, &Genome), With<Organism>>,
) {
    for (pos, genome) in &organisms {
        let x = (pos.0.x as u32).min(tile_map.width - 1);
        let y = (pos.0.y as u32).min(tile_map.height - 1);
        let tile = tile_map.get_mut(x, y);

        // Photosynthesizers increase vegetation and moisture
        if genome.photosynthesis_rate > 0.1 && genome.has_photo_surface() {
            tile.vegetation_density = (tile.vegetation_density + 0.001).min(1.0);
            tile.moisture = (tile.moisture + 0.0005).min(1.0);
        }

        // All organisms slightly increase nutrients (waste products)
        tile.nutrients = (tile.nutrients + 0.0001).min(1.0);
    }
}

fn metabolism_system(
    config: Res<SimConfig>,
    mut organisms: Query<(&mut Energy, &mut Health, &mut Age, &BodySize, &Genome), With<Organism>>,
) {
    for (mut energy, mut health, mut age, body_size, genome) in &mut organisms {
        age.0 += 1;

        // Minimum base cost prevents tiny organisms from cheating the energy economy
        let effective_size = body_size.0.max(0.5);
        let mut cost = config.base_metabolism_cost * effective_size * (1.0 + genome.speed_factor * 0.2);
        cost += genome.body_segments.len() as f32 * 0.005;
        cost += genome.neurons.len() as f32 * 0.001;
        cost += genome.armor_value() * 0.01;
        cost += genome.claw_power() * 0.008;

        // Aging: metabolism cost increases after maturity (age 500 ticks ~ 17 seconds)
        let age_factor = if age.0 > 500 {
            1.0 + (age.0 - 500) as f32 * 0.0005
        } else {
            1.0
        };
        cost *= age_factor;

        energy.0 -= cost;

        // Health regenerates slower with age
        let regen_rate = 0.005 / age_factor;
        health.0 = (health.0 + regen_rate).min(1.0);

        // Old age death: after ~3000 ticks (~100 seconds), health degrades
        if age.0 > 3000 {
            health.0 -= 0.002;
            if health.0 <= 0.0 {
                energy.0 = 0.0; // triggers death
            }
        }
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
    spatial_hash: Res<SpatialHash>,
    mut organisms: Query<
        (Entity, &Position, &mut Energy, &Genome, &BrainOutput, &BodySize, &SpeciesId, &Generation),
        With<Organism>,
    >,
    mut stats: ResMut<SimStats>,
) {
    let mut rng = rand::thread_rng();

    // Collect potential mate data upfront to avoid query conflicts
    let mate_candidates: Vec<(Entity, Vec2, f32, Genome, u64)> = organisms
        .iter()
        .filter(|(_, _, _, _, output, _, _, _)| output.reproduce > 0.5)
        .map(|(e, pos, energy, genome, _, _, species, _)| {
            (e, pos.0, energy.0, genome.clone(), species.0)
        })
        .collect();

    let mut new_organisms: Vec<(Vec2, Genome, u64, u32)> = Vec::new();
    let current_pop = organisms.iter().len();
    let max_pop = 2000usize;
    let mut already_mated: Vec<Entity> = Vec::new();

    for (entity, pos, mut energy, genome, output, body_size, species, generation) in &mut organisms {
        if current_pop + new_organisms.len() >= max_pop {
            break;
        }
        if already_mated.contains(&entity) {
            continue;
        }
        // Reproduction cost scales with body size — small organisms can't reproduce for free
        let repro_cost = config.reproduction_energy_cost * (0.5 + body_size.0 * 0.5);
        let repro_threshold = config.reproduction_energy_threshold * (0.5 + body_size.0 * 0.5);
        if output.reproduce > 0.5 && energy.0 > repro_threshold {
            energy.0 -= repro_cost;

            // Try to find a mate from pre-collected candidates
            let mate_range = body_size.0 * 8.0;
            let nearby = spatial_hash.query_radius(pos.0, mate_range);
            let mut mate_genome: Option<Genome> = None;

            for &nearby_entity in &nearby {
                if nearby_entity == entity || already_mated.contains(&nearby_entity) {
                    continue;
                }
                if let Some((_, _, mate_energy, mate_g, mate_species)) =
                    mate_candidates.iter().find(|(e, _, _, _, _)| *e == nearby_entity)
                {
                    if *mate_species == species.0 && *mate_energy > config.reproduction_energy_threshold {
                        mate_genome = Some(mate_g.clone());
                        already_mated.push(nearby_entity);
                        break;
                    }
                }
            }

            let mut child_genome = if let Some(mate_g) = mate_genome {
                genome.crossover(&mate_g, &mut rng)
            } else {
                genome.clone()
            };

            child_genome.mutate(&mut innovation, &mut rng, config.mutation_rate, config.mutation_strength);

            let offset = Vec2::new(
                rng.gen_range(-5.0..5.0),
                rng.gen_range(-5.0..5.0),
            );
            let child_pos = Vec2::new(
                (pos.0.x + offset.x).rem_euclid(config.world_width as f32),
                (pos.0.y + offset.y).rem_euclid(config.world_height as f32),
            );

            new_organisms.push((child_pos, child_genome, species.0, generation.0 + 1));
            already_mated.push(entity);
        }
    }

    for (child_pos, child_genome, parent_species, child_gen) in new_organisms {
        let brain = Brain::from_genome(&child_genome);
        let body_size = child_genome.body_size;

        commands.spawn((
            Organism,
            Energy(config.reproduction_energy_cost * 0.8),
            Health(1.0),
            Position(child_pos),
            Velocity(Vec2::ZERO),
            BodySize(body_size),
            Age(0),
            Generation(child_gen),
            SpeciesId(parent_species),
            BrainOutput::default(),
            BrainMemory([0.0; NUM_MEMORY]),
            brain,
            child_genome,
        ));

        stats.total_births += 1;
        if child_gen > stats.max_generation {
            stats.max_generation = child_gen;
        }
    }
}

fn species_classification_system(
    time: Res<Time>,
    mut timer: ResMut<SpeciesClassificationTimer>,
    config: Res<SimConfig>,
    mut organisms: Query<(Entity, &Genome, &mut SpeciesId), With<Organism>>,
    mut stats: ResMut<SimStats>,
    mut species_colors: ResMut<SpeciesColors>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let org_data: Vec<(Entity, Genome, u64)> = organisms
        .iter()
        .map(|(e, g, s)| (e, g.clone(), s.0))
        .collect();

    if org_data.is_empty() {
        return;
    }

    let mut species_reps: Vec<(u64, Genome)> = Vec::new();
    let mut next_species_id: u64 = 1;

    let mut seen_species: HashMap<u64, usize> = HashMap::new();
    for (_entity, genome, species_id) in &org_data {
        if *species_id > 0 && !seen_species.contains_key(species_id) {
            seen_species.insert(*species_id, species_reps.len());
            species_reps.push((*species_id, genome.clone()));
            if *species_id >= next_species_id {
                next_species_id = *species_id + 1;
            }
        }
    }

    let mut assignments: HashMap<Entity, u64> = HashMap::new();

    for (entity, genome, _old_species) in &org_data {
        let mut best_species = None;
        let mut best_dist = f32::MAX;

        for (species_id, rep_genome) in &species_reps {
            let dist = genome.compatibility_distance(rep_genome);
            if dist < config.species_compat_threshold && dist < best_dist {
                best_dist = dist;
                best_species = Some(*species_id);
            }
        }

        let assigned = if let Some(id) = best_species {
            id
        } else {
            let new_id = next_species_id;
            next_species_id += 1;
            species_reps.push((new_id, genome.clone()));
            species_colors.get_or_create(new_id);
            new_id
        };

        assignments.insert(*entity, assigned);
    }

    for (entity, _genome, mut species_id) in &mut organisms {
        if let Some(&new_id) = assignments.get(&entity) {
            species_id.0 = new_id;
        }
    }

    stats.species_count = species_reps.len() as u32;
}

fn record_population_history(
    time: Res<Time>,
    mut timer: ResMut<PopHistoryTimer>,
    stats: Res<SimStats>,
    organisms: Query<&Organism>,
    food: Query<&Food>,
    mut history: ResMut<PopulationHistory>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let org_count = organisms.iter().len() as u32;
    let food_count = food.iter().len() as u32;
    history.record(&stats, org_count, food_count);
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
            Health(1.0),
            Position(Vec2::new(x, y)),
            Velocity(Vec2::ZERO),
            BodySize(body_size),
            Age(0),
            Generation(0),
            SpeciesId(0),
            BrainOutput::default(),
            BrainMemory([0.0; NUM_MEMORY]),
            brain,
            genome,
        ));
    }
}
