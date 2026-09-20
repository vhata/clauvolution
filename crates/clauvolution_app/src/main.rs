mod script;

use bevy::core::{TaskPoolOptions, TaskPoolPlugin, TaskPoolThreadAssignmentPolicy};
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::PrimaryWindow;
use clauvolution_body::{update_body_plans, BodyPlugin};
use clauvolution_core::*;
use clauvolution_genome::InnovationCounter;
use clauvolution_phylogeny::{PhyloTree, PhylogenyPlugin, WorldChronicle};
use clauvolution_render::{MainCamera, RenderPlugin};
use clauvolution_sim::save;
use clauvolution_sim::{SimPlugin, SimTick};
use clauvolution_ui::UiPlugin;
use clauvolution_world::{self, TileMap, WorldPlugin};
use rand::SeedableRng;
use script::{load_script, script_runner_system, ScriptState};

/// Default cap on Bevy's compute task pool workers — leaves cores free for
/// the rest of the OS so running this sim doesn't spin up the fans and
/// bog everything else down. Override with the `CLAU_WORKERS` env var.
const DEFAULT_WORKER_CAP: usize = 6;

/// Default virtual-time multiplier in headless mode. Headless frames advance
/// the clock by exactly one fixed timestep (see `HEADLESS_FRAME_DELTA`), so
/// `--speed N` means N simulation ticks per frame; the run itself goes as
/// fast as the CPU allows at every speed. Override with `--speed N`.
const DEFAULT_HEADLESS_SPEED: f32 = 10.0;

/// Real-time delta applied per headless frame: one 30 Hz fixed timestep.
///
/// Headless runs use `TimeUpdateStrategy::ManualDuration` instead of the wall
/// clock. With the wall clock, the number of fixed ticks a frame ran depended
/// on how long the previous frame took, so anything scheduled per frame
/// interleaved with the tick chain differently on every run (at the time,
/// `update_body_plans` in `PostUpdate`; see `BodyPlugin`). Fixing the
/// per-frame delta removes wall-clock time from the run entirely, whatever
/// the per-frame schedules hold.
const HEADLESS_FRAME_DELTA: std::time::Duration = std::time::Duration::from_nanos(33_333_333);

