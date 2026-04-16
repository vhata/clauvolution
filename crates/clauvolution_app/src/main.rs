use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use clauvolution_body::BodyPlugin;
use bevy::window::PrimaryWindow;
use clauvolution_core::*;
use clauvolution_genome::InnovationCounter;
use clauvolution_render::{MainCamera, RenderPlugin};
use clauvolution_phylogeny::PhylogenyPlugin;
use clauvolution_sim::SimPlugin;
use clauvolution_world::{self, TileMap, WorldPlugin};

fn main() {
    let screenshot_mode = std::env::args().any(|a| a == "--screenshot");

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
    .add_systems(Startup, (setup_world, set_window_title));

    if screenshot_mode {
        app.insert_resource(ScreenshotSchedule::new())
            .add_systems(Update, screenshot_system);
    }

    app.run();
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
