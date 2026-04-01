use bevy::prelude::*;
use clauvolution_body::BodyPlugin;
use clauvolution_core::*;
use clauvolution_genome::InnovationCounter;
use clauvolution_render::RenderPlugin;
use clauvolution_sim::SimPlugin;
use clauvolution_world::{self, TileMap, WorldPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Clauvolution".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(CorePlugin)
        .add_plugins(WorldPlugin)
        .add_plugins(BodyPlugin)
        .add_plugins(SimPlugin)
        .add_plugins(RenderPlugin)
        .insert_resource(InnovationCounter(100))
        .add_systems(Startup, setup_world)
        .run();
}

fn setup_world(
    mut commands: Commands,
    config: Res<SimConfig>,
    mut innovation: ResMut<InnovationCounter>,
    mut stats: ResMut<SimStats>,
) {
    let mut rng = rand::thread_rng();

    // Generate terrain
    let tile_map = TileMap::generate(config.world_width, config.world_height, &mut rng);

    // Spawn initial food (biome-aware)
    clauvolution_world::spawn_initial_food(&mut commands, &config, &tile_map, &mut rng);

    // Spawn initial organisms
    clauvolution_sim::spawn_initial_population(&mut commands, &config, &mut innovation, &mut rng);

    // Insert tile map as resource
    commands.insert_resource(tile_map);

    stats.total_organisms = config.initial_population;

    info!(
        "Clauvolution initialized: {} organisms, world {}x{} with biomes",
        config.initial_population, config.world_width, config.world_height
    );
}