fn compute_worker_cap() -> usize {
    std::env::var("CLAU_WORKERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n: &usize| n >= 1)
        .unwrap_or(DEFAULT_WORKER_CAP)
}

/// When the app is launched via Finder / LaunchServices (double-click the
/// `.app`), the working directory is `/`, which is read-only on macOS.
/// Session::new tries to create `sessions/<name>/` in cwd and panics.
///
/// Detect that case and hop to `~/Documents/Clauvolution/`. Documents
/// (rather than Application Support) because the contents — screenshots,
/// save-world files, chronicle logs — are user-facing artefacts the
/// user may want to browse, share, or delete in Finder. Application
/// Support is for app-internal state (preferences, caches), which
/// isn't what our `sessions/` dir contains.
///
/// No-op when launched from a terminal (the usual `cargo run` path)
/// because cwd is the project root there, not `/`.
fn chdir_to_writable_if_bundled() {
    let Ok(cwd) = std::env::current_dir() else {
        return;
    };
    if cwd != std::path::Path::new("/") {
        return;
    }
    let Ok(home) = std::env::var("HOME") else {
        return;
    };
    let dir = std::path::PathBuf::from(home)
        .join("Documents")
        .join("Clauvolution");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    if std::env::set_current_dir(&dir).is_ok() {
        eprintln!(
            "Launched as .app — using {} as working directory (sessions/ live here).",
            dir.display()
        );
    }
}

fn task_pool_plugin(worker_cap: usize) -> TaskPoolPlugin {
    TaskPoolPlugin {
        task_pool_options: TaskPoolOptions {
            compute: TaskPoolThreadAssignmentPolicy {
                min_threads: 1,
                max_threads: worker_cap,
                percent: 1.0,
            },
            ..TaskPoolOptions::default()
        },
    }
}

fn main() {
    chdir_to_writable_if_bundled();
    let args: Vec<String> = std::env::args().collect();
    let screenshot_mode = args.iter().any(|a| a == "--screenshot");
    let load_path = args
        .iter()
        .position(|a| a == "--load")
        .and_then(|i| args.get(i + 1).cloned());
    let seed: Option<u64> = args
        .iter()
        .position(|a| a == "--seed")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok());
    let headless_ticks: Option<u64> = args
        .iter()
        .position(|a| a == "--headless")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok());
    let headless_speed: f32 = args
        .iter()
        .position(|a| a == "--speed")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_HEADLESS_SPEED);
    let script_path: Option<String> = args
        .iter()
        .position(|a| a == "--script")
        .and_then(|i| args.get(i + 1).cloned());
    let save_as: Option<String> = args
        .iter()
        .position(|a| a == "--save-as")
        .and_then(|i| args.get(i + 1).cloned());
    let dump_history: Option<String> = args
        .iter()
        .position(|a| a == "--dump-history")
        .and_then(|i| args.get(i + 1).cloned());
    let overrides = ConfigOverrides::parse(&args);

    let worker_cap = compute_worker_cap();
    eprintln!(
        "Compute pool capped at {} workers (set CLAU_WORKERS to override)",
        worker_cap
    );

    if let Some(ticks) = headless_ticks {
        run_headless(
            ticks,
            seed,
            worker_cap,
            headless_speed,
            load_path,
            save_as,
            dump_history,
            overrides,
        );
        return;
    }

    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Clauvolution".to_string(),
                    resolution: (1920.0_f32, 1080.0_f32).into(),
                    ..default()
                }),
                ..default()
            })
            .set(task_pool_plugin(worker_cap)),
    )
    .add_plugins(CorePlugin)
    .add_plugins(WorldPlugin)
    .add_plugins(BodyPlugin)
    .add_plugins(SimPlugin)
    .add_plugins(PhylogenyPlugin)
    .add_plugins(RenderPlugin)
    .add_plugins(UiPlugin)
    .insert_resource(InnovationCounter(100))
    .insert_resource(LoadPath(load_path))
    .insert_resource(SeedOverride(seed))
    .insert_resource(overrides)
    .add_systems(
        Startup,
        (
            apply_seed_override,
            apply_config_overrides,
            startup_system,
            set_window_title,
        )
            .chain(),
    )
    // Body plans are part of the tick, not the frame; see `BodyPlugin`.
    .add_systems(FixedUpdate, update_body_plans.after(SimTick));

    if screenshot_mode {
        app.insert_resource(ScreenshotSchedule::new())
            .add_systems(Update, screenshot_system);
    }

    if let Some(path) = script_path {
        match load_script(std::path::Path::new(&path)) {
            Ok(script) => {
                eprintln!(
                    "Loaded script with {} action(s) from {}",
                    script.actions.len(),
                    path
                );
                app.insert_resource(ScriptState {
                    script,
                    next_action: 0,
                })
                .add_systems(Update, script_runner_system);
            }
            Err(e) => {
                eprintln!("Failed to load script: {}", e);
                std::process::exit(1);
            }
        }
    }

    app.run();
}

#[derive(Resource)]
struct SeedOverride(Option<u64>);

/// If --seed N was passed on the command line, stamp it into SimConfig before
/// startup_system reads it to seed the terrain and SimRng. Otherwise the
/// default random seed from SimConfig::default() stands.
fn apply_seed_override(seed_override: Res<SeedOverride>, mut config: ResMut<SimConfig>) {
    if let Some(seed) = seed_override.0 {
        config.terrain_seed = seed;
        info!("Using seed from CLI: {}", seed);
    }
}

#[derive(Resource)]
struct LoadPath(Option<String>);

fn startup_system(
    commands: Commands,
    config: ResMut<SimConfig>,
    innovation: ResMut<InnovationCounter>,
    stats: ResMut<SimStats>,
    tick: ResMut<TickCounter>,
    season: ResMut<Season>,
    phylo: ResMut<PhyloTree>,
    chronicle: ResMut<WorldChronicle>,
    ledger: ResMut<EnergyLedger>,
    load_path: Res<LoadPath>,
) {
    if let Some(ref path) = load_path.0 {
        let save_path = std::path::Path::new(path).join("save.json");
        if save_path.exists() {
            load_saved_world(
                commands, config, innovation, stats, tick, season, phylo, chronicle, ledger,
                &save_path,
            );
            return;
        } else {
            warn!(
                "Save file not found: {}, starting fresh",
                save_path.display()
            );
        }
    }
    fresh_world(commands, config, innovation, ledger);
}

