use bevy::prelude::*;
use clauvolution_body::BodyPlan;
use clauvolution_core::*;
use clauvolution_genome::{Genome, SegmentType};
use clauvolution_world::TileMap;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(
                PostUpdate,
                (
                    spawn_terrain_sprites,
                    sync_organism_transforms,
                    sync_food_transforms,
                    camera_control_system,
                    update_stats_text,
                )
                    .chain(),
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

#[derive(Component)]
pub struct TerrainRendered;

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

/// Render terrain tiles as colored rectangles
fn spawn_terrain_sprites(
    mut commands: Commands,
    tile_map: Option<Res<TileMap>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    existing: Query<&TerrainRendered>,
) {
    // Only render once
    if !existing.is_empty() {
        return;
    }

    let Some(tile_map) = tile_map else {
        return;
    };

    // Render terrain in chunks for performance (4x4 tile blocks)
    let chunk_size = 4u32;
    let tile_mesh = meshes.add(Rectangle::new(chunk_size as f32, chunk_size as f32));

    for cy in (0..tile_map.height).step_by(chunk_size as usize) {
        for cx in (0..tile_map.width).step_by(chunk_size as usize) {
            // Sample center tile for color
            let sample_x = (cx + chunk_size / 2).min(tile_map.width - 1);
            let sample_y = (cy + chunk_size / 2).min(tile_map.height - 1);
            let tile = tile_map.get(sample_x, sample_y);

            // Modulate color by vegetation density
            let base = tile.terrain.base_color();
            let base_rgba = base.to_srgba();
            let veg = tile.vegetation_density;
            let color = Color::srgb(
                base_rgba.red * (1.0 - veg * 0.2),
                (base_rgba.green + veg * 0.15).min(1.0),
                base_rgba.blue * (1.0 - veg * 0.1),
            );

            let material = materials.add(ColorMaterial::from(color));

            commands.spawn((
                Mesh2d(tile_mesh.clone()),
                MeshMaterial2d(material),
                Transform::from_xyz(
                    cx as f32 + chunk_size as f32 / 2.0,
                    cy as f32 + chunk_size as f32 / 2.0,
                    0.0,
                ),
                TerrainRendered,
            ));
        }
    }
}

fn segment_color(seg_type: SegmentType, genome: &Genome) -> Color {
    match seg_type {
        SegmentType::Torso => {
            let aqua = genome.aquatic_adaptation;
            Color::srgb(0.7 - aqua * 0.3, 0.5 + aqua * 0.3, 0.5 + aqua * 0.4)
        }
        SegmentType::Limb => Color::srgb(0.6, 0.45, 0.35),
        SegmentType::Fin => Color::srgb(0.3, 0.5, 0.8),
        SegmentType::Eye => Color::srgb(0.9, 0.9, 0.1),
        SegmentType::Mouth => Color::srgb(0.8, 0.2, 0.2),
        SegmentType::PhotoSurface => Color::srgb(0.1, 0.7, 0.15),
    }
}

fn segment_mesh(seg_type: SegmentType, size: f32, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    match seg_type {
        SegmentType::Torso => meshes.add(Ellipse::new(size, size * 0.7)),
        SegmentType::Limb => meshes.add(Rectangle::new(size * 0.3, size)),
        SegmentType::Fin => meshes.add(Triangle2d::new(
            Vec2::new(0.0, size * 0.5),
            Vec2::new(-size * 0.4, -size * 0.3),
            Vec2::new(size * 0.4, -size * 0.3),
        )),
        SegmentType::Eye => meshes.add(Circle::new(size * 0.25)),
        SegmentType::Mouth => meshes.add(Circle::new(size * 0.3)),
        SegmentType::PhotoSurface => meshes.add(Ellipse::new(size * 0.6, size * 0.2)),
    }
}

/// Sync organism Position to Transform, spawn body part sprites
fn sync_organism_transforms(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    organisms_without_sprite: Query<
        (Entity, &Position, &Genome, &BodyPlan),
        (With<Organism>, Without<OrganismSprite>),
    >,
    mut organisms_with_sprite: Query<
        (&Position, &Energy, &mut Transform),
        (With<Organism>, With<OrganismSprite>),
    >,
    camera: Query<&OrthographicProjection, With<MainCamera>>,
    config: Res<SimConfig>,
) {
    let zoom_scale = camera
        .get_single()
        .map(|p| p.scale)
        .unwrap_or(1.0);

    // Determine LOD level
    let use_detailed = zoom_scale < 0.8;

    // Spawn sprites for new organisms
    for (entity, pos, genome, body_plan) in &organisms_without_sprite {
        if use_detailed && !body_plan.parts.is_empty() {
            // Detailed view: render body parts
            // Spawn organism as parent entity with first part
            let first = &body_plan.parts[0];
            let mesh = segment_mesh(first.segment_type, first.size, &mut meshes);
            let color = segment_color(first.segment_type, genome);
            let material = materials.add(ColorMaterial::from(color));

            commands.entity(entity).insert((
                Mesh2d(mesh),
                MeshMaterial2d(material),
                Transform::from_xyz(pos.0.x, pos.0.y, 1.0),
                OrganismSprite,
            ));

            // Spawn child entities for additional body parts
            for part in body_plan.parts.iter().skip(1) {
                let mesh = segment_mesh(part.segment_type, part.size, &mut meshes);
                let color = segment_color(part.segment_type, genome);
                let material = materials.add(ColorMaterial::from(color));

                let child = commands
                    .spawn((
                        Mesh2d(mesh),
                        MeshMaterial2d(material),
                        Transform::from_xyz(part.offset.x, part.offset.y, 0.1)
                            .with_rotation(Quat::from_rotation_z(part.angle)),
                    ))
                    .id();

                commands.entity(entity).add_child(child);
            }
        } else {
            // Simple view: colored circle
            let r = (genome.speed_factor / 3.0).min(1.0);
            let g = (genome.photosynthesis_rate * 2.0 + genome.aquatic_adaptation * 0.5).min(1.0);
            let b = (genome.aquatic_adaptation).min(1.0);

            let mesh = meshes.add(Circle::new(1.0));
            let material = materials.add(ColorMaterial::from(Color::srgb(r, g, b)));

            commands.entity(entity).insert((
                Mesh2d(mesh),
                MeshMaterial2d(material),
                Transform::from_xyz(pos.0.x, pos.0.y, 1.0)
                    .with_scale(Vec3::splat(genome.body_size * 2.0)),
                OrganismSprite,
            ));
        }
    }

    // Update existing transforms
    for (pos, energy, mut transform) in &mut organisms_with_sprite {
        transform.translation.x = pos.0.x;
        transform.translation.y = pos.0.y;
        transform.translation.z = 1.0;

        let energy_factor = (energy.0 / config.max_organism_energy).clamp(0.5, 1.0);
        let current_scale = transform.scale.x;
        transform.scale = Vec3::splat(current_scale.abs() * energy_factor / current_scale.abs().max(0.01) * current_scale.abs().max(0.01));
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
        projection.scale = (projection.scale * (1.0 + zoom_delta)).clamp(0.05, 10.0);
    }
}

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
