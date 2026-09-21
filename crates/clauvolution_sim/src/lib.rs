// Bevy systems have complex Query signatures by design — clippy's
// type_complexity and too_many_arguments lints misfire constantly here.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

pub mod save;

use bevy::prelude::*;
use clauvolution_brain::Brain;
use clauvolution_core::*;
use clauvolution_genome::{Genome, InnovationCounter, NUM_INPUTS, NUM_MEMORY};
/// Re-exported so callers that classified through this crate keep working.
pub use clauvolution_phylogeny::classify_strategy;
use clauvolution_phylogeny::{PhyloTree, SpeciesStrategy, SpeciesTraits, WorldChronicle};
use clauvolution_world::{
    food_regeneration_system, tile_dynamics_system, update_spatial_hash, SpatialHash, TerrainType,
    TileMap,
};
use rand::Rng;
use std::collections::{HashMap, HashSet};

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

/// Half-width, in tiles, of the square window over which photosynthesisers
/// share light: a 9 by 9 window at 4. A plant's own footprint is under a
/// tile, so this is a statement about how far light competition reaches.
/// Swept once with `SimConfig::leaf_capacity_per_tile` and then left alone;
/// see `docs/DECISIONS.md`, "Canopy light sharing".
const CANOPY_RADIUS: usize = 4;

/// Raw multiplier on photosynthesis yield. Scales how much energy the sun
/// gives. Lowering this makes photosynthesis less free-lunch and evens the
/// scales with foraging/predation.
/// Tuning history:
///   2.0 — initial, plants dominate 90%+
///   1.0 — 50% cut, plants still 91% but more starvation
///   0.7 — 65% cut, back to 92% plants — too lenient
///   0.5 — 75% cut: broke monoculture (~72/26/1% split) at the time.
///   The values above were measured while killed photosynthesisers kept
///   photosynthesising and could be killed again (see the `Killed` marker in
///   DECISIONS.md), so they describe a different sim. With kills final:
///   0.5 — plants extinct by 5000 ticks on one of four seeds, 29 left on seed 42
///   0.75 — plants survive everywhere; predators 3-32
///   1.0 — (current) plants survive everywhere. Seed 42 at 5000 ticks over
///         five runs: plants 1165-1468, foragers 530-825, predators 2-49.
///         Same-seed runs are not reproducible at this length, so single
///         runs carry that spread. Seeds 1 and 2 still drift to plant
///         monoculture and predators fade slowly on every seed; that is an
///         attractor question for the design doc, not this constant.
const PHOTO_OUTPUT_MULTIPLIER: f32 = 1.0;

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

/// Max distance at which two organisms are considered "in contact" for
/// the purposes of forming a symbiotic link.
/// Tuning history:
///   6.0  — initial. 3–35 pairs across seeds, rate near 0.
///   12.0 — wider made it WORSE (3–4 pairs): more candidates compete
///          for "nearest" status, streak resets when rankings swap.
///   6.0  — (current) reverted. Narrow range + loose streak works
///          better than wide range + strict streak.
const SYMBIOSIS_RANGE: f32 = 6.0;

/// Per-tick energy a partner gives up at `symbiosis_rate = 1.0`. Negative
/// rates drain from the partner at the same scale.
/// Tuning history:
///   0.05 — initial. Too weak: at full |rate|, an organism loses/gains
///          1–2 energy over a typical 30-tick link lifetime — noise vs
///          the ~50-unit energy pool, so selection couldn't distinguish.
///   0.15 — tripled; pair count jumped with short threshold but avg
///          rate still indistinguishable from drift (+0.03 to +0.12).
///   0.30 — doubled further, no stronger selection signal — something
///          other than transfer magnitude is keeping rate near zero.
///   0.15 — (current) reverted. No point in the extra magnitude.
const SYMBIOSIS_TRANSFER_RATE: f32 = 0.15;

/// Tile column and row for a world position, clamped into the map.
fn tile_index(pos: Vec2, w: usize, h: usize) -> (usize, usize) {
    let tx = (pos.x.max(0.0) as usize).min(w - 1);
    let ty = (pos.y.max(0.0) as usize).min(h - 1);
    (tx, ty)
}

/// Summed-area table of a `w` by `h` grid: `(w + 1) * (h + 1)` entries where
/// `sat[(y + 1) * (w + 1) + (x + 1)]` is the sum of every cell with column
/// `<= x` and row `<= y`. Any rectangle's total is then four lookups.
pub fn summed_area_table(grid: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut sat = Vec::new();
    summed_area_table_into(grid, w, h, &mut sat);
    sat
}

/// `summed_area_table` into a caller-owned buffer, resized as needed, so a
/// per-tick caller allocates nothing once warm.
pub fn summed_area_table_into(grid: &[f32], w: usize, h: usize, sat: &mut Vec<f32>) {
    let stride = w + 1;
    sat.clear();
    sat.resize(stride * (h + 1), 0.0);
    for y in 0..h {
        let mut row = 0.0f32;
        for x in 0..w {
            row += grid[y * w + x];
            sat[(y + 1) * stride + (x + 1)] = sat[y * stride + (x + 1)] + row;
        }
    }
}

/// Total over the square window of half-width `radius` around `(cx, cy)`,
/// clipped to the grid, and the number of tiles the clipped window covers.
pub fn window_sum(
    sat: &[f32],
    w: usize,
    h: usize,
    cx: usize,
    cy: usize,
    radius: usize,
) -> (f32, usize) {
    let stride = w + 1;
    let x0 = cx.saturating_sub(radius);
    let y0 = cy.saturating_sub(radius);
    let x1 = (cx + radius + 1).min(w);
    let y1 = (cy + radius + 1).min(h);
    let total = sat[y1 * stride + x1] - sat[y0 * stride + x1] - sat[y1 * stride + x0]
        + sat[y0 * stride + x0];
    (total.max(0.0), (x1 - x0) * (y1 - y0))
}

/// Share of full light for a photosynthesiser whose window holds
/// `leaf_in_window` leaf area over `tiles` tiles that can each fully light
/// `capacity_per_tile` of leaf: `min(1, tiles * capacity / leaf)`. Full light
/// until the leaves exceed what the ground can light, then proportional.
pub fn canopy_light_share(leaf_in_window: f32, tiles: usize, capacity_per_tile: f32) -> f32 {
    let capacity = tiles as f32 * capacity_per_tile.max(0.0);
    if leaf_in_window <= capacity || leaf_in_window <= 0.0 {
        1.0
    } else {
        capacity / leaf_in_window
    }
}

/// Speed multiplier from photosynthetic surface area: `1 / (1 + area × drag)`,
/// the same shape as armour drag. `SimConfig::photo_drag` is the coefficient.
pub fn photo_drag_factor(photo_area: f32, drag: f32) -> f32 {
    1.0 / (1.0 + photo_area.max(0.0) * drag.max(0.0))
}

/// Split a meal into the share the eater keeps and the share lost to
/// digestion, from a digestion efficiency in 0..1 (`Genome::plant_efficiency`
/// or `animal_efficiency`). `kept + wasted == gross`.
pub fn digest(gross: f32, efficiency: f32) -> (f32, f32) {
    let kept = gross * efficiency.clamp(0.0, 1.0);
    (kept, gross - kept)
}

/// Credit `amount` to `energy`, capped at `cap`, and return the part the cap
/// discarded so the caller can book it under `EnergyFlows::clamp`. Every
/// `.min(max_organism_energy)` on income goes through here so the ledger
/// sees what the clamp destroys.
fn credit_clamped(energy: &mut Energy, amount: f32, cap: f32) -> f32 {
    let unclamped = energy.0 + amount;
    let clamped = unclamped.min(cap);
    energy.0 = clamped;
    unclamped - clamped
}

pub struct SimPlugin;

/// The simulation tick: every `FixedUpdate` system that reads or writes
/// simulation state runs inside this set, in one fixed chain. Anything else
/// scheduled in `FixedUpdate` (the headless tick counter, for example) orders
/// itself `.after(SimTick)` so it sees a complete tick. Bevy's executor is
/// otherwise free to run an unordered system anywhere in the tick that its
/// data access allows, and where that lands can vary from run to run.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SimTick;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                sim_speed_system,
                keyboard_to_events_system,
                mass_extinction_input_system,
                save_system,
            )
                .chain(),
        )
        // One strict chain. Bevy implements the system-tuple traits up to
        // twenty elements, so the chain is written as two chained groups.
        .add_systems(
            FixedUpdate,
            (
                (
                    tick_counter_system,
                    tile_dynamics_system,
                    food_regeneration_system,
                    update_spatial_hash,
                    update_food_snapshot,
                    sensing_and_brain_system,
                    action_system,
                    predation_system,
                    photosynthesis_system,
                    niche_construction_system,
                )
                    .chain(),
                (
                    disease_transmission_system,
                    disease_effects_system,
                    symbiosis_tracking_system,
                    symbiosis_transfer_system,
                    metabolism_system,
                    death_system,
                    reproduction_system,
                    ledger_system,
                    species_classification_system,
                    record_population_history,
                    record_trail_history,
                )
                    .chain(),
            )
                .chain()
                .in_set(SimTick),
        )
        .insert_resource(Time::<Fixed>::from_hz(30.0))
        .insert_resource(Time::<Virtual>::from_max_delta(
            std::time::Duration::from_millis(100),
        ))
        .insert_resource(SpeciesClassificationTimer(Timer::from_seconds(
            SPECIES_CLASSIFICATION_PERIOD_SECS,
            TimerMode::Repeating,
        )))
        .insert_resource(ExtinctionCooldown(Timer::from_seconds(
            WORLD_EVENT_COOLDOWN_SECS,
            TimerMode::Once,
        )))
        .insert_resource(PopHistoryTimer(Timer::from_seconds(
            POP_HISTORY_SAMPLE_SECS,
            TimerMode::Repeating,
        )))
        .init_resource::<CanopyGrid>()
        .init_resource::<ConvergenceHighWater>();
    }
}