fn load_saved_world(
    mut commands: Commands,
    mut config: ResMut<SimConfig>,
    mut innovation: ResMut<InnovationCounter>,
    mut stats: ResMut<SimStats>,
    mut tick: ResMut<TickCounter>,
    mut season: ResMut<Season>,
    mut phylo: ResMut<PhyloTree>,
    mut chronicle: ResMut<WorldChronicle>,
    mut ledger: ResMut<EnergyLedger>,
    save_path: &std::path::Path,
) {
    let Some(state) = save::load_world(save_path) else {
        warn!("Failed to load save file, starting fresh");
        return;
    };

    info!(
        "Loading world from {} ({} organisms, {} food)",
        save_path.display(),
        state.organisms.len(),
        state.food.len()
    );

    // Restore state
    tick.0 = state.tick;
    season.current_tick = state.season_tick;
    stats.total_births = state.stats.total_births;
    stats.total_deaths = state.stats.total_deaths;
    stats.max_generation = state.stats.max_generation;
    innovation.0 = state.innovation_counter;
    config.terrain_seed = state.terrain_seed;

    // Generate terrain from seed — same seed = same terrain
    let mut rng = rand::rngs::StdRng::seed_from_u64(config.terrain_seed);
    let tile_map =
        clauvolution_world::TileMap::generate(config.world_width, config.world_height, &mut rng);
    commands.insert_resource(tile_map);

    // Reseed SimRng from the saved seed. (Mid-run save/load diverges from
    // the original trajectory at this point — the RNG state history isn't
    // serialised; same-seed runs from tick 0 still match.)
    commands.insert_resource(SimRng::from_seed(config.terrain_seed));

    // Restore organisms and food. The ledger starts from the loaded energy so
    // the first tick does not compare against an empty world.
    let total_energy = save::spawn_saved_organisms(&mut commands, &state.organisms);
    ledger.reset_baseline(total_energy);
    save::spawn_saved_food(&mut commands, &state.food);

    // Restore phylo tree and chronicle
    save::restore_phylo(&mut phylo, &state.phylo_nodes);
    save::restore_chronicle(&mut chronicle, &state.chronicle_entries);
    chronicle.log(tick.0, "World loaded from save".to_string());
}

fn fresh_world(
    commands: Commands,
    config: ResMut<SimConfig>,
    innovation: ResMut<InnovationCounter>,
    ledger: ResMut<EnergyLedger>,
) {
    setup_world(commands, config, innovation, ledger);
}

fn setup_world(
    mut commands: Commands,
    config: ResMut<SimConfig>,
    mut innovation: ResMut<InnovationCounter>,
    mut ledger: ResMut<EnergyLedger>,
) {
    // Seed deterministic terrain generation
    let mut terrain_rng = rand::rngs::StdRng::seed_from_u64(config.terrain_seed);
    let tile_map = TileMap::generate(config.world_width, config.world_height, &mut terrain_rng);

    // Seed the sim-wide RNG used by food regen, disease, reproduction, etc.
    // Same seed → same simulation trajectory.
    let mut sim_rng = SimRng::from_seed(config.terrain_seed);
    clauvolution_world::spawn_initial_food(&mut commands, &config, &tile_map, &mut sim_rng.0);
    let founders = clauvolution_sim::spawn_initial_population(
        &mut commands,
        &config,
        &tile_map,
        &mut innovation,
        &mut sim_rng.0,
    );
    ledger.reset_baseline(founders.total_energy);
    info!("{founders}");
    commands.insert_resource(founders);
    commands.insert_resource(tile_map);
    commands.insert_resource(sim_rng);

    info!(
        "Clauvolution initialized: {} organisms, world {}x{} with biomes (seed {})",
        config.initial_population, config.world_width, config.world_height, config.terrain_seed
    );
}

// --- Screenshot mode ---

#[derive(Resource)]
struct ScreenshotSchedule {
    shots: Vec<ScreenshotStep>,
    current: usize,
    frame_count: u32,
}

struct ScreenshotStep {
    wait_frames: u32,
    zoom: f32,
    label: String,
}

