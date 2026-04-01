use bevy::prelude::*;
use clauvolution_core::*;
use clauvolution_genome::Genome;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(
                PostUpdate,
                (
                    sync_organism_transforms,
                    sync_food_transforms,
                    camera_control_system,
                    update_stats_text,
                ),
            );
    }
}

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct StatsText;

#[derive(Component)]
pub struct OrganismSprite;

#[derive(Component)]
pub struct FoodSprite;

fn setup_camera(mut commands: Commands, config: Res<SimConfig>) {
    let center_x = config.world_width as f32 / 2.0;
    let center_y = config.world_height as f32 / 2.0;

    commands.spawn((
        Camera2d,
        Transform::from_xyz(center_x, center_y, 1000.0),
        OrthographicProjection {
            scale: 1.0,
            ..OrthographicProjection::default_2d()
        },
        MainCamera,
    ));

    // Stats overlay
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            ..default()
        },
        StatsText,
    ));
}

/// Sync organism Position component to Transform for rendering
fn sync_organism_transforms(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    organisms_without_sprite: Query<(Entity, &Position, &BodySize, &Genome), (With<Organism>, Without<OrganismSprite>)>,
    mut organisms_with_sprite: Query<(&Position, &BodySize, &Energy, &mut Transform), (With<Organism>, With<OrganismSprite>)>,
    config: Res<SimConfig>,
) {
    // Spawn sprites for new organisms
    for (entity, pos, body_size, genome) in &organisms_without_sprite {
        // Color based on genome traits — speed vs sense tradeoff gives visual diversity
        let r = (genome.speed_factor / 3.0).min(1.0);
        let g = (genome.sense_range / 150.0).min(1.0);
        let b = (genome.body_size / 3.0).min(1.0);

        let mesh = meshes.add(Circle::new(1.0));
        let material = materials.add(ColorMaterial::from(Color::srgb(r, g, b)));

        commands.entity(entity).insert((
            Mesh2d(mesh),
            MeshMaterial2d(material),
            Transform::from_xyz(pos.0.x, pos.0.y, 1.0)
                .with_scale(Vec3::splat(body_size.0 * 2.0)),
            OrganismSprite,
        ));
    }

    // Update existing transforms
    for (pos, body_size, energy, mut transform) in &mut organisms_with_sprite {
        transform.translation.x = pos.0.x;
        transform.translation.y = pos.0.y;
        transform.translation.z = 1.0;

        // Scale based on body size, slightly dim when low energy
        let energy_factor = (energy.0 / config.max_organism_energy).clamp(0.3, 1.0);
        transform.scale = Vec3::splat(body_size.0 * 2.0 * energy_factor);
    }
}

/// Sync food Position to Transform
fn sync_food_transforms(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    food_without_sprite: Query<(Entity, &Position), (With<Food>, Without<FoodSprite>)>,
) {
    for (entity, pos) in &food_without_sprite {
        let mesh = meshes.add(Circle::new(1.0));
        let material = materials.add(ColorMaterial::from(Color::srgb(0.2, 0.8, 0.2)));

        commands.entity(entity).insert((
            Mesh2d(mesh),
            MeshMaterial2d(material),
            Transform::from_xyz(pos.0.x, pos.0.y, 0.5)
                .with_scale(Vec3::splat(1.5)),
            FoodSprite,
        ));
    }
}

/// Camera pan (WASD/arrows) and zoom (scroll wheel)
fn camera_control_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
    mut camera: Query<(&mut Transform, &mut OrthographicProjection), With<MainCamera>>,
    time: Res<Time>,
) {
    let Ok((mut transform, mut projection)) = camera.get_single_mut() else {
        return;
    };

    let speed = 200.0 * projection.scale * time.delta_secs();

    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        transform.translation.y += speed;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        transform.translation.y -= speed;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        transform.translation.x -= speed;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        transform.translation.x += speed;
    }

    for event in scroll_events.read() {
        let zoom_delta = -event.y * 0.1;
        projection.scale = (projection.scale * (1.0 + zoom_delta)).clamp(0.1, 10.0);
    }
}

/// Update the stats text overlay
fn update_stats_text(
    stats: Res<SimStats>,
    organisms: Query<&Organism>,
    food: Query<&Food>,
    mut text_query: Query<&mut Text, With<StatsText>>,
) {
    let Ok(mut text) = text_query.get_single_mut() else {
        return;
    };

    let org_count = organisms.iter().len();
    let food_count = food.iter().len();

    **text = format!(
        "Organisms: {}\nFood: {}\nBirths: {}\nDeaths: {}",
        org_count, food_count, stats.total_births, stats.total_deaths,
    );
}
