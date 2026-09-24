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
/// Summed proximity pressure at or below which a healthy organism skips the
/// transmission roll entirely. Below this the chance would round to nothing,
/// and skipping saves the RNG draw.
const DISEASE_TRANSMISSION_PRESSURE_FLOOR: f32 = 0.001;
/// Lowest severity a transmitted infection can carry, so a strain decaying
/// through many hosts never fades to a harmless trace.
const DISEASE_TRANSMISSION_SEVERITY_MIN: f32 = 0.1;
/// Highest severity a transmitted infection can carry (the unit ceiling).
const DISEASE_TRANSMISSION_SEVERITY_MAX: f32 = 1.0;
/// Percentage of the source's remaining duration a transmitted infection
/// inherits, before `DISEASE_TRANSMISSION_MIN_DURATION_TICKS` applies.
/// Integer so the tick count truncates the way it always has.
const DISEASE_TRANSMISSION_DURATION_RETAINED_PERCENT: u32 = 80;

/// Energy-drain multiplier applied per tick to infected organisms (scales on severity and resistance).
const DISEASE_DRAIN_MULTIPLIER: f32 = 1.6;
/// Weight on resistance in the drain cushion `(1 - res × weight)²`. At 0.5,
/// full resistance cuts the drain to a quarter rather than to zero, so an
/// infection always costs something. Mortality uses the unweighted
/// `(1 - res)²`; see docs/DECISIONS.md "Disease: direct mortality + energy drain".
const DISEASE_DRAIN_RESISTANCE_WEIGHT: f32 = 0.5;
/// Base per-tick chance of direct disease-caused death (scales on severity and (1 - resistance)).
/// At severity 0.5 and zero resistance this gives ~22% cumulative chance over 20s.
const DISEASE_MORTALITY_RATE: f32 = 0.0015;

// -----------------------------------------------------------------------------
// Niche construction tuning constants
//
// Per-tick deposits an organism makes on the tile it stands on, each clamped
// to the tile's 0..1 range at the point of use. `tile_dynamics_system` in the
// world crate pulls vegetation toward its carrying capacity, so these are
// nudges on top of that, not the main driver.
// -----------------------------------------------------------------------------

/// Vegetation density a photosynthesiser adds to its tile each tick.
const NICHE_VEGETATION_DEPOSIT: f32 = 0.001;
/// Moisture a photosynthesiser adds to its tile each tick.
const NICHE_MOISTURE_DEPOSIT: f32 = 0.0005;
/// Nutrients any organism adds to its tile each tick (waste products).
const NICHE_NUTRIENT_DEPOSIT: f32 = 0.0001;

// -----------------------------------------------------------------------------
// Movement tuning constants
//
// Used by action_system. Speed is `speed_factor × BASE_MOVE_SPEED / √size`,
// slowed by armour and photosynthetic drag; the energy a move costs is then
// scaled by a terrain cost built from the constants below.
// -----------------------------------------------------------------------------

/// World units per tick an organism of body size 1.0 moves at
/// `speed_factor` 1.0 and full brain output, before any drag.
const BASE_MOVE_SPEED: f32 = 2.0;
/// Armour drag coefficient: speed is multiplied by `1 / (1 + armour × drag)`.
/// `photo_drag_factor` has the same shape for leaf area.
const ARMOR_DRAG: f32 = 0.3;
/// How far aquatic adaptation moves the terrain cost: water costs
/// `1 - aquatic × weight` of its base, land `1 + aquatic × weight`. One
/// weight for both, so adapting to water costs as much on land as it saves
/// in it.
const AQUATIC_MOVE_COST_WEIGHT: f32 = 0.5;
/// Water move-cost reduction per unit of fin area.
const FIN_MOVE_BONUS_PER_AREA: f32 = 0.3;
/// Most of the water move cost fins can remove.
const FIN_MOVE_BONUS_CAP: f32 = 0.5;
/// Land move-cost reduction per limb.
const LIMB_MOVE_BONUS_PER_LIMB: f32 = 0.15;
/// Most of the land move cost limbs can remove.
const LIMB_MOVE_BONUS_CAP: f32 = 0.4;
/// Floor on the terrain cost multiplier, on water and land alike, so no
/// combination of adaptations makes moving close to free.
const TERRAIN_MOVE_COST_FLOOR: f32 = 0.5;

// -----------------------------------------------------------------------------
// Feeding and predation tuning constants
//
// Food-item eating lives in action_system and attacks in predation_system.
// Plant bites in grazing_system read `SimConfig::bite_reach` and
// `bite_fraction` instead; see DECISIONS.md, "Grazing through eat".
// -----------------------------------------------------------------------------

/// Reach, in multiples of body size, within which `eat` takes a food item.
const FOOD_EAT_REACH: f32 = 3.0;
/// The `attack` brain output must exceed this for the organism to strike.
const ATTACK_INTENT_THRESHOLD: f32 = 0.5;
/// Reach, in multiples of body size, within which an attacker can strike.
const ATTACK_REACH: f32 = 4.0;
/// Share of the target's defence (`armour × body size`) subtracted from the
/// attacker's strike force (`claw power × body size`) to give the damage.
const DEFENCE_WEIGHT: f32 = 0.5;
/// Size gate: the attacker's body size must exceed the target's times this.
/// Below 1.0, so an attacker can take prey somewhat larger than itself.
const PREY_SIZE_RATIO: f32 = 0.6;
/// Damage gate: the damage must exceed this for a strike to kill. A clawless
/// attacker does no damage and never passes it.
const MIN_KILL_DAMAGE: f32 = 0.1;

// -----------------------------------------------------------------------------
// Reproduction tuning constants
//
// Used by reproduction_system. `CHILD_ENERGY_FRACTION` and
// `CEILING_RELEASE_FRACTION` sit beside the system.
// -----------------------------------------------------------------------------

/// The `reproduce` brain output must exceed this for the organism to want a
/// child. A mate must pass the same gate.
const REPRODUCE_INTENT_THRESHOLD: f32 = 0.5;
/// Reproduction cost and threshold scale on body size by
/// `REPRODUCTION_SIZE_SCALE_BASE + body_size × REPRODUCTION_SIZE_SCALE_PER_SIZE`,
/// which is 1.0 at body size 1.0. This is the constant term.
const REPRODUCTION_SIZE_SCALE_BASE: f32 = 0.5;
/// The per-unit-of-body-size term of the reproduction size scale.
const REPRODUCTION_SIZE_SCALE_PER_SIZE: f32 = 0.5;
/// Reach, in multiples of body size, within which a parent looks for a mate.
const MATE_SEARCH_REACH: f32 = 8.0;
/// A child spawns at up to this many world units from its parent on each
/// axis, drawn uniformly from `-offset..offset`.
const CHILD_SPAWN_OFFSET: f32 = 5.0;

// -----------------------------------------------------------------------------
// Action flash constants
//
// `ActionFlash` is visual only: the render crate pulses an organism's sprite
// while the timer runs. Nothing in the simulation reads it.
// -----------------------------------------------------------------------------

/// Seconds an action flash lasts after eating, grazing, a kill or a birth.
const ACTION_FLASH_SECS: f32 = 0.3;
/// Seconds taken off a running action flash each tick, about one 30 Hz tick.
const ACTION_FLASH_DECAY_PER_TICK: f32 = 0.033;

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

/// Share of a meal a mouthed eater takes, food item or plant bite.
const MOUTH_BONUS: f32 = 1.0;

/// Share of a meal an eater without a mouth segment takes. The rest stays
/// where it was: on the food item, which is consumed anyway, or in the
/// plant, which keeps it.
const MOUTHLESS_BONUS: f32 = 0.3;

/// How much of a meal this eater's mouth takes: `MOUTH_BONUS` with a mouth
/// segment, `MOUTHLESS_BONUS` without.
pub fn mouth_bonus(genome: &Genome) -> f32 {
    if genome.has_mouth() {
        MOUTH_BONUS
    } else {
        MOUTHLESS_BONUS
    }
}

/// The mouth bonus on a plant bite: `MOUTH_BONUS` with a mouth segment,
/// `mouthless` (`SimConfig::mouthless_bite_bonus`) without.
pub fn bite_mouth_bonus(genome: &Genome, mouthless: f32) -> f32 {
    if genome.has_mouth() {
        MOUTH_BONUS
    } else {
        mouthless
    }
}

/// One bite of a living plant through `eat`: `bite_fraction` of what the
/// plant holds, scaled by the eater's mouth. This is what the plant loses and
/// what the eater then digests at its `plant_efficiency`.
pub fn graze_bite(plant_energy: f32, bite_fraction: f32, mouth_bonus: f32) -> f32 {
    plant_energy.max(0.0) * bite_fraction * mouth_bonus
}