impl ScreenshotSchedule {
    fn new() -> Self {
        Self {
            shots: vec![
                ScreenshotStep {
                    wait_frames: 30,
                    zoom: 1.0,
                    label: "01_overview".to_string(),
                },
                ScreenshotStep {
                    wait_frames: 60,
                    zoom: 1.0,
                    label: "02_after_2sec".to_string(),
                },
                ScreenshotStep {
                    wait_frames: 30,
                    zoom: 0.5,
                    label: "03_medium_zoom".to_string(),
                },
                ScreenshotStep {
                    wait_frames: 30,
                    zoom: 0.15,
                    label: "04_close_zoom".to_string(),
                },
                ScreenshotStep {
                    wait_frames: 150,
                    zoom: 1.0,
                    label: "05_after_7sec".to_string(),
                },
                ScreenshotStep {
                    wait_frames: 300,
                    zoom: 1.0,
                    label: "06_after_17sec".to_string(),
                },
            ],
            current: 0,
            frame_count: 0,
        }
    }
}

fn screenshot_system(
    mut commands: Commands,
    mut schedule: ResMut<ScreenshotSchedule>,
    session: Res<Session>,
    mut camera: Query<(&mut Transform, &mut OrthographicProjection), With<MainCamera>>,
    mut exit: EventWriter<AppExit>,
    config: Res<SimConfig>,
) {
    schedule.frame_count += 1;

    if schedule.current >= schedule.shots.len() {
        info!("All screenshots captured, exiting.");
        exit.send(AppExit::Success);
        return;
    }

    let step = &schedule.shots[schedule.current];

    if schedule.frame_count >= step.wait_frames {
        // Set camera zoom
        if let Ok((mut transform, mut projection)) = camera.get_single_mut() {
            projection.scale = step.zoom;
            // Center on world
            transform.translation.x = config.world_width as f32 / 2.0;
            transform.translation.y = config.world_height as f32 / 2.0;
        }

        let path = session
            .screenshot_path(&step.label)
            .to_string_lossy()
            .to_string();
        info!("Capturing screenshot: {}", path);

        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));

        schedule.current += 1;
        schedule.frame_count = 0;
    }
}

fn set_window_title(session: Res<Session>, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.title = format!("Clauvolution — {}", session.name);
    }
}

// --- Headless mode ---

fn run_headless(
    ticks: u64,
    seed: Option<u64>,
    worker_cap: usize,
    speed: f32,
    load_path: Option<String>,
    save_as: Option<String>,
    dump_history: Option<String>,
    overrides: ConfigOverrides,
) {
    use bevy::app::ScheduleRunnerPlugin;

    let start = std::time::Instant::now();
    eprintln!(
        "Headless run: {} ticks{} at {}× virtual-time",
        ticks,
        seed.map(|s| format!(", seed {}", s)).unwrap_or_default(),
        speed
    );

    let mut app = App::new();

    // Pick a real named session when the user asked to save, otherwise
    // use the ephemeral session so we don't clutter sessions/.
    let session = match &save_as {
        Some(name) => {
            eprintln!("Will save to sessions/{} at end of run", name);
            Session::with_name(name)
        }
        None => Session::new_ephemeral(),
    };
    app.insert_resource(session);

    if let Some(ref path) = load_path {
        eprintln!("Loading session from {}", path);
    }

    // MinimalPlugins gives us Time + ScheduleRunner. We don't want it to sleep
    // between frames, so configure run_loop with zero duration.
    // Apply the same worker cap as the main app so headless also behaves.
    app.add_plugins(
        MinimalPlugins
            .set(ScheduleRunnerPlugin::run_loop(std::time::Duration::ZERO))
            .set(task_pool_plugin(worker_cap)),
    );
    // Decouple the clock from wall time so the tick/frame interleaving, and
    // with it the whole run, is a function of the seed alone.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        HEADLESS_FRAME_DELTA,
    ));
    // InputPlugin registers ButtonInput<KeyCode> etc. The sim's
    // keyboard_to_events_system reads it; it'll just be empty in headless
    // (no keys ever pressed) but the resource has to exist.
    app.add_plugins(bevy::input::InputPlugin);

    // The sim pipeline (no render, no UI).
    app.add_plugins(CorePlugin)
        .add_plugins(WorldPlugin)
        .add_plugins(BodyPlugin)
        .add_plugins(SimPlugin)
        .add_plugins(PhylogenyPlugin)
        .insert_resource(InnovationCounter(100))
        .insert_resource(LoadPath(load_path))
        .insert_resource(SeedOverride(seed))
        .insert_resource(HeadlessSpeed(speed))
        .insert_resource(HeadlessSaveAtEnd(save_as.is_some()))
        .insert_resource(overrides)
        .add_systems(
            Startup,
            (
                apply_seed_override,
                apply_config_overrides,
                startup_system,
                set_headless_speed,
                unbound_history,
            )
                .chain(),
        );

    // Counter system that exits after N FixedUpdate ticks from whatever
    // tick we're at when the loop starts (so `--load X --headless 300`
    // runs 300 more ticks on top of the loaded state, not 300 absolute).
    // Ordered after the sim chain so the summary and history dump describe
    // the completed target tick rather than whatever point in the tick the
    // executor happened to reach.
    app.insert_resource(HeadlessTickTarget {
        ticks_to_run: ticks,
        absolute_target: None,
    })
    .add_systems(
        FixedUpdate,
        (update_body_plans, headless_tick_counter)
            .chain()
            .after(SimTick),
    );

    // Headless runs virtual time at `--speed` ×, so FixedUpdate can fire
    // faster than 30Hz wall-clock up to whatever the CPU can sustain. The
    // fixed timestep stays at 30 Hz and every sim timer (species
    // classification, pop history, bloom durations) is defined in virtual
    // seconds, so their semantics stay intact — a run that used to take 50s
    // wall-clock just completes in 5s at speed=10. The GUI speed control
    // (`sim_speed_system` in clauvolution_sim) scales virtual time the same
    // way, so both modes run the same per-tick simulation.
    // On an M4 Max at speed=10 we keep up with the CPU; push it further
    // and Bevy's catchup starts clamping via the 100ms max_delta cap.

    if let Some(path) = &dump_history {
        app.insert_resource(HeadlessDumpHistoryPath(path.clone()));
    }

    app.run();

    let elapsed = start.elapsed();
    eprintln!("Headless run complete in {:.2}s", elapsed.as_secs_f64());
}

