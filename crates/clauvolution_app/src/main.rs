mod cli;
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
    // Check the arguments before anything else, so a typo or `--help` exits
    // here instead of falling through to the GUI.
    let args: Vec<String> = std::env::args().collect();
    match cli::check(&args) {
        Ok(cli::Outcome::Run) => {}
        Ok(cli::Outcome::Help) => {
            print!("{}", cli::usage());
            return;
        }
        Err(e) => {
            eprintln!("clauvolution: {e}\n\n{}", cli::usage());
            std::process::exit(2);
        }
    }
    chdir_to_writable_if_bundled();
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
    let seed_with = load_seed_creatures(&seed_with_paths(&args));

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
            seed_with,
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
    .insert_resource(seed_with)
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

/// Creature files named by `--seed-with`, already read and checked, each
/// with the path it came from. Their genomes join the founding population
/// of a fresh world; a loaded save ignores them.
#[derive(Resource, Default)]
struct SeedWith(Vec<(std::path::PathBuf, save::CreatureFile)>);

/// Every path given to `--seed-with`. The flag may be repeated and each
/// occurrence may name several files, so `--seed-with a.json b.json
/// --seed-with c.json` gives all three.
fn seed_with_paths(args: &[String]) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--seed-with" {
            i += 1;
            while i < args.len() && !args[i].starts_with("--") {
                paths.push(std::path::PathBuf::from(&args[i]));
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    paths
}

