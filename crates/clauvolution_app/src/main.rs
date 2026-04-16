use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use clauvolution_body::BodyPlugin;
use bevy::window::PrimaryWindow;
use clauvolution_core::*;
use clauvolution_sim::save;
use clauvolution_genome::InnovationCounter;
use clauvolution_render::{MainCamera, RenderPlugin};
use clauvolution_phylogeny::{PhylogenyPlugin, PhyloTree, WorldChronicle};
use clauvolution_sim::SimPlugin;
use clauvolution_world::{self, TileMap, WorldPlugin};

fn main() {
    let screenshot_mode = std::env::args().any(|a| a == "--screenshot");
    let load_path = std::env::args()
        .position(|a| a == "--load")
        .and_then(|i| std::env::args().nth(i + 1));

    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Clauvolution".to_string(),
            resolution: (1920.0, 1080.0).into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(CorePlugin)
    .add_plugins(WorldPlugin)
    .add_plugins(BodyPlugin)
    .add_plugins(SimPlugin)
    .add_plugins(PhylogenyPlugin)
    .add_plugins(RenderPlugin)
    .insert_resource(InnovationCounter(100))
    .insert_resource(LoadPath(load_path))
    .add_systems(Startup, (startup_system, set_window_title));

    if screenshot_mode {
        app.insert_resource(ScreenshotSchedule::new())
            .add_systems(Update, screenshot_system);
    }

    app.run();
}

#[derive(Resource)]
struct LoadPath(Option<String>);

fn startup_system(
    commands: Commands,
    config: Res<SimConfig>,
    innovation: ResMut<InnovationCounter>,
    stats: ResMut<SimStats>,
    tick: ResMut<TickCounter>,
    season: ResMut<Season>,
    phylo: ResMut<PhyloTree>,
    chronicle: ResMut<WorldChronicle>,
    load_path: Res<LoadPath>,
) {
    if let Some(ref path) = load_path.0 {
        let save_path = std::path::Path::new(path).join("save.json");
        if save_path.exists() {
            load_saved_world(commands, config, innovation, stats, tick, season, phylo, chronicle, &save_path);
            return;
        } else {
            warn!("Save file not found: {}, starting fresh", save_path.display());
        }
    }
    fresh_world(commands, config, innovation, stats);
}

fn load_saved_world(
    mut commands: Commands,
    config: Res<SimConfig>,
    mut innovation: ResMut<InnovationCounter>,
    mut stats: ResMut<SimStats>,
    mut tick: ResMut<TickCounter>,
    mut season: ResMut<Season>,
    mut phylo: ResMut<PhyloTree>,
    mut chronicle: ResMut<WorldChronicle>,
    save_path: &std::path::Path,
) {
    let Some(state) = save::load_world(save_path) else {
        warn!("Failed to load save file, starting fresh");
        return;
    };

    info!("Loading world from {} ({} organisms, {} food)", save_path.display(), state.organisms.len(), state.food.len());

    // Restore state
    tick.0 = state.tick;
    season.current_tick = state.season_tick;
    stats.total_births = state.stats.total_births;
    stats.total_deaths = state.stats.total_deaths;
    stats.max_generation = state.stats.max_generation;
    innovation.0 = state.innovation_counter;

    // Generate terrain (same seed = same terrain for now)
    let mut rng = rand::thread_rng();
    let tile_map = clauvolution_world::TileMap::generate(config.world_width, config.world_height, &mut rng);
    commands.insert_resource(tile_map);

    // Restore organisms and food
    save::spawn_saved_organisms(&mut commands, &state.organisms);
    save::spawn_saved_food(&mut commands, &state.food);

    // Restore phylo tree and chronicle
    save::restore_phylo(&mut phylo, &state.phylo_nodes);
    save::restore_chronicle(&mut chronicle, &state.chronicle_entries);
    chronicle.log(tick.0, "World loaded from save".to_string());

    stats.total_organisms = state.organisms.len() as u32;
}

fn fresh_world(
    commands: Commands,
    config: Res<SimConfig>,
    innovation: ResMut<InnovationCounter>,
    stats: ResMut<SimStats>,
) {
    setup_world(commands, config, innovation, stats);
}

fn setup_world(
    mut commands: Commands,
    config: Res<SimConfig>,
    mut innovation: ResMut<InnovationCounter>,
    mut stats: ResMut<SimStats>,
) {
    let mut rng = rand::thread_rng();

    let tile_map = TileMap::generate(config.world_width, config.world_height, &mut rng);
    clauvolution_world::spawn_initial_food(&mut commands, &config, &tile_map, &mut rng);
    clauvolution_sim::spawn_initial_population(&mut commands, &config, &mut innovation, &mut rng);
    commands.insert_resource(tile_map);

    stats.total_organisms = config.initial_population;

    info!(
        "Clauvolution initialized: {} organisms, world {}x{} with biomes",
        config.initial_population, config.world_width, config.world_height
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

        let path = session.screenshot_path(&step.label).to_string_lossy().to_string();
        info!("Capturing screenshot: {}", path);

        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));

        schedule.current += 1;
        schedule.frame_count = 0;
    }
}

fn set_window_title(
    session: Res<Session>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.title = format!("Clauvolution — {}", session.name);
    }
}