#[derive(Resource)]
struct HeadlessDumpHistoryPath(String);

fn dump_history_csv(
    path: &str,
    history: &clauvolution_core::PopulationHistory,
) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    writeln!(
        f,
        "tick,sim_second,organisms,food,species,plants,grazers,hunters,omnivores,infected,\
         avg_lifespan,avg_body_size,avg_speed,avg_armor,avg_attack,avg_photo,avg_diet,\
         avg_photo_area,avg_speed_plants,avg_speed_eaters,avg_light_share,\
         ready_share_plants,ready_share_eaters,\
         avg_disease_resistance,avg_symbiosis_rate,symbiotic_pairs,\
         deaths_starvation,deaths_predation,deaths_old_age,deaths_disease,deaths_event,\
         energy_total,flow_photosynthesis,flow_food,flow_predation,flow_grazing,flow_symbiosis,\
         flow_metabolism,flow_movement,flow_disease,flow_reproduction_spent,\
         flow_reproduction_received,flow_death,flow_clamp,flow_digestion,\
         ledger_max_residual,ledger_cumulative_residual"
    )?;
    for s in &history.snapshots {
        let fl = &s.energy_flows;
        writeln!(
            f,
            "{},{:.1},{},{},{},{},{},{},{},{},{:.2},{:.3},{:.3},{:.3},{:.3},{:.3},{:+.3},{:.3},{:.4},{:.4},{:.3},{:.3},{:.3},{:.3},{:.3},{},{},{},{},{},{},\
             {:.2},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.6},{:.6}",
            s.tick,
            s.tick as f64 / 30.0,
            s.organisms,
            s.food,
            s.species,
            s.plants,
            s.grazers,
            s.hunters,
            s.omnivores,
            s.infected,
            s.avg_lifespan,
            s.avg_body_size,
            s.avg_speed,
            s.avg_armor,
            s.avg_attack,
            s.avg_photo,
            s.avg_diet,
            s.avg_photo_area,
            s.avg_speed_plants,
            s.avg_speed_eaters,
            s.avg_light_share,
            s.ready_share_plants,
            s.ready_share_eaters,
            s.avg_disease_resistance,
            s.avg_symbiosis_rate,
            s.symbiotic_pairs,
            s.deaths_starvation,
            s.deaths_predation,
            s.deaths_old_age,
            s.deaths_disease,
            s.deaths_event,
            s.energy_total,
            fl.photosynthesis,
            fl.food,
            fl.predation,
            fl.grazing,
            fl.symbiosis,
            fl.metabolism,
            fl.movement,
            fl.disease,
            fl.reproduction_spent,
            fl.reproduction_received,
            fl.death,
            fl.clamp,
            fl.digestion,
            s.ledger_max_residual,
            s.ledger_cumulative_residual,
        )?;
    }
    Ok(())
}

