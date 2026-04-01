use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use clauvolution_body::BodyPlan;
use clauvolution_core::*;
use clauvolution_genome::{Genome, SegmentType};
use clauvolution_world::TileMap;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraDragState>()
            .add_systems(Startup, setup_camera)
            .add_systems(
                Update,
                (speed_control_system, click_select_system),
            )
            .add_systems(
                PostUpdate,
                (
                    spawn_terrain_sprites,
                    sync_organism_transforms,
                    sync_food_transforms,
                    camera_control_system,
                    update_stats_text,
                    update_inspect_panel,
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
pub struct InspectPanel;

#[derive(Component)]
pub struct OrganismSprite;

#[derive(Component)]
pub struct SelectionRing;

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

    // Stats overlay (top-left)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 16.0,
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

    // Inspect panel (top-right)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(10.0),
            top: Val::Px(10.0),
            ..default()
        },
        InspectPanel,
    ));
}

/// Keyboard speed controls: Space = pause, [ = slower, ] = faster
fn speed_control_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut speed: ResMut<SimSpeed>,
) {
    if keys.just_pressed(KeyCode::Space) {
        speed.paused = !speed.paused;
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        speed.multiplier = (speed.multiplier * 0.5).max(0.125);
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        speed.multiplier = (speed.multiplier * 2.0).min(16.0);
    }
}

/// Click to select an organism for inspection
fn click_select_system(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Transform, &OrthographicProjection), With<MainCamera>>,
    organisms: Query<(Entity, &Position, &BodySize), With<Organism>>,
    mut selected: ResMut<SelectedOrganism>,
    mut commands: Commands,
    existing_rings: Query<Entity, With<SelectionRing>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Don't select when dragging
    if keys.pressed(KeyCode::ShiftLeft) {
        return;
    }

    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.get_single() else { return };
    let Ok((cam_transform, projection)) = camera.get_single() else { return };

    let Some(cursor_pos) = window.cursor_position() else { return };

    // Convert screen position to world position
    let window_size = Vec2::new(window.width(), window.height());
    let ndc = (cursor_pos / window_size) * 2.0 - Vec2::ONE;
    let world_pos = Vec2::new(
        cam_transform.translation.x + ndc.x * window_size.x * 0.5 * projection.scale,
        cam_transform.translation.y - ndc.y * window_size.y * 0.5 * projection.scale,
    );

    // Find nearest organism to click
    let mut nearest = None;
    let mut nearest_dist = f32::MAX;
    let click_radius = 5.0 * projection.scale;

    for (entity, pos, body_size) in &organisms {
        let dist = (pos.0 - world_pos).length();
        let hit_radius = (body_size.0 * 2.0).max(click_radius);
        if dist < hit_radius && dist < nearest_dist {
            nearest_dist = dist;
            nearest = Some(entity);
        }
    }

    // Remove old selection ring
    for ring in &existing_rings {
        commands.entity(ring).despawn();
    }

    if let Some(entity) = nearest {
        selected.entity = Some(entity);

        // Add selection ring
        let mesh = meshes.add(Circle::new(1.0));
        let material = materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 0.0, 0.5)));
        if let Ok((_, pos, body_size)) = organisms.get(entity) {
            commands.spawn((
                Mesh2d(mesh),
                MeshMaterial2d(material),
                Transform::from_xyz(pos.0.x, pos.0.y, 0.9)
                    .with_scale(Vec3::splat(body_size.0 * 3.5)),
                SelectionRing,
            ));
        }
    } else {
        selected.entity = None;
    }
}