/// Read every creature file up front, before the app exists, so a typo or
/// a broken file is reported immediately and the run does not start.
fn load_seed_creatures(paths: &[std::path::PathBuf]) -> SeedWith {
    let mut creatures = Vec::with_capacity(paths.len());
    for path in paths {
        match save::load_creature(path) {
            Ok(creature) => {
                eprintln!(
                    "Seeding with {} ({}, generation {}) from {}",
                    creature
                        .species_name
                        .as_deref()
                        .unwrap_or("an unnamed creature"),
                    creature.strategy.as_deref().unwrap_or("strategy unknown"),
                    creature.generation,
                    path.display()
                );
                creatures.push((path.clone(), creature));
            }
            Err(e) => {
                eprintln!("Failed to load creature file: {}", e);
                std::process::exit(1);
            }
        }
    }
    SeedWith(creatures)
}

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
    seed_with: Res<SeedWith>,
) {
    if let Some(ref path) = load_path.0 {
        let save_path = std::path::Path::new(path).join("save.json");
        if save_path.exists() {
            if !seed_with.0.is_empty() {
                warn!(
                    "--seed-with only applies to a fresh world; ignoring {} creature file(s) while loading a save",
                    seed_with.0.len()
                );
            }
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
    fresh_world(commands, config, innovation, ledger, chronicle, &seed_with);
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

    // Terrain type, elevation and light come from the seed; the fields that
    // change at runtime (vegetation, moisture, nutrients, temperature) are
    // then restored from the save when it carries them.
    let mut rng = rand::rngs::StdRng::seed_from_u64(config.terrain_seed);
    let mut tile_map =
        clauvolution_world::TileMap::generate(config.world_width, config.world_height, &mut rng);
    let terrain_restored = save::restore_terrain(&mut tile_map, state.terrain.as_ref());
    commands.insert_resource(tile_map);

    // Reseed SimRng from the saved seed. (Mid-run save/load diverges from
    // the original trajectory at this point — the RNG state history isn't
    // serialised; same-seed runs from tick 0 still match.)
    commands.insert_resource(SimRng::from_seed(config.terrain_seed));

    // Restore organisms and food. The ledger starts from the loaded energy so
    // the first tick does not compare against an empty world.
    let total_energy = save::spawn_saved_organisms(&mut commands, &state.organisms);
    ledger.reset_baseline(total_energy);
    save::spawn_saved_food(&mut commands, &state.food, config.food_energy_value);

    // Restore phylo tree and chronicle
    save::restore_phylo(&mut phylo, &state.phylo_nodes);
    save::restore_chronicle(&mut chronicle, &state.chronicle_entries);
    chronicle.log(tick.0, "World loaded from save".to_string());
    if let Err(message) = terrain_restored {
        eprintln!("Warning: {}", message);
        chronicle.log(tick.0, message);
    }
}

fn fresh_world(
    commands: Commands,
    config: ResMut<SimConfig>,
    innovation: ResMut<InnovationCounter>,
    ledger: ResMut<EnergyLedger>,
    chronicle: ResMut<WorldChronicle>,
    seed_with: &SeedWith,
) {
    setup_world(commands, config, innovation, ledger, chronicle, seed_with);
}

fn setup_world(
    mut commands: Commands,
    config: ResMut<SimConfig>,
    mut innovation: ResMut<InnovationCounter>,
    mut ledger: ResMut<EnergyLedger>,
    mut chronicle: ResMut<WorldChronicle>,
    seed_with: &SeedWith,
) {
    // Seed deterministic terrain generation
    let mut terrain_rng = rand::rngs::StdRng::seed_from_u64(config.terrain_seed);
    let tile_map = TileMap::generate(config.world_width, config.world_height, &mut terrain_rng);

    // Seed the sim-wide RNG used by food regen, disease, reproduction, etc.
    // Same seed → same simulation trajectory.
    let mut sim_rng = SimRng::from_seed(config.terrain_seed);
    clauvolution_world::spawn_initial_food(&mut commands, &config, &tile_map, &mut sim_rng.0);
    // Imported creatures are extra founders; they draw their placement from
    // the same SimRng as everyone else so a seeded run stays reproducible.
    let imported: Vec<clauvolution_genome::Genome> =
        seed_with.0.iter().map(|(_, c)| c.genome()).collect();
    let founders = clauvolution_sim::spawn_initial_population(
        &mut commands,
        &config,
        &tile_map,
        &mut innovation,
        &imported,
        &mut sim_rng.0,
    );
    ledger.reset_baseline(founders.total_energy);
    info!("{founders}");
    for (path, creature) in &seed_with.0 {
        let origin = match (creature.origin_session.as_deref(), creature.origin_seed) {
            (Some(session), Some(seed)) => format!(" from {session} (seed {seed})"),
            (Some(session), None) => format!(" from {session}"),
            (None, Some(seed)) => format!(" from seed {seed}"),
            (None, None) => String::new(),
        };
        chronicle.log(
            0,
            format!(
                "Seeded with {}{} via {}",
                creature
                    .species_name
                    .as_deref()
                    .unwrap_or("an unnamed creature"),
                origin,
                path.display()
            ),
        );
    }
    commands.insert_resource(founders);
    commands.insert_resource(tile_map);
    commands.insert_resource(sim_rng);

    info!(
        "Clauvolution initialized: {} organisms ({} imported), world {}x{} with biomes (seed {})",
        config.initial_population + imported.len() as u32,
        imported.len(),
        config.world_width,
        config.world_height,
        config.terrain_seed
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
    seed_with: SeedWith,
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
        .insert_resource(seed_with)
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

    let exit = app.run();

    let elapsed = start.elapsed();
    eprintln!("Headless run complete in {:.2}s", elapsed.as_secs_f64());

    // A failed end-of-run save exits non-zero so scripts notice.
    if let AppExit::Error(code) = exit {
        std::process::exit(i32::from(code.get()));
    }
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
         ledger_max_residual,ledger_cumulative_residual,\
         grazes_eat,grazes_attack,kills_consumer,kills_plant,grazer_kills,grazer_kills_consumer,\
         attacks_no_plant_in_reach,grazes_eat_by_plant,kills_plant_by_consumer,\
         plant_kill_energy_consumer,\
         hunter_intents,hunter_strikes,hunter_consumer_in_reach,hunter_kills_consumer,\
         hunter_rejected_size,hunter_rejected_damage,hunter_rejected_both,{},\
         species_pass_tick,species_pass_organisms,species_pass_near_other,\
         species_pass_drifting,species_pass_isolated,{}",
        band_csv_header(),
        geo_csv_header()
    )?;
    for s in &history.snapshots {
        let fl = &s.energy_flows;
        writeln!(
            f,
            "{},{:.1},{},{},{},{},{},{},{},{},{:.2},{:.3},{:.3},{:.3},{:.3},{:.3},{:+.3},{:.3},{:.4},{:.4},{:.3},{:.3},{:.3},{:.3},{:.3},{},{},{},{},{},{},\
             {:.2},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.6},{:.6},\
             {},{},{},{},{},{},{},{},{},{:.3},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
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
            s.feeding.grazes_eat,
            s.feeding.grazes_attack,
            s.feeding.kills_consumer,
            s.feeding.kills_plant,
            s.feeding.grazer_kills,
            s.feeding.grazer_kills_consumer,
            s.feeding.attacks_no_plant_in_reach,
            s.feeding.grazes_eat_by_plant,
            s.feeding.kills_plant_by_consumer,
            s.feeding.plant_kill_energy_consumer,
            s.feeding.hunter_gates.intents,
            s.feeding.hunter_gates.strikes,
            s.feeding.hunter_gates.consumer_in_reach,
            s.feeding.hunter_gates.kills_consumer,
            s.feeding.hunter_gates.rejected_size,
            s.feeding.hunter_gates.rejected_damage,
            s.feeding.hunter_gates.rejected_both,
            band_csv_row(s),
            s.species_pass.tick,
            s.species_pass.organisms,
            s.species_pass.near_other,
            s.species_pass.drifting,
            s.species_pass.isolated,
            geo_csv_row(s),
        )?;
    }
    Ok(())
}

/// Trailing history columns for the hunter-bridge instruments
/// (`plans/2026-09-24-hunter-bridge.md`, step 1): omnivore gate outcomes,
/// then per diet band (`DIET_BAND_KEYS`) organism-ticks, energy kept by
/// source, costs paid, births, deaths by the main causes and the summed age
/// at death, then the off-diagonal cells of the parent-to-child label
/// matrix. Appended after every older column so that cutting them off
/// leaves the history exactly as it was.
fn band_csv_header() -> String {
    let mut cols: Vec<String> = [
        "omnivore_intents",
        "omnivore_strikes",
        "omnivore_consumer_in_reach",
        "omnivore_kills_consumer",
        "omnivore_rejected_size",
        "omnivore_rejected_damage",
        "omnivore_rejected_both",
    ]
    .iter()
    .map(|c| c.to_string())
    .collect();
    for key in clauvolution_core::DIET_BAND_KEYS {
        for field in [
            "ticks",
            "food_items",
            "bite_count",
            "plant_gross",
            "food",
            "bites",
            "kill_consumer",
            "kill_plant",
            "metabolism",
            "movement",
            "strike",
            "births",
            "deaths",
            "deaths_starvation",
            "deaths_predation",
            "deaths_disease",
            "death_age_sum",
        ] {
            cols.push(format!("{key}_{field}"));
        }
    }
    let labels = clauvolution_core::STRATEGY_LABEL_KEYS;
    for (from, from_key) in labels.iter().enumerate() {
        for (to, to_key) in labels.iter().enumerate() {
            if from != to {
                cols.push(format!("cross_{from_key}_{to_key}"));
            }
        }
    }
    cols.join(",")
}

fn band_csv_row(s: &clauvolution_core::PopSnapshot) -> String {
    use clauvolution_core::DeathCause;
    let g = &s.feeding.omnivore_gates;
    let mut cols: Vec<String> = [
        g.intents,
        g.strikes,
        g.consumer_in_reach,
        g.kills_consumer,
        g.rejected_size,
        g.rejected_damage,
        g.rejected_both,
    ]
    .iter()
    .map(|n| n.to_string())
    .collect();
    let b = &s.bands;
    for band in 0..clauvolution_core::DIET_BAND_COUNT {
        let e = &b.energy[band];
        let d = &b.deaths[band];
        cols.push(e.organism_ticks.to_string());
        cols.push(e.food_items.to_string());
        cols.push(e.bites.to_string());
        for v in [
            e.plant_gross,
            e.food,
            e.bite_energy,
            e.consumer_kill_energy,
            e.plant_kill_energy,
            e.metabolism,
            e.movement,
            e.strike,
        ] {
            cols.push(format!("{v:.3}"));
        }
        cols.push(b.births[band].to_string());
        cols.push(d.total().to_string());
        for cause in [
            DeathCause::Starvation,
            DeathCause::Predation,
            DeathCause::Disease,
        ] {
            cols.push(d.count[cause as usize].to_string());
        }
        cols.push(d.age_sum.iter().sum::<u64>().to_string());
    }
    for (from, row) in b.label_transitions.iter().enumerate() {
        for (to, n) in row.iter().enumerate() {
            if from != to {
                cols.push(n.to_string());
            }
        }
    }
    cols.join(",")
}

/// Trailing history columns for the phase 2 geography instruments
/// (`plans/2026-09-25-phase2-biomes.md`, step 1), after the diet-band
/// columns: organism-ticks in all and on deep and shallow water by kind
/// and aquatic band, movement energy paid on water, crossings, new-region
/// events, food items spawned and eaten by where they were, population by
/// biome and by region column per strategy label, and the region and
/// biome separation numbers with their shuffled nulls.
fn geo_csv_header() -> String {
    let mut cols: Vec<String> = Vec::new();
    for kind in GEO_KIND_KEYS {
        for aq in AQUATIC_BAND_KEYS {
            for place in ["all", "deep", "shallow"] {
                cols.push(format!("ticks_{place}_{kind}_{aq}"));
            }
        }
    }
    for kind in GEO_KIND_KEYS {
        for depth in WATER_DEPTH_KEYS {
            cols.push(format!("water_movement_{kind}_{depth}"));
        }
    }
    for c in [
        "crossings_plant",
        "crossings_consumer",
        "new_region_events",
        "food_spawned_deep",
        "food_spawned_shallow",
        "food_spawned_land",
        "food_eaten",
        "food_eaten_deep",
        "food_eaten_shallow",
    ] {
        cols.push(c.to_string());
    }
    for label in STRATEGY_LABEL_KEYS {
        for biome in BIOME_KEYS {
            cols.push(format!("{label}_on_{biome}"));
        }
    }
    for label in STRATEGY_LABEL_KEYS {
        for slot in 0..REGION_SLOTS {
            cols.push(format!("{label}_in_r{slot}"));
        }
        cols.push(format!("{label}_in_rother"));
        cols.push(format!("{label}_in_rminor"));
    }
    for place in ["region", "biome"] {
        for field in ["sep", "null", "species", "confined", "confined_null"] {
            cols.push(format!("{place}_{field}"));
        }
    }
    cols.join(",")
}

fn geo_csv_row(s: &PopSnapshot) -> String {
    let g = &s.geo;
    let c = &s.census;
    let mut cols: Vec<String> = Vec::new();
    for kind in 0..2 {
        for aq in 0..AQUATIC_BAND_COUNT {
            cols.push(g.ticks[kind][aq].to_string());
            cols.push(g.water_ticks[kind][0][aq].to_string());
            cols.push(g.water_ticks[kind][1][aq].to_string());
        }
    }
    for kind in 0..2 {
        for depth in 0..2 {
            cols.push(format!("{:.3}", g.water_movement[kind][depth]));
        }
    }
    for n in [
        g.crossings[0],
        g.crossings[1],
        g.new_region_events,
        g.food_spawned[0],
        g.food_spawned[1],
        g.food_spawned[2],
        g.food_eaten,
        g.food_eaten_on_water[0],
        g.food_eaten_on_water[1],
    ] {
        cols.push(n.to_string());
    }
    for row in &c.by_biome {
        cols.extend(row.iter().map(|n| n.to_string()));
    }
    for row in &c.by_region {
        cols.extend(row.iter().map(|n| n.to_string()));
    }
    for sep in [&c.region, &c.biome] {
        cols.push(format!("{:.4}", sep.observed));
        cols.push(format!("{:.4}", sep.null));
        cols.push(sep.species.to_string());
        cols.push(sep.confined.to_string());
        cols.push(sep.confined_null.to_string());
    }
    cols.join(",")
}

/// Ticks between rows of the headless summary's geography timeline.
const GEOGRAPHY_TIMELINE_STEP: u64 = 1000;

/// Samples before this tick are left out of the run-level separation means
/// and histograms: the founders are placed by biome area, not by species,
/// so the opening seconds say nothing about separation.
const GEOGRAPHY_SETTLE_TICK: u64 = 1000;

/// Share of `part` in `whole` as a percentage, 0 when `whole` is 0.
fn pct(part: f64, whole: f64) -> f64 {
    if whole > 0.0 {
        100.0 * part / whole
    } else {
        0.0
    }
}

/// The phase 2 geography instruments (`plans/2026-09-25-phase2-biomes.md`,
/// step 1): the region layout, a timeline of population by region and
/// the separation numbers every `GEOGRAPHY_TIMELINE_STEP` ticks, then run
/// totals for crossings, time on water by aquatic band, food landing on
/// water, and the separation means and dominant-share histograms after
/// `GEOGRAPHY_SETTLE_TICK`.
fn print_geography_summary(run: &GeographyStats, history: &PopulationHistory, tile_map: &TileMap) {
    let regions = &tile_map.regions;
    let land = tile_map
        .tiles
        .iter()
        .filter(|t| !t.terrain.is_water())
        .count();
    eprintln!("Geography (phase 2 step 1 instruments):");
    eprintln!(
        "  Regions: {} major {:?}, {} minor components ({} tiles), land {} tiles",
        regions.count(),
        regions.sizes,
        regions.minor_components,
        regions.minor_tiles,
        land
    );
    if history.snapshots.is_empty() {
        return;
    }
    let shown = regions.count().min(REGION_SLOTS);
    eprintln!(
        "  Timeline (every {GEOGRAPHY_TIMELINE_STEP} ticks; plants and consumers by region r0..r{} / minor / water at the sample; separation observed/null (species counted, confined observed/null); crossings plant/consumer and new-region events in the window):",
        shown.saturating_sub(1)
    );
    let mut window = GeographyStats::default();
    let mut next = GEOGRAPHY_TIMELINE_STEP;
    let last = history.snapshots.len() - 1;
    for (i, s) in history.snapshots.iter().enumerate() {
        window.add(&s.geo);
        if s.tick >= next || i == last {
            let c = &s.census;
            let by_kind = |labels: &[usize]| {
                let mut cols: Vec<String> = (0..shown)
                    .map(|r| labels.iter().map(|&l| c.by_region[l][r]).sum::<u32>())
                    .map(|n| n.to_string())
                    .collect();
                cols.push(
                    labels
                        .iter()
                        .map(|&l| c.by_region[l][REGION_SLOTS + 1])
                        .sum::<u32>()
                        .to_string(),
                );
                cols.push(
                    labels
                        .iter()
                        .map(|&l| c.by_biome[l][0] + c.by_biome[l][1])
                        .sum::<u32>()
                        .to_string(),
                );
                cols.join("/")
            };
            eprintln!(
                "    tick {:>5}: plants {} consumers {} | region {:.3}/{:.3} ({}, {}/{}) biome {:.3}/{:.3} ({}, {}/{}) | crossings {}/{} new-region {}",
                s.tick,
                by_kind(&[0]),
                by_kind(&[1, 2, 3]),
                c.region.observed,
                c.region.null,
                c.region.species,
                c.region.confined,
                c.region.confined_null,
                c.biome.observed,
                c.biome.null,
                c.biome.species,
                c.biome.confined,
                c.biome.confined_null,
                window.crossings[0],
                window.crossings[1],
                window.new_region_events,
            );
            while next <= s.tick {
                next += GEOGRAPHY_TIMELINE_STEP;
            }
            window = GeographyStats::default();
        }
    }

    let run_secs = history.snapshots[last].tick as f64 / 30.0;
    eprintln!(
        "  Crossings (whole run): plants {}, consumers {} ({:.2} per second); new-region events {}",
        run.crossings[0],
        run.crossings[1],
        (run.crossings[0] + run.crossings[1]) as f64 / run_secs.max(1.0),
        run.new_region_events
    );
    eprintln!("  Organism-ticks on water by aquatic band (share of the band's organism-ticks on deep / shallow water):");
    for (kind, name) in GEO_KIND_KEYS.iter().enumerate() {
        let row: Vec<String> = (0..AQUATIC_BAND_COUNT)
            .map(|aq| {
                let all = run.ticks[kind][aq] as f64;
                format!(
                    "{} {:.0} ticks {:.2}% / {:.2}%",
                    AQUATIC_BAND_KEYS[aq],
                    all,
                    pct(run.water_ticks[kind][0][aq] as f64, all),
                    pct(run.water_ticks[kind][1][aq] as f64, all)
                )
            })
            .collect();
        eprintln!("    {:<9} {}", name, row.join(" | "));
    }
    eprintln!(
        "  Movement energy paid on deep / shallow water: plants {:.1} / {:.1}, consumers {:.1} / {:.1}",
        run.water_movement[0][0],
        run.water_movement[0][1],
        run.water_movement[1][0],
        run.water_movement[1][1]
    );
    let spawned: u64 = run.food_spawned.iter().sum();
    eprintln!(
        "  Food items spawned: {} ({:.1}% on deep water, {:.1}% on shallow); eaten {} ({:.2}% by eaters on deep water, {:.2}% on shallow)",
        spawned,
        pct(run.food_spawned[0] as f64, spawned as f64),
        pct(run.food_spawned[1] as f64, spawned as f64),
        run.food_eaten,
        pct(run.food_eaten_on_water[0] as f64, run.food_eaten as f64),
        pct(run.food_eaten_on_water[1] as f64, run.food_eaten as f64)
    );

    let settled: Vec<&PopSnapshot> = history
        .snapshots
        .iter()
        .filter(|s| s.tick >= GEOGRAPHY_SETTLE_TICK)
        .collect();
    if settled.is_empty() {
        return;
    }
    let n = settled.len() as f64;
    for (name, pick) in [
        (
            "region",
            (|c: &GeographyCensus| c.region) as fn(&GeographyCensus) -> Separation,
        ),
        ("biome", |c: &GeographyCensus| c.biome),
    ] {
        let mut obs = 0.0;
        let mut null = 0.0;
        let mut species = 0.0;
        let mut confined = 0.0;
        let mut confined_null = 0.0;
        let mut hist = [0u64; SHARE_BINS];
        let mut hist_null = [0u64; SHARE_BINS];
        for s in &settled {
            let sep = pick(&s.census);
            obs += sep.observed as f64;
            null += sep.null as f64;
            species += sep.species as f64;
            confined += sep.confined as f64;
            confined_null += sep.confined_null as f64;
            for b in 0..SHARE_BINS {
                hist[b] += sep.hist[b] as u64;
                hist_null[b] += sep.hist_null[b] as u64;
            }
        }
        eprintln!(
            "  {name} separation from tick {GEOGRAPHY_SETTLE_TICK}: mean {:.3} against null {:.3} over {:.1} species counted; confined {:.1} against {:.1} under the null",
            obs / n,
            null / n,
            species / n,
            confined / n,
            confined_null / n
        );
        let fmt = |h: &[u64; SHARE_BINS]| {
            h.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        };
        eprintln!("    dominant-share histogram, species-samples in 0.05 bins from 0:");
        eprintln!("      observed: {}", fmt(&hist));
        eprintln!("      null:     {}", fmt(&hist_null));
    }
    eprintln!();
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
    bite_reach: Option<f32>,
    mouthless_bite: Option<f32>,
    kill_transfer: Option<f32>,
    kill_transfer_animal: Option<f32>,
    kill_transfer_plant: Option<f32>,
    strike_cost: Option<f32>,
    photo_drag: Option<f32>,
    leaf_capacity: Option<f32>,
    founder_diet_spread: Option<f32>,
    animal_efficiency: Option<f32>,
    diet_exponent: Option<f32>,
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
            bite_reach: flag(args, "--bite-reach"),
            mouthless_bite: flag(args, "--mouthless-bite"),
            kill_transfer: flag(args, "--kill-transfer"),
            kill_transfer_animal: flag(args, "--kill-transfer-animal"),
            kill_transfer_plant: flag(args, "--kill-transfer-plant"),
            strike_cost: flag(args, "--strike-cost"),
            photo_drag: flag(args, "--photo-drag"),
            leaf_capacity: flag(args, "--leaf-capacity"),
            founder_diet_spread: flag(args, "--founder-diet-spread"),
            animal_efficiency: flag(args, "--animal-efficiency"),
            diet_exponent: flag(args, "--diet-exponent"),
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
    set(&mut config.bite_reach, overrides.bite_reach, "bite_reach");
    set(
        &mut config.mouthless_bite_bonus,
        overrides.mouthless_bite,
        "mouthless_bite_bonus",
    );
    // `--kill-transfer` predates the split by victim tissue and sets both
    // shares, so older audit commands still mean what they meant; a
    // per-tissue flag given alongside it wins for its own share.
    set(
        &mut config.kill_transfer_animal,
        overrides.kill_transfer_animal.or(overrides.kill_transfer),
        "kill_transfer_animal",
    );
    set(
        &mut config.kill_transfer_plant,
        overrides.kill_transfer_plant.or(overrides.kill_transfer),
        "kill_transfer_plant",
    );
    set(
        &mut config.strike_cost,
        overrides.strike_cost,
        "strike_cost",
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
    // `cli::check` has already refused a value below
    // `MIN_DIET_EFFICIENCY_EXPONENT`.
    set(
        &mut config.diet_efficiency_exponent,
        overrides.diet_exponent,
        "diet_efficiency_exponent",
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
    bands: Res<clauvolution_core::DietBandStats>,
    history: Res<clauvolution_core::PopulationHistory>,
    ledger: Res<clauvolution_core::EnergyLedger>,
    config: Res<clauvolution_core::SimConfig>,
    save_at_end: Res<HeadlessSaveAtEnd>,
    dump_path: Option<Res<HeadlessDumpHistoryPath>>,
    founders: Option<Res<clauvolution_sim::FounderReport>>,
    save_report: Res<clauvolution_sim::SaveReport>,
    mut events: EventWriter<clauvolution_core::WorldEventRequest>,
    mut exit: EventWriter<AppExit>,
    (geo, tile_map): (Res<GeographyStats>, Res<TileMap>),
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
            print_diet_band_summary(&bands, &predation, &history);
            print_species_pass_summary(&history, founders.is_some());
            print_geography_summary(&geo, &history, &tile_map);
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

/// Ticks between rows of the headless summary's grazer timeline.
const GRAZER_TIMELINE_STEP: u64 = 500;

/// The first row boundary after `first_tick`, the first snapshot's tick. A
/// run from tick 0 gets its first row at `GRAZER_TIMELINE_STEP`, as before.
/// A run loaded from a save starts at the save's tick, so a fixed first
/// boundary would already have passed and the first row would cover a single
/// snapshot; this puts it at the next multiple of the step instead.
fn first_timeline_boundary(first_tick: u64) -> u64 {
    (first_tick / GRAZER_TIMELINE_STEP + 1) * GRAZER_TIMELINE_STEP
}

/// Grazer size, armour and eat grazing through the run, one row per
/// `GRAZER_TIMELINE_STEP` ticks. Eat reach is `bite_reach × body size`, so
/// the size column is also the grazers' mean reach in units of `bite_reach`.
/// The grazes column sums the snapshots since the previous row.
fn print_grazer_timeline(
    history: &clauvolution_core::PopulationHistory,
    config: &clauvolution_core::SimConfig,
) {
    if history.snapshots.is_empty() {
        return;
    }
    eprintln!();
    eprintln!(
        "Grazer timeline (every {GRAZER_TIMELINE_STEP} ticks; eat reach = {} x body size):",
        config.bite_reach
    );
    eprintln!(
        "   tick  plants  grazers  hunters  grazer size  grazer armour  eat grazes  per grazer-s"
    );
    let mut next = first_timeline_boundary(history.snapshots[0].tick);
    let mut grazes = 0u64;
    let mut grazer_seconds = 0u64;
    let last = history.snapshots.len() - 1;
    for (i, s) in history.snapshots.iter().enumerate() {
        grazes += s.feeding.grazes_eat;
        grazer_seconds += s.grazers as u64;
        if s.tick >= next || i == last {
            eprintln!(
                "  {:>5} {:>7} {:>8} {:>8} {:>12.2} {:>14.2} {:>11} {:>13.2}",
                s.tick,
                s.plants,
                s.grazers,
                s.hunters,
                s.avg_grazer_body_size,
                s.avg_grazer_armor,
                grazes,
                grazes as f64 / grazer_seconds.max(1) as f64
            );
            while next <= s.tick {
                next += GRAZER_TIMELINE_STEP;
            }
            grazes = 0;
            grazer_seconds = 0;
        }
    }
}

/// Ticks between blocks of the headless summary's diet-band timeline.
const DIET_BAND_TIMELINE_STEP: u64 = 500;

/// Consumer strategy labels in the order `DietBandStats::label_energy`
/// returns them.
const CONSUMER_LABELS: [&str; 3] = ["grazers", "omnivores", "hunters"];

/// One row of the diet-band tables: mean alive, energy kept per
/// organism-tick and its split by source, cost per organism-tick, births,
/// deaths by cause and mean age at death.
fn print_band_row(
    label: &str,
    energy: &clauvolution_core::BandEnergy,
    deaths: &clauvolution_core::BandDeaths,
    births: u64,
    ticks: u64,
) {
    use clauvolution_core::DeathCause;
    let per_tick = |v: f64| v / energy.organism_ticks.max(1) as f64;
    let share = |v: f64| {
        let income = energy.income();
        if income > 0.0 {
            100.0 * v / income
        } else {
            0.0
        }
    };
    eprintln!(
        "    {:<13} {:>7.1} {:>8.3} {:>9.3} {:>10.3} {:>5.2} {:>6.1} {:>6.1} {:>6.1} {:>6.1} {:>7} {:>7} {:>6} {:>6} {:>6} {:>5} {:>9.0}",
        label,
        energy.organism_ticks as f64 / ticks.max(1) as f64,
        per_tick(energy.income()),
        per_tick(energy.cost()),
        per_tick(energy.plant_gross),
        if energy.plant_gross > 0.0 {
            (energy.food + energy.bite_energy) / energy.plant_gross
        } else {
            0.0
        },
        share(energy.food),
        share(energy.bite_energy),
        share(energy.consumer_kill_energy),
        share(energy.plant_kill_energy),
        births,
        deaths.total(),
        deaths.count[DeathCause::Starvation as usize],
        deaths.count[DeathCause::Predation as usize],
        deaths.count[DeathCause::Disease as usize],
        deaths.count[DeathCause::OldAge as usize],
        deaths.mean_age(),
    );
}

fn print_band_header() {
    eprintln!(
        "    band            alive  in/tick cost/tick taken/tick   eff  food%  bite% ckill% pkill%  births  deaths  starv   pred    dis   old  mean age"
    );
}

/// The species-distance instruments (`plans/2026-09-24-innovation-keying.md`,
/// step 1): per classification pass, the share of organisms within the join
/// threshold of another species' representative, and the organisms past the
/// stay threshold from their own that are also past the join threshold from
/// every other, which is what founding a species needs.
/// `founded_here` is whether this run spawned the founders (a
/// `FounderReport` exists). Only then is the first recorded pass the founding
/// pass; after `--load` every recorded pass is an ordinary one.
fn print_species_pass_summary(history: &clauvolution_core::PopulationHistory, founded_here: bool) {
    // Each pass shows up in every snapshot until the next one; keep one copy.
    let mut passes: Vec<clauvolution_core::SpeciesPassCounts> = Vec::new();
    for s in &history.snapshots {
        let p = s.species_pass;
        if p.tick > 0 && passes.last().map(|l| l.tick) != Some(p.tick) {
            passes.push(p);
        }
    }
    eprintln!();
    eprintln!("Species classification passes (plans/2026-09-24-innovation-keying.md):");
    if passes.is_empty() {
        eprintln!("  none recorded");
        return;
    }
    // In a fresh world the first pass founds species from unclassified
    // founders, so the drift counts mean something only after it. A loaded
    // world's species already exist, so every pass counts.
    let later = if founded_here {
        &passes[1..]
    } else {
        &passes[..]
    };
    let share = |p: &clauvolution_core::SpeciesPassCounts| {
        if p.organisms > 0 {
            p.near_other as f64 / p.organisms as f64
        } else {
            0.0
        }
    };
    if founded_here {
        eprintln!(
            "  passes: {} (first at tick {}, founding {} organisms)",
            passes.len(),
            passes[0].tick,
            passes[0].organisms
        );
    } else {
        eprintln!(
            "  passes: {} (first at tick {}; loaded world, no founding pass)",
            passes.len(),
            passes[0].tick
        );
    }
    if !later.is_empty() {
        let shares: Vec<f64> = later.iter().map(share).collect();
        let min = shares.iter().cloned().fold(f64::MAX, f64::min);
        let max = shares.iter().cloned().fold(0.0, f64::max);
        let mean = shares.iter().sum::<f64>() / shares.len() as f64;
        eprintln!(
            "  within join of another species' rep: mean {:.1}%, min {:.1}%, max {:.1}% of organisms per pass",
            mean * 100.0,
            min * 100.0,
            max * 100.0
        );
        let drifting: u64 = later.iter().map(|p| p.drifting as u64).sum();
        let isolated: u64 = later.iter().map(|p| p.isolated as u64).sum();
        eprintln!(
            "  past stay from own rep: {} over the passes; of them past join from every other rep: {}",
            drifting, isolated
        );
    }
    if let Some(last) = passes.last() {
        eprintln!(
            "  last pass (tick {}): {} organisms, {} near another rep, {} drifting, {} isolated",
            last.tick, last.organisms, last.near_other, last.drifting, last.isolated
        );
    }
}

/// The hunter-bridge instruments (`plans/2026-09-24-hunter-bridge.md`,
/// step 1): a per-band block every `DIET_BAND_TIMELINE_STEP` ticks, the
/// run totals by band and by label, mean age at death by cause, and the
/// parent-to-child label matrix. "alive" is organism-ticks over the window's
/// ticks; "in/tick" (energy kept) and "cost/tick" (metabolism, movement,
/// strikes) are per organism-tick; "taken/tick" is plant tissue taken from
/// food items and bites before digestion, per organism-tick, and "eff" what
/// share of it was kept; the percentages split income by source (food
/// items, eat bites, consumer kills, plant kills).
fn print_diet_band_summary(
    run: &clauvolution_core::DietBandStats,
    predation: &clauvolution_core::PredationStats,
    history: &clauvolution_core::PopulationHistory,
) {
    use clauvolution_core::{DeathCause, DietBandStats, DIET_BAND_LABELS, STRATEGY_LABEL_KEYS};
    if history.snapshots.is_empty() {
        return;
    }
    eprintln!("Diet-band timeline (every {DIET_BAND_TIMELINE_STEP} ticks; consumers in halves of each strategy label):");
    let mut window = DietBandStats::default();
    let mut window_start = 0u64;
    let mut next = DIET_BAND_TIMELINE_STEP;
    let last = history.snapshots.len() - 1;
    for (i, s) in history.snapshots.iter().enumerate() {
        window.add(&s.bands);
        if s.tick >= next || i == last {
            let ticks = s.tick - window_start;
            eprintln!(
                "  to tick {} (crossings {}; omnivore -> grazer {}, omnivore -> hunter {}, grazer -> omnivore {}, hunter -> omnivore {})",
                s.tick,
                window.crossings(),
                window.label_transitions[2][1],
                window.label_transitions[2][3],
                window.label_transitions[1][2],
                window.label_transitions[3][2],
            );
            print_band_header();
            for (band, label) in DIET_BAND_LABELS.iter().enumerate() {
                let e = &window.energy[band];
                let d = &window.deaths[band];
                if e.organism_ticks == 0 && d.total() == 0 && window.births[band] == 0 {
                    continue;
                }
                print_band_row(label, e, d, window.births[band], ticks);
            }
            while next <= s.tick {
                next += DIET_BAND_TIMELINE_STEP;
            }
            window_start = s.tick;
            window = DietBandStats::default();
        }
    }

    let run_ticks = history.snapshots[last].tick;
    eprintln!();
    eprintln!("Diet bands, whole run:");
    print_band_header();
    for (band, label) in DIET_BAND_LABELS.iter().enumerate() {
        print_band_row(
            label,
            &run.energy[band],
            &run.deaths[band],
            run.births[band],
            run_ticks,
        );
    }
    let label_energy = run.label_energy();
    let label_deaths = run.label_deaths();
    for (i, name) in CONSUMER_LABELS.iter().enumerate() {
        let births = run.births[2 * i] + run.births[2 * i + 1];
        print_band_row(name, &label_energy[i], &label_deaths[i], births, run_ticks);
    }
    eprintln!("  Energy kept / paid by label (whole run):");
    eprintln!(
        "    label           food items      eat bites  consumer kills   plant kills     metabolism     movement     strikes"
    );
    for (i, name) in CONSUMER_LABELS.iter().enumerate() {
        let e = &label_energy[i];
        eprintln!(
            "    {:<10} {:>15.1} {:>14.1} {:>15.1} {:>13.1} {:>14.1} {:>12.1} {:>11.1}",
            name,
            e.food,
            e.bite_energy,
            e.consumer_kill_energy,
            e.plant_kill_energy,
            e.metabolism,
            e.movement,
            e.strike
        );
    }
    eprintln!("  Mean age at death by cause (starvation / predation / disease / old age):");
    for (i, name) in CONSUMER_LABELS.iter().enumerate() {
        let d = &label_deaths[i];
        eprintln!(
            "    {:<10} {:.0} / {:.0} / {:.0} / {:.0}",
            name,
            d.mean_age_of(DeathCause::Starvation),
            d.mean_age_of(DeathCause::Predation),
            d.mean_age_of(DeathCause::Disease),
            d.mean_age_of(DeathCause::OldAge),
        );
    }
    eprintln!("  Births by the reproducing parent's label (rows) and the child's (columns):");
    eprint!("    {:<10}", "");
    for key in STRATEGY_LABEL_KEYS {
        eprint!(" {key:>9}");
    }
    eprintln!();
    for (from, row) in run.label_transitions.iter().enumerate() {
        eprint!("    {:<10}", STRATEGY_LABEL_KEYS[from]);
        for n in row {
            eprint!(" {n:>9}");
        }
        eprintln!();
    }
    eprintln!(
        "  Crossings: {} of {} births; {} births had a mate of another label",
        run.crossings(),
        run.label_transitions.iter().flatten().sum::<u64>(),
        run.mixed_label_matings
    );
    let g = &predation.feeding.omnivore_gates;
    eprintln!(
        "  Omnivore attacks: {} intents, {} strikes, {} with a consumer in reach: {} kills, {} plant instead, {} size-only, {} damage-only, {} both, {} mixed",
        g.intents,
        g.strikes,
        g.consumer_in_reach,
        g.kills_consumer,
        g.kills_plant_instead,
        g.rejected_size,
        g.rejected_damage,
        g.rejected_both,
        g.rejected_mixed
    );
    eprintln!();
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
    // Who the kills were: the graze/attack split in
    // plans/2026-09-21-pyramid-top.md is judged by these lines.
    let feeding = &predation.feeding;
    eprintln!(
        "    kills of consumers / plants: {} / {}",
        feeding.kills_consumer, feeding.kills_plant
    );
    eprintln!(
        "    by grazers (diet < 0):       {} ({} of consumers)",
        feeding.grazer_kills, feeding.grazer_kills_consumer
    );
    eprintln!(
        "    plants killed by consumers:  {} (kept {:.1} energy)",
        feeding.kills_plant_by_consumer, feeding.plant_kill_energy_consumer
    );
    eprintln!(
        "    plants killed by hunters:    {} (kept {:.1} energy)",
        predation.hunter_plant_kills, predation.hunter_plant_kill_energy
    );
    eprintln!("  by Old age:            {}", stats.deaths_by_cause[2]);
    eprintln!("  by Disease:            {}", stats.deaths_by_cause[3]);
    eprintln!("  by Event:              {}", stats.deaths_by_cause[4]);
    if let Some(f) = founders {
        let died = stats.founder_hunter_deaths;
        let mean = if died > 0 {
            stats.founder_hunter_age_sum as f64 / died as f64
        } else {
            0.0
        };
        eprintln!(
            "Founding hunters died:   {} of {} (mean age {:.0} ticks, oldest {})",
            died, f.hunters, mean, stats.founder_hunter_age_max
        );
    }
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
    print_grazer_timeline(history, config);
    eprintln!();
    eprintln!("Predation funnel:");
    eprintln!("  Attack intents:      {}", predation.attacks_attempted);
    eprintln!("  Targets considered:  {}", predation.targets_considered);
    eprintln!("  Rejected (size):     {}", predation.rejected_size_gate);
    eprintln!("  Rejected (damage):   {}", predation.rejected_damage);
    eprintln!("  Kills:               {}", predation.kills);
    eprintln!(
        "  Strikes (paid):      {} ({:.1} energy)",
        predation.strikes, predation.strike_energy
    );
    eprintln!(
        "  No plant in reach:   {}",
        predation.feeding.attacks_no_plant_in_reach
    );
    eprintln!(
        "  Grazes (eat/attack): {} / {}",
        predation.feeding.grazes_eat, predation.feeding.grazes_attack
    );
    eprintln!(
        "    eat bites by plants: {}",
        predation.feeding.grazes_eat_by_plant
    );
    // Step 5 of plans/2026-09-21-pyramid-top.md: what stops consumer
    // attackers on the positive side of the diet axis from killing consumers.
    eprintln!();
    eprintln!("Consumer-prey gates (attacks with a consumer in reach):");
    eprintln!(
        "  band            intents  strikes  in reach    kill  plant  size-only  damage-only  both  mixed"
    );
    for (label, g) in [
        ("diet >= 0", &predation.diet_nonneg_gates),
        ("hunters", &predation.feeding.hunter_gates),
        ("founding hunt.", &predation.founder_hunter_gates),
    ] {
        eprintln!(
            "  {:<14} {:>8} {:>8} {:>9} {:>7} {:>6} {:>10} {:>12} {:>5} {:>6}",
            label,
            g.intents,
            g.strikes,
            g.consumer_in_reach,
            g.kills_consumer,
            g.kills_plant_instead,
            g.rejected_size,
            g.rejected_damage,
            g.rejected_both,
            g.rejected_mixed
        );
    }
    let fmt_ages = |ages: &[u64]| {
        ages.iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(" / ")
    };
    eprintln!("  Hunter intents by age (<100/<200/<300/<500/<1000/older):");
    eprintln!(
        "    all:         {}",
        fmt_ages(&predation.hunter_intent_ages)
    );
    eprintln!(
        "    in reach:    {}",
        fmt_ages(&predation.hunter_reach_ages)
    );
    let fired = predation.founder_hunters_fired.len() as u64;
    let reached = predation.founder_hunters_reached.len() as u64;
    let mean = |sum: u64, n: u64| if n > 0 { sum as f64 / n as f64 } else { 0.0 };
    eprintln!(
        "  Founding hunters that fired: {} (first at mean age {:.0}); with a consumer in reach: {} (first at mean age {:.0})",
        fired,
        mean(predation.founder_hunter_first_intent_age_sum, fired),
        reached,
        mean(predation.founder_hunter_first_reach_age_sum, reached)
    );
    eprintln!(
        "  Founding hunters that killed a consumer: {}",
        predation.founder_hunters_killed.len()
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grazer_timeline_first_row_starts_from_the_first_snapshot() {
        // A run from tick 0: the first row lands at the first step, as before.
        assert_eq!(first_timeline_boundary(30), GRAZER_TIMELINE_STEP);
        // A run loaded at tick 3010 must not treat 500 as its first boundary.
        assert_eq!(first_timeline_boundary(3041), 3500);
        // A first snapshot exactly on a boundary starts the next interval.
        assert_eq!(first_timeline_boundary(1000), 1500);
    }

    #[test]
    fn seed_with_accepts_repeats_and_several_paths_per_flag() {
        let args: Vec<String> = [
            "clauvolution",
            "--seed-with",
            "a.json",
            "b.json",
            "--seed",
            "1",
            "--seed-with",
            "c.json",
            "--headless",
            "10",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let paths = seed_with_paths(&args);
        let names: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
        assert_eq!(names, ["a.json", "b.json", "c.json"]);

        let none: Vec<String> = ["clauvolution", "--headless", "10"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(seed_with_paths(&none).is_empty());
    }
}