#[derive(Resource)]
struct SpeciesClassificationTimer(Timer);

#[derive(Resource)]
struct PopHistoryTimer(Timer);

/// Scratch buffers for canopy light sharing, kept between ticks so
/// `photosynthesis_system` allocates nothing per tick: leaf area per tile and
/// its summed-area table (`(w + 1) * (h + 1)` entries).
#[derive(Resource, Default)]
struct CanopyGrid {
    leaf: Vec<f32>,
    sat: Vec<f32>,
}

impl CanopyGrid {
    /// Zero the leaf grid for a `w` by `h` map, resizing on first use.
    fn reset(&mut self, w: usize, h: usize) {
        self.leaf.clear();
        self.leaf.resize(w * h, 0.0);
    }
}

#[derive(Resource)]
struct ExtinctionCooldown(Timer);

/// Highest independent-lineage count already chronicled per strategy.
///
/// `species_classification_system` re-detects convergent evolution every
/// pass, so without this the same "N independent lineages evolved X" line
/// would be logged every five seconds. The map starts empty on every run,
/// including a run restored from a save, so the first pass after a load
/// restates the current convergence state once and then goes quiet.
#[derive(Resource, Default)]
struct ConvergenceHighWater(HashMap<SpeciesStrategy, usize>);

impl ConvergenceHighWater {
    /// Records `lineage_count` for `strategy` and returns whether it is a new
    /// high worth chronicling. Equal or lower counts return false and leave
    /// the recorded high untouched.
    fn record(&mut self, strategy: SpeciesStrategy, lineage_count: usize) -> bool {
        let high = self.0.entry(strategy).or_insert(0);
        if lineage_count > *high {
            *high = lineage_count;
            true
        } else {
            false
        }
    }
}

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

fn tick_counter_system(
    mut tick: ResMut<TickCounter>,
    mut season: ResMut<Season>,
    mut chronicle: ResMut<WorldChronicle>,
    session: Res<Session>,
    mut bloom: ResMut<BloomEffects>,
) {
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

/// Apply `SimSpeed` to the virtual clock. This is the only place the sim
/// touches `Time<Virtual>`: the GUI speed keys and headless `--speed` both
/// write `SimSpeed.multiplier`, and the multiplier scales virtual time while
/// the fixed timestep stays at 30 Hz. One tick therefore always means 1/30 s
/// of sim time, and every virtual-time timer (species classification,
/// history sampling, seasons) fires after the same number of ticks at every
/// speed in both modes. Relative speed and pause are independent fields on
/// the clock, so the multiplier survives a pause/unpause cycle.
fn sim_speed_system(speed: Res<SimSpeed>, mut virtual_time: ResMut<Time<Virtual>>) {
    if !speed.is_changed() {
        return;
    }
    virtual_time.set_relative_speed(speed.multiplier);
    if speed.paused {
        virtual_time.pause();
    } else {
        virtual_time.unpause();
    }
}

/// Translate keyboard hotkeys into WorldEventRequest events
fn keyboard_to_events_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<WorldEventRequest>,
) {
    if keys.just_pressed(KeyCode::KeyX) {
        events.send(WorldEventRequest::Asteroid);
    }
    if keys.just_pressed(KeyCode::KeyI) {
        events.send(WorldEventRequest::IceAge);
    }
    if keys.just_pressed(KeyCode::KeyV) {
        events.send(WorldEventRequest::Volcano);
    }
    if keys.just_pressed(KeyCode::KeyB) {
        events.send(WorldEventRequest::SolarBloom);
    }
    if keys.just_pressed(KeyCode::KeyN) {
        events.send(WorldEventRequest::NutrientRain);
    }
    if keys.just_pressed(KeyCode::KeyJ) {
        events.send(WorldEventRequest::CambrianSpark);
    }
    if keys.just_pressed(KeyCode::F5) {
        events.send(WorldEventRequest::Save);
    }
}