fn spawn_terrain_sprites(
    mut commands: Commands,
    tile_map: Option<Res<TileMap>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    existing: Query<&TerrainRendered>,
) {
    if !existing.is_empty() {
        return;
    }

    let Some(tile_map) = tile_map else {
        return;
    };

    let chunk_size = 4u32;
    let tile_mesh = meshes.add(Rectangle::new(chunk_size as f32, chunk_size as f32));

    for cy in (0..tile_map.height).step_by(chunk_size as usize) {
        for cx in (0..tile_map.width).step_by(chunk_size as usize) {
            let sample_x = (cx + chunk_size / 2).min(tile_map.width - 1);
            let sample_y = (cy + chunk_size / 2).min(tile_map.height - 1);
            let tile = tile_map.get(sample_x, sample_y);

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
        SegmentType::Claw => Color::srgb(0.85, 0.4, 0.1),
        SegmentType::ArmorPlate => Color::srgb(0.55, 0.55, 0.6),
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
        SegmentType::Claw => meshes.add(Triangle2d::new(
            Vec2::new(0.0, size * 0.6),
            Vec2::new(-size * 0.2, -size * 0.2),
            Vec2::new(size * 0.2, -size * 0.2),
        )),
        SegmentType::ArmorPlate => meshes.add(Rectangle::new(size * 0.5, size * 0.4)),
    }
}

/// Sync organism Position to Transform, spawn body part sprites
fn sync_organism_transforms(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    organisms_without_sprite: Query<
        (Entity, &Position, &Genome, &BodyPlan, &SpeciesId),
        (With<Organism>, Without<OrganismSprite>),
    >,
    mut organisms_with_sprite: Query<
        (&Position, &Energy, &BodySize, &mut Transform),
        (With<Organism>, With<OrganismSprite>),
    >,
    camera: Query<&OrthographicProjection, With<MainCamera>>,
    config: Res<SimConfig>,
    mut species_colors: ResMut<SpeciesColors>,
    selected: Res<SelectedOrganism>,
    mut selection_rings: Query<&mut Transform, (With<SelectionRing>, Without<Organism>)>,
) {
    let zoom_scale = camera
        .get_single()
        .map(|p| p.scale)
        .unwrap_or(1.0);

    let use_detailed = zoom_scale < 0.3;

    for (entity, pos, genome, body_plan, species_id) in &organisms_without_sprite {
        if use_detailed && !body_plan.parts.is_empty() {
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
            // Species colour with trait modulation
            let base_color = species_colors.get_or_create(species_id.0);
            let base_rgba = base_color.to_srgba();

            // Tint green for photosynthesizers
            let photo = genome.photosynthesis_rate;
            let r = (base_rgba.red * (1.0 - photo * 0.5)).max(0.0);
            let g = (base_rgba.green + photo * 0.3).min(1.0);
            let b = base_rgba.blue * (1.0 - photo * 0.3);

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
    for (pos, energy, body_size, mut transform) in &mut organisms_with_sprite {
        transform.translation.x = pos.0.x;
        transform.translation.y = pos.0.y;
        transform.translation.z = 1.0;

        let energy_factor = (energy.0 / config.max_organism_energy).clamp(0.5, 1.0);
        transform.scale = Vec3::splat(body_size.0 * 2.0 * energy_factor);
    }

    // Update selection ring position
    if let Some(sel_entity) = selected.entity {
        if let Ok((pos, _, body_size, _)) = organisms_with_sprite.get(sel_entity) {
            for mut ring_transform in &mut selection_rings {
                ring_transform.translation.x = pos.0.x;
                ring_transform.translation.y = pos.0.y;
                ring_transform.scale = Vec3::splat(body_size.0 * 3.5);
            }
        }
    }
}

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

#[derive(Resource, Default)]
pub struct CameraDragState {
    dragging: bool,
    last_pos: Vec2,
}

fn camera_control_system(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
    mut cursor_events: EventReader<CursorMoved>,
    mut camera: Query<(&mut Transform, &mut OrthographicProjection), With<MainCamera>>,
    mut drag_state: ResMut<CameraDragState>,
    time: Res<Time>,
) {
    let Ok((mut transform, mut projection)) = camera.get_single_mut() else {
        return;
    };

    let dt = time.delta_secs();

    let speed = 200.0 * projection.scale * dt;
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

    let zoom_speed = 2.0 * dt;
    if keys.pressed(KeyCode::KeyE) || keys.pressed(KeyCode::Equal) {
        projection.scale *= 1.0 - zoom_speed;
    }
    if keys.pressed(KeyCode::KeyQ) || keys.pressed(KeyCode::Minus) {
        projection.scale *= 1.0 + zoom_speed;
    }

    for event in scroll_events.read() {
        let zoom_factor = 1.0 + (-event.y * 0.02).clamp(-0.15, 0.15);
        projection.scale *= zoom_factor;
    }

    projection.scale = projection.scale.clamp(0.02, 15.0);

    let dragging = mouse_buttons.pressed(MouseButton::Middle)
        || mouse_buttons.pressed(MouseButton::Right)
        || (mouse_buttons.pressed(MouseButton::Left) && keys.pressed(KeyCode::ShiftLeft));

    let mut latest_cursor_pos = None;
    for event in cursor_events.read() {
        latest_cursor_pos = Some(event.position);
    }

    if dragging {
        if let Some(cursor_pos) = latest_cursor_pos {
            if drag_state.dragging {
                let delta = cursor_pos - drag_state.last_pos;
                transform.translation.x -= delta.x * projection.scale;
                transform.translation.y += delta.y * projection.scale;
            }
            drag_state.last_pos = cursor_pos;
            drag_state.dragging = true;
        }
    } else {
        drag_state.dragging = false;
        if let Some(cursor_pos) = latest_cursor_pos {
            drag_state.last_pos = cursor_pos;
        }
    }
}

fn update_stats_text(
    stats: Res<SimStats>,
    organisms: Query<&Organism>,
    food: Query<&Food>,
    speed: Res<SimSpeed>,
    mut text_query: Query<&mut Text, With<StatsText>>,
) {
    let Ok(mut text) = text_query.get_single_mut() else {
        return;
    };

    let org_count = organisms.iter().len();
    let food_count = food.iter().len();

    let speed_str = if speed.paused {
        "PAUSED".to_string()
    } else if speed.multiplier == 1.0 {
        "1x".to_string()
    } else if speed.multiplier < 1.0 {
        format!("{:.2}x", speed.multiplier)
    } else {
        format!("{}x", speed.multiplier as u32)
    };

    **text = format!(
        "Speed: {}  [Space=pause, [/]=speed]\n\
         Organisms: {}  |  Species: {}\n\
         Food: {}\n\
         Births: {}  |  Deaths: {}\n\
         \n\
         X=asteroid  I=ice age  V=volcano\n\
         Click organism to inspect",
        speed_str, org_count, stats.species_count,
        food_count, stats.total_births, stats.total_deaths,
    );
}

/// Show details about selected organism
fn update_inspect_panel(
    selected: Res<SelectedOrganism>,
    organisms: Query<(&Energy, &Health, &BodySize, &Genome, &SpeciesId, &Position), With<Organism>>,
    mut text_query: Query<&mut Text, With<InspectPanel>>,
    tile_map: Option<Res<TileMap>>,
    config: Res<SimConfig>,
) {
    let Ok(mut text) = text_query.get_single_mut() else {
        return;
    };

    let Some(entity) = selected.entity else {
        **text = String::new();
        return;
    };

    let Ok((energy, health, body_size, genome, species, pos)) = organisms.get(entity) else {
        **text = "Selected organism died".to_string();
        return;
    };

    let terrain_name = if let Some(tm) = &tile_map {
        let tile = tm.tile_at_pos(pos.0);
        format!("{:?}", tile.terrain)
    } else {
        "?".to_string()
    };

    let body_parts: Vec<String> = genome
        .body_segments
        .iter()
        .map(|s| format!("{:?}", s.segment_type))
        .collect();

    let strategy = if genome.photosynthesis_rate > 0.3 && genome.has_photo_surface() {
        "Photosynthesizer"
    } else if genome.claw_power() > 0.5 {
        "Predator"
    } else {
        "Forager"
    };

    **text = format!(
        "--- ORGANISM ---\n\
         Species: {}  ({})\n\
         Energy: {:.1} / {:.0}\n\
         Health: {:.0}%\n\
         Position: ({:.0}, {:.0})\n\
         Terrain: {}\n\
         \n\
         --- BODY ---\n\
         Size: {:.2}\n\
         Speed: {:.2}\n\
         Sense range: {:.1}\n\
         Aquatic: {:.0}%\n\
         Photo: {:.0}%\n\
         Attack: {:.2}\n\
         Armor: {:.2}\n\
         Parts: {}\n\
         \n\
         --- BRAIN ---\n\
         Neurons: {}\n\
         Connections: {}\n",
        species.0, strategy,
        energy.0, config.max_organism_energy,
        health.0 * 100.0,
        pos.0.x, pos.0.y,
        terrain_name,
        body_size.0,
        genome.speed_factor,
        genome.effective_sense_range(),
        genome.aquatic_adaptation * 100.0,
        genome.photosynthesis_rate * 100.0,
        genome.claw_power(),
        genome.armor_value(),
        body_parts.join(", "),
        genome.neurons.len(),
        genome.connections.iter().filter(|c| c.enabled).count(),
    );
}
