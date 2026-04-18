pub mod save;

use bevy::prelude::*;
use clauvolution_brain::Brain;
use clauvolution_core::*;
use clauvolution_genome::{Genome, InnovationCounter, NUM_INPUTS, NUM_MEMORY};
use clauvolution_phylogeny::{PhyloTree, SpeciesStrategy, SpeciesTraits, WorldChronicle};
use clauvolution_world::{SpatialHash, TileMap};
use rand::Rng;
use std::collections::HashMap;

// -----------------------------------------------------------------------------
// Disease tuning constants
//
// Grouped here so tuning doesn't require hunting for literals scattered through
// disease_transmission_system / disease_effects_system. Adjust, rebuild, observe.
// See docs/ROADMAP.md "Current tuning state" for the history of these values.
// -----------------------------------------------------------------------------

/// How often (in sim ticks) the background spontaneous-infection roll runs.
/// Ticks are 1/30s, so 30 = once per sim-second.
const DISEASE_BACKGROUND_PERIOD_TICKS: u64 = 30;
/// Per-check chance each healthy organism gets spontaneously infected (before resistance).
const DISEASE_BACKGROUND_RATE: f32 = 0.001;
/// Severity range \[min, max) for background spontaneous infections.
const DISEASE_BACKGROUND_SEVERITY: std::ops::Range<f32> = 0.3..0.7;
/// Duration range \[min, max) (in ticks) for background spontaneous infections. 10–23s at 30Hz.
const DISEASE_BACKGROUND_DURATION_TICKS: std::ops::Range<u32> = 300..700;

/// Radius (world units) within which infection can transmit between organisms.
const DISEASE_TRANSMISSION_RANGE: f32 = 20.0;
/// Per-tick infection chance multiplier from proximity pressure.
const DISEASE_TRANSMISSION_RATE: f32 = 0.005;
/// Hard cap on per-tick transmission chance (prevents runaway at dense cluster centres).
const DISEASE_TRANSMISSION_CHANCE_CAP: f32 = 0.1;
/// Fraction of source severity retained when the disease transmits to a new host.
const DISEASE_TRANSMISSION_SEVERITY_DECAY: f32 = 0.9;
/// Minimum duration (in ticks) for a transmitted infection.
const DISEASE_TRANSMISSION_MIN_DURATION_TICKS: u32 = 200;

/// Energy-drain multiplier applied per tick to infected organisms (scales on severity and resistance).
const DISEASE_DRAIN_MULTIPLIER: f32 = 1.6;
/// Base per-tick chance of direct disease-caused death (scales on severity and (1 - resistance)).
/// At severity 0.5 and zero resistance this gives ~22% cumulative chance over 20s.
const DISEASE_MORTALITY_RATE: f32 = 0.0015;

// -----------------------------------------------------------------------------
// Bloom event tuning constants
// -----------------------------------------------------------------------------

/// Duration of a bloom effect in sim ticks. 900 ticks = 30 seconds at 30Hz.
const BLOOM_DURATION_TICKS: u64 = 900;
/// Multiplier applied to light intensity during a solar bloom.
const SOLAR_BLOOM_LIGHT_MULTIPLIER: f32 = 2.0;
/// Multiplier applied to mutation rate during a Cambrian spark.
const CAMBRIAN_MUTATION_MULTIPLIER: f32 = 3.0;
/// Fraction of world tiles that receive a food drop during a nutrient rain.
const NUTRIENT_RAIN_DENSITY: f32 = 0.05;

// -----------------------------------------------------------------------------
// Other simulation tuning constants
// -----------------------------------------------------------------------------

/// Penalty coefficient for plants sharing a tile. Yield = 1 / (1 + others × k).
/// Higher k = steeper penalty. Raising this combats green-world monocultures.
const PLANT_DENSITY_PENALTY: f32 = 0.2;

/// Minimum real-time seconds between extinction/bloom events (prevents spam).
const WORLD_EVENT_COOLDOWN_SECS: f32 = 2.0;

/// Real-time seconds between species-classification passes.
/// Higher = more stable species names; lower = faster speciation detection.
const SPECIES_CLASSIFICATION_PERIOD_SECS: f32 = 5.0;

/// Seconds between population-history samples (drives Graphs tab granularity).
const POP_HISTORY_SAMPLE_SECS: f32 = 1.0;

/// Hysteresis factor: organisms stay in their current species if their distance
/// is within this multiple of the compatibility threshold. Prevents flip-flopping.
const SPECIES_HYSTERESIS_FACTOR: f32 = 1.3;

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (sim_speed_system, keyboard_to_events_system, mass_extinction_input_system, save_system).chain())
            .add_systems(
                FixedUpdate,
                (
                    tick_counter_system,
                    update_food_snapshot,
                    sensing_and_brain_system,
                    action_system,
                    predation_system,
                    photosynthesis_system,
                    niche_construction_system,
                    disease_transmission_system,
                    disease_effects_system,
                    metabolism_system,
                    death_system,
                    reproduction_system,
                    species_classification_system,
                    record_population_history,
                    record_trail_history,
                )
                    .chain(),
            )
            .insert_resource(Time::<Fixed>::from_hz(30.0))
            .insert_resource(Time::<Virtual>::from_max_delta(std::time::Duration::from_millis(100)))
            .insert_resource(SpeciesClassificationTimer(Timer::from_seconds(
                SPECIES_CLASSIFICATION_PERIOD_SECS,
                TimerMode::Repeating,
            )))
            .insert_resource(ExtinctionCooldown(Timer::from_seconds(WORLD_EVENT_COOLDOWN_SECS, TimerMode::Once)))
            .insert_resource(PopHistoryTimer(Timer::from_seconds(POP_HISTORY_SAMPLE_SECS, TimerMode::Repeating)));
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