#[derive(Resource)]
struct HeadlessSpeed(f32);

/// Hand `--speed` to `SimSpeed`; `sim_speed_system` in `clauvolution_sim`
/// applies it to `Time<Virtual>` for both headless and GUI runs.
/// Headless runs keep every population snapshot so `--dump-history` covers
/// the whole run. The GUI keeps the 300-entry ring buffer the Graphs tab
/// is sized for.
fn unbound_history(mut history: ResMut<clauvolution_core::PopulationHistory>) {
    history.max_entries = usize::MAX;
}

fn set_headless_speed(speed: Res<HeadlessSpeed>, mut sim_speed: ResMut<SimSpeed>) {
    sim_speed.multiplier = speed.0;
}

/// `SimConfig` fields that can be overridden from the command line, applied
/// before `startup_system` in both the GUI and headless startup chains so the
/// founders are spawned under the overridden values (a loaded save restores
/// only `terrain_seed`, so nothing is undone). These are the knobs the tuning
/// passes sweep; each maps to one `--flag VALUE`.
#[derive(Resource, Default)]
struct ConfigOverrides {
    species_threshold: Option<f32>,
    bite_fraction: Option<f32>,
    kill_transfer: Option<f32>,
    photo_drag: Option<f32>,
    leaf_capacity: Option<f32>,
    founder_diet_spread: Option<f32>,
    animal_efficiency: Option<f32>,
    max_energy: Option<f32>,
    max_food_density: Option<f32>,
    population_ceiling: Option<u32>,
}

impl ConfigOverrides {
    fn parse(args: &[String]) -> Self {
        fn flag<T: std::str::FromStr>(args: &[String], name: &str) -> Option<T> {
            args.iter()
                .position(|a| a == name)
                .and_then(|i| args.get(i + 1))
                .and_then(|s| s.parse().ok())
        }
        Self {
            species_threshold: flag(args, "--species-threshold"),
            bite_fraction: flag(args, "--bite-fraction"),
            kill_transfer: flag(args, "--kill-transfer"),
            photo_drag: flag(args, "--photo-drag"),
            leaf_capacity: flag(args, "--leaf-capacity"),
            founder_diet_spread: flag(args, "--founder-diet-spread"),
            animal_efficiency: flag(args, "--animal-efficiency"),
            max_energy: flag(args, "--max-energy"),
            max_food_density: flag(args, "--max-food-density"),
            population_ceiling: flag(args, "--population-ceiling"),
        }
    }
}

fn apply_config_overrides(overrides: Res<ConfigOverrides>, mut config: ResMut<SimConfig>) {
    fn set<T: Copy + std::fmt::Display>(slot: &mut T, value: Option<T>, name: &str) {
        if let Some(v) = value {
            *slot = v;
            eprintln!("Config override: {name} = {v}");
        }
    }
    set(
        &mut config.species_compat_threshold,
        overrides.species_threshold,
        "species_compat_threshold",
    );
    set(
        &mut config.bite_fraction,
        overrides.bite_fraction,
        "bite_fraction",
    );
    set(
        &mut config.kill_transfer_fraction,
        overrides.kill_transfer,
        "kill_transfer_fraction",
    );
    set(&mut config.photo_drag, overrides.photo_drag, "photo_drag");
    set(
        &mut config.leaf_capacity_per_tile,
        overrides.leaf_capacity,
        "leaf_capacity_per_tile",
    );
    set(
        &mut config.founder_diet_spread,
        overrides.founder_diet_spread,
        "founder_diet_spread",
    );
    set(
        &mut config.animal_efficiency_multiplier,
        overrides.animal_efficiency,
        "animal_efficiency_multiplier",
    );
    set(
        &mut config.max_organism_energy,
        overrides.max_energy,
        "max_organism_energy",
    );
    set(
        &mut config.max_food_density,
        overrides.max_food_density,
        "max_food_density",
    );
    set(
        &mut config.population_ceiling,
        overrides.population_ceiling,
        "population_ceiling",
    );
}