/// Process WorldEventRequest events — fired by keyboard, UI buttons, etc.
fn mass_extinction_input_system(
    mut requests: EventReader<WorldEventRequest>,
    mut commands: Commands,
    mut cooldown: ResMut<ExtinctionCooldown>,
    time: Res<Time>,
    organisms: Query<(Entity, &Position, &Energy, &EnergyFlows), With<Organism>>,
    mut tile_map: Option<ResMut<TileMap>>,
    mut stats: ResMut<SimStats>,
    tick: Res<TickCounter>,
    mut chronicle: ResMut<WorldChronicle>,
    config: Res<SimConfig>,
    mut bloom: ResMut<BloomEffects>,
    mut sim_rng: ResMut<SimRng>,
    mut ledger: ResMut<EnergyLedger>,
) {
    cooldown.0.tick(time.delta());

    // Find the first ext/bloom event this frame (skip Save — handled elsewhere).
    // If cooldown is active, ignore ext/bloom events.
    let req = requests
        .read()
        .find(|r| !matches!(r, WorldEventRequest::Save))
        .copied();

    if !cooldown.0.finished() {
        return;
    }

    let Some(req) = req else { return };

    let mut triggered = false;
    let rng = &mut sim_rng.0;

    // Asteroid (kill 70% randomly)
    if matches!(req, WorldEventRequest::Asteroid) {
        info!("MASS EXTINCTION: Asteroid impact!");
        let mut killed = 0u32;
        for (entity, pos, energy, flows) in &organisms {
            if rng.gen::<f32>() < 0.7 {
                commands.spawn((
                    DeathMarker {
                        timer: 0.5,
                        was_predated: false,
                    },
                    Position(pos.0),
                ));
                commands.entity(entity).try_despawn_recursive();
                // This runs in Update, between ticks, so the per-organism
                // flows are already zeroed; folding them in anyway keeps the
                // books right if that ever changes.
                ledger.tick.add(flows);
                ledger.tick.death += energy.0 as f64;
                killed += 1;
            }
        }
        stats.total_deaths += killed as u64;
        stats.deaths_by_cause[DeathCause::Event as usize] += killed as u64;
        chronicle.log(
            tick.0,
            format!("ASTEROID IMPACT! {} organisms killed", killed),
        );
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
            chronicle.log(
                tick.0,
                "ICE AGE! Temperature halved, moisture reduced".to_string(),
            );
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
        for (entity, pos, energy, flows) in &organisms {
            let dist = ((pos.0.x - center_x).powi(2) + (pos.0.y - center_y).powi(2)).sqrt();
            if dist < radius {
                commands.spawn((
                    DeathMarker {
                        timer: 0.5,
                        was_predated: false,
                    },
                    Position(pos.0),
                ));
                commands.entity(entity).try_despawn_recursive();
                ledger.tick.add(flows);
                ledger.tick.death += energy.0 as f64;
                killed += 1;
            }
        }
        stats.total_deaths += killed as u64;
        stats.deaths_by_cause[DeathCause::Event as usize] += killed as u64;

        // Boost nutrients in affected area
        if let Some(ref mut tm) = tile_map {
            for y in 0..tm.height {
                for x in 0..tm.width {
                    let dist =
                        ((x as f32 - center_x).powi(2) + (y as f32 - center_y).powi(2)).sqrt();
                    if dist < radius {
                        let tile = tm.get_mut(x, y);
                        tile.nutrients = (tile.nutrients + 0.5).min(1.0);
                    }
                }
            }
        }
        chronicle.log(
            tick.0,
            format!(
                "VOLCANIC ERUPTION! {} organisms killed near ({:.0}, {:.0})",
                killed, center_x, center_y
            ),
        );
        triggered = true;
    }

    // Solar bloom (boost light for a fixed duration)
    if matches!(req, WorldEventRequest::SolarBloom) {
        info!("BLOOM: Solar bloom!");
        bloom.solar_bloom = SOLAR_BLOOM_LIGHT_MULTIPLIER;
        bloom.solar_ticks = BLOOM_DURATION_TICKS;
        chronicle.log(
            tick.0,
            "SOLAR BLOOM! Light doubled — photosynthesizers surge".to_string(),
        );
        triggered = true;
    }

    // Nutrient rain (massive food burst)
    if matches!(req, WorldEventRequest::NutrientRain) {
        info!("BLOOM: Nutrient rain!");
        let food_count =
            (config.world_width as f32 * config.world_height as f32 * NUTRIENT_RAIN_DENSITY) as u32;
        for _ in 0..food_count {
            let x = rng.gen_range(0.0..config.world_width as f32);
            let y = rng.gen_range(0.0..config.world_height as f32);
            commands.spawn((
                Food,
                FoodEnergy(config.food_energy_value),
                Position(Vec2::new(x, y)),
            ));
        }
        chronicle.log(
            tick.0,
            format!(
                "NUTRIENT RAIN! {} food spawned across the world",
                food_count
            ),
        );
        triggered = true;
    }

    // Cambrian spark (boost mutation rate for a fixed duration)
    if matches!(req, WorldEventRequest::CambrianSpark) {
        info!("BLOOM: Cambrian spark!");
        bloom.mutation_boost = CAMBRIAN_MUTATION_MULTIPLIER;
        bloom.mutation_ticks = BLOOM_DURATION_TICKS;
        chronicle.log(
            tick.0,
            "CAMBRIAN SPARK! Mutation rate tripled — rapid speciation".to_string(),
        );
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
    snapshot
        .entries
        .extend(food_query.iter().map(|(e, p, fe)| (e, p.0, fe.0)));
}

fn sensing_and_brain_system(
    config: Res<SimConfig>,
    spatial_hash: Res<SpatialHash>,
    tile_map: Res<TileMap>,
    mut organisms: Query<
        (
            Entity,
            &Position,
            &Energy,
            &Health,
            &Genome,
            &Brain,
            &BodySize,
            &SpeciesId,
            &BrainMemory,
            &mut BrainOutput,
            &mut GroupSize,
            &mut BrainActivations,
        ),
        With<Organism>,
    >,
    food_snapshot: Res<FoodSnapshot>,
    all_org_data: Query<
        (&Position, &BodySize, &SpeciesId, &Genome, &Signal),
        (With<Organism>, Without<Food>),
    >,
) {
    // Parallelised across organisms. Each iteration only reads from shared
    // Res/Query (spatial_hash, food_snapshot, all_org_data — all Sync) and
    // writes to its own BrainOutput + GroupSize + BrainActivations via the
    // per-iter Mut guard, so there's no cross-organism data dependency.
    //
    // Brain eval dominates the per-tick cost at 2000 organisms. Rayon
    // (via Bevy's TaskPool) spreads it across cores.
    organisms.par_iter_mut().for_each(|(entity, pos, energy, health, genome, brain, body_size, species_id, memory, mut output, mut group_size, mut activations)| {
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
                // Nothing has moved since `update_spatial_hash` ran this tick, so every
                // returned entity must still fall inside the queried cell range.
                debug_assert!({
                    let range_cells = (sense_range / spatial_hash.cell_size).ceil() as i32 + 1;
                    let (cx, cy) = spatial_hash.cell_key(pos.0);
                    let (kx, ky) = spatial_hash.cell_key(other_pos.0);
                    (kx - cx).abs() <= range_cells && (ky - cy).abs() <= range_cells
                }, "spatial hash returned an entity outside the queried cells; is update_spatial_hash still ordered before sensing?");
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

        let (brain_out, trace) = brain.evaluate_trace(&inputs);
        output.move_x = brain_out[0];
        output.move_y = brain_out[1];
        output.eat = brain_out[2];
        output.reproduce = brain_out[3];
        output.attack = brain_out[4];
        output.signal = brain_out[5];
        output.memory_out = [brain_out[6], brain_out[7], brain_out[8]];
        activations.values = trace;
    });
}

fn action_system(
    config: Res<SimConfig>,
    tile_map: Res<TileMap>,
    mut organisms: Query<
        (
            &mut Position,
            &mut Energy,
            &mut BrainMemory,
            &mut ActionFlash,
            &mut Signal,
            &mut Velocity,
            &BrainOutput,
            &Genome,
            &BodySize,
        ),
        (With<Organism>, Without<Food>),
    >,
    food_snapshot: Res<FoodSnapshot>,
    mut commands: Commands,
    mut ledger: ResMut<EnergyLedger>,
) {
    let foods = &food_snapshot.entries;

    let mut eaten_food: Vec<Entity> = Vec::new();

    for (
        mut pos,
        mut energy,
        mut memory,
        mut flash,
        mut signal,
        mut velocity,
        output,
        genome,
        body_size,
    ) in &mut organisms
    {
        // Tick down flash timer
        flash.timer = (flash.timer - 0.033).max(0.0);
        if flash.timer <= 0.0 {
            flash.action = ActionType::None;
        }
        // Update memory
        memory.0 = output.memory_out;
        signal.0 = output.signal.clamp(-1.0, 1.0);

        let move_dir = Vec2::new(output.move_x, output.move_y);
        // Armor slows you down — heavy organisms are slower
        let armor_drag = 1.0 / (1.0 + genome.armor_value() * 0.3);
        // So does a light-catching surface: broad and flat, it is a sail.
        // Nothing forbids a photosynthesiser from moving; a leafy one is
        // slow and a small-leaved one is not. See DECISIONS.md.
        let photo_drag = photo_drag_factor(genome.total_photo_surface_area(), config.photo_drag);
        let speed = genome.speed_factor * 2.0 / body_size.0.sqrt() * armor_drag * photo_drag;
        let movement = move_dir * speed;
        // Recorded so the history can average movement by strategy.
        velocity.0 = movement;

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

        let move_cost =
            movement.length() * config.movement_energy_cost * body_size.0 * terrain_cost;
        energy.0 -= move_cost;
        ledger.tick.movement += move_cost as f64;

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
                    // Food items are plant tissue. The undigested share is not
                    // organism energy (nor is the part a weak mouth leaves), so
                    // only what the eater keeps enters the ledger.
                    let (gained, _wasted) =
                        digest(food_energy * mouth_bonus, genome.plant_efficiency());
                    ledger.tick.clamp +=
                        credit_clamped(&mut energy, gained, config.max_organism_energy) as f64;
                    ledger.tick.food += gained as f64;
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
        (
            Entity,
            &Position,
            &mut Energy,
            &mut Health,
            &mut ActionFlash,
            &Genome,
            &BodySize,
            &BrainOutput,
        ),
        With<Organism>,
    >,
    mut commands: Commands,
    mut predation_stats: ResMut<PredationStats>,
    mut ledger: ResMut<EnergyLedger>,
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
    predation_stats.attacks_attempted += attackers.len() as u64;

    // (killer, victim, victim_energy) — energy transfer computed at kill time.
    // A victim is claimed at most once per tick: the first attacker to land a
    // kill takes the energy transfer, and later attackers skip that target and
    // keep scanning. Without this, several attackers could each be paid 10% of
    // the same victim's energy and each count a kill. See DECISIONS.md.
    let mut kills: Vec<(Entity, Entity, f32)> = Vec::new();
    // (grazer, plant, bite) — an attack on a photosynthesiser is a graze. The
    // same claim rule applies, so a plant takes one bite per tick.
    let mut grazes: Vec<(Entity, Entity, f32)> = Vec::new();
    let mut claimed_victims: HashSet<Entity> = HashSet::new();

    for (attacker_entity, attacker_pos, attack_str, attack_range, attacker_size) in &attackers {
        let nearby = spatial_hash.query_radius(*attacker_pos, *attack_range);

        for &target_entity in &nearby {
            if target_entity == *attacker_entity || claimed_victims.contains(&target_entity) {
                continue;
            }

            if let Ok((
                _,
                target_pos,
                target_energy,
                target_health,
                _,
                target_genome,
                target_body_size,
                _,
            )) = organisms.get(target_entity)
            {
                // A target at zero health is already dead (killed earlier this
                // tick, or dying of old age) and is not prey.
                if target_health.0 <= 0.0 {
                    continue;
                }
                let dist = (target_pos.0 - *attacker_pos).length();
                if dist > *attack_range {
                    continue;
                }

                predation_stats.targets_considered += 1;

                let defense = target_genome.armor_value() * target_body_size.0;
                let damage = (attack_str - defense * 0.5).max(0.0);
                let is_plant = target_genome.is_photosynthesiser();
                // A graze passes only the damage gate: a small grazer can bite
                // a large plant, and plant armour still defends against it.
                let size_ok = is_plant || *attacker_size > target_body_size.0 * 0.6;
                let damage_ok = damage > 0.1;

                if !size_ok {
                    predation_stats.rejected_size_gate += 1;
                }
                if !damage_ok {
                    predation_stats.rejected_damage += 1;
                }

                if damage_ok && size_ok {
                    if is_plant {
                        let bite = target_energy.0.max(0.0) * config.bite_fraction;
                        grazes.push((*attacker_entity, target_entity, bite));
                    } else {
                        kills.push((*attacker_entity, target_entity, target_energy.0));
                    }
                    claimed_victims.insert(target_entity);
                    break;
                }
            }
        }
    }

    predation_stats.kills += kills.len() as u64;
    predation_stats.grazes += grazes.len() as u64;

    for (grazer, plant, bite) in grazes {
        let Ok((_, _, _, _, _, grazer_genome, _, _)) = organisms.get(grazer) else {
            continue;
        };
        let (kept, wasted) = digest(bite, grazer_genome.plant_efficiency());
        let Ok((_, _, mut plant_energy, _, _, _, _, _)) = organisms.get_mut(plant) else {
            continue;
        };
        // The bite leaves the plant whole or not; the plant keeps its health
        // and is not marked Killed. If the bite empties it, death_system
        // reads that as any other energy loss.
        plant_energy.0 -= bite;
        if let Ok((_, _, mut grazer_energy, _, mut grazer_flash, _, _, _)) =
            organisms.get_mut(grazer)
        {
            ledger.tick.clamp +=
                credit_clamped(&mut grazer_energy, kept, config.max_organism_energy) as f64;
            ledger.tick.grazing += kept as f64;
            ledger.tick.digestion += wasted as f64;
            grazer_flash.action = ActionType::Grazing;
            grazer_flash.timer = 0.3;
        }
    }

    for (killer, victim, victim_energy_before) in kills {
        let Ok((_, _, _, _, _, killer_genome, _, _)) = organisms.get(killer) else {
            continue;
        };
        // Energy pyramid: the killer is offered a fixed share of the prey's
        // stored energy (most is lost as heat) and keeps what its diet lets
        // it digest. The undigested share is booked to digestion and the
        // rest of the victim's energy to death.
        let (energy_gained, wasted) = digest(
            victim_energy_before * config.kill_transfer_fraction,
            killer_genome.animal_efficiency() * config.animal_efficiency_multiplier,
        );
        if let Ok((_, _, mut killer_energy, _, mut killer_flash, _, _, _)) =
            organisms.get_mut(killer)
        {
            ledger.tick.clamp += credit_clamped(
                &mut killer_energy,
                energy_gained,
                config.max_organism_energy,
            ) as f64;
            ledger.tick.predation += energy_gained as f64;
            killer_flash.action = ActionType::Attacking;
            killer_flash.timer = 0.3;
        }
        if let Ok((_, _, mut victim_energy, mut victim_health, _, _, _, _)) =
            organisms.get_mut(victim)
        {
            // Whatever the victim still holds leaves the world here; the
            // killer's share was booked above as a separate flow, and the
            // undigested part of that share as digestion.
            ledger.tick.death += (victim_energy.0 - wasted) as f64;
            ledger.tick.digestion += wasted as f64;
            victim_energy.0 = 0.0;
            victim_health.0 = 0.0;
            // The marker is what makes the kill final: the systems between here
            // and death_system skip it, and death_system despawns it with this
            // cause. Zeroing energy and health alone let a photosynthesiser
            // refill in the same tick and survive at zero health to be killed
            // and paid for again (see DECISIONS.md).
            commands
                .entity(victim)
                .insert(Killed(DeathCause::Predation));
        }
    }
}

fn photosynthesis_system(
    tile_map: Res<TileMap>,
    mut organisms: Query<
        (
            &Position,
            &mut Energy,
            &mut EnergyFlows,
            &mut LightShare,
            &Genome,
        ),
        (With<Organism>, Without<Killed>),
    >,
    config: Res<SimConfig>,
    season: Res<Season>,
    bloom: Res<BloomEffects>,
    mut canopy: ResMut<CanopyGrid>,
) {
    let light_mult = season.light_multiplier() * bloom.light_multiplier();

    // Light is shared over a canopy. First pass: leaf area per tile, then a
    // summed-area table so any window's total is four lookups. Every
    // organism that earns any sun also shades (`can_photosynthesise`).
    let w = tile_map.width as usize;
    let h = tile_map.height as usize;
    canopy.reset(w, h);
    let CanopyGrid { leaf, sat } = &mut *canopy;
    for (pos, _, _, _, genome) in organisms.iter() {
        if genome.can_photosynthesise() {
            let (tx, ty) = tile_index(pos.0, w, h);
            leaf[ty * w + tx] += genome.total_photo_surface_area();
        }
    }
    summed_area_table_into(leaf, w, h, sat);
    let sat: &[f32] = sat;

    // Second pass: each photosynthesiser's light share is what its window's
    // tiles can light divided by the leaf area in the window, capped at 1,
    // so where leaves exceed the ground everyone there is shaded in
    // proportion. Parallelised: the table is read-only here and each
    // organism writes only its own components; the shared EnergyLedger is
    // never touched from here. Safe for par_iter_mut.
    organisms
        .par_iter_mut()
        .for_each(|(pos, mut energy, mut flows, mut light_share, genome)| {
            if genome.can_photosynthesise() {
                let tile = tile_map.tile_at_pos(pos.0);
                let photo_area = genome.total_photo_surface_area();

                let (tx, ty) = tile_index(pos.0, w, h);
                let (leaf_in_window, tiles) = window_sum(sat, w, h, tx, ty, CANOPY_RADIUS);
                let share =
                    canopy_light_share(leaf_in_window, tiles, config.leaf_capacity_per_tile);
                light_share.0 = share;

                let gained = genome.photosynthesis_rate
                    * photo_area
                    * tile.light_level
                    * light_mult
                    * share
                    * PHOTO_OUTPUT_MULTIPLIER;
                flows.clamp +=
                    credit_clamped(&mut energy, gained, config.max_organism_energy) as f64;
                flows.photosynthesis += gained as f64;
            }
        });
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
        if genome.is_photosynthesiser() {
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
    mut sim_rng: ResMut<SimRng>,
) {
    let rng = &mut sim_rng.0;

    // 1. Background spontaneous infection — keeps disease present even when
    // populations would otherwise clear all pathogens.
    if tick.0.is_multiple_of(DISEASE_BACKGROUND_PERIOD_TICKS) {
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
            if sick_entity == entity {
                continue;
            }
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

        if infection_pressure <= 0.001 {
            continue;
        }

        // Per-tick infection chance, reduced by resistance, capped.
        let chance =
            (infection_pressure * DISEASE_TRANSMISSION_RATE * (1.0 - genome.disease_resistance))
                .min(DISEASE_TRANSMISSION_CHANCE_CAP);
        if rng.gen::<f32>() < chance {
            // Inherit roughly the strain's severity & duration, slightly weakened.
            commands.entity(entity).insert(Infection {
                severity: (best_severity * DISEASE_TRANSMISSION_SEVERITY_DECAY).clamp(0.1, 1.0),
                ticks_remaining: (best_remaining * 8 / 10)
                    .max(DISEASE_TRANSMISSION_MIN_DURATION_TICKS),
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
    mut sim_rng: ResMut<SimRng>,
    mut ledger: ResMut<EnergyLedger>,
) {
    let rng = &mut sim_rng.0;
    for (entity, mut energy, mut infection, genome) in &mut infected {
        // Resistance protection is QUADRATIC so evolving upward is actually
        // rewarded. Linear (1-res) gave too small a fitness delta across the
        // realistic resistance range (5-20%) — orgs evolving resistance had
        // nearly the same disease mortality as orgs that didn't, so the gene
        // drifted with no signal. See DECISIONS.md "Disease: direct mortality
        // + energy drain" and its tuning follow-up.
        let res = genome.disease_resistance.clamp(0.0, 1.0);
        let drain_factor = (1.0 - res * 0.5).powi(2);
        let mortality_factor = (1.0 - res).powi(2);

        // Resistance cushions the drain; multiplier cranked above 1.0 so
        // photosynthesisers can't trivially out-absorb the cost from sunlight.
        let drain = config.base_metabolism_cost
            * infection.severity
            * drain_factor
            * DISEASE_DRAIN_MULTIPLIER;
        energy.0 -= drain;
        ledger.tick.disease += drain as f64;

        // Direct mortality chance per tick — ignores energy reserves so
        // photosynthesisers can't just sun-bathe through an infection.
        // Zero only energy (not health) so death_system attributes to Disease.
        let mortality = DISEASE_MORTALITY_RATE * infection.severity * mortality_factor;
        if rng.gen::<f32>() < mortality {
            ledger.tick.death += energy.0 as f64;
            energy.0 = 0.0;
        }

        infection.ticks_remaining = infection.ticks_remaining.saturating_sub(1);
        if infection.ticks_remaining == 0 {
            commands.entity(entity).remove::<Infection>();
        }
    }
}

/// Per organism, find the nearest same-tick neighbour within
/// `SYMBIOSIS_RANGE` and update its `Symbiosis` tracking. If the current
/// nearest matches the stored `link_target`, increment the streak;
/// otherwise replace the target and reset the streak. The transfer
/// system downstream turns a long-enough mutual streak into an actual
/// energy exchange.
fn symbiosis_tracking_system(
    spatial_hash: Res<SpatialHash>,
    mut organisms: Query<(Entity, &Position, &mut Symbiosis), With<Organism>>,
    all_positions: Query<&Position, With<Organism>>,
) {
    organisms
        .par_iter_mut()
        .for_each(|(entity, pos, mut symbiosis)| {
            let nearby = spatial_hash.query_radius(pos.0, SYMBIOSIS_RANGE);
            let mut best: Option<(Entity, f32)> = None;
            for &other in &nearby {
                if other == entity {
                    continue;
                }
                if let Ok(other_pos) = all_positions.get(other) {
                    let dist2 = (other_pos.0 - pos.0).length_squared();
                    if best.is_none_or(|(_, d)| dist2 < d) {
                        best = Some((other, dist2));
                    }
                }
            }
            let current = best.map(|(e, _)| e);
            if current.is_some() && current == symbiosis.link_target {
                symbiosis.link_ticks = symbiosis.link_ticks.saturating_add(1);
            } else {
                symbiosis.link_target = current;
                symbiosis.link_ticks = if current.is_some() { 1 } else { 0 };
            }
        });
}

/// Walk linked pairs — entities where both sides have each other as
/// their `link_target` with streaks past the threshold — and exchange
/// energy according to both parties' `symbiosis_rate`. Each organism
/// transfers `rate * SYMBIOSIS_TRANSFER_RATE` from itself to its partner
/// each tick; negative rates reverse the flow. Net effect on A per
/// tick is `(rate_b - rate_a) * SYMBIOSIS_TRANSFER_RATE`.
fn symbiosis_transfer_system(
    // A killed organism drops out of pairing here, so its partner neither pays
    // nor receives this tick and no energy moves to or from a corpse.
    organisms: Query<(Entity, &Genome, &Symbiosis), (With<Organism>, Without<Killed>)>,
    mut energies: Query<&mut Energy, With<Organism>>,
    config: Res<SimConfig>,
    mut ledger: ResMut<EnergyLedger>,
) {
    let mut pairs: Vec<(Entity, Entity, f32, f32)> = Vec::new();
    let mut handled: std::collections::HashSet<Entity> = std::collections::HashSet::new();

    for (a_entity, a_genome, a_sym) in organisms.iter() {
        if handled.contains(&a_entity) {
            continue;
        }
        if a_sym.link_ticks < SYMBIOSIS_LINK_THRESHOLD {
            continue;
        }
        let Some(b_entity) = a_sym.link_target else {
            continue;
        };
        let Ok((_, b_genome, b_sym)) = organisms.get(b_entity) else {
            continue;
        };
        if b_sym.link_ticks < SYMBIOSIS_LINK_THRESHOLD {
            continue;
        }
        if b_sym.link_target != Some(a_entity) {
            continue;
        }
        handled.insert(a_entity);
        handled.insert(b_entity);
        pairs.push((
            a_entity,
            b_entity,
            a_genome.symbiosis_rate,
            b_genome.symbiosis_rate,
        ));
    }

    for (a, b, rate_a, rate_b) in pairs {
        let net_a = (rate_b - rate_a) * SYMBIOSIS_TRANSFER_RATE;
        let (payer, receiver, amount) = if net_a >= 0.0 {
            (b, a, net_a)
        } else {
            (a, b, -net_a)
        };
        let Ok([mut payer_energy, mut receiver_energy]) = energies.get_many_mut([payer, receiver])
        else {
            continue;
        };
        // The payer gives only what it holds. Both sides used to be clamped
        // at zero after the transfer, which credited the receiver with the
        // full amount while the payer lost less than that, minting the
        // difference; the ledger made the leak visible. A payer drained to
        // zero dies in death_system this tick either way.
        let paid = amount.min(payer_energy.0.max(0.0));
        payer_energy.0 -= paid;
        ledger.tick.clamp +=
            credit_clamped(&mut receiver_energy, paid, config.max_organism_energy) as f64;
        ledger.tick.symbiosis += paid as f64;
    }
}

fn metabolism_system(
    config: Res<SimConfig>,
    mut organisms: Query<
        (
            &mut Energy,
            &mut Health,
            &mut Age,
            &mut EnergyFlows,
            &BodySize,
            &Genome,
            &GroupSize,
        ),
        With<Organism>,
    >,
) {
    // Parallelised: per-organism reads+writes only, no cross-organism data
    // dependency, no shared mutable state. Bevy's task pool (capped via
    // CLAU_WORKERS) does the fan-out. Same safety reasoning as
    // sensing_and_brain_system — each iteration gets its own Mut<T>. The
    // ledger is written through the organism's own EnergyFlows record, never
    // the shared resource.
    organisms.par_iter_mut().for_each(
        |(mut energy, mut health, mut age, mut flows, body_size, genome, group_size)| {
            age.0 += 1;

            // Body size costs quadratically — being big is VERY expensive
            let effective_size = body_size.0.max(0.5);
            let size_cost = effective_size * effective_size;
            let mut cost =
                config.base_metabolism_cost * size_cost * (1.0 + genome.speed_factor * 0.2);
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
            flows.metabolism += cost as f64;

            // Health regenerates slower with age.
            // Skip regen if already at zero — a fatally-wounded organism shouldn't
            // heal itself between predation and death_system. Without this gate,
            // predation kills get misattributed to Starvation because the victim's
            // health bounces back to ~0.005 before death_system reads it.
            if health.0 > 0.0 {
                let regen_rate = 0.005 / age_factor;
                health.0 = (health.0 + regen_rate).min(1.0);
            }

            // Old age death: after ~3000 ticks (~100 seconds), health degrades
            if age.0 > 3000 {
                health.0 -= 0.002;
                if health.0 <= 0.0 {
                    flows.death += energy.0 as f64;
                    energy.0 = 0.0; // triggers death
                }
            }
        },
    );
}

fn death_system(
    mut commands: Commands,
    organisms: Query<
        (
            Entity,
            &Energy,
            &Health,
            &Position,
            &Age,
            &EnergyFlows,
            Option<&Infection>,
            Option<&Killed>,
        ),
        With<Organism>,
    >,
    mut stats: ResMut<SimStats>,
    mut fitness: ResMut<FitnessTracker>,
    mut ledger: ResMut<EnergyLedger>,
) {
    for (entity, energy, health, pos, age, flows, infection, killed) in &organisms {
        if energy.0 <= 0.0 || health.0 <= 0.0 || killed.is_some() {
            // The despawn takes this organism's per-tick flow record with it
            // before ledger_system can sum it, so fold it in here, then book
            // whatever energy it still held (negative if metabolism overdrew).
            ledger.tick.add(flows);
            ledger.tick.death += energy.0 as f64;

            // A kill records its own cause on the `Killed` marker. Anything
            // else died of depletion, attributed by priority: old age (health
            // decayed to zero after 3000 ticks) > disease > starvation.
            let cause = match killed {
                Some(k) => k.0,
                None if age.0 > 3000 => DeathCause::OldAge,
                None if infection.is_some() => DeathCause::Disease,
                None => DeathCause::Starvation,
            };
            stats.deaths_by_cause[cause as usize] += 1;

            // Spawn death marker before despawning
            commands.spawn((
                DeathMarker {
                    timer: 0.5,
                    was_predated: cause == DeathCause::Predation,
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

/// Fraction of the parent's size-scaled reproduction cost that becomes the
/// child's starting energy; the rest is the overhead of building a body.
/// At body_size 1.0 this gives 32 energy, the fixed value every child
/// received before the cost and the child's energy were tied together.
/// See DECISIONS.md "Child starting energy is a fraction of what the parent paid".
const CHILD_ENERGY_FRACTION: f32 = 0.8;

/// A population-ceiling episode ends once the population has fallen below
/// this fraction of `SimConfig::population_ceiling`. Without the hysteresis a
/// population hovering just under the ceiling would open and close an
/// episode every few ticks and fill the chronicle with entries.
const CEILING_RELEASE_FRACTION: f32 = 0.95;

fn reproduction_system(
    mut commands: Commands,
    config: Res<SimConfig>,
    mut innovation: ResMut<InnovationCounter>,
    spatial_hash: Res<SpatialHash>,
    mut organisms: Query<
        (
            Entity,
            &Position,
            &mut Energy,
            &mut ActionFlash,
            &Genome,
            &BrainOutput,
            &BodySize,
            &SpeciesId,
            &Generation,
        ),
        With<Organism>,
    >,
    mut stats: ResMut<SimStats>,
    bloom: Res<BloomEffects>,
    mut sim_rng: ResMut<SimRng>,
    mut ledger: ResMut<EnergyLedger>,
    tick: Res<TickCounter>,
    mut chronicle: ResMut<WorldChronicle>,
) {
    let mut rng = &mut sim_rng.0;

    // Collect potential mate data upfront to avoid query conflicts
    let mate_candidates: Vec<(Entity, Vec2, f32, Genome, u64)> = organisms
        .iter()
        .filter(|(_, _, _, _, _, output, _, _, _)| output.reproduce > 0.5)
        .map(|(e, pos, energy, _, genome, _, _, species, _)| {
            (e, pos.0, energy.0, genome.clone(), species.0)
        })
        .collect();

    // (position, genome, parent species, generation, starting energy)
    let mut new_organisms: Vec<(Vec2, Genome, u64, u32, f32)> = Vec::new();
    let current_pop = organisms.iter().len();
    // `population_ceiling` is the only birth limiter. The population is
    // meant to settle where the energy flows put it, and under the current
    // rules it does not (see DECISIONS.md "Emergent carrying capacity"), so
    // every engagement is recorded: the chronicle and the headless summary
    // say how often the ceiling was the thing deciding the population.
    let ceiling = config.population_ceiling as usize;
    let mut blocked_births = 0u64;
    let mut already_mated: Vec<Entity> = Vec::new();

    // When more parents want a child than the ceiling has room for, admit
    // each with probability slots / wanting rather than in query iteration
    // order. Iteration order follows archetype and spawn order, so first
    // come first served handed the slots to whichever lineage happened to
    // sit first in the tables, tick after tick. See DECISIONS.md, "Diet
    // axis tuning pass".
    let slots = ceiling.saturating_sub(current_pop);
    let wanting = organisms
        .iter()
        .filter(|(_, _, energy, _, _, output, body_size, _, _)| {
            output.reproduce > 0.5 && energy.0 > reproduction_threshold(&config, body_size.0)
        })
        .count();
    let admit_probability = if wanting > slots {
        slots as f32 / wanting as f32
    } else {
        1.0
    };

    for (entity, pos, mut energy, mut flash, genome, output, body_size, species, generation) in
        &mut organisms
    {
        if already_mated.contains(&entity) {
            continue;
        }
        // Reproduction cost scales with body size — small organisms can't reproduce for free
        let repro_cost = config.reproduction_energy_cost * (0.5 + body_size.0 * 0.5);
        let repro_threshold = reproduction_threshold(&config, body_size.0);
        let wants_child = output.reproduce > 0.5 && energy.0 > repro_threshold;
        if wants_child && admit_probability < 1.0 && rng.gen::<f32>() >= admit_probability {
            blocked_births += 1;
            continue;
        }
        if current_pop + new_organisms.len() >= ceiling {
            // Blocked parents keep their energy; only the birth is refused.
            if wants_child {
                blocked_births += 1;
            }
            continue;
        }
        if wants_child {
            energy.0 -= repro_cost;
            ledger.tick.reproduction_spent += repro_cost as f64;
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
                if let Some((_, _, mate_energy, mate_g, mate_species)) = mate_candidates
                    .iter()
                    .find(|(e, _, _, _, _)| *e == nearby_entity)
                {
                    if *mate_species == species.0
                        && *mate_energy > config.reproduction_energy_threshold
                    {
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
            child_genome.mutate(
                &mut innovation,
                &mut rng,
                effective_mutation_rate,
                config.mutation_strength,
            );

            let offset = Vec2::new(rng.gen_range(-5.0..5.0), rng.gen_range(-5.0..5.0));
            let child_pos = Vec2::new(
                (pos.0.x + offset.x).rem_euclid(config.world_width as f32),
                (pos.0.y + offset.y).rem_euclid(config.world_height as f32),
            );

            // The child receives a fixed fraction of what this parent paid, so
            // a birth never creates energy regardless of the parent's size.
            let child_energy = repro_cost * CHILD_ENERGY_FRACTION;
            new_organisms.push((
                child_pos,
                child_genome,
                species.0,
                generation.0 + 1,
                child_energy,
            ));
            already_mated.push(entity);
        }
    }

    // One chronicle entry per engagement episode, not per blocked tick.
    if blocked_births > 0 {
        stats.ceiling_blocked_births += blocked_births;
        if stats.ceiling_engaged_since.is_none() {
            stats.ceiling_engaged_since = Some(tick.0);
            stats.ceiling_episodes += 1;
            chronicle.log(tick.0, format!(
                "POPULATION CEILING! {} organisms hit the {} safety ceiling; births blocked (episode {})",
                current_pop, ceiling, stats.ceiling_episodes
            ));
        }
    } else if let Some(since) = stats.ceiling_engaged_since {
        let release_below = (ceiling as f32 * CEILING_RELEASE_FRACTION) as usize;
        if current_pop < release_below {
            stats.ceiling_engaged_since = None;
            chronicle.log(
                tick.0,
                format!(
                    "Population ceiling released after {} ticks; {} organisms remain",
                    tick.0 - since,
                    current_pop
                ),
            );
        }
    }

    for (child_pos, child_genome, parent_species, child_gen, child_energy) in new_organisms {
        let brain = Brain::from_genome(&child_genome);
        let body_size = child_genome.body_size;
        ledger.tick.reproduction_received += child_energy as f64;

        commands
            .spawn((
                Organism,
                Energy(child_energy),
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
                ParentInfo {
                    parent_species_id: Some(parent_species),
                },
            ))
            .insert((
                brain,
                child_genome,
                TrailHistory::default(),
                BrainActivations::default(),
                Symbiosis::default(),
                EnergyFlows::default(),
                LightShare::default(),
            ));

        stats.total_births += 1;
        if child_gen > stats.max_generation {
            stats.max_generation = child_gen;
        }
    }
}

/// Close the tick's energy books. Runs after every system that moves
/// organism energy and before the recorders read the ledger. Sums the live
/// total and the per-organism `EnergyFlows` records that the parallel systems
/// wrote (zeroing them for the next tick), then asks the ledger for the
/// residual: the change in total energy since the previous tick minus the
/// net of the recorded flows. Zero, up to f32 rounding, means every energy
/// movement in the tick was booked; anything larger is a missing or
/// double-counted flow and would have caught the 2026-09-17 minting bugs.
fn ledger_system(
    tick: Res<TickCounter>,
    mut ledger: ResMut<EnergyLedger>,
    mut organisms: Query<(&Energy, &mut EnergyFlows), With<Organism>>,
    mut chronicle: ResMut<WorldChronicle>,
) {
    let mut total = 0.0f64;
    let mut summed = EnergyFlows::default();
    for (energy, mut flows) in &mut organisms {
        total += energy.0 as f64;
        summed.add(&flows);
        flows.clear();
    }
    ledger.tick.add(&summed);

    let tick_flows = ledger.tick;
    let had_baseline = ledger.has_baseline();
    let residual = ledger.close_tick(total);

    debug_assert!(
        residual.abs() <= EnergyLedger::TOLERANCE,
        "energy ledger residual {residual:+.4} at tick {} exceeds tolerance {} (baseline set: {had_baseline}); tick flows: {tick_flows:?}",
        tick.0,
        EnergyLedger::TOLERANCE,
    );
    if residual.abs() > EnergyLedger::TOLERANCE && ledger.should_warn(tick.0) {
        chronicle.log(
            tick.0,
            format!(
                "Energy ledger out of balance: residual {:+.3} this tick, {} ticks over tolerance so far",
                residual, ledger.breaches
            ),
        );
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
    mut convergence_high: ResMut<ConvergenceHighWater>,
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
            let strategy = classify_strategy(genome);
            // Parent is the old species this organism was classified as
            let parent = if *_old_species > 0 {
                Some(*_old_species)
            } else {
                None
            };
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

            let species_name = phylo
                .nodes
                .get(&new_id)
                .map(|n| n.name.as_str())
                .unwrap_or("Unknown");
            let parent_str = if let Some(p) = parent {
                let parent_name = phylo
                    .nodes
                    .get(&p)
                    .map(|n| n.name.as_str())
                    .unwrap_or("unknown");
                format!(" (from {})", parent_name)
            } else {
                String::new()
            };
            chronicle.log(
                tick.0,
                format!("New species: {}{}", species_name, parent_str),
            );

            new_id
        };

        assignments.insert(*entity, assigned);
    }

    // Update population counts in phylo tree
    let mut species_counts: HashMap<u64, u32> = HashMap::new();
    for (_, _, _old_species) in &org_data {
        // Use assigned species, not old
    }
    for assigned_id in assignments.values() {
        *species_counts.entry(*assigned_id).or_insert(0) += 1;
    }
    // Detect extinctions before updating
    let previously_living: Vec<u64> = phylo
        .nodes
        .iter()
        .filter(|(_, n)| n.extinct_tick.is_none() && n.current_population > 0)
        .map(|(id, _)| *id)
        .collect();

    phylo.update_populations(&species_counts, tick.0);

    // Log extinctions
    for species_id in &previously_living {
        if let Some(node) = phylo.nodes.get(species_id) {
            if node.current_population == 0 && node.peak_population >= 10 {
                let age_secs = tick.0.saturating_sub(node.born_tick) / 30;
                chronicle.log(
                    tick.0,
                    format!(
                        "{} went extinct (peak: {}, lived {}s)",
                        node.name, node.peak_population, age_secs
                    ),
                );
            }
        }
    }

    for (entity, _genome, mut species_id) in &mut organisms {
        if let Some(&new_id) = assignments.get(&entity) {
            species_id.0 = new_id;
        }
    }

    // Report the number of populated species. In practice this equals
    // species_reps.len(): a representative is at distance 0 from itself and
    // so always stays in its own species, and a new species is created with
    // the organism that founded it, so no species ends a pass empty.
    // species_counts is still the quantity meant here, and species_reps is
    // rebuilt from living organisms every pass, so nothing needs pruning.
    stats.species_count = species_counts.len() as u32;

    // Detect convergent evolution. Each strategy is chronicled only when its
    // independent-lineage count exceeds the highest count already logged.
    for (strategy, lineage_count) in phylo.detect_convergence() {
        if convergence_high.record(strategy, lineage_count) {
            chronicle.log(
                tick.0,
                format!(
                    "Convergent evolution! {} independent lineages evolved {}",
                    lineage_count,
                    strategy.activity()
                ),
            );
        }
    }
}

fn record_population_history(
    time: Res<Time>,
    mut timer: ResMut<PopHistoryTimer>,
    tick: Res<TickCounter>,
    stats: Res<SimStats>,
    organisms: Query<
        (
            Entity,
            &Genome,
            &Symbiosis,
            Option<&Infection>,
            &Velocity,
            &LightShare,
            &Energy,
            &BodySize,
        ),
        With<Organism>,
    >,
    food: Query<&Food>,
    mut history: ResMut<PopulationHistory>,
    fitness: Res<FitnessTracker>,
    mut ledger: ResMut<EnergyLedger>,
    config: Res<SimConfig>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let mut plants = 0u32;
    let mut grazers = 0u32;
    let mut hunters = 0u32;
    let mut omnivores = 0u32;
    let mut infected = 0u32;

    // Trait running sums for averaging
    let mut sum_resist = 0.0f32;
    let mut sum_body = 0.0f32;
    let mut sum_speed = 0.0f32;
    let mut sum_armor = 0.0f32;
    let mut sum_attack = 0.0f32;
    let mut sum_photo = 0.0f32;
    let mut sum_diet = 0.0f32;
    let mut sum_symbiosis = 0.0f32;
    let mut n = 0u32;
    // Plant physics instruments, split by plants and eaters.
    let mut sum_photo_area = 0.0f32;
    let mut sum_speed_plants = 0.0f32;
    let mut sum_speed_eaters = 0.0f32;
    let mut sum_light_share = 0.0f32;
    let mut ready_plants = 0u32;
    let mut ready_eaters = 0u32;

    // For counting mutual symbiotic pairs we need to look each partner up.
    // Build a small map once, then walk the ones that claim a link.
    let mut sym_by_entity: std::collections::HashMap<Entity, (Option<Entity>, u32)> =
        std::collections::HashMap::with_capacity(organisms.iter().len());

    for (entity, genome, symbiosis, inf, velocity, light_share, energy, body_size) in &organisms {
        let strategy = classify_strategy(genome);
        let ready = energy.0 > reproduction_threshold(&config, body_size.0);
        if strategy == SpeciesStrategy::Photosynthesizer {
            sum_photo_area += genome.total_photo_surface_area();
            sum_speed_plants += velocity.0.length();
            sum_light_share += light_share.0;
            ready_plants += ready as u32;
        } else {
            sum_speed_eaters += velocity.0.length();
            ready_eaters += ready as u32;
        }
        match strategy {
            SpeciesStrategy::Photosynthesizer => plants += 1,
            SpeciesStrategy::Grazer => grazers += 1,
            SpeciesStrategy::Hunter => hunters += 1,
            SpeciesStrategy::Omnivore => omnivores += 1,
        }
        if strategy != SpeciesStrategy::Photosynthesizer {
            // Diet is averaged over the organisms that eat; see PopSnapshot.
            sum_diet += genome.diet;
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
        sum_symbiosis += genome.symbiosis_rate;
        n += 1;

        sym_by_entity.insert(entity, (symbiosis.link_target, symbiosis.link_ticks));
    }

    // Count mutual linked pairs (each pair once)
    let mut symbiotic_pairs = 0u32;
    let mut seen: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    for (&entity, &(target, ticks)) in &sym_by_entity {
        if seen.contains(&entity) {
            continue;
        }
        if ticks < SYMBIOSIS_LINK_THRESHOLD {
            continue;
        }
        let Some(partner) = target else { continue };
        let Some(&(partner_target, partner_ticks)) = sym_by_entity.get(&partner) else {
            continue;
        };
        if partner_ticks < SYMBIOSIS_LINK_THRESHOLD || partner_target != Some(entity) {
            continue;
        }
        seen.insert(entity);
        seen.insert(partner);
        symbiotic_pairs += 1;
    }

    let div = n.max(1) as f32;
    let eaters = grazers + hunters + omnivores;
    let org_count = plants + eaters;
    let food_count = food.iter().len() as u32;

    history.record(
        &stats,
        &mut ledger,
        PopSnapshotInput {
            tick: tick.0,
            organisms: org_count,
            food: food_count,
            plants,
            grazers,
            hunters,
            omnivores,
            avg_lifespan: fitness.avg_lifespan,
            infected,
            avg_disease_resistance: sum_resist / div,
            avg_body_size: sum_body / div,
            avg_speed: sum_speed / div,
            avg_armor: sum_armor / div,
            avg_attack: sum_attack / div,
            avg_photo: sum_photo / div,
            avg_diet: sum_diet / (grazers + hunters + omnivores).max(1) as f32,
            avg_photo_area: sum_photo_area / plants.max(1) as f32,
            avg_speed_plants: sum_speed_plants / plants.max(1) as f32,
            avg_speed_eaters: sum_speed_eaters / eaters.max(1) as f32,
            avg_light_share: sum_light_share / plants.max(1) as f32,
            ready_share_plants: ready_plants as f32 / plants.max(1) as f32,
            ready_share_eaters: ready_eaters as f32 / eaters.max(1) as f32,
            symbiotic_pairs,
            avg_symbiosis_rate: sum_symbiosis / div,
        },
    );
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
    if !tick.0.is_multiple_of(3) {
        return;
    }
    for (pos, mut trail) in &mut organisms {
        trail.push(pos.0);
    }
}

/// Land biomes the founding population is seeded into, in the order the
/// per-biome counts are reported. Water is not a founding habitat.
pub const FOUNDING_BIOMES: [TerrainType; 4] = [
    TerrainType::Sand,
    TerrainType::Grassland,
    TerrainType::Forest,
    TerrainType::Rock,
];

/// Minimum share of the founding population a biome receives once its area
/// is meaningful, so no land biome starts empty.
const FOUNDER_FLOOR_SHARE: f32 = 0.05;

/// A biome's share of founding-biome land must reach this for the floor to
/// apply; smaller patches take only their proportional share.
const FOUNDER_MEANINGFUL_AREA_SHARE: f32 = 0.01;

/// Founders start at this fraction of their own body-scaled reproduction
/// threshold, so the first birth has to be paid for with earned energy.
/// See `docs/DECISIONS.md`, "Per-biome seeding".
const FOUNDER_ENERGY_FRACTION: f32 = 0.9;

/// Energy an organism of `body_size` must exceed before `reproduction_system`
/// lets it reproduce: the configured threshold scaled by `0.5 + body_size * 0.5`.
pub fn reproduction_threshold(config: &SimConfig, body_size: f32) -> f32 {
    config.reproduction_energy_threshold * (0.5 + body_size * 0.5)
}

/// Split `total` founders across biomes in proportion to `areas` (tile counts,
/// one per entry of `FOUNDING_BIOMES`). Shares are rounded by largest
/// remainder so the result sums to `total`. A biome with no tiles gets none;
/// a biome holding at least `FOUNDER_MEANINGFUL_AREA_SHARE` of the counted
/// land is raised to `FOUNDER_FLOOR_SHARE` of `total`, taking the difference
/// from the most populous biomes. Returns all zeros when there is no land.
pub fn founder_allocation(areas: &[usize], total: u32) -> Vec<u32> {
    let total_area: usize = areas.iter().sum();
    if total_area == 0 {
        return vec![0; areas.len()];
    }

    // Proportional share, rounded by largest remainder.
    let quotas: Vec<f64> = areas
        .iter()
        .map(|&a| total as f64 * a as f64 / total_area as f64)
        .collect();
    let mut counts: Vec<u32> = quotas.iter().map(|q| q.floor() as u32).collect();
    let mut leftover = total - counts.iter().sum::<u32>();
    let mut by_remainder: Vec<usize> = (0..areas.len()).collect();
    by_remainder.sort_by(|&a, &b| {
        let ra = quotas[a] - quotas[a].floor();
        let rb = quotas[b] - quotas[b].floor();
        rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal)
    });
    for &i in by_remainder.iter().cycle() {
        if leftover == 0 {
            break;
        }
        if areas[i] > 0 {
            counts[i] += 1;
            leftover -= 1;
        }
    }

    // Floor for biomes with a meaningful area, paid for by the largest.
    let floor = ((total as f32 * FOUNDER_FLOOR_SHARE).ceil() as u32).min(total);
    for i in 0..areas.len() {
        let share = areas[i] as f32 / total_area as f32;
        if share < FOUNDER_MEANINGFUL_AREA_SHARE {
            continue;
        }
        while counts[i] < floor {
            let donor = (0..areas.len())
                .filter(|&j| j != i)
                .max_by_key(|&j| counts[j])
                .filter(|&j| counts[j] > floor);
            match donor {
                Some(j) => {
                    counts[j] -= 1;
                    counts[i] += 1;
                }
                None => break,
            }
        }
    }

    counts
}

/// How the founding population was seeded. The app inserts it as a
/// resource so the headless summary can print it; a loaded save has none.
#[derive(Resource, Debug, Clone)]
pub struct FounderReport {
    /// Founders and land tiles per entry of `FOUNDING_BIOMES`.
    pub by_biome: Vec<(TerrainType, u32, usize)>,
    pub plants: u32,
    pub grazers: u32,
    pub hunters: u32,
    pub omnivores: u32,
    pub min_energy: f32,
    pub max_energy: f32,
    /// Total energy spawned; the `EnergyLedger` baseline.
    pub total_energy: f64,
}

impl std::fmt::Display for FounderReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let by_biome: Vec<String> = self
            .by_biome
            .iter()
            .map(|(t, n, a)| format!("{t:?} {n} (tiles {a})"))
            .collect();
        write!(
            f,
            "Founders by biome: {}; strategies: {} plants, {} grazers, {} hunters, {} omnivores; starting energy {:.1}..{:.1}",
            by_biome.join(", "),
            self.plants,
            self.grazers,
            self.hunters,
            self.omnivores,
            self.min_energy,
            self.max_energy
        )
    }
}

/// Spawn the founding population.
///
/// Founders are placed on land in proportion to each biome's area (see
/// `founder_allocation`), and every founder starts just below its own
/// reproduction threshold, so the opening is colonisation from modest
/// beginnings rather than a burst of births on free energy. The genome
/// split (roughly 30% photosynthesisers, the rest minimal foragers) is
/// unchanged. Returns the per-biome and per-strategy founder counts and the
/// total energy spawned, which sets the `EnergyLedger` baseline.
pub fn spawn_initial_population(
    commands: &mut Commands,
    config: &SimConfig,
    tile_map: &TileMap,
    innovation: &mut InnovationCounter,
    rng: &mut impl Rng,
) -> FounderReport {
    use rand::seq::SliceRandom;

    let photo_count = config.initial_population / 3; // 30% photosynthesizers
    let mut total_energy = 0.0f64;

    // Tile indices per founding biome.
    let mut biome_tiles: Vec<Vec<u32>> = vec![Vec::new(); FOUNDING_BIOMES.len()];
    for (idx, tile) in tile_map.tiles.iter().enumerate() {
        if let Some(b) = FOUNDING_BIOMES.iter().position(|t| *t == tile.terrain) {
            biome_tiles[b].push(idx as u32);
        }
    }
    let areas: Vec<usize> = biome_tiles.iter().map(Vec::len).collect();
    let counts = founder_allocation(&areas, config.initial_population);

    // One biome slot per founder, shuffled so the photosynthesiser share
    // lands in every biome rather than in whichever biome is listed first.
    let mut slots: Vec<Option<usize>> = Vec::with_capacity(config.initial_population as usize);
    for (b, &n) in counts.iter().enumerate() {
        slots.extend(std::iter::repeat_n(Some(b), n as usize));
    }
    // No land at all: fall back to uniform placement.
    slots.resize(config.initial_population as usize, None);
    slots.shuffle(rng);

    let mut strategy_counts = [0u32; 4];
    let (mut min_energy, mut max_energy) = (f32::MAX, f32::MIN);

    for (i, slot) in slots.iter().enumerate() {
        let (x, y) = match slot {
            Some(b) => {
                let idx = *biome_tiles[*b]
                    .choose(rng)
                    .expect("biome with founders has tiles");
                let tx = (idx % tile_map.width) as f32;
                let ty = (idx / tile_map.width) as f32;
                (tx + rng.gen::<f32>(), ty + rng.gen::<f32>())
            }
            None => (
                rng.gen_range(0.0..config.world_width as f32),
                rng.gen_range(0.0..config.world_height as f32),
            ),
        };

        let genome = if (i as u32) < photo_count {
            Genome::new_photosynthesizer_with_diet(innovation, rng, config.founder_diet_spread)
        } else {
            Genome::new_minimal_with_diet(innovation, rng, config.founder_diet_spread)
        };

        match classify_strategy(&genome) {
            SpeciesStrategy::Photosynthesizer => strategy_counts[0] += 1,
            SpeciesStrategy::Grazer => strategy_counts[1] += 1,
            SpeciesStrategy::Hunter => strategy_counts[2] += 1,
            SpeciesStrategy::Omnivore => strategy_counts[3] += 1,
        }

        let brain = Brain::from_genome(&genome);
        let body_size = genome.body_size;
        let energy = reproduction_threshold(config, body_size) * FOUNDER_ENERGY_FRACTION;
        min_energy = min_energy.min(energy);
        max_energy = max_energy.max(energy);

        total_energy += energy as f64;
        commands
            .spawn((
                Organism,
                Energy(energy),
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
            ))
            .insert((
                brain,
                genome,
                TrailHistory::default(),
                BrainActivations::default(),
                Symbiosis::default(),
                EnergyFlows::default(),
                LightShare::default(),
            ));
    }

    FounderReport {
        by_biome: FOUNDING_BIOMES
            .iter()
            .zip(counts.iter().zip(areas.iter()))
            .map(|(t, (n, a))| (*t, *n, *a))
            .collect(),
        plants: strategy_counts[0],
        grazers: strategy_counts[1],
        hunters: strategy_counts[2],
        omnivores: strategy_counts[3],
        min_energy,
        max_energy,
        total_energy,
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
    organisms: Query<
        (
            &Position,
            &Energy,
            &Health,
            &Age,
            &Generation,
            &SpeciesId,
            &Signal,
            &BrainMemory,
            &Genome,
        ),
        With<Organism>,
    >,
    food: Query<(&Position, &FoodEnergy), With<Food>>,
    phylo: Res<PhyloTree>,
    chronicle: Res<WorldChronicle>,
) {
    let save_requested = events.read().any(|r| matches!(r, WorldEventRequest::Save));
    if !save_requested {
        return;
    }

    let org_data: Vec<_> = organisms
        .iter()
        .map(
            |(pos, energy, health, age, gen, species, signal, memory, genome)| {
                (
                    pos.0,
                    energy.0,
                    health.0,
                    age.0,
                    gen.0,
                    species.0,
                    signal.0,
                    memory.0,
                    genome.clone(),
                )
            },
        )
        .collect();

    let food_data: Vec<_> = food.iter().map(|(pos, fe)| (pos.0, fe.0)).collect();

    let save_path = session.dir.join("save.json");
    save::save_world(
        &save_path,
        &tick,
        &season,
        &stats,
        &innovation,
        &config,
        &org_data,
        &food_data,
        &phylo,
        &chronicle,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::CommandQueue;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn convergence_high_water_logs_only_new_highs() {
        let mut high = ConvergenceHighWater::default();
        // First sighting of a strategy is always a new high.
        assert!(high.record(SpeciesStrategy::Grazer, 2));
        // Repeating the same count every pass is what the old text scan
        // failed to suppress.
        assert!(!high.record(SpeciesStrategy::Grazer, 2));
        assert!(!high.record(SpeciesStrategy::Grazer, 2));
        // A higher count is logged once, a lower one never.
        assert!(high.record(SpeciesStrategy::Grazer, 3));
        assert!(!high.record(SpeciesStrategy::Grazer, 3));
        assert!(!high.record(SpeciesStrategy::Grazer, 2));
        // Dropping back and returning to the old high stays quiet.
        assert!(!high.record(SpeciesStrategy::Grazer, 3));
    }

    #[test]
    fn convergence_high_water_tracks_strategies_independently() {
        let mut high = ConvergenceHighWater::default();
        assert!(high.record(SpeciesStrategy::Hunter, 4));
        for strategy in SpeciesStrategy::ALL {
            if strategy != SpeciesStrategy::Hunter {
                assert!(high.record(strategy, 2), "{strategy:?} first sighting");
                assert!(!high.record(strategy, 2), "{strategy:?} repeat");
            }
        }
        assert!(!high.record(SpeciesStrategy::Hunter, 3));
        assert!(high.record(SpeciesStrategy::Hunter, 5));
    }

    #[test]
    fn founder_allocation_is_proportional_and_sums_to_total() {
        // 50% / 30% / 20% / 0% of the land.
        let counts = founder_allocation(&[5000, 3000, 2000, 0], 400);
        assert_eq!(counts, vec![200, 120, 80, 0]);
        assert_eq!(counts.iter().sum::<u32>(), 400);
    }

    #[test]
    fn founder_allocation_floors_small_but_meaningful_biomes() {
        // Rock is 2% of the land: proportional share would be 8, floor is 20.
        // The largest biome pays the difference.
        let counts = founder_allocation(&[3000, 4800, 2000, 200], 400);
        assert_eq!(counts.iter().sum::<u32>(), 400);
        assert_eq!(counts[3], 20);
        assert_eq!(counts[1], 192 - 12);
        // Below 1% of the land, no floor applies.
        let counts = founder_allocation(&[5000, 4950, 0, 50], 400);
        assert_eq!(counts.iter().sum::<u32>(), 400);
        assert_eq!(counts[3], 2);
        assert_eq!(counts[2], 0);
    }

    #[test]
    fn founder_allocation_handles_no_land() {
        assert_eq!(founder_allocation(&[0, 0, 0, 0], 400), vec![0, 0, 0, 0]);
    }

    /// Runs the real spawner against a generated map without a Bevy `App`:
    /// founders land in the biome they were allocated to, none sits in water,
    /// the photosynthesiser share is unchanged, and every founder starts
    /// below its own reproduction threshold.
    #[test]
    fn founders_land_in_allocated_biomes_below_reproduction_threshold() {
        let config = SimConfig {
            terrain_seed: 42,
            ..SimConfig::default()
        };
        let mut terrain_rng = StdRng::seed_from_u64(config.terrain_seed);
        let tile_map = TileMap::generate(config.world_width, config.world_height, &mut terrain_rng);
        let mut innovation = InnovationCounter(0);
        let mut rng = StdRng::seed_from_u64(config.terrain_seed);

        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let report = {
            let mut commands = Commands::new(&mut queue, &world);
            spawn_initial_population(&mut commands, &config, &tile_map, &mut innovation, &mut rng)
        };
        queue.apply(&mut world);
        println!("{report}");
        assert_eq!(
            report.plants + report.grazers + report.hunters + report.omnivores,
            config.initial_population
        );
        assert_eq!(report.plants, config.initial_population / 3);
        assert!(report.by_biome.iter().all(|(_, n, a)| *a > 0 || *n == 0));

        let mut areas = vec![0usize; FOUNDING_BIOMES.len()];
        for tile in &tile_map.tiles {
            if let Some(b) = FOUNDING_BIOMES.iter().position(|t| *t == tile.terrain) {
                areas[b] += 1;
            }
        }
        let expected = founder_allocation(&areas, config.initial_population);

        let mut per_biome = vec![0u32; FOUNDING_BIOMES.len()];
        let mut plants = 0u32;
        let mut total = 0u32;
        let mut query =
            world.query_filtered::<(&Position, &Energy, &BodySize, &Genome), With<Organism>>();
        for (pos, energy, body, genome) in query.iter(&world) {
            total += 1;
            let terrain = tile_map.tile_at_pos(pos.0).terrain;
            assert!(
                !terrain.is_water(),
                "founder placed in water at {:?}",
                pos.0
            );
            let b = FOUNDING_BIOMES.iter().position(|t| *t == terrain).unwrap();
            per_biome[b] += 1;
            assert!(
                energy.0 < reproduction_threshold(&config, body.0),
                "founder starts able to reproduce: energy {} body {}",
                energy.0,
                body.0
            );
            if classify_strategy(genome) == SpeciesStrategy::Photosynthesizer {
                plants += 1;
            }
        }

        assert_eq!(total, config.initial_population);
        assert_eq!(per_biome, expected, "founders per biome (areas {areas:?})");
        assert_eq!(plants, config.initial_population / 3);
    }
}

#[cfg(test)]
mod digestion_tests {
    use super::*;

    #[test]
    fn summed_area_table_gives_rectangle_totals() {
        // 3 x 2 grid, row-major.
        let grid = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let sat = summed_area_table(&grid, 3, 2);
        assert_eq!(window_sum(&sat, 3, 2, 0, 0, 0), (1.0, 1));
        assert_eq!(window_sum(&sat, 3, 2, 2, 1, 0), (6.0, 1));
        // Whole grid from the centre with a radius that overshoots.
        assert_eq!(window_sum(&sat, 3, 2, 1, 0, 5), (21.0, 6));
        // 2 x 2 window at the top-left corner, clipped.
        assert_eq!(window_sum(&sat, 3, 2, 0, 0, 1), (1.0 + 2.0 + 4.0 + 5.0, 4));
    }

    #[test]
    fn canopy_light_share_is_full_until_leaves_exceed_the_ground() {
        // 81 tiles at 0.02 light 1.62 units of leaf.
        assert_eq!(canopy_light_share(1.0, 81, 0.02), 1.0);
        assert_eq!(canopy_light_share(1.62, 81, 0.02), 1.0);
        assert!((canopy_light_share(3.24, 81, 0.02) - 0.5).abs() < 1e-6);
        assert!((canopy_light_share(16.2, 81, 0.02) - 0.1).abs() < 1e-6);
        // No leaves, no shading; a zero capacity shades everything fully.
        assert_eq!(canopy_light_share(0.0, 81, 0.02), 1.0);
        assert_eq!(canopy_light_share(1.0, 81, 0.0), 0.0);
    }

    #[test]
    fn photo_drag_leaves_the_leafless_alone_and_slows_the_leafy() {
        assert_eq!(photo_drag_factor(0.0, 1.0), 1.0);
        assert_eq!(photo_drag_factor(2.0, 0.0), 1.0);
        assert!((photo_drag_factor(1.0, 1.0) - 0.5).abs() < 1e-6);
        assert!((photo_drag_factor(2.0, 1.0) - 1.0 / 3.0).abs() < 1e-6);
        // Monotone in area and in the coefficient.
        assert!(photo_drag_factor(0.5, 1.0) > photo_drag_factor(1.5, 1.0));
        assert!(photo_drag_factor(1.0, 0.3) > photo_drag_factor(1.0, 3.0));
    }

    #[test]
    fn digest_conserves_the_meal() {
        for eff in [0.0, 0.25, 0.44, 1.0] {
            let (kept, wasted) = digest(40.0, eff);
            assert!((kept + wasted - 40.0).abs() < 1e-5, "eff {eff}");
            assert!((kept - 40.0 * eff).abs() < 1e-5, "eff {eff}");
        }
    }

    #[test]
    fn digest_clamps_efficiency_to_unit_range() {
        assert_eq!(digest(10.0, 2.0), (10.0, 0.0));
        assert_eq!(digest(10.0, -1.0), (0.0, 10.0));
    }

    #[test]
    fn bite_and_pyramid_shares_are_fractions_of_the_prey() {
        // A bite is `bite_fraction` of what the plant holds, a kill offers
        // `kill_transfer_fraction`; both are then digested.
        let plant_energy = 80.0;
        let bite = plant_energy * SimConfig::default().bite_fraction;
        let (kept, wasted) = digest(bite, 1.0);
        assert!((kept - bite).abs() < 1e-5 && wasted.abs() < 1e-5);
        assert!(bite > 0.0 && bite < plant_energy);
        let (kept, wasted) = digest(
            plant_energy * SimConfig::default().kill_transfer_fraction,
            0.25,
        );
        assert!((kept - 2.0).abs() < 1e-5 && (wasted - 6.0).abs() < 1e-5);
    }
}