/// The efficiency a killer digests its kill at. Digestion follows the
/// tissue, not the act: a plant victim is plant tissue and is digested at
/// `plant_efficiency`; an animal victim at `animal_efficiency` times the
/// hunting multiplier (`SimConfig::animal_efficiency_multiplier`).
pub fn kill_digestion_efficiency(
    killer: &Genome,
    victim_is_plant: bool,
    animal_efficiency_multiplier: f32,
) -> f32 {
    if victim_is_plant {
        killer.plant_efficiency()
    } else {
        killer.animal_efficiency() * animal_efficiency_multiplier
    }
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
                export_organism_system,
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
                    grazing_system,
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
                    death_marker_expiry_system,
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
        .insert_resource(SaveReport::default())
        .insert_resource(OrganismExportReport::default())
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
        .init_resource::<AteFoodThisTick>()
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

/// The organisms that ate a food item in this tick's `action_system`.
/// `grazing_system` skips them: food items are free tissue and are eaten
/// first, and an eater takes one meal per tick.
#[derive(Resource, Default)]
struct AteFoodThisTick(HashSet<Entity>);

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

/// Translate keyboard hotkeys into WorldEventRequest events. Silent while
/// egui has keyboard focus, like every other hotkey, so typing into a text
/// field never fires a world event.
fn keyboard_to_events_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<WorldEventRequest>,
    ui_input: Res<UiInputState>,
) {
    if ui_input.wants_keyboard {
        return;
    }
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

/// Roll the centre of a volcanic eruption uniformly over the whole world,
/// `[0, world_width) x [0, world_height)` in tile units.
pub fn volcano_center(rng: &mut impl Rng, config: &SimConfig) -> Vec2 {
    Vec2::new(
        rng.gen_range(0.0..config.world_width as f32),
        rng.gen_range(0.0..config.world_height as f32),
    )
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
                        timer: DEATH_MARKER_SECS,
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
        let center = volcano_center(rng, &config);
        let (center_x, center_y) = (center.x, center.y);
        let radius = 40.0;

        let mut killed = 0u32;
        for (entity, pos, energy, flows) in &organisms {
            let dist = ((pos.0.x - center_x).powi(2) + (pos.0.y - center_y).powi(2)).sqrt();
            if dist < radius {
                commands.spawn((
                    DeathMarker {
                        timer: DEATH_MARKER_SECS,
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
        chronicle.log_location(
            tick.0,
            format!(
                "VOLCANIC ERUPTION! {} organisms killed near ({:.0}, {:.0})",
                killed, center_x, center_y
            ),
            center,
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
        // The nearest non-photosynthesiser: what a hunter can steer at when
        // the nearest organism overall is a plant.
        let mut nearest_eater_dist = f32::MAX;
        let mut nearest_eater_dir = Vec2::ZERO;
        let mut nearest_eater_size_ratio = 1.0f32;

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

                if dist < nearest_eater_dist && dist < sense_range && !other_genome.is_photosynthesiser() {
                    nearest_eater_dist = dist;
                    nearest_eater_dir = if dist > 0.001 { diff / dist } else { Vec2::ZERO };
                    nearest_eater_size_ratio = other_size.0 / body_size.0;
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
        // Nearest eater, encoded as the nearest-organism inputs are.
        if nearest_eater_dist < f32::MAX {
            inputs[22] = nearest_eater_dir.x;
            inputs[23] = nearest_eater_dir.y;
            inputs[24] = 1.0 - (nearest_eater_dist / sense_range).min(1.0);
            inputs[25] = nearest_eater_size_ratio.min(2.0) / 2.0;
        }

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
            Entity,
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
    mut ate_food: ResMut<AteFoodThisTick>,
) {
    let foods = &food_snapshot.entries;

    // `eaten_food` keeps the despawn order; `eaten` is the per-snapshot-index
    // flag the scan checks, so skipping an eaten item is O(1) rather than a
    // linear search of everything eaten so far this tick.
    let mut eaten_food: Vec<Entity> = Vec::new();
    let mut eaten = vec![false; foods.len()];
    ate_food.0.clear();

    for (
        entity,
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
        flash.timer = (flash.timer - ACTION_FLASH_DECAY_PER_TICK).max(0.0);
        if flash.timer <= 0.0 {
            flash.action = ActionType::None;
        }
        // Update memory
        memory.0 = output.memory_out;
        signal.0 = output.signal.clamp(-1.0, 1.0);

        let move_dir = Vec2::new(output.move_x, output.move_y);
        // Armor slows you down — heavy organisms are slower
        let armor_drag = 1.0 / (1.0 + genome.armor_value() * ARMOR_DRAG);
        // So does a light-catching surface: broad and flat, it is a sail.
        // Nothing forbids a photosynthesiser from moving; a leafy one is
        // slow and a small-leaved one is not. See DECISIONS.md.
        let photo_drag = photo_drag_factor(genome.total_photo_surface_area(), config.photo_drag);
        let speed =
            genome.speed_factor * BASE_MOVE_SPEED / body_size.0.sqrt() * armor_drag * photo_drag;
        let movement = move_dir * speed;
        // Recorded so the history can average movement by strategy.
        velocity.0 = movement;

        let tile = tile_map.tile_at_pos(pos.0);

        let aqua = genome.aquatic_adaptation;
        let fin_bonus = genome.fin_area() * FIN_MOVE_BONUS_PER_AREA;
        let limb_bonus = genome.limb_count() as f32 * LIMB_MOVE_BONUS_PER_LIMB;

        let terrain_cost = if tile.terrain.is_water() {
            let base = tile.terrain.water_move_cost();
            (base
                * (1.0 - aqua * AQUATIC_MOVE_COST_WEIGHT)
                * (1.0 - fin_bonus.min(FIN_MOVE_BONUS_CAP)))
            .max(TERRAIN_MOVE_COST_FLOOR)
        } else {
            let base = tile.terrain.land_move_cost();
            (base
                * (1.0 + aqua * AQUATIC_MOVE_COST_WEIGHT)
                * (1.0 - limb_bonus.min(LIMB_MOVE_BONUS_CAP)))
            .max(TERRAIN_MOVE_COST_FLOOR)
        };

        pos.0 += movement;
        pos.0.x = pos.0.x.rem_euclid(config.world_width as f32);
        pos.0.y = pos.0.y.rem_euclid(config.world_height as f32);

        let move_cost =
            movement.length() * config.movement_energy_cost * body_size.0 * terrain_cost;
        energy.0 -= move_cost;
        ledger.tick.movement += move_cost as f64;

        // Eating food. A living plant is bitten through the same output in
        // grazing_system, which runs next and skips anyone fed here.
        if output.eat > 0.0 {
            let mouth_bonus = mouth_bonus(genome);
            let eat_range = body_size.0 * FOOD_EAT_REACH;
            for (index, &(food_entity, food_pos, food_energy)) in foods.iter().enumerate() {
                if eaten[index] {
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
                    eaten[index] = true;
                    eaten_food.push(food_entity);
                    ate_food.0.insert(entity);
                    flash.action = ActionType::Eating;
                    flash.timer = ACTION_FLASH_SECS;
                    break;
                }
            }
        }
    }

    for food_entity in eaten_food {
        commands.entity(food_entity).try_despawn();
    }
}

/// A neighbour that passed every gate for one attacker (or eater) this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
struct StrikeCandidate {
    entity: Entity,
    /// Distance from the attacker.
    dist: f32,
    /// A photosynthesiser: a kill of it is digested as plant tissue.
    is_plant: bool,
    /// The target's energy when it was gated.
    energy: f32,
}

/// The candidate an attacker strikes, or an eater bites: the nearest one.
/// Plants and non-plants compete on distance alone. Ties go to the earlier
/// candidate, so list order only matters between equals.
fn nearest_target(candidates: &[StrikeCandidate]) -> Option<&StrikeCandidate> {
    candidates
        .iter()
        .fold(None, |best: Option<&StrikeCandidate>, c| match best {
            Some(b) if b.dist <= c.dist => Some(b),
            _ => Some(c),
        })
}

/// Grazing: the `eat` output bites the nearest living photosynthesiser
/// within `bite_reach × body size`. The bite is `graze_bite` (the plant's
/// energy times `bite_fraction` times the eater's mouth bonus); the plant
/// loses it and keeps its health, and the eater keeps it times its
/// `plant_efficiency`. No claw or size gate: the mouth is the grazer's tool.
/// An eater that ate a food item in `action_system` this tick does not also
/// bite, and a plant takes one bite per tick, first eater wins, the rule
/// kills use. See DECISIONS.md, "Grazing through eat".
///
/// A system of its own, between action and predation, rather than a branch
/// of `action_system`: biting reads other organisms' energy and genome,
/// which `action_system`'s one mutable query cannot do while iterating.
fn grazing_system(
    spatial_hash: Res<SpatialHash>,
    config: Res<SimConfig>,
    ate_food: Res<AteFoodThisTick>,
    mut organisms: Query<
        (
            Entity,
            &Position,
            &mut Energy,
            &Health,
            &mut ActionFlash,
            &Genome,
            &BodySize,
            &BrainOutput,
        ),
        With<Organism>,
    >,
    mut predation_stats: ResMut<PredationStats>,
    mut ledger: ResMut<EnergyLedger>,
) {
    // (eater, position, reach, mouth bonus, eater is a plant)
    let eaters: Vec<(Entity, Vec2, f32, f32, bool)> = organisms
        .iter()
        .filter(|(e, _, _, _, _, _, _, output)| output.eat > 0.0 && !ate_food.0.contains(e))
        .map(|(e, pos, _, _, _, genome, body_size, _)| {
            (
                e,
                pos.0,
                body_size.0 * config.bite_reach,
                bite_mouth_bonus(genome, config.mouthless_bite_bonus),
                genome.is_photosynthesiser(),
            )
        })
        .collect();

    // (eater, plant, bite)
    let mut bites: Vec<(Entity, Entity, f32)> = Vec::new();
    let mut claimed_plants: HashSet<Entity> = HashSet::new();
    let mut candidates: Vec<StrikeCandidate> = Vec::new();

    for (eater, pos, reach, bonus, eater_is_plant) in &eaters {
        candidates.clear();
        for target in spatial_hash.query_radius(*pos, *reach) {
            if target == *eater || claimed_plants.contains(&target) {
                continue;
            }
            let Ok((_, target_pos, target_energy, target_health, _, target_genome, _, _)) =
                organisms.get(target)
            else {
                continue;
            };
            if target_health.0 <= 0.0 || !target_genome.is_photosynthesiser() {
                continue;
            }
            // The hash was built before action_system moved everyone, so
            // re-check the real distance, as the food reach does.
            let dist = (target_pos.0 - *pos).length();
            if dist >= *reach {
                continue;
            }
            candidates.push(StrikeCandidate {
                entity: target,
                dist,
                is_plant: true,
                energy: target_energy.0,
            });
        }
        if let Some(plant) = nearest_target(&candidates) {
            bites.push((
                *eater,
                plant.entity,
                graze_bite(plant.energy, config.bite_fraction, *bonus),
            ));
            claimed_plants.insert(plant.entity);
            predation_stats.feeding.grazes_eat += 1;
            if *eater_is_plant {
                predation_stats.feeding.grazes_eat_by_plant += 1;
            }
        }
    }

    for (eater, plant, bite) in bites {
        let Ok((_, _, _, _, _, eater_genome, _, _)) = organisms.get(eater) else {
            continue;
        };
        let (kept, wasted) = digest(bite, eater_genome.plant_efficiency());
        let Ok((_, _, mut plant_energy, _, _, _, _, _)) = organisms.get_mut(plant) else {
            continue;
        };
        // The plant keeps its health and is not marked Killed. If the bite
        // empties it, death_system reads that as any other energy loss.
        plant_energy.0 -= bite;
        if let Ok((_, _, mut eater_energy, _, mut eater_flash, _, _, _)) = organisms.get_mut(eater)
        {
            ledger.tick.clamp +=
                credit_clamped(&mut eater_energy, kept, config.max_organism_energy) as f64;
            ledger.tick.grazing += kept as f64;
            ledger.tick.digestion += wasted as f64;
            eater_flash.action = ActionType::Grazing;
            eater_flash.timer = ACTION_FLASH_SECS;
        }
    }
}

/// Energy one tick of a strike costs: `rate` per unit of strike force
/// (`claw_power × body size`). A heavier strike costs more; an organism with
/// no claws strikes for free, and cannot pass the damage gate either.
pub fn strike_cost(attack_strength: f32, rate: f32) -> f32 {
    (attack_strength * rate).max(0.0)
}

/// Which instrumented bands an attacker falls in, for the gate counters of
/// step 5 of `plans/2026-09-21-pyramid-top.md`. Read only by counters.
#[derive(Clone, Copy, Debug, Default)]
struct AttackerBand {
    /// A consumer (not a photosynthesiser) with `diet >= 0`.
    diet_nonneg: bool,
    /// Labelled hunter: a consumer with `diet >= 1/3`.
    hunter: bool,
    /// A hunter of generation 0.
    founder_hunter: bool,
    /// The attacker's age in ticks.
    age: u64,
}

impl AttackerBand {
    fn of(genome: &Genome, lineage: Option<(&Age, &Generation)>) -> Self {
        let consumer = !genome.is_photosynthesiser();
        let hunter = classify_strategy(genome) == SpeciesStrategy::Hunter;
        let (age, generation) = lineage.map(|(a, g)| (a.0, g.0)).unwrap_or((0, u32::MAX));
        Self {
            diet_nonneg: consumer && genome.diet >= 0.0,
            hunter,
            founder_hunter: hunter && generation == 0,
            age,
        }
    }

    fn tracked(&self) -> bool {
        self.diet_nonneg
    }
}

/// Which gates the consumers in an attacker's reach passed.
#[derive(Clone, Copy, Debug)]
struct ConsumerGates {
    any_size: bool,
    any_damage: bool,
    any_both: bool,
    struck_consumer: bool,
}

/// Count one attack by a tracked attacker into its bands' gate counters
/// (step 5 of `plans/2026-09-21-pyramid-top.md`). Counters only.
fn record_band_attack(
    stats: &mut PredationStats,
    band: &AttackerBand,
    anyone_in_reach: bool,
    consumers: Option<ConsumerGates>,
    attacker: Entity,
) {
    let bump = |g: &mut GateOutcomes| {
        g.intents += 1;
        if anyone_in_reach {
            g.strikes += 1;
        }
        if let Some(c) = consumers {
            g.record_consumer_attack(c.any_size, c.any_damage, c.any_both, c.struck_consumer);
        }
    };
    bump(&mut stats.diet_nonneg_gates);
    if band.hunter {
        bump(&mut stats.feeding.hunter_gates);
        let bucket = age_bucket(band.age);
        stats.hunter_intent_ages[bucket] += 1;
        if consumers.is_some() {
            stats.hunter_reach_ages[bucket] += 1;
        }
    }
    if band.founder_hunter {
        bump(&mut stats.founder_hunter_gates);
        if stats.founder_hunters_fired.insert(attacker) {
            stats.founder_hunter_first_intent_age_sum += band.age;
        }
        if consumers.is_some() && stats.founder_hunters_reached.insert(attacker) {
            stats.founder_hunter_first_reach_age_sum += band.age;
        }
        if consumers.is_some_and(|c| c.any_both && c.struck_consumer) {
            stats.founder_hunters_killed.insert(attacker);
        }
    }
}

/// Predation: organisms can attack and eat each other. Every strike is a
/// kill attempt, plant or animal, under the same size and damage gates;
/// grazing is `eat`'s job (`grazing_system`).
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
    lineage: Query<(&Age, &Generation), With<Organism>>,
    mut commands: Commands,
    mut predation_stats: ResMut<PredationStats>,
    mut ledger: ResMut<EnergyLedger>,
) {
    // Collect attack intents
    let attackers: Vec<(Entity, Vec2, f32, f32, f32, AttackerBand)> = organisms
        .iter()
        .filter(|(_, _, _, _, _, _, _, output)| output.attack > ATTACK_INTENT_THRESHOLD)
        .map(|(e, pos, _, _, _, genome, body_size, _)| {
            let attack_str = genome.claw_power() * body_size.0;
            let attack_range = body_size.0 * ATTACK_REACH;
            let band = AttackerBand::of(genome, lineage.get(e).ok());
            (e, pos.0, attack_str, attack_range, body_size.0, band)
        })
        .collect();
    predation_stats.attacks_attempted += attackers.len() as u64;

    // (killer, victim, victim_energy, victim_is_plant) — energy transfer
    // computed at kill time. A victim is claimed at most once per tick: the
    // first attacker to land a kill takes the energy transfer, and later
    // attackers skip that target and keep scanning. Without this, several
    // attackers could each be paid 10% of the same victim's energy and each
    // count a kill. See DECISIONS.md.
    let mut kills: Vec<(Entity, Entity, f32, bool)> = Vec::new();
    let mut claimed_victims: HashSet<Entity> = HashSet::new();
    let mut candidates: Vec<StrikeCandidate> = Vec::new();
    // (attacker, energy owed) for every attacker that struck at something
    // this tick; charged after the kills are resolved.
    let mut strike_costs: Vec<(Entity, f32)> = Vec::new();

    for (attacker_entity, attacker_pos, attack_str, attack_range, attacker_size, band) in &attackers
    {
        let nearby = spatial_hash.query_radius(*attacker_pos, *attack_range);
        candidates.clear();
        // Instrument only: which gates the unclaimed consumers in reach
        // passed (step 5 of plans/2026-09-21-pyramid-top.md).
        let mut consumer_in_reach = false;
        let mut consumer_size_ok = false;
        let mut consumer_damage_ok = false;
        let mut consumer_both_ok = false;
        // Instrument only: whether any living plant was within reach,
        // claimed or not. It does not affect which target is struck.
        let mut plant_in_reach = false;
        // Whether anything alive was within reach: a strike, not a flail.
        let mut anyone_in_reach = false;

        for &target_entity in &nearby {
            if target_entity == *attacker_entity {
                continue;
            }
            let claimed = claimed_victims.contains(&target_entity);

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
                let is_plant = target_genome.is_photosynthesiser();
                plant_in_reach |= is_plant;
                anyone_in_reach = true;
                if claimed {
                    continue;
                }

                predation_stats.targets_considered += 1;

                let defense = target_genome.armor_value() * target_body_size.0;
                let damage = (attack_str - defense * DEFENCE_WEIGHT).max(0.0);
                // A plant is prey like any other: the size gate applies.
                let size_ok = *attacker_size > target_body_size.0 * PREY_SIZE_RATIO;
                let damage_ok = damage > MIN_KILL_DAMAGE;

                if !size_ok {
                    predation_stats.rejected_size_gate += 1;
                }
                if !damage_ok {
                    predation_stats.rejected_damage += 1;
                }
                if !is_plant {
                    consumer_in_reach = true;
                    consumer_size_ok |= size_ok;
                    consumer_damage_ok |= damage_ok;
                    consumer_both_ok |= size_ok && damage_ok;
                }

                if damage_ok && size_ok {
                    candidates.push(StrikeCandidate {
                        entity: target_entity,
                        dist,
                        is_plant,
                        energy: target_energy.0,
                    });
                }
            }
        }

        // The neighbour list is in hash bucket order, not distance order, so
        // the first passing neighbour is an arbitrary one. The attacker
        // strikes the nearest. See DECISIONS.md.
        if !plant_in_reach {
            predation_stats.feeding.attacks_no_plant_in_reach += 1;
        }
        if anyone_in_reach {
            strike_costs.push((
                *attacker_entity,
                strike_cost(*attack_str, config.strike_cost),
            ));
        }
        let struck = nearest_target(&candidates);
        if band.tracked() {
            record_band_attack(
                &mut predation_stats,
                band,
                anyone_in_reach,
                consumer_in_reach.then_some(ConsumerGates {
                    any_size: consumer_size_ok,
                    any_damage: consumer_damage_ok,
                    any_both: consumer_both_ok,
                    struck_consumer: struck.is_some_and(|t| !t.is_plant),
                }),
                *attacker_entity,
            );
        }
        if let Some(target) = struck {
            kills.push((
                *attacker_entity,
                target.entity,
                target.energy,
                target.is_plant,
            ));
            claimed_victims.insert(target.entity);
        }
    }

    predation_stats.kills += kills.len() as u64;

    for (killer, victim, victim_energy_before, victim_is_plant) in kills {
        let Ok((_, _, _, _, _, killer_genome, _, _)) = organisms.get(killer) else {
            continue;
        };
        // Energy pyramid: the killer is offered a share of the prey's stored
        // energy, set by the prey's tissue (`SimConfig::kill_share`; most is
        // lost as heat), and keeps what it can digest
        // of that tissue: plant at plant efficiency, animal at animal
        // efficiency. The undigested share is booked to digestion and the
        // rest of the victim's energy to death.
        let (energy_gained, wasted) = digest(
            victim_energy_before * config.kill_share(victim_is_plant),
            kill_digestion_efficiency(
                killer_genome,
                victim_is_plant,
                config.animal_efficiency_multiplier,
            ),
        );
        predation_stats.feeding.record_kill(
            killer_genome.diet,
            killer_genome.is_photosynthesiser(),
            victim_is_plant,
            energy_gained,
        );
        if victim_is_plant && classify_strategy(killer_genome) == SpeciesStrategy::Hunter {
            predation_stats.hunter_plant_kills += 1;
            predation_stats.hunter_plant_kill_energy += energy_gained as f64;
        }
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
            killer_flash.timer = ACTION_FLASH_SECS;
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

    // A strike costs the attacker whether it landed, bounced or lost its
    // target to another attacker; the cost does not read what the target
    // was. An attacker killed this tick has already left the ledger, so it
    // is not charged. Booked as movement: a strike is muscular work.
    predation_stats.strikes += strike_costs.len() as u64;
    for (attacker, cost) in strike_costs {
        if cost <= 0.0 || claimed_victims.contains(&attacker) {
            continue;
        }
        if let Ok((_, _, mut attacker_energy, _, _, _, _, _)) = organisms.get_mut(attacker) {
            attacker_energy.0 -= cost;
            ledger.tick.movement += cost as f64;
            predation_stats.strike_energy += cost as f64;
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
            tile.vegetation_density = (tile.vegetation_density + NICHE_VEGETATION_DEPOSIT).min(1.0);
            tile.moisture = (tile.moisture + NICHE_MOISTURE_DEPOSIT).min(1.0);
        }

        // All organisms slightly increase nutrients (waste products)
        tile.nutrients = (tile.nutrients + NICHE_NUTRIENT_DEPOSIT).min(1.0);
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

        if infection_pressure <= DISEASE_TRANSMISSION_PRESSURE_FLOOR {
            continue;
        }

        // Per-tick infection chance, reduced by resistance, capped.
        let chance =
            (infection_pressure * DISEASE_TRANSMISSION_RATE * (1.0 - genome.disease_resistance))
                .min(DISEASE_TRANSMISSION_CHANCE_CAP);
        if rng.gen::<f32>() < chance {
            // Inherit roughly the strain's severity & duration, slightly weakened.
            commands.entity(entity).insert(Infection {
                severity: (best_severity * DISEASE_TRANSMISSION_SEVERITY_DECAY).clamp(
                    DISEASE_TRANSMISSION_SEVERITY_MIN,
                    DISEASE_TRANSMISSION_SEVERITY_MAX,
                ),
                ticks_remaining: (best_remaining * DISEASE_TRANSMISSION_DURATION_RETAINED_PERCENT
                    / 100)
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
        let drain_factor = (1.0 - res * DISEASE_DRAIN_RESISTANCE_WEIGHT).powi(2);
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

/// Count every `DeathMarker` down by one fixed timestep and despawn the
/// expired ones. Runs in the sim so headless runs, which have no render
/// crate, do not accumulate a marker per death for the rest of the run.
/// Scheduled before `death_system`, so a marker spawned this tick is first
/// aged on the next one and lives `DEATH_MARKER_SECS` of virtual time.
fn death_marker_expiry_system(
    mut commands: Commands,
    time: Res<Time>,
    mut markers: Query<(Entity, &mut DeathMarker)>,
) {
    let dt = time.delta_secs();
    for (entity, mut marker) in &mut markers {
        marker.timer -= dt;
        if marker.timer <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
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
            Option<(&Genome, &Generation)>,
        ),
        With<Organism>,
    >,
    mut stats: ResMut<SimStats>,
    mut fitness: ResMut<FitnessTracker>,
    mut ledger: ResMut<EnergyLedger>,
) {
    for (entity, energy, health, pos, age, flows, infection, killed, lineage) in &organisms {
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
            if let Some((genome, generation)) = lineage {
                if generation.0 == 0 && classify_strategy(genome) == SpeciesStrategy::Hunter {
                    stats.founder_hunter_deaths += 1;
                    stats.founder_hunter_age_sum += age.0;
                    stats.founder_hunter_age_max = stats.founder_hunter_age_max.max(age.0);
                }
            }

            // Spawn death marker before despawning
            commands.spawn((
                DeathMarker {
                    timer: DEATH_MARKER_SECS,
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

    // (position, genome, parent species, generation, starting energy)
    let mut new_organisms: Vec<(Vec2, Genome, u64, u32, f32)> = Vec::new();
    // (parent, reproduction cost): the energy deduction and action flash
    // each parent receives once the read-only pass below has released the
    // query.
    let mut parents: Vec<(Entity, f32)> = Vec::new();
    let current_pop = organisms.iter().len();
    // `population_ceiling` is the only birth limiter. The population is
    // meant to settle where the energy flows put it, and under the current
    // rules it does not (see DECISIONS.md "Emergent carrying capacity"), so
    // every engagement is recorded: the chronicle and the headless summary
    // say how often the ceiling was the thing deciding the population.
    let ceiling = config.population_ceiling as usize;
    let mut blocked_births = 0u64;
    // Membership only, never iterated, so the set has no bearing on the
    // deterministic order in which parents are visited and the RNG is drawn.
    let mut already_mated: HashSet<Entity> = HashSet::new();

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
            output.reproduce > REPRODUCE_INTENT_THRESHOLD
                && energy.0 > reproduction_threshold(&config, body_size.0)
        })
        .count();
    let admit_probability = if wanting > slots {
        slots as f32 / wanting as f32
    } else {
        1.0
    };

    // The mate search borrows the mate's genome straight from the query, so
    // this pass is read-only: it decides every birth, draws from the RNG in
    // parent order, and books the ledger, while the parents' own energy and
    // flash writes wait for the pass after it. Cloning a genome per
    // candidate up front was the alternative, and genomes are large.
    for (entity, pos, energy, _, genome, output, body_size, species, generation) in &organisms {
        if already_mated.contains(&entity) {
            continue;
        }
        // Reproduction cost scales with body size — small organisms can't reproduce for free
        let repro_cost = config.reproduction_energy_cost
            * (REPRODUCTION_SIZE_SCALE_BASE + body_size.0 * REPRODUCTION_SIZE_SCALE_PER_SIZE);
        let repro_threshold = reproduction_threshold(&config, body_size.0);
        let wants_child =
            output.reproduce > REPRODUCE_INTENT_THRESHOLD && energy.0 > repro_threshold;
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
            ledger.tick.reproduction_spent += repro_cost as f64;
            parents.push((entity, repro_cost));

            // Try to find a mate: a nearby organism of the same species that
            // also wants a child and has the energy for one. Its energy is
            // read as it stood at the start of the tick, before any parent
            // in this pass is charged.
            let mate_range = body_size.0 * MATE_SEARCH_REACH;
            let nearby = spatial_hash.query_radius(pos.0, mate_range);
            let mut mate_genome: Option<&Genome> = None;

            for &nearby_entity in &nearby {
                if nearby_entity == entity || already_mated.contains(&nearby_entity) {
                    continue;
                }
                let Ok((_, _, mate_energy, _, mate_g, mate_output, _, mate_species, _)) =
                    organisms.get(nearby_entity)
                else {
                    continue;
                };
                if mate_output.reproduce > REPRODUCE_INTENT_THRESHOLD
                    && mate_species.0 == species.0
                    && mate_energy.0 > config.reproduction_energy_threshold
                {
                    mate_genome = Some(mate_g);
                    already_mated.insert(nearby_entity);
                    break;
                }
            }

            let mut child_genome = if let Some(mate_g) = mate_genome {
                genome.crossover(mate_g, &mut rng)
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

            let offset = Vec2::new(
                rng.gen_range(-CHILD_SPAWN_OFFSET..CHILD_SPAWN_OFFSET),
                rng.gen_range(-CHILD_SPAWN_OFFSET..CHILD_SPAWN_OFFSET),
            );
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
            already_mated.insert(entity);
        }
    }

    for (entity, repro_cost) in parents {
        if let Ok((_, _, mut energy, mut flash, ..)) = organisms.get_mut(entity) {
            energy.0 -= repro_cost;
            flash.action = ActionType::Reproducing;
            flash.timer = ACTION_FLASH_SECS;
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

/// The species an organism belongs to this pass, or `None` if it fits no
/// existing species and must found a new one.
///
/// An organism with a current species keeps it while its distance to that
/// species' representative is below `stay_threshold`, even when another
/// species' representative is closer. Past that it joins the nearest other
/// species within `join_threshold`. Before 2026-09-23 the current species was
/// only preferred through its looser threshold, and the organism went to
/// whichever eligible representative was nearest; with the distance ceiling
/// between unrelated genomes near the join threshold, that moved about 45% of
/// all organisms to a different species on every pass. See `docs/DECISIONS.md`,
/// "Species classification".
fn choose_species(
    genome: &Genome,
    current_species: u64,
    species_reps: &[(u64, Genome)],
    join_threshold: f32,
    stay_threshold: f32,
) -> Option<u64> {
    if current_species > 0 {
        if let Some((_, rep)) = species_reps.iter().find(|(id, _)| *id == current_species) {
            if genome.compatibility_distance(rep) < stay_threshold {
                return Some(current_species);
            }
        }
    }
    let mut best_species = None;
    let mut best_dist = f32::MAX;
    for (species_id, rep_genome) in species_reps {
        if *species_id == current_species {
            continue;
        }
        let dist = genome.compatibility_distance(rep_genome);
        if dist < join_threshold && dist < best_dist {
            best_dist = dist;
            best_species = Some(*species_id);
        }
    }
    best_species
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
    // Start past every id the tree has ever held, extinct species included:
    // `record_species` ignores an id it already has, so a reused id would
    // fold the newcomer into the dead species' record. The scan below still
    // raises it past any living id missing from the tree.
    let mut next_species_id = phylo.next_species_id();

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

    // An organism stays in its current species while it is within the stay
    // threshold of that species' representative; only past it does it look
    // for another species (within the join threshold) or found a new one.
    let join_threshold = config.species_compat_threshold;
    let stay_threshold = join_threshold * SPECIES_HYSTERESIS_FACTOR;

    for (entity, genome, _old_species) in &org_data {
        let best_species = choose_species(
            genome,
            *_old_species,
            &species_reps,
            join_threshold,
            stay_threshold,
        );

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
            chronicle.log_species(
                tick.0,
                format!("New species: {}{}", species_name, parent_str),
                new_id,
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
                chronicle.log_species(
                    tick.0,
                    format!(
                        "{} went extinct (peak: {}, lived {}s)",
                        node.name, node.peak_population, age_secs
                    ),
                    *species_id,
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
    predation: Res<PredationStats>,
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
    // Grazer traits for the headless summary's grazer timeline.
    let mut sum_grazer_body = 0.0f32;
    let mut sum_grazer_armor = 0.0f32;

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
            SpeciesStrategy::Grazer => {
                grazers += 1;
                sum_grazer_body += genome.body_size;
                sum_grazer_armor += genome.armor_value();
            }
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
        &predation,
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
            avg_grazer_body_size: sum_grazer_body / grazers.max(1) as f32,
            avg_grazer_armor: sum_grazer_armor / grazers.max(1) as f32,
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
    config.reproduction_energy_threshold
        * (REPRODUCTION_SIZE_SCALE_BASE + body_size * REPRODUCTION_SIZE_SCALE_PER_SIZE)
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
    /// Founders whose genomes came from creature files (`--seed-with`),
    /// counted in the biome and strategy totals as well.
    pub imported: u32,
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
            "Founders by biome: {}; strategies: {} plants, {} grazers, {} hunters, {} omnivores; imported {}; starting energy {:.1}..{:.1}",
            by_biome.join(", "),
            self.plants,
            self.grazers,
            self.hunters,
            self.omnivores,
            self.imported,
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
///
/// `imported` genomes (from `--seed-with` creature files) become extra
/// founders on top of `config.initial_population`. They take the same path
/// as every other founder: a biome slot from the same allocation, a tile
/// and offset drawn from the same `rng`, the same starting energy rule and
/// the same component set. Their NEAT innovation numbers come from another
/// world, so the counter is first raised past the largest of them; the
/// generated founders then cannot reuse a number an import already holds.
/// With no imports the RNG sequence and the counter are exactly as before.
pub fn spawn_initial_population(
    commands: &mut Commands,
    config: &SimConfig,
    tile_map: &TileMap,
    innovation: &mut InnovationCounter,
    imported: &[Genome],
    rng: &mut impl Rng,
) -> FounderReport {
    use rand::seq::SliceRandom;

    let photo_count = config.initial_population / 3; // 30% photosynthesizers
    let total_founders = config.initial_population + imported.len() as u32;
    let mut total_energy = 0.0f64;

    if let Some(max_innovation) = imported
        .iter()
        .flat_map(|g| g.connections.iter().map(|c| c.innovation))
        .max()
    {
        innovation.0 = innovation.0.max(max_innovation + 1);
    }

    // Tile indices per founding biome.
    let mut biome_tiles: Vec<Vec<u32>> = vec![Vec::new(); FOUNDING_BIOMES.len()];
    for (idx, tile) in tile_map.tiles.iter().enumerate() {
        if let Some(b) = FOUNDING_BIOMES.iter().position(|t| *t == tile.terrain) {
            biome_tiles[b].push(idx as u32);
        }
    }
    let areas: Vec<usize> = biome_tiles.iter().map(Vec::len).collect();
    let counts = founder_allocation(&areas, total_founders);

    // One biome slot per founder, shuffled so the photosynthesiser share
    // (and any imports) lands in every biome rather than in whichever
    // biome is listed first.
    let mut slots: Vec<Option<usize>> = Vec::with_capacity(total_founders as usize);
    for (b, &n) in counts.iter().enumerate() {
        slots.extend(std::iter::repeat_n(Some(b), n as usize));
    }
    // No land at all: fall back to uniform placement.
    slots.resize(total_founders as usize, None);
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

        // Generated founders first, imports in the trailing slots; the
        // shuffle above already decided which biome each slot is in.
        let genome = if (i as u32) < photo_count {
            Genome::new_photosynthesizer_with_diet(innovation, rng, config.founder_diet_spread)
        } else if (i as u32) < config.initial_population {
            Genome::new_minimal_with_diet(innovation, rng, config.founder_diet_spread)
        } else {
            imported[i - config.initial_population as usize].clone()
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
        imported: imported.len() as u32,
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
    tile_map: Option<Res<TileMap>>,
    mut chronicle: ResMut<WorldChronicle>,
    mut report: ResMut<SaveReport>,
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
    let result = save::save_world(
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
        tile_map.as_deref(),
    );

    // Surface the outcome everywhere the user might be looking: the log,
    // the chronicle panel, and the SaveReport resource the headless runner
    // prints from. A failed write costs one save, not the running world.
    match &result {
        Ok(()) => {
            info!("World saved to {}", save_path.display());
            chronicle.log(tick.0, format!("World saved to {}", save_path.display()));
        }
        Err(e) => {
            error!("Save failed: {}", e);
            chronicle.log(tick.0, format!("Save failed: {}", e));
        }
    }
    report.last = Some(result.map(|()| save_path).map_err(|e| e.to_string()));
}

/// Outcome of the most recent save request, for callers outside the sim
/// (the headless runner) that cannot see the log. `None` until a save has
/// been attempted this run.
#[derive(Resource, Default)]
pub struct SaveReport {
    /// The path written on success, or the error text on failure.
    pub last: Option<Result<std::path::PathBuf, String>>,
}

/// The Inspect panel's export button: write the selected organism's genome
/// and provenance to `<session>/<species>-t<tick>.json` (see
/// `save::creature_file_name`). Like `save_system`, the outcome goes to the
/// log, the chronicle and a report resource the panel reads; a failed
/// write costs one export, not the running world.
fn export_organism_system(
    mut events: EventReader<WorldEventRequest>,
    session: Res<Session>,
    tick: Res<TickCounter>,
    config: Res<SimConfig>,
    organisms: Query<(&Genome, &Generation, &SpeciesId), With<Organism>>,
    phylo: Res<PhyloTree>,
    mut chronicle: ResMut<WorldChronicle>,
    mut report: ResMut<OrganismExportReport>,
) {
    for entity in events.read().filter_map(|r| match r {
        WorldEventRequest::ExportOrganism(e) => Some(*e),
        _ => None,
    }) {
        let Ok((genome, generation, species)) = organisms.get(entity) else {
            let msg = "the selected organism is no longer alive".to_string();
            warn!("Export failed: {msg}");
            report.last = Some(Err(msg));
            continue;
        };
        let species_name = phylo.nodes.get(&species.0).map(|n| n.name.as_str());
        let creature = save::CreatureFile::new(
            genome,
            species_name,
            classify_strategy(genome).label(),
            generation.0,
            tick.0,
            config.terrain_seed,
            &session.name,
        );
        let path = session
            .dir
            .join(save::creature_file_name(species_name, entity, tick.0));
        let result = save::export_creature(&path, &creature);
        match &result {
            Ok(()) => {
                info!("Creature exported to {}", path.display());
                chronicle.log(
                    tick.0,
                    format!(
                        "Exported {} to {}",
                        species_name.unwrap_or("an unnamed organism"),
                        path.display()
                    ),
                );
            }
            Err(e) => {
                error!("Export failed: {e}");
                chronicle.log(tick.0, format!("Export failed: {e}"));
            }
        }
        report.last = Some(result.map(|()| path).map_err(|e| e.to_string()));
    }
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

    /// The eruption centre used to be rolled in 0..256 on a 512x512 world,
    /// so it could never land in three quarters of the map. Sample many
    /// centres and check they stay in bounds and reach the far half of both
    /// axes.
    #[test]
    fn volcano_center_covers_the_whole_world() {
        let config = SimConfig::default();
        assert_eq!((config.world_width, config.world_height), (512, 512));
        let mut rng = StdRng::seed_from_u64(7);
        let (mut far_x, mut far_y) = (0u32, 0u32);
        for _ in 0..1000 {
            let c = volcano_center(&mut rng, &config);
            assert!(c.x >= 0.0 && c.x < config.world_width as f32, "{c:?}");
            assert!(c.y >= 0.0 && c.y < config.world_height as f32, "{c:?}");
            far_x += (c.x >= 256.0) as u32;
            far_y += (c.y >= 256.0) as u32;
        }
        // Roughly half of a uniform sample lands beyond the old ceiling.
        assert!((400..=600).contains(&far_x), "far_x = {far_x}");
        assert!((400..=600).contains(&far_y), "far_y = {far_y}");

        // A non-square world is honoured on each axis independently.
        let wide = SimConfig {
            world_width: 1024,
            world_height: 64,
            ..SimConfig::default()
        };
        let mut rng = StdRng::seed_from_u64(7);
        let mut past_default_width = 0u32;
        for _ in 0..1000 {
            let c = volcano_center(&mut rng, &wide);
            assert!(c.x < 1024.0 && c.y < 64.0, "{c:?}");
            past_default_width += (c.x >= 512.0) as u32;
        }
        assert!(past_default_width > 0);
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
            spawn_initial_population(
                &mut commands,
                &config,
                &tile_map,
                &mut innovation,
                &[],
                &mut rng,
            )
        };
        queue.apply(&mut world);
        println!("{report}");
        assert_eq!(
            report.plants + report.grazers + report.hunters + report.omnivores,
            config.initial_population
        );
        assert_eq!(report.imported, 0);
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

    /// Imported genomes are extra founders: the population grows by their
    /// number, each one is present with its genome intact and the same
    /// components as any founder, and the innovation counter has moved
    /// past every number an import carries. A run with no imports draws
    /// the same RNG sequence as before this parameter existed.
    #[test]
    fn imported_genomes_join_the_founders_through_the_same_path() {
        let config = SimConfig {
            terrain_seed: 42,
            ..SimConfig::default()
        };
        let mut terrain_rng = StdRng::seed_from_u64(config.terrain_seed);
        let tile_map = TileMap::generate(config.world_width, config.world_height, &mut terrain_rng);

        // Genomes "from another world": innovation numbers well past what
        // this world's founders would draw, and a distinctive diet.
        let mut foreign_innovation = InnovationCounter(50_000);
        let mut foreign_rng = StdRng::seed_from_u64(9);
        let imported: Vec<Genome> = (0..3)
            .map(|_| {
                let mut g =
                    Genome::new_minimal_with_diet(&mut foreign_innovation, &mut foreign_rng, 1.0);
                g.diet = 0.987;
                g
            })
            .collect();
        let max_import_innovation = imported
            .iter()
            .flat_map(|g| g.connections.iter().map(|c| c.innovation))
            .max()
            .unwrap();

        let mut innovation = InnovationCounter(100);
        let mut rng = StdRng::seed_from_u64(config.terrain_seed);
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let report = {
            let mut commands = Commands::new(&mut queue, &world);
            spawn_initial_population(
                &mut commands,
                &config,
                &tile_map,
                &mut innovation,
                &imported,
                &mut rng,
            )
        };
        queue.apply(&mut world);

        let expected_total = config.initial_population + 3;
        assert_eq!(report.imported, 3);
        assert_eq!(
            report.plants + report.grazers + report.hunters + report.omnivores,
            expected_total
        );
        assert_eq!(
            report.by_biome.iter().map(|(_, n, _)| *n).sum::<u32>(),
            expected_total
        );
        assert!(innovation.0 > max_import_innovation);

        let mut query = world.query_filtered::<(
            &Position,
            &Energy,
            &BodySize,
            &Genome,
            &Generation,
            &SpeciesId,
        ), (
            With<Organism>,
            With<Brain>,
            With<EnergyFlows>,
            With<LightShare>,
        )>();
        let mut total = 0u32;
        let mut found_imports = 0u32;
        for (pos, energy, body, genome, generation, species) in query.iter(&world) {
            total += 1;
            assert!(!tile_map.tile_at_pos(pos.0).terrain.is_water());
            assert!(energy.0 < reproduction_threshold(&config, body.0));
            assert_eq!((generation.0, species.0), (0, 0));
            if genome.diet == 0.987 {
                found_imports += 1;
                assert!(genome.connections.iter().all(|c| c.innovation >= 50_000));
            }
        }
        assert_eq!(total, expected_total);
        assert_eq!(found_imports, 3);

        // No imports: the same seed gives the same founders as before the
        // parameter existed, so headless determinism is untouched.
        let positions = |imported: &[Genome]| -> Vec<(f32, f32)> {
            let mut innovation = InnovationCounter(100);
            let mut rng = StdRng::seed_from_u64(config.terrain_seed);
            let mut world = World::new();
            let mut queue = CommandQueue::default();
            {
                let mut commands = Commands::new(&mut queue, &world);
                spawn_initial_population(
                    &mut commands,
                    &config,
                    &tile_map,
                    &mut innovation,
                    imported,
                    &mut rng,
                );
            }
            queue.apply(&mut world);
            let mut q = world.query_filtered::<&Position, With<Organism>>();
            q.iter(&world).map(|p| (p.0.x, p.0.y)).collect()
        };
        assert_eq!(positions(&[]), positions(&[]));
        assert_ne!(positions(&[]), positions(&imported));
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
        // A bite is `bite_fraction` of what the plant holds (with a mouth),
        // a kill offers `kill_share` of the victim's tissue; both are then
        // digested.
        let plant_energy = 80.0;
        let bite = graze_bite(plant_energy, SimConfig::default().bite_fraction, 1.0);
        let (kept, wasted) = digest(bite, 1.0);
        assert!((kept - bite).abs() < 1e-5 && wasted.abs() < 1e-5);
        assert!(bite > 0.0 && bite < plant_energy);
        let (kept, wasted) = digest(plant_energy * SimConfig::default().kill_share(true), 0.25);
        assert!((kept - 2.0).abs() < 1e-5 && (wasted - 6.0).abs() < 1e-5);
    }
}

#[cfg(test)]
mod reproduction_tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use rand::{rngs::StdRng, SeedableRng};

    /// A world holding only what `reproduction_system` reads and writes.
    fn reproduction_world() -> World {
        let mut world = World::new();
        world.insert_resource(SimConfig::default());
        world.insert_resource(InnovationCounter(0));
        world.insert_resource(SpatialHash::new(32.0));
        world.insert_resource(SimStats::default());
        world.insert_resource(BloomEffects::default());
        world.insert_resource(SimRng::from_seed(7));
        world.insert_resource(EnergyLedger::default());
        world.insert_resource(TickCounter(0));
        world.insert_resource(WorldChronicle::default());
        world
    }

    fn spawn_adult(world: &mut World, pos: Vec2, energy: f32, reproduce: f32) -> Entity {
        let mut innovation = InnovationCounter(0);
        let mut rng = StdRng::seed_from_u64(1);
        let genome = Genome::new_minimal(&mut innovation, &mut rng);
        let entity = world
            .spawn((
                Organism,
                Position(pos),
                Energy(energy),
                ActionFlash::default(),
                genome,
                BrainOutput {
                    reproduce,
                    ..Default::default()
                },
                BodySize(1.0),
                SpeciesId(1),
                Generation(3),
            ))
            .id();
        world.resource_mut::<SpatialHash>().insert(entity, pos);
        entity
    }

    fn organism_count(world: &mut World) -> usize {
        world
            .query_filtered::<(), With<Organism>>()
            .iter(world)
            .len()
    }

    /// Two willing neighbours of one species produce one child: whichever is
    /// visited first pays the reproduction cost and flashes, the other is its
    /// mate, keeps its energy, and is not visited as a parent in its turn.
    #[test]
    fn a_pair_produces_one_child_and_only_the_parent_pays() {
        let mut world = reproduction_world();
        let a = spawn_adult(&mut world, Vec2::new(10.0, 10.0), 200.0, 1.0);
        let b = spawn_adult(&mut world, Vec2::new(12.0, 10.0), 200.0, 1.0);
        let cost = SimConfig::default().reproduction_energy_cost;

        world.run_system_once(reproduction_system).unwrap();

        assert_eq!(organism_count(&mut world), 3);
        assert_eq!(world.resource::<SimStats>().total_births, 1);
        let energies = [a, b].map(|e| world.get::<Energy>(e).unwrap().0);
        let flashes = [a, b].map(|e| world.get::<ActionFlash>(e).unwrap().action.clone());
        let (parent, mate) = if energies[0] < energies[1] {
            (0, 1)
        } else {
            (1, 0)
        };
        assert!(
            (energies[parent] - (200.0 - cost)).abs() < 1e-5,
            "{energies:?}"
        );
        assert_eq!(energies[mate], 200.0, "{energies:?}");
        assert!(flashes[parent] == ActionType::Reproducing);
        assert!(flashes[mate] == ActionType::None);

        let ledger = world.resource::<EnergyLedger>();
        assert!((ledger.tick.reproduction_spent - cost as f64).abs() < 1e-6);
        assert!(
            (ledger.tick.reproduction_received - (cost * CHILD_ENERGY_FRACTION) as f64).abs()
                < 1e-6
        );
        let mut children = world.query_filtered::<(&Generation, &SpeciesId, &Energy), With<Age>>();
        let (generation, species, energy) = children.single(&world);
        assert_eq!(generation.0, 4);
        assert_eq!(species.0, 1);
        assert!((energy.0 - cost * CHILD_ENERGY_FRACTION).abs() < 1e-5);
    }

    /// A willing organism with no eligible neighbour reproduces alone; a
    /// neighbour that does not want a child is not taken as a mate.
    #[test]
    fn a_lone_parent_reproduces_asexually() {
        let mut world = reproduction_world();
        let a = spawn_adult(&mut world, Vec2::new(10.0, 10.0), 200.0, 1.0);
        let bystander = spawn_adult(&mut world, Vec2::new(12.0, 10.0), 200.0, 0.0);
        let cost = SimConfig::default().reproduction_energy_cost;

        world.run_system_once(reproduction_system).unwrap();

        assert_eq!(organism_count(&mut world), 3);
        assert!((world.get::<Energy>(a).unwrap().0 - (200.0 - cost)).abs() < 1e-5);
        assert_eq!(world.get::<Energy>(bystander).unwrap().0, 200.0);
        assert!(world.get::<ActionFlash>(bystander).unwrap().action == ActionType::None);
    }
}

#[cfg(test)]
mod predation_target_tests {
    use super::*;

    fn candidate(index: u32, dist: f32, is_plant: bool) -> StrikeCandidate {
        StrikeCandidate {
            entity: Entity::from_raw(index),
            dist,
            is_plant,
            energy: 10.0,
        }
    }

    #[test]
    fn nearest_wins_regardless_of_list_order() {
        // Hash bucket order puts the far consumer first.
        let candidates = [
            candidate(1, 3.0, false),
            candidate(2, 0.5, true),
            candidate(3, 1.5, false),
        ];
        assert_eq!(nearest_target(&candidates), Some(&candidates[1]));
    }

    #[test]
    fn plants_and_non_plants_are_both_eligible() {
        let plant_near = [candidate(1, 2.0, false), candidate(2, 1.0, true)];
        assert_eq!(nearest_target(&plant_near), Some(&plant_near[1]));
        let consumer_near = [candidate(1, 1.0, false), candidate(2, 2.0, true)];
        assert_eq!(nearest_target(&consumer_near), Some(&consumer_near[0]));
    }

    #[test]
    fn ties_keep_the_earlier_candidate() {
        let candidates = [candidate(1, 1.0, true), candidate(2, 1.0, false)];
        assert_eq!(nearest_target(&candidates), Some(&candidates[0]));
    }

    #[test]
    fn no_candidates_means_no_strike() {
        assert_eq!(nearest_target(&[]), None);
    }
}

#[cfg(test)]
mod species_classification_tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use rand::{rngs::StdRng, SeedableRng};

    /// A world holding what `species_classification_system` reads and
    /// writes, with virtual time already past the classification period so
    /// the first run is a pass. The threshold is tiny so that any two
    /// distinct genomes fall in different species.
    fn classification_world(tick: u64) -> World {
        let mut world = World::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs_f32(
            SPECIES_CLASSIFICATION_PERIOD_SECS + 0.1,
        ));
        world.insert_resource(time);
        world.insert_resource(SpeciesClassificationTimer(Timer::from_seconds(
            SPECIES_CLASSIFICATION_PERIOD_SECS,
            TimerMode::Repeating,
        )));
        world.insert_resource(SimConfig {
            species_compat_threshold: 1e-6,
            ..SimConfig::default()
        });
        world.insert_resource(TickCounter(tick));
        world.insert_resource(SimStats::default());
        world.insert_resource(SpeciesColors::default());
        world.insert_resource(PhyloTree::default());
        world.insert_resource(WorldChronicle::default());
        world.insert_resource(ConvergenceHighWater::default());
        world
    }

    fn spawn_member(world: &mut World, genome_seed: u64, species: u64) -> Entity {
        let mut innovation = InnovationCounter(0);
        let mut rng = StdRng::seed_from_u64(genome_seed);
        let genome = Genome::new_minimal(&mut innovation, &mut rng);
        world.spawn((Organism, genome, SpeciesId(species))).id()
    }

    fn minimal_genome(genome_seed: u64) -> Genome {
        let mut innovation = InnovationCounter(0);
        let mut rng = StdRng::seed_from_u64(genome_seed);
        Genome::new_minimal(&mut innovation, &mut rng)
    }

    /// Thresholds that put `near` inside the join threshold and `own` inside
    /// the stay threshold but outside the join threshold.
    fn thresholds_between(near: f32, own: f32) -> (f32, f32) {
        assert!(near < own, "test genomes must have the other rep nearer");
        let join = (near + own) / 2.0;
        (join, own + 0.01)
    }

    /// An organism within the stay threshold of its own species keeps it,
    /// even when another species' representative is nearer and within the
    /// join threshold.
    #[test]
    fn a_member_within_the_stay_threshold_keeps_its_species() {
        let member = minimal_genome(10);
        let own_rep = minimal_genome(11);
        let own_dist = member.compatibility_distance(&own_rep);
        let other_rep = (100..300)
            .map(minimal_genome)
            .find(|g| member.compatibility_distance(g) < own_dist)
            .expect("some genome is nearer to the member than its own rep");
        let other_dist = member.compatibility_distance(&other_rep);
        let (join, stay) = thresholds_between(other_dist, own_dist);
        let reps = vec![(1, own_rep), (2, other_rep)];

        assert_eq!(choose_species(&member, 1, &reps, join, stay), Some(1));
        // Past the stay threshold it moves to the nearer species instead.
        assert_eq!(choose_species(&member, 1, &reps, join, own_dist), Some(2));
    }

    /// Past the stay threshold with nothing else within the join threshold,
    /// the organism founds a new species; an unclassified organism (species
    /// 0) joins the nearest species within the join threshold.
    #[test]
    fn leaving_and_unclassified_members_use_the_join_threshold() {
        let member = minimal_genome(20);
        let a = minimal_genome(21);
        let b = minimal_genome(22);
        let da = member.compatibility_distance(&a);
        let db = member.compatibility_distance(&b);
        let reps = vec![(1, a), (2, b)];
        let lo = da.min(db);
        assert_eq!(choose_species(&member, 1, &reps, lo * 0.5, lo * 0.5), None);
        let nearest = if da <= db { 1 } else { 2 };
        let join = da.max(db) + 0.01;
        assert_eq!(choose_species(&member, 0, &reps, join, join), Some(nearest));
    }

    /// Species 2 was the highest id ever issued and has died out; species 1
    /// is alive and one of its members has drifted far enough to found a
    /// new species. The newcomer must get an id the tree has never held, so
    /// that it becomes its own node rather than being folded into the dead
    /// species' record.
    #[test]
    fn a_new_species_never_reuses_an_extinct_species_id() {
        let mut world = classification_world(900);
        {
            let mut phylo = world.resource_mut::<PhyloTree>();
            phylo.record_species(
                1,
                None,
                0,
                Color::WHITE,
                SpeciesStrategy::Photosynthesizer,
                None,
            );
            phylo.record_species(2, Some(1), 300, Color::WHITE, SpeciesStrategy::Hunter, None);
            let mut counts = HashMap::new();
            counts.insert(1, 2);
            phylo.update_populations(&counts, 600);
            assert_eq!(phylo.nodes[&2].extinct_tick, Some(600));
        }
        let stayer = spawn_member(&mut world, 1, 1);
        let drifter = spawn_member(&mut world, 2, 1);

        world
            .run_system_once(species_classification_system)
            .unwrap();

        let ids = [stayer, drifter].map(|e| world.get::<SpeciesId>(e).unwrap().0);
        let new_id = if ids[0] == 1 { ids[1] } else { ids[0] };
        assert!(ids.contains(&1), "one member keeps species 1: {ids:?}");
        assert!(new_id > 2, "new species reused an issued id: {ids:?}");

        let phylo = world.resource::<PhyloTree>();
        let dead = &phylo.nodes[&2];
        assert_eq!(dead.extinct_tick, Some(600), "the dead species stays dead");
        assert_eq!(dead.current_population, 0);
        let newcomer = &phylo.nodes[&new_id];
        assert_eq!(newcomer.parent_id, Some(1));
        assert_eq!(newcomer.born_tick, 900);
        assert_eq!(newcomer.extinct_tick, None);
    }
}

#[cfg(test)]
mod grazing_tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use clauvolution_genome::{BodySegmentGene, SegmentType, Symmetry};
    use rand::{rngs::StdRng, SeedableRng};

    /// A world holding what `grazing_system` and `predation_system` read.
    /// Bite reach is set to 3 × body size and strikes are free, so the
    /// geometry and energies below do not move with the shipped defaults.
    fn feeding_world() -> World {
        let mut world = World::new();
        world.insert_resource(SimConfig {
            bite_reach: 3.0,
            strike_cost: 0.0,
            ..SimConfig::default()
        });
        world.insert_resource(SpatialHash::new(32.0));
        world.insert_resource(AteFoodThisTick::default());
        world.insert_resource(PredationStats::default());
        world.insert_resource(EnergyLedger::default());
        world
    }

    fn segment(segment_type: SegmentType) -> BodySegmentGene {
        BodySegmentGene {
            segment_type,
            size: 1.0,
            attachment_angle: 0.0,
            attachment_slot: 0,
            symmetry: Symmetry::None,
        }
    }

    /// A genome with only the parts under test: a torso, optionally a mouth
    /// and a photo surface, the given diet and claw strength, no armour.
    fn genome(plant: bool, mouth: bool, diet: f32, claws: f32) -> Genome {
        let mut innovation = InnovationCounter(0);
        let mut rng = StdRng::seed_from_u64(1);
        let mut g = Genome::new_minimal(&mut innovation, &mut rng);
        g.body_segments = vec![segment(SegmentType::Torso)];
        if mouth {
            g.body_segments.push(segment(SegmentType::Mouth));
        }
        if plant {
            g.body_segments.push(segment(SegmentType::PhotoSurface));
        }
        g.photosynthesis_rate = if plant { 0.5 } else { 0.0 };
        g.diet = diet;
        g.attack_power = claws;
        g.armor = 0.0;
        g
    }

    fn spawn(
        world: &mut World,
        pos: Vec2,
        energy: f32,
        genome: Genome,
        size: f32,
        output: BrainOutput,
    ) -> Entity {
        let entity = world
            .spawn((
                Organism,
                Position(pos),
                Energy(energy),
                Health(1.0),
                ActionFlash::default(),
                genome,
                BodySize(size),
                output,
            ))
            .id();
        world.resource_mut::<SpatialHash>().insert(entity, pos);
        entity
    }

    fn eating() -> BrainOutput {
        BrainOutput {
            eat: 1.0,
            ..Default::default()
        }
    }

    fn attacking() -> BrainOutput {
        BrainOutput {
            attack: 1.0,
            ..Default::default()
        }
    }

    fn idle() -> BrainOutput {
        BrainOutput::default()
    }

    fn energy(world: &World, e: Entity) -> f32 {
        world.get::<Energy>(e).unwrap().0
    }

    #[test]
    fn graze_bite_scales_by_fraction_and_mouth() {
        assert!((graze_bite(80.0, 0.3, MOUTH_BONUS) - 24.0).abs() < 1e-5);
        assert!((graze_bite(80.0, 0.3, MOUTHLESS_BONUS) - 7.2).abs() < 1e-5);
        assert_eq!(graze_bite(-5.0, 0.3, MOUTH_BONUS), 0.0);
    }

    #[test]
    fn mouth_bonus_reads_the_mouth_segment() {
        assert_eq!(mouth_bonus(&genome(false, true, 0.0, 0.0)), MOUTH_BONUS);
        assert_eq!(
            mouth_bonus(&genome(false, false, 0.0, 0.0)),
            MOUTHLESS_BONUS
        );
    }

    /// Digestion follows the tissue: a plant kill at plant efficiency, an
    /// animal kill at animal efficiency times the hunting multiplier.
    #[test]
    fn kills_are_digested_by_tissue() {
        let herbivore = genome(false, true, -1.0, 1.0);
        let carnivore = genome(false, true, 1.0, 1.0);
        assert_eq!(kill_digestion_efficiency(&herbivore, true, 1.0), 1.0);
        assert_eq!(kill_digestion_efficiency(&herbivore, false, 1.0), 0.0);
        assert_eq!(kill_digestion_efficiency(&carnivore, true, 1.0), 0.0);
        assert_eq!(kill_digestion_efficiency(&carnivore, false, 1.0), 1.0);
        // Switching hunting off leaves plant kills alone.
        assert_eq!(kill_digestion_efficiency(&herbivore, true, 0.0), 1.0);
        assert_eq!(kill_digestion_efficiency(&carnivore, false, 0.0), 0.0);
    }

    /// `eat` bites the nearest living plant, with no claws: the plant loses
    /// the bite and lives, the eater keeps it at its plant efficiency.
    #[test]
    fn eat_bites_the_nearest_plant() {
        let mut world = feeding_world();
        let eater = spawn(
            &mut world,
            Vec2::new(10.0, 10.0),
            50.0,
            genome(false, true, -1.0, 0.0),
            1.0,
            eating(),
        );
        let far = spawn(
            &mut world,
            Vec2::new(12.0, 10.0),
            100.0,
            genome(true, false, 0.0, 0.0),
            1.0,
            idle(),
        );
        let near = spawn(
            &mut world,
            Vec2::new(11.0, 10.0),
            80.0,
            genome(true, false, 0.0, 0.0),
            1.0,
            idle(),
        );

        world.run_system_once(grazing_system).unwrap();

        assert!((energy(&world, near) - 56.0).abs() < 1e-4);
        assert_eq!(energy(&world, far), 100.0);
        assert!((energy(&world, eater) - 74.0).abs() < 1e-4);
        assert_eq!(world.get::<Health>(near).unwrap().0, 1.0);
        assert!(world.get::<ActionFlash>(eater).unwrap().action == ActionType::Grazing);
        let stats = world.resource::<PredationStats>();
        assert_eq!(stats.feeding.grazes_eat, 1);
        assert_eq!(stats.feeding.grazes_eat_by_plant, 0);
        assert_eq!(stats.feeding.grazes_attack, 0);
        let ledger = world.resource::<EnergyLedger>();
        assert!((ledger.tick.grazing - 24.0).abs() < 1e-4);
        assert!(ledger.tick.digestion.abs() < 1e-6);
    }

    /// Without a mouth the bite is smaller, and a generalist keeps a quarter
    /// of it; the plant loses the bite and the rest is digestion loss.
    #[test]
    fn a_mouthless_generalist_takes_a_small_bite_and_keeps_a_quarter() {
        let mut world = feeding_world();
        let eater = spawn(
            &mut world,
            Vec2::new(10.0, 10.0),
            50.0,
            genome(false, false, 0.0, 0.0),
            1.0,
            eating(),
        );
        let plant = spawn(
            &mut world,
            Vec2::new(11.0, 10.0),
            80.0,
            genome(true, false, 0.0, 0.0),
            1.0,
            idle(),
        );

        world.run_system_once(grazing_system).unwrap();

        assert!((energy(&world, plant) - 72.8).abs() < 1e-4);
        assert!((energy(&world, eater) - 51.8).abs() < 1e-4);
        let ledger = world.resource::<EnergyLedger>();
        assert!((ledger.tick.grazing - 1.8).abs() < 1e-4);
        assert!((ledger.tick.digestion - 5.4).abs() < 1e-4);
    }

    /// A plant takes one bite per tick; the eater that loses the race bites
    /// the next plant it can reach instead.
    #[test]
    fn a_plant_takes_one_bite_per_tick() {
        let mut world = feeding_world();
        let a = spawn(
            &mut world,
            Vec2::new(10.0, 10.0),
            50.0,
            genome(false, true, -1.0, 0.0),
            1.0,
            eating(),
        );
        let b = spawn(
            &mut world,
            Vec2::new(10.0, 10.5),
            50.0,
            genome(false, true, -1.0, 0.0),
            1.0,
            eating(),
        );
        let shared = spawn(
            &mut world,
            Vec2::new(11.0, 10.0),
            100.0,
            genome(true, false, 0.0, 0.0),
            1.0,
            idle(),
        );
        let second = spawn(
            &mut world,
            Vec2::new(12.0, 10.5),
            100.0,
            genome(true, false, 0.0, 0.0),
            1.0,
            idle(),
        );

        world.run_system_once(grazing_system).unwrap();

        assert!((energy(&world, shared) - 70.0).abs() < 1e-4);
        assert!((energy(&world, second) - 70.0).abs() < 1e-4);
        assert!((energy(&world, a) - 80.0).abs() < 1e-4);
        assert!((energy(&world, b) - 80.0).abs() < 1e-4);
        assert_eq!(world.resource::<PredationStats>().feeding.grazes_eat, 2);
    }

    /// No bite out of reach, of a consumer, without the eat output, or when
    /// the eater already had a food item this tick.
    #[test]
    fn no_bite_without_a_plant_in_reach_or_after_a_food_item() {
        let mut world = feeding_world();
        // Reach is 3 × body size: 3.5 away is out of it.
        let out_of_reach = spawn(
            &mut world,
            Vec2::new(10.0, 10.0),
            50.0,
            genome(false, true, -1.0, 0.0),
            1.0,
            eating(),
        );
        let far_plant = spawn(
            &mut world,
            Vec2::new(13.5, 10.0),
            100.0,
            genome(true, false, 0.0, 0.0),
            1.0,
            idle(),
        );
        // A consumer next to another consumer bites nothing.
        let beside_consumer = spawn(
            &mut world,
            Vec2::new(100.0, 100.0),
            50.0,
            genome(false, true, -1.0, 0.0),
            1.0,
            eating(),
        );
        let consumer = spawn(
            &mut world,
            Vec2::new(101.0, 100.0),
            50.0,
            genome(false, true, 0.0, 0.0),
            1.0,
            idle(),
        );
        // An idle organism and one fed on a food item leave the plant alone.
        let not_eating = spawn(
            &mut world,
            Vec2::new(200.0, 200.0),
            50.0,
            genome(false, true, -1.0, 0.0),
            1.0,
            idle(),
        );
        let fed = spawn(
            &mut world,
            Vec2::new(200.0, 201.0),
            50.0,
            genome(false, true, -1.0, 0.0),
            1.0,
            eating(),
        );
        let plant = spawn(
            &mut world,
            Vec2::new(201.0, 200.0),
            100.0,
            genome(true, false, 0.0, 0.0),
            1.0,
            idle(),
        );
        world.resource_mut::<AteFoodThisTick>().0.insert(fed);

        world.run_system_once(grazing_system).unwrap();

        for e in [out_of_reach, beside_consumer, consumer, not_eating, fed] {
            assert_eq!(energy(&world, e), 50.0);
        }
        assert_eq!(energy(&world, far_plant), 100.0);
        assert_eq!(energy(&world, plant), 100.0);
        assert_eq!(world.resource::<PredationStats>().feeding.grazes_eat, 0);
    }

    /// An attack on a plant is a kill attempt: it must pass the size gate,
    /// and a kill is digested as plant tissue.
    #[test]
    fn attacking_a_plant_is_a_kill_attempt() {
        let mut world = feeding_world();
        let small = spawn(
            &mut world,
            Vec2::new(10.0, 10.0),
            50.0,
            genome(false, false, -1.0, 2.0),
            0.5,
            attacking(),
        );
        let big_plant = spawn(
            &mut world,
            Vec2::new(11.0, 10.0),
            100.0,
            genome(true, false, 0.0, 0.0),
            1.0,
            idle(),
        );
        let killer = spawn(
            &mut world,
            Vec2::new(100.0, 100.0),
            50.0,
            genome(false, false, -1.0, 1.0),
            1.0,
            attacking(),
        );
        let plant = spawn(
            &mut world,
            Vec2::new(101.0, 100.0),
            80.0,
            genome(true, false, 0.0, 0.0),
            1.0,
            idle(),
        );

        world.run_system_once(predation_system).unwrap();

        // Size gate: 0.5 is not above 0.6 of 1.0.
        assert_eq!(energy(&world, big_plant), 100.0);
        assert_eq!(energy(&world, small), 50.0);
        assert!(world.get::<Killed>(big_plant).is_none());
        // The kill: a tenth of the plant, digested at plant efficiency 1.0.
        assert!(world.get::<Killed>(plant).is_some());
        assert_eq!(energy(&world, plant), 0.0);
        assert!((energy(&world, killer) - 58.0).abs() < 1e-4);
        let stats = world.resource::<PredationStats>();
        assert_eq!(stats.kills, 1);
        assert_eq!(stats.rejected_size_gate, 1);
        assert_eq!(stats.feeding.kills_plant, 1);
        assert_eq!(stats.feeding.kills_plant_by_consumer, 1);
        assert!((stats.feeding.plant_kill_energy_consumer - 8.0).abs() < 1e-4);
        assert_eq!(stats.feeding.grazes_attack, 0);
    }

    #[test]
    fn strike_cost_scales_with_strike_force() {
        assert!((strike_cost(2.0, 0.3) - 0.6).abs() < 1e-6);
        assert_eq!(strike_cost(0.0, 0.3), 0.0);
        assert_eq!(strike_cost(2.0, 0.0), 0.0);
    }

    /// A strike costs the attacker whether it lands or bounces; firing with
    /// nobody in reach is free, and the cost is booked as movement.
    #[test]
    fn a_strike_costs_the_attacker_and_a_flail_does_not() {
        let mut world = feeding_world();
        world.resource_mut::<SimConfig>().strike_cost = 0.5;
        // Lands: claws 1 × size 1 against an unarmoured consumer.
        let killer = spawn(
            &mut world,
            Vec2::new(10.0, 10.0),
            50.0,
            genome(false, false, -1.0, 1.0),
            1.0,
            attacking(),
        );
        let victim = spawn(
            &mut world,
            Vec2::new(11.0, 10.0),
            40.0,
            genome(false, false, -1.0, 0.0),
            1.0,
            idle(),
        );
        // Bounces: too small to pass the size gate on its neighbour.
        let bouncer = spawn(
            &mut world,
            Vec2::new(100.0, 100.0),
            50.0,
            genome(false, false, -1.0, 2.0),
            0.5,
            attacking(),
        );
        spawn(
            &mut world,
            Vec2::new(101.0, 100.0),
            50.0,
            genome(false, false, -1.0, 0.0),
            1.0,
            idle(),
        );
        // Flails: nobody within attack range.
        let flailer = spawn(
            &mut world,
            Vec2::new(300.0, 300.0),
            50.0,
            genome(false, false, -1.0, 2.0),
            1.0,
            attacking(),
        );

        world.run_system_once(predation_system).unwrap();

        assert!(world.get::<Killed>(victim).is_some());
        // A tenth of 40 at animal efficiency 0 is nothing; the strike costs
        // 0.5 × claws 1 × size 1.
        assert!((energy(&world, killer) - 49.5).abs() < 1e-4);
        // 0.5 × claws 2 × size 0.5.
        assert!((energy(&world, bouncer) - 49.5).abs() < 1e-4);
        assert_eq!(energy(&world, flailer), 50.0);
        let stats = world.resource::<PredationStats>();
        assert_eq!(stats.attacks_attempted, 3);
        assert_eq!(stats.strikes, 2);
        assert!((stats.strike_energy - 1.0).abs() < 1e-6);
        let ledger = world.resource::<EnergyLedger>();
        assert!((ledger.tick.movement - 1.0).abs() < 1e-6);
    }

    /// The step 5 gate counters sort each hunter attack on consumer prey
    /// by the gate that stopped it, and leave grazer attackers out.
    #[test]
    fn hunter_gate_counters_name_the_binding_gate() {
        let mut world = feeding_world();
        // (attacker size, claws) beside an unarmoured size-1 consumer.
        let cases = [
            (1.0, 1.0),  // kills
            (0.5, 2.0),  // size gate: 0.5 is not above 0.6
            (1.0, 0.05), // damage gate: 0.05 is not above 0.1
            (0.5, 0.05), // both
        ];
        let mut founder = None;
        for (i, (size, claws)) in cases.iter().enumerate() {
            let x = 100.0 * (i as f32 + 1.0);
            let e = spawn(
                &mut world,
                Vec2::new(x, 10.0),
                50.0,
                genome(false, false, 0.8, *claws),
                *size,
                attacking(),
            );
            if i == 0 {
                world.entity_mut(e).insert((Age(40), Generation(0)));
                founder = Some(e);
            }
            spawn(
                &mut world,
                Vec2::new(x + 1.0, 10.0),
                50.0,
                genome(false, false, -1.0, 0.0),
                1.0,
                idle(),
            );
        }
        // A hunter with only a plant in reach, and a grazer attacker.
        spawn(
            &mut world,
            Vec2::new(600.0, 10.0),
            50.0,
            genome(false, false, 0.8, 1.0),
            1.0,
            attacking(),
        );
        spawn(
            &mut world,
            Vec2::new(601.0, 10.0),
            50.0,
            genome(true, false, 0.0, 0.0),
            1.0,
            idle(),
        );
        spawn(
            &mut world,
            Vec2::new(700.0, 10.0),
            50.0,
            genome(false, false, -1.0, 1.0),
            1.0,
            attacking(),
        );
        spawn(
            &mut world,
            Vec2::new(701.0, 10.0),
            50.0,
            genome(false, false, -1.0, 0.0),
            1.0,
            idle(),
        );

        world.run_system_once(predation_system).unwrap();

        let stats = world.resource::<PredationStats>();
        let h = stats.feeding.hunter_gates;
        assert_eq!((h.intents, h.strikes, h.consumer_in_reach), (5, 5, 4));
        assert_eq!(
            (
                h.kills_consumer,
                h.rejected_size,
                h.rejected_damage,
                h.rejected_both
            ),
            (1, 1, 1, 1)
        );
        assert_eq!(stats.diet_nonneg_gates, h);
        let f = stats.founder_hunter_gates;
        assert_eq!((f.intents, f.kills_consumer), (1, 1));
        assert!(stats.founder_hunters_reached.contains(&founder.unwrap()));
        assert_eq!(stats.founder_hunter_first_reach_age_sum, 40);
        assert_eq!(stats.founder_hunters_killed.len(), 1);
        // Every attacker is younger than 100 ticks (one is 40, the rest have
        // no Age and read 0), so everything lands in the first age bucket.
        assert_eq!(stats.hunter_reach_ages[0], 4);
        assert_eq!(stats.hunter_intent_ages[0], 5);
    }
}

#[cfg(test)]
mod death_marker_tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn markers(world: &mut World) -> usize {
        world.query::<&DeathMarker>().iter(world).count()
    }

    #[test]
    fn markers_expire_after_their_lifetime_without_a_renderer() {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        world.spawn((
            DeathMarker {
                timer: DEATH_MARKER_SECS,
                was_predated: false,
            },
            Position(Vec2::ZERO),
        ));
        let step = std::time::Duration::from_secs_f64(1.0 / 30.0);
        let lifetime_ticks = (DEATH_MARKER_SECS * 30.0).round() as usize;

        for _ in 0..lifetime_ticks - 1 {
            world.resource_mut::<Time>().advance_by(step);
            world.run_system_once(death_marker_expiry_system).unwrap();
        }
        assert_eq!(markers(&mut world), 1, "marker expired early");

        for _ in 0..2 {
            world.resource_mut::<Time>().advance_by(step);
            world.run_system_once(death_marker_expiry_system).unwrap();
        }
        assert_eq!(markers(&mut world), 0, "marker outlived its lifetime");
        assert_eq!(world.entities().len(), 0);
    }
}