/// Number of additional ticks to run in headless mode. `--headless N` means
/// "run N more ticks from wherever we start", so a loaded save at tick 1500
/// with `--headless 300` runs to tick 1800.
#[derive(Resource)]
struct HeadlessTickTarget {
    ticks_to_run: u64,
    /// Absolute target tick, computed on the first frame once we know
    /// what TickCounter says after load/init.
    absolute_target: Option<u64>,
}

#[derive(Resource)]
struct HeadlessSaveAtEnd(bool);

fn headless_tick_counter(
    mut target: ResMut<HeadlessTickTarget>,
    tick: Res<clauvolution_core::TickCounter>,
    stats: Res<clauvolution_core::SimStats>,
    predation: Res<clauvolution_core::PredationStats>,
    history: Res<clauvolution_core::PopulationHistory>,
    ledger: Res<clauvolution_core::EnergyLedger>,
    config: Res<clauvolution_core::SimConfig>,
    save_at_end: Res<HeadlessSaveAtEnd>,
    dump_path: Option<Res<HeadlessDumpHistoryPath>>,
    founders: Option<Res<clauvolution_sim::FounderReport>>,
    save_report: Res<clauvolution_sim::SaveReport>,
    mut events: EventWriter<clauvolution_core::WorldEventRequest>,
    mut exit: EventWriter<AppExit>,
    // 0 = running, 1 = summary printed + save requested, 2 = waited a frame
    // for save_system to run, 3 = exit sent
    mut phase: Local<u8>,
) {
    if *phase >= 3 {
        return;
    }
    let ticks_to_run = target.ticks_to_run;
    let abs_target = *target
        .absolute_target
        .get_or_insert_with(|| tick.0 + ticks_to_run);
    if tick.0 < abs_target && *phase == 0 {
        return;
    }
    match *phase {
        0 => {
            print_headless_summary(
                &stats,
                &predation,
                &history,
                &ledger,
                &config,
                founders.as_deref(),
            );
            if let Some(dp) = &dump_path {
                match dump_history_csv(&dp.0, &history) {
                    Ok(_) => eprintln!("Wrote {} snapshots to {}", history.snapshots.len(), dp.0),
                    Err(e) => eprintln!("Failed to write history CSV to {}: {}", dp.0, e),
                }
            }
            if save_at_end.0 {
                events.send(clauvolution_core::WorldEventRequest::Save);
            }
            *phase = 1;
        }
        1 => {
            // save_system reads the event this frame and writes synchronously
            *phase = 2;
        }
        2 => {
            // FixedUpdate can run several times per frame, so save_system
            // (an Update system) may not have run yet. Wait for its report
            // before deciding how the run ends.
            if save_at_end.0 && save_report.last.is_none() {
                return;
            }
            let outcome = match &save_report.last {
                Some(Ok(path)) => {
                    eprintln!("Saved world to {}", path.display());
                    AppExit::Success
                }
                Some(Err(e)) => {
                    eprintln!("Save failed: {}", e);
                    AppExit::error()
                }
                None => AppExit::Success,
            };
            exit.send(outcome);
            *phase = 3;
        }
        _ => {}
    }
}