fn tick_counter_system(mut tick: ResMut<TickCounter>, mut season: ResMut<Season>, mut chronicle: ResMut<WorldChronicle>, session: Res<Session>, mut bloom: ResMut<BloomEffects>) {
    // Set chronicle log path from session on first tick
    if tick.0 == 0 {
        chronicle.log_path = Some(session.log_path());
        chronicle.log(0, format!("Session '{}' started", session.name));
    }
    tick.0 += 1;
    bloom.tick();
    let old_name = season.name();
    season.advance();
    let new_name = season.name();
    if old_name != new_name {
        let name = match new_name {
            SeasonName::Spring => "Spring arrives — light and food increasing",
            SeasonName::Summer => "Summer — peak light and food production",
            SeasonName::Autumn => "Autumn — light fading, food declining",
            SeasonName::Winter => "Winter begins — scarce food, low light",
        };
        chronicle.log(tick.0, name.to_string());
    }
}

fn sim_speed_system(
    speed: Res<SimSpeed>,
    mut fixed_time: ResMut<Time<Fixed>>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    if speed.paused {
        virtual_time.pause();
    } else {
        virtual_time.unpause();
        fixed_time.set_timestep_hz(30.0 * speed.multiplier as f64);
    }
}

/// Translate keyboard hotkeys into WorldEventRequest events
fn keyboard_to_events_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<WorldEventRequest>,
) {
    if keys.just_pressed(KeyCode::KeyX) { events.send(WorldEventRequest::Asteroid); }
    if keys.just_pressed(KeyCode::KeyI) { events.send(WorldEventRequest::IceAge); }
    if keys.just_pressed(KeyCode::KeyV) { events.send(WorldEventRequest::Volcano); }
    if keys.just_pressed(KeyCode::KeyB) { events.send(WorldEventRequest::SolarBloom); }
    if keys.just_pressed(KeyCode::KeyN) { events.send(WorldEventRequest::NutrientRain); }
    if keys.just_pressed(KeyCode::KeyJ) { events.send(WorldEventRequest::CambrianSpark); }
    if keys.just_pressed(KeyCode::F5)    { events.send(WorldEventRequest::Save); }
}