fn print_headless_summary(
    stats: &clauvolution_core::SimStats,
    predation: &clauvolution_core::PredationStats,
    history: &clauvolution_core::PopulationHistory,
    ledger: &clauvolution_core::EnergyLedger,
    config: &clauvolution_core::SimConfig,
    founders: Option<&clauvolution_sim::FounderReport>,
) {
    eprintln!();
    // Population is read from the last 1Hz snapshot so it matches the
    // strategy breakdown below exactly.
    let latest = history.snapshots.last();
    eprintln!("=== Headless summary ===");
    if let Some(f) = founders {
        eprintln!("{f}");
    }
    eprintln!(
        "Total organisms (final): {}",
        latest.map(|s| s.organisms).unwrap_or(0)
    );
    eprintln!("Species (final):         {}", stats.species_count);
    eprintln!("Max generation:          {}", stats.max_generation);
    eprintln!("Total births:            {}", stats.total_births);
    eprintln!("Total deaths:            {}", stats.total_deaths);
    eprintln!("  by Starvation:         {}", stats.deaths_by_cause[0]);
    eprintln!("  by Predation:          {}", stats.deaths_by_cause[1]);
    eprintln!("  by Old age:            {}", stats.deaths_by_cause[2]);
    eprintln!("  by Disease:            {}", stats.deaths_by_cause[3]);
    eprintln!("  by Event:              {}", stats.deaths_by_cause[4]);
    // Any engagement here means the population was capped by a rule rather
    // than by energy; the counts make that visible in every audit summary.
    eprintln!(
        "Population ceiling:      {} (engaged {} times, {} births blocked)",
        config.population_ceiling, stats.ceiling_episodes, stats.ceiling_blocked_births
    );
    if let Some(latest) = latest {
        eprintln!();
        eprintln!("Final strategy breakdown:");
        eprintln!("  Plants:              {}", latest.plants);
        eprintln!("  Grazers:             {}", latest.grazers);
        eprintln!("  Hunters:             {}", latest.hunters);
        eprintln!("  Omnivores:           {}", latest.omnivores);
        eprintln!("  Infected:            {}", latest.infected);
        eprintln!();
        eprintln!("Final trait averages:");
        eprintln!("  Body size:           {:.2}", latest.avg_body_size);
        eprintln!("  Speed:               {:.2}", latest.avg_speed);
        eprintln!("  Attack:              {:.2}", latest.avg_attack);
        eprintln!("  Armor:               {:.2}", latest.avg_armor);
        eprintln!("  Photosynthesis:      {:.0}%", latest.avg_photo * 100.0);
        eprintln!("  Diet (consumers):    {:+.2}", latest.avg_diet);
        eprintln!("  Leaf area (plants):  {:.2}", latest.avg_photo_area);
        eprintln!(
            "  Speed plants/eaters: {:.3} / {:.3} per tick",
            latest.avg_speed_plants, latest.avg_speed_eaters
        );
        eprintln!("  Light share (plants): {:.2}", latest.avg_light_share);
        eprintln!(
            "  Ready plants/eaters: {:.0}% / {:.0}%",
            latest.ready_share_plants * 100.0,
            latest.ready_share_eaters * 100.0
        );
        eprintln!(
            "  Disease resistance:  {:.0}%",
            latest.avg_disease_resistance * 100.0
        );
        eprintln!("  Symbiosis rate:      {:+.2}", latest.avg_symbiosis_rate);
        eprintln!("  Symbiotic pairs:     {}", latest.symbiotic_pairs);
        eprintln!("  Avg lifespan:        {:.0} ticks", latest.avg_lifespan);
    }
    eprintln!();
    eprintln!("Predation funnel:");
    eprintln!("  Attack intents:      {}", predation.attacks_attempted);
    eprintln!("  Targets considered:  {}", predation.targets_considered);
    eprintln!("  Rejected (size):     {}", predation.rejected_size_gate);
    eprintln!("  Rejected (damage):   {}", predation.rejected_damage);
    eprintln!("  Kills:               {}", predation.kills);
    eprintln!("  Grazes:              {}", predation.grazes);
    eprintln!();
    // Cumulative flows are magnitudes; the sign column says which way each
    // one moves organism energy. Grazing and symbiosis are transfers between
    // organisms and have no sign.
    eprintln!("Energy ledger:");
    eprintln!("  Baseline energy:     {:.1}", ledger.baseline);
    eprintln!("  Final live energy:   {:.1}", ledger.total);
    eprintln!("  Cumulative flows:");
    let signs = [
        "+", "+", "+", " ", " ", "-", "-", "-", "-", "+", "-", "-", "-",
    ];
    for ((label, value), sign) in ledger.cumulative.entries().iter().zip(signs) {
        eprintln!("    {sign} {label:<22} {value:>14.1}");
    }
    eprintln!("  Net of flows:        {:+.1}", ledger.cumulative.net());
    eprintln!(
        "  Largest |residual|:  {:.6} per tick (tolerance {})",
        ledger.max_abs_residual,
        clauvolution_core::EnergyLedger::TOLERANCE
    );
    eprintln!("  Final residual:      {:+.6}", ledger.last_residual);
    eprintln!("  Cumulative residual: {:+.6}", ledger.cumulative_residual);
    eprintln!("  Ticks over tolerance: {}", ledger.breaches);
    eprintln!();
}