/// Process WorldEventRequest events — fired by keyboard, UI buttons, etc.
fn mass_extinction_input_system(
    mut requests: EventReader<WorldEventRequest>,
    mut commands: Commands,
    mut cooldown: ResMut<ExtinctionCooldown>,
    time: Res<Time>,
    organisms: Query<(Entity, &Position, &Energy), With<Organism>>,
    mut tile_map: Option<ResMut<TileMap>>,
    mut stats: ResMut<SimStats>,
    tick: Res<TickCounter>,
    mut chronicle: ResMut<WorldChronicle>,
    config: Res<SimConfig>,
    mut bloom: ResMut<BloomEffects>,
) {
    cooldown.0.tick(time.delta());

    // Find the first ext/bloom event this frame (skip Save — handled elsewhere).
    // If cooldown is active, ignore ext/bloom events.
    let req = requests.read().find(|r| !matches!(r, WorldEventRequest::Save)).copied();

    if !cooldown.0.finished() {
        return;
    }

    let Some(req) = req else { return };

    let mut triggered = false;
    let mut rng = rand::thread_rng();

    // Asteroid (kill 70% randomly)
    if matches!(req, WorldEventRequest::Asteroid) {
        info!("MASS EXTINCTION: Asteroid impact!");
        let mut killed = 0u32;
        for (entity, _, _) in &organisms {
            if rng.gen::<f32>() < 0.7 {
                commands.entity(entity).try_despawn_recursive();
                killed += 1;
            }
        }
        stats.total_deaths += killed as u64;
        chronicle.log(tick.0, format!("ASTEROID IMPACT! {} organisms killed", killed));
        triggered = true;
    }

    // Ice age (reduce temperature globally)
    if matches!(req, WorldEventRequest::IceAge) {
        if let Some(ref mut tm) = tile_map {
            info!("MASS EXTINCTION: Ice age!");
            for tile in &mut tm.tiles {
                tile.temperature *= 0.5;
                tile.moisture *= 0.7;
            }
            chronicle.log(tick.0, "ICE AGE! Temperature halved, moisture reduced".to_string());
            triggered = true;
        }
    }

    // Volcanic eruption (random kill zone + nutrient boost)
    if matches!(req, WorldEventRequest::Volcano) {
        info!("MASS EXTINCTION: Volcanic eruption!");
        let center_x = rng.gen_range(0.0..256.0f32);
        let center_y = rng.gen_range(0.0..256.0f32);
        let radius = 40.0;

        let mut killed = 0u32;
        for (entity, pos, _) in &organisms {
            let dist = ((pos.0.x - center_x).powi(2) + (pos.0.y - center_y).powi(2)).sqrt();
            if dist < radius {
                commands.entity(entity).try_despawn_recursive();
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
        chronicle.log(tick.0, format!("VOLCANIC ERUPTION! {} organisms killed near ({:.0}, {:.0})", killed, center_x, center_y));
        triggered = true;
    }

    // Solar bloom (boost light for a fixed duration)
    if matches!(req, WorldEventRequest::SolarBloom) {
        info!("BLOOM: Solar bloom!");
        bloom.solar_bloom = SOLAR_BLOOM_LIGHT_MULTIPLIER;
        bloom.solar_ticks = BLOOM_DURATION_TICKS;
        chronicle.log(tick.0, "SOLAR BLOOM! Light doubled — photosynthesizers surge".to_string());
        triggered = true;
    }

    // Nutrient rain (massive food burst)
    if matches!(req, WorldEventRequest::NutrientRain) {
        info!("BLOOM: Nutrient rain!");
        let mut rng = rand::thread_rng();
        let food_count = (config.world_width as f32 * config.world_height as f32 * NUTRIENT_RAIN_DENSITY) as u32;
        for _ in 0..food_count {
            let x = rng.gen_range(0.0..config.world_width as f32);
            let y = rng.gen_range(0.0..config.world_height as f32);
            commands.spawn((
                Food,
                FoodEnergy(config.food_energy_value),
                Position(Vec2::new(x, y)),
            ));
        }
        chronicle.log(tick.0, format!("NUTRIENT RAIN! {} food spawned across the world", food_count));
        triggered = true;
    }

    // Cambrian spark (boost mutation rate for a fixed duration)
    if matches!(req, WorldEventRequest::CambrianSpark) {
        info!("BLOOM: Cambrian spark!");
        bloom.mutation_boost = CAMBRIAN_MUTATION_MULTIPLIER;
        bloom.mutation_ticks = BLOOM_DURATION_TICKS;
        chronicle.log(tick.0, "CAMBRIAN SPARK! Mutation rate tripled — rapid speciation".to_string());
        triggered = true;
    }

    if triggered {
        cooldown.0.reset();
    }
}

/// Rebuild the FoodSnapshot resource once per tick. Both sensing_and_brain_system
/// and action_system read food positions; this saves building the Vec twice.
fn update_food_snapshot(
    mut snapshot: ResMut<FoodSnapshot>,
    food_query: Query<(Entity, &Position, &FoodEnergy), (With<Food>, Without<Organism>)>,
) {
    snapshot.entries.clear();
    snapshot.entries.extend(food_query.iter().map(|(e, p, fe)| (e, p.0, fe.0)));
}

fn sensing_and_brain_system(
    config: Res<SimConfig>,
    spatial_hash: Res<SpatialHash>,
    tile_map: Res<TileMap>,
    mut organisms: Query<
        (Entity, &Position, &Energy, &Health, &Genome, &Brain, &BodySize, &SpeciesId, &BrainMemory, &mut BrainOutput, &mut GroupSize),
        With<Organism>,
    >,
    food_snapshot: Res<FoodSnapshot>,
    all_org_data: Query<(&Position, &BodySize, &SpeciesId, &Genome, &Signal), (With<Organism>, Without<Food>)>,
) {

    for (entity, pos, energy, health, genome, brain, body_size, species_id, memory, mut output, mut group_size) in &mut organisms {
        let mut inputs = [0.0f32; NUM_INPUTS];

        inputs[0] = energy.0 / config.max_organism_energy;

        let sense_range = genome.effective_sense_range();
        let mut nearest_food_dist = f32::MAX;
        let mut nearest_food_dir = Vec2::ZERO;

        for &(_food_entity, food_pos, _fe) in &food_snapshot.entries {
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
        let mut nearest_org_signal = 0.0f32;

        // Social sensing: count nearby same-species, average their signals
        let mut same_species_count = 0u32;
        let mut same_species_signal_sum = 0.0f32;

        for &nearby_entity in &nearby_entities {
            if nearby_entity == entity {
                continue;
            }
            if let Ok((other_pos, other_size, other_species, other_genome, other_signal)) = all_org_data.get(nearby_entity) {
                let diff = other_pos.0 - pos.0;
                let dist = diff.length();

                // Track same-species neighbours for social inputs
                if dist < sense_range && other_species.0 == species_id.0 {
                    same_species_count += 1;
                    same_species_signal_sum += other_signal.0;
                }

                if dist < nearest_org_dist && dist < sense_range {
                    nearest_org_dist = dist;
                    nearest_org_dir = if dist > 0.001 { diff / dist } else { Vec2::ZERO };
                    nearest_org_size_ratio = other_size.0 / body_size.0;
                    nearest_org_same_species = if other_species.0 == species_id.0 { 1.0 } else { 0.0 };
                    nearest_org_photo_hint = other_genome.photosynthesis_rate.min(1.0);
                    nearest_org_signal = other_signal.0;
                }
            }
        }

        // Store group size for metabolism system
        group_size.0 = same_species_count;

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
        inputs[18] = nearest_org_signal;
        // Social inputs
        inputs[19] = (same_species_count as f32 / 10.0).min(1.0); // 0=alone, 1=10+ nearby
        inputs[20] = if same_species_count > 0 {
            same_species_signal_sum / same_species_count as f32
        } else {
            0.0
        };
        inputs[21] = 1.0; // bias

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
        (&mut Position, &mut Energy, &mut BrainMemory, &mut ActionFlash, &mut Signal, &BrainOutput, &Genome, &BodySize),
        (With<Organism>, Without<Food>),
    >,
    food_snapshot: Res<FoodSnapshot>,
    mut commands: Commands,
) {
    let foods = &food_snapshot.entries;

    let mut eaten_food: Vec<Entity> = Vec::new();

    for (mut pos, mut energy, mut memory, mut flash, mut signal, output, genome, body_size) in &mut organisms {
        // Tick down flash timer
        flash.timer = (flash.timer - 0.033).max(0.0);
        if flash.timer <= 0.0 { flash.action = ActionType::None; }
        // Update memory
        memory.0 = output.memory_out;
        signal.0 = output.signal.clamp(-1.0, 1.0);

        let move_dir = Vec2::new(output.move_x, output.move_y);
        // Armor slows you down — heavy organisms are slower
        let armor_drag = 1.0 / (1.0 + genome.armor_value() * 0.3);
        let speed = genome.speed_factor * 2.0 / body_size.0.sqrt() * armor_drag;
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
            for &(food_entity, food_pos, food_energy) in foods {
                if eaten_food.contains(&food_entity) {
                    continue;
                }
                let dist = (pos.0 - food_pos).length();
                if dist < eat_range {
                    energy.0 = (energy.0 + food_energy * mouth_bonus).min(config.max_organism_energy);
                    eaten_food.push(food_entity);
                    flash.action = ActionType::Eating;
                    flash.timer = 0.3;
                    break;
                }
            }
        }
    }

    for food_entity in eaten_food {
        commands.entity(food_entity).try_despawn();
    }
}

/// Predation: organisms can attack and eat each other
fn predation_system(
    spatial_hash: Res<SpatialHash>,
    config: Res<SimConfig>,
    mut organisms: Query<
        (Entity, &Position, &mut Energy, &mut Health, &mut ActionFlash, &Genome, &BodySize, &BrainOutput),
        With<Organism>,
    >,
    _commands: Commands,
    _stats: ResMut<SimStats>,
) {
    // Collect attack intents
    let attackers: Vec<(Entity, Vec2, f32, f32, f32)> = organisms
        .iter()
        .filter(|(_, _, _, _, _, _, _, output)| output.attack > 0.5)
        .map(|(e, pos, _, _, _, genome, body_size, _)| {
            let attack_str = genome.claw_power() * body_size.0;
            let attack_range = body_size.0 * 4.0;
            (e, pos.0, attack_str, attack_range, body_size.0)
        })
        .collect();

    // (killer, victim, victim_energy) — energy transfer computed at kill time
    let mut kills: Vec<(Entity, Entity, f32)> = Vec::new();

    for (attacker_entity, attacker_pos, attack_str, attack_range, attacker_size) in &attackers {
        let nearby = spatial_hash.query_radius(*attacker_pos, *attack_range);

        for &target_entity in &nearby {
            if target_entity == *attacker_entity {
                continue;
            }

            if let Ok((_, target_pos, target_energy, _, _, target_genome, target_body_size, _)) =
                organisms.get(target_entity)
            {
                let dist = (target_pos.0 - *attacker_pos).length();
                if dist > *attack_range {
                    continue;
                }

                let defense = target_genome.armor_value() * target_body_size.0;
                let damage = (attack_str - defense * 0.5).max(0.0);

                if damage > 0.1 && *attacker_size > target_body_size.0 * 0.6 {
                    // Energy pyramid: predator gets 10% of prey's actual stored energy.
                    // This is thermodynamics — most energy is lost as heat.
                    let energy_gained = target_energy.0 * 0.1;
                    kills.push((*attacker_entity, target_entity, energy_gained));
                    break;
                }
            }
        }
    }

    for (killer, victim, energy_gained) in kills {
        if let Ok((_, _, mut killer_energy, _, mut killer_flash, _, _, _)) = organisms.get_mut(killer) {
            killer_energy.0 = (killer_energy.0 + energy_gained).min(config.max_organism_energy);
            killer_flash.action = ActionType::Attacking;
            killer_flash.timer = 0.3;
        }
        if let Ok((_, _, mut victim_energy, mut victim_health, _, _, _, _)) = organisms.get_mut(victim) {
            victim_energy.0 = 0.0;
            victim_health.0 = 0.0;
        }
    }
}

fn photosynthesis_system(
    tile_map: Res<TileMap>,
    mut organisms: Query<(&Position, &mut Energy, &Genome), With<Organism>>,
    config: Res<SimConfig>,
    season: Res<Season>,
    bloom: Res<BloomEffects>,
) {
    let light_mult = season.light_multiplier() * bloom.light_multiplier();

    // First pass: count plants per tile for density competition.
    // Plants on the same tile shade each other — prevents green-world monoculture.
    let mut plants_per_tile: HashMap<(u32, u32), u32> = HashMap::new();
    for (pos, _, genome) in organisms.iter() {
        if genome.photosynthesis_rate > 0.2 && genome.has_photo_surface() {
            let tx = (pos.0.x as u32).min(tile_map.width - 1);
            let ty = (pos.0.y as u32).min(tile_map.height - 1);
            *plants_per_tile.entry((tx, ty)).or_insert(0) += 1;
        }
    }

    // Second pass: apply photosynthesis with density-dependent competition.
    // density_factor = 1 / (1 + other_plants * 0.2)
    //   1 plant alone: 1.0 (full yield)
    //   5 plants:      0.56
    //   10 plants:     0.36
    //   20 plants:     0.21
    for (pos, mut energy, genome) in &mut organisms {
        if genome.photosynthesis_rate > 0.01 && genome.has_photo_surface() {
            let tile = tile_map.tile_at_pos(pos.0);
            let photo_area = genome.total_photo_surface_area();

            let tx = (pos.0.x as u32).min(tile_map.width - 1);
            let ty = (pos.0.y as u32).min(tile_map.height - 1);
            let tile_plants = plants_per_tile.get(&(tx, ty)).copied().unwrap_or(1);
            let others = tile_plants.saturating_sub(1);
            let density_factor = 1.0 / (1.0 + others as f32 * PLANT_DENSITY_PENALTY);

            let gained = genome.photosynthesis_rate * photo_area * tile.light_level * light_mult * density_factor * 2.0;
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

/// Spread infection between nearby organisms and seed rare background infections.
/// Runs before metabolism so infection status this tick can affect energy drain.
fn disease_transmission_system(
    spatial_hash: Res<SpatialHash>,
    mut commands: Commands,
    healthy: Query<(Entity, &Position, &Genome), (With<Organism>, Without<Infection>)>,
    infected: Query<(&Position, &Infection), With<Organism>>,
    tick: Res<TickCounter>,
) {
    let mut rng = rand::thread_rng();

    // 1. Background spontaneous infection — keeps disease present even when
    // populations would otherwise clear all pathogens.
    if tick.0 % DISEASE_BACKGROUND_PERIOD_TICKS == 0 {
        for (entity, _, genome) in &healthy {
            if rng.gen::<f32>() < DISEASE_BACKGROUND_RATE * (1.0 - genome.disease_resistance) {
                commands.entity(entity).insert(Infection {
                    severity: rng.gen_range(DISEASE_BACKGROUND_SEVERITY),
                    ticks_remaining: rng.gen_range(DISEASE_BACKGROUND_DURATION_TICKS),
                });
            }
        }
    }

    // 2. Proximity transmission — spreads from infected to nearby healthy.
    for (entity, healthy_pos, genome) in &healthy {
        let nearby = spatial_hash.query_radius(healthy_pos.0, DISEASE_TRANSMISSION_RANGE);

        let mut infection_pressure = 0.0f32;
        let mut best_severity = 0.0f32;
        let mut best_remaining = 0u32;

        for &sick_entity in &nearby {
            if sick_entity == entity { continue; }
            if let Ok((sick_pos, sick_inf)) = infected.get(sick_entity) {
                let dist = (sick_pos.0 - healthy_pos.0).length();
                if dist < DISEASE_TRANSMISSION_RANGE {
                    // Closer + more severe = more pressure
                    let prox = 1.0 - (dist / DISEASE_TRANSMISSION_RANGE);
                    infection_pressure += sick_inf.severity * prox;
                    if sick_inf.severity > best_severity {
                        best_severity = sick_inf.severity;
                        best_remaining = sick_inf.ticks_remaining;
                    }
                }
            }
        }

        if infection_pressure <= 0.001 { continue; }

        // Per-tick infection chance, reduced by resistance, capped.
        let chance = (infection_pressure * DISEASE_TRANSMISSION_RATE * (1.0 - genome.disease_resistance))
            .min(DISEASE_TRANSMISSION_CHANCE_CAP);
        if rng.gen::<f32>() < chance {
            // Inherit roughly the strain's severity & duration, slightly weakened.
            commands.entity(entity).insert(Infection {
                severity: (best_severity * DISEASE_TRANSMISSION_SEVERITY_DECAY).clamp(0.1, 1.0),
                ticks_remaining: (best_remaining * 8 / 10).max(DISEASE_TRANSMISSION_MIN_DURATION_TICKS),
            });
        }
    }
}

/// Apply per-tick disease effects: energy drain, direct mortality chance,
/// tick down timer, remove when expired.
fn disease_effects_system(
    mut commands: Commands,
    mut infected: Query<(Entity, &mut Energy, &mut Infection, &Genome), With<Organism>>,
    config: Res<SimConfig>,
) {
    let mut rng = rand::thread_rng();
    for (entity, mut energy, mut infection, genome) in &mut infected {
        // Resistance cushions the drain; multiplier cranked above 1.0 so
        // photosynthesisers can't trivially out-absorb the cost from sunlight.
        let drain = config.base_metabolism_cost
            * infection.severity
            * (1.0 - genome.disease_resistance * 0.5)
            * DISEASE_DRAIN_MULTIPLIER;
        energy.0 -= drain;

        // Direct mortality chance per tick — ignores energy reserves so
        // photosynthesisers can't just sun-bathe through an infection.
        // Zero only energy (not health) so death_system attributes to Disease.
        let mortality = DISEASE_MORTALITY_RATE * infection.severity * (1.0 - genome.disease_resistance);
        if rng.gen::<f32>() < mortality {
            energy.0 = 0.0;
        }

        infection.ticks_remaining = infection.ticks_remaining.saturating_sub(1);
        if infection.ticks_remaining == 0 {
            commands.entity(entity).remove::<Infection>();
        }
    }
}

fn metabolism_system(
    config: Res<SimConfig>,
    mut organisms: Query<(&mut Energy, &mut Health, &mut Age, &BodySize, &Genome, &GroupSize), With<Organism>>,
) {
    for (mut energy, mut health, mut age, body_size, genome, group_size) in &mut organisms {
        age.0 += 1;

        // Body size costs quadratically — being big is VERY expensive
        let effective_size = body_size.0.max(0.5);
        let size_cost = effective_size * effective_size;
        let mut cost = config.base_metabolism_cost * size_cost * (1.0 + genome.speed_factor * 0.2);
        // Each body part has a maintenance cost scaled by body size
        cost += genome.body_segments.len() as f32 * 0.015 * effective_size;
        cost += genome.neurons.len() as f32 * 0.001;
        // Armor, claws, speed all cost quadratically
        let armor = genome.armor_value();
        cost += armor * armor * 0.05;
        let claws = genome.claw_power();
        cost += claws * claws * 0.03;
        cost += genome.speed_factor * genome.speed_factor * 0.015;

        // Group discount: reduced vigilance cost when near same-species.
        // Diminishing returns — most benefit from first few neighbours, caps at ~5%.
        // group_size.0 is count of same-species within sense range.
        let group_discount = 1.0 - (group_size.0 as f32 / (group_size.0 as f32 + 5.0)) * 0.05;
        cost *= group_discount;

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
    organisms: Query<(Entity, &Energy, &Health, &Position, &Age, Option<&Infection>), With<Organism>>,
    mut stats: ResMut<SimStats>,
    mut fitness: ResMut<FitnessTracker>,
) {
    for (entity, energy, health, pos, age, infection) in &organisms {
        if energy.0 <= 0.0 {
            // Determine cause of death — priority: predation > old age > disease > starvation
            let cause = if health.0 <= 0.0 {
                DeathCause::Predation
            } else if age.0 > 3000 {
                DeathCause::OldAge
            } else if infection.is_some() {
                DeathCause::Disease
            } else {
                DeathCause::Starvation
            };
            stats.deaths_by_cause[cause as usize] += 1;

            // Spawn death marker before despawning
            commands.spawn((
                DeathMarker {
                    timer: 0.5,
                    was_predated: health.0 <= 0.0,
                },
                Position(pos.0),
            ));

            commands.entity(entity).try_despawn_recursive();
            stats.total_deaths += 1;

            // Record lifespan for fitness tracking
            fitness.recent_lifespans.push(age.0);
            if fitness.recent_lifespans.len() > 200 {
                fitness.recent_lifespans.remove(0);
            }
            if !fitness.recent_lifespans.is_empty() {
                let sum: u64 = fitness.recent_lifespans.iter().sum();
                fitness.avg_lifespan = sum as f32 / fitness.recent_lifespans.len() as f32;
            }
        }
    }
}

fn reproduction_system(
    mut commands: Commands,
    config: Res<SimConfig>,
    mut innovation: ResMut<InnovationCounter>,
    spatial_hash: Res<SpatialHash>,
    mut organisms: Query<
        (Entity, &Position, &mut Energy, &mut ActionFlash, &Genome, &BrainOutput, &BodySize, &SpeciesId, &Generation),
        With<Organism>,
    >,
    mut stats: ResMut<SimStats>,
    bloom: Res<BloomEffects>,
) {
    let mut rng = rand::thread_rng();

    // Collect potential mate data upfront to avoid query conflicts
    let mate_candidates: Vec<(Entity, Vec2, f32, Genome, u64)> = organisms
        .iter()
        .filter(|(_, _, _, _, _, output, _, _, _)| output.reproduce > 0.5)
        .map(|(e, pos, energy, _, genome, _, _, species, _)| {
            (e, pos.0, energy.0, genome.clone(), species.0)
        })
        .collect();

    let mut new_organisms: Vec<(Vec2, Genome, u64, u32)> = Vec::new();
    let current_pop = organisms.iter().len();
    let max_pop = 2000usize;
    let mut already_mated: Vec<Entity> = Vec::new();

    for (entity, pos, mut energy, mut flash, genome, output, body_size, species, generation) in &mut organisms {
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
            flash.action = ActionType::Reproducing;
            flash.timer = 0.3;

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

            let effective_mutation_rate = config.mutation_rate * bloom.mutation_multiplier();
            child_genome.mutate(&mut innovation, &mut rng, effective_mutation_rate, config.mutation_strength);

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
            ActionFlash::default(),
            Signal::default(),
            GroupSize::default(),
            ParentInfo { parent_species_id: Some(parent_species) },
        )).insert((brain, child_genome, TrailHistory::default()));

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
    tick: Res<TickCounter>,
    mut organisms: Query<(Entity, &Genome, &mut SpeciesId), With<Organism>>,
    mut stats: ResMut<SimStats>,
    mut species_colors: ResMut<SpeciesColors>,
    mut phylo: ResMut<PhyloTree>,
    mut chronicle: ResMut<WorldChronicle>,
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

        // Hysteresis: prefer current species — only leave if nothing fits within threshold
        // but give current species a bonus (1.5x threshold to stay)
        let stay_threshold = config.species_compat_threshold * SPECIES_HYSTERESIS_FACTOR;

        for (species_id, rep_genome) in &species_reps {
            let dist = genome.compatibility_distance(rep_genome);
            let effective_threshold = if *species_id == *_old_species {
                stay_threshold // easier to stay in current species
            } else {
                config.species_compat_threshold
            };
            if dist < effective_threshold && dist < best_dist {
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
            let color = species_colors.get_or_create(new_id);

            // Record new species in phylogenetic tree
            let strategy = if genome.photosynthesis_rate > 0.2 && genome.has_photo_surface() {
                SpeciesStrategy::Photosynthesizer
            } else if genome.claw_power() > 0.5 {
                SpeciesStrategy::Predator
            } else {
                SpeciesStrategy::Forager
            };
            // Parent is the old species this organism was classified as
            let parent = if *_old_species > 0 { Some(*_old_species) } else { None };
            let traits = SpeciesTraits {
                strategy,
                aquatic: genome.aquatic_adaptation,
                body_size: genome.body_size,
                speed: genome.speed_factor,
                armor: genome.armor_value(),
                has_fins: genome.has_fins(),
                has_eyes: genome.eye_count() > 0,
                has_claws: genome.has_claws(),
                has_armor_plates: genome.has_armor(),
            };
            phylo.record_species(new_id, parent, tick.0, color, strategy, Some(&traits));

            let species_name = phylo.nodes.get(&new_id).map(|n| n.name.as_str()).unwrap_or("Unknown");
            let parent_str = if let Some(p) = parent {
                let parent_name = phylo.nodes.get(&p).map(|n| n.name.as_str()).unwrap_or("unknown");
                format!(" (from {})", parent_name)
            } else {
                String::new()
            };
            chronicle.log(tick.0, format!("New species: {}{}", species_name, parent_str));

            new_id
        };

        assignments.insert(*entity, assigned);
    }

    // Update population counts in phylo tree
    let mut species_counts: HashMap<u64, u32> = HashMap::new();
    for (_, _, _old_species) in &org_data {
        // Use assigned species, not old
    }
    for (_, assigned_id) in &assignments {
        *species_counts.entry(*assigned_id).or_insert(0) += 1;
    }
    // Detect extinctions before updating
    let previously_living: Vec<u64> = phylo.nodes.iter()
        .filter(|(_, n)| n.extinct_tick.is_none() && n.current_population > 0)
        .map(|(id, _)| *id)
        .collect();

    phylo.update_populations(&species_counts, tick.0);

    // Log extinctions
    for species_id in &previously_living {
        if let Some(node) = phylo.nodes.get(species_id) {
            if node.current_population == 0 && node.peak_population >= 10 {
                let age_secs = tick.0.saturating_sub(node.born_tick) / 30;
                chronicle.log(tick.0, format!(
                    "{} went extinct (peak: {}, lived {}s)",
                    node.name, node.peak_population, age_secs
                ));
            }
        }
    }

    for (entity, _genome, mut species_id) in &mut organisms {
        if let Some(&new_id) = assignments.get(&entity) {
            species_id.0 = new_id;
        }
    }

    stats.species_count = species_reps.len() as u32;

    // Detect convergent evolution — only log when lineage count increases
    let convergences = phylo.detect_convergence();
    for (strategy, lineage_count) in convergences {
        let strategy_name = match strategy {
            SpeciesStrategy::Photosynthesizer => "photosynthesis",
            SpeciesStrategy::Predator => "predation",
            SpeciesStrategy::Forager => "foraging",
        };
        // Only log if this is a new high for this strategy
        let already_logged = chronicle.entries.iter()
            .filter(|e| e.text.contains(&format!("{} lineages evolved {}", lineage_count, strategy_name)))
            .count();
        if already_logged == 0 {
            chronicle.log(tick.0, format!(
                "Convergent evolution! {} independent lineages evolved {}",
                lineage_count, strategy_name
            ));
        }
    }
}

fn record_population_history(
    time: Res<Time>,
    mut timer: ResMut<PopHistoryTimer>,
    stats: Res<SimStats>,
    organisms: Query<(&Genome, Option<&Infection>), With<Organism>>,
    food: Query<&Food>,
    mut history: ResMut<PopulationHistory>,
    fitness: Res<FitnessTracker>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let mut plants = 0u32;
    let mut predators = 0u32;
    let mut foragers = 0u32;
    let mut infected = 0u32;

    // Trait running sums for averaging
    let mut sum_resist = 0.0f32;
    let mut sum_body = 0.0f32;
    let mut sum_speed = 0.0f32;
    let mut sum_armor = 0.0f32;
    let mut sum_attack = 0.0f32;
    let mut sum_photo = 0.0f32;
    let mut n = 0u32;

    for (genome, inf) in &organisms {
        if genome.photosynthesis_rate > 0.2 && genome.has_photo_surface() {
            plants += 1;
        } else if genome.claw_power() > 0.5 {
            predators += 1;
        } else {
            foragers += 1;
        }
        if inf.is_some() {
            infected += 1;
        }
        sum_resist += genome.disease_resistance;
        sum_body += genome.body_size;
        sum_speed += genome.speed_factor;
        sum_armor += genome.armor_value();
        sum_attack += genome.claw_power();
        sum_photo += genome.photosynthesis_rate;
        n += 1;
    }

    let div = n.max(1) as f32;
    let org_count = plants + predators + foragers;
    let food_count = food.iter().len() as u32;

    history.record(&stats, PopSnapshotInput {
        organisms: org_count,
        food: food_count,
        plants,
        predators,
        foragers,
        avg_lifespan: fitness.avg_lifespan,
        infected,
        avg_disease_resistance: sum_resist / div,
        avg_body_size: sum_body / div,
        avg_speed: sum_speed / div,
        avg_armor: sum_armor / div,
        avg_attack: sum_attack / div,
        avg_photo: sum_photo / div,
    });
}

/// Sample each organism's position into its trail ring buffer.
/// Runs every 3 ticks — 20 samples × 3 ticks ≈ 2 seconds of trail at 30hz.
fn record_trail_history(
    tick: Res<TickCounter>,
    trails_visible: Res<TrailsVisible>,
    mut organisms: Query<(&Position, &mut TrailHistory), With<Organism>>,
) {
    // Skip if trails are off — save the writes and keep deques empty
    if !trails_visible.0 {
        return;
    }
    if tick.0 % 3 != 0 {
        return;
    }
    for (pos, mut trail) in &mut organisms {
        trail.push(pos.0);
    }
}

pub fn spawn_initial_population(
    commands: &mut Commands,
    config: &SimConfig,
    innovation: &mut InnovationCounter,
    rng: &mut impl Rng,
) {
    let photo_count = config.initial_population / 3; // 30% photosynthesizers

    for i in 0..config.initial_population {
        let x = rng.gen_range(0.0..config.world_width as f32);
        let y = rng.gen_range(0.0..config.world_height as f32);

        let genome = if i < photo_count {
            Genome::new_photosynthesizer(innovation, rng)
        } else {
            Genome::new_minimal(innovation, rng)
        };

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
            ActionFlash::default(),
            Signal::default(),
            GroupSize::default(),
            ParentInfo::default(),
        )).insert((brain, genome, TrailHistory::default()));
    }
}

/// F5 saves the world to the session directory
fn save_system(
    mut events: EventReader<WorldEventRequest>,
    session: Res<Session>,
    tick: Res<TickCounter>,
    season: Res<Season>,
    stats: Res<SimStats>,
    config: Res<SimConfig>,
    innovation: Res<InnovationCounter>,
    organisms: Query<(&Position, &Energy, &Health, &Age, &Generation, &SpeciesId, &Signal, &BrainMemory, &Genome), With<Organism>>,
    food: Query<(&Position, &FoodEnergy), With<Food>>,
    phylo: Res<PhyloTree>,
    chronicle: Res<WorldChronicle>,
) {
    let save_requested = events.read().any(|r| matches!(r, WorldEventRequest::Save));
    if !save_requested {
        return;
    }

    let org_data: Vec<_> = organisms.iter()
        .map(|(pos, energy, health, age, gen, species, signal, memory, genome)| {
            (pos.0, energy.0, health.0, age.0, gen.0, species.0, signal.0, memory.0, genome.clone())
        })
        .collect();

    let food_data: Vec<_> = food.iter()
        .map(|(pos, fe)| (pos.0, fe.0))
        .collect();

    let save_path = session.dir.join("save.json");
    save::save_world(&save_path, &tick, &season, &stats, &innovation, &config, &org_data, &food_data, &phylo, &chronicle);
}
