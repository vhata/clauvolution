use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use clauvolution_body::BodyPlan;
use clauvolution_core::*;
use clauvolution_genome::{Genome, SegmentType};
use clauvolution_phylogeny::PhyloTree;
use clauvolution_world::TileMap;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraDragState>()
            .init_resource::<SharedMeshes>()
            .add_systems(Startup, (setup_camera, setup_shared_meshes))
            .add_systems(
                Update,
                (speed_control_system, click_select_system, toggle_graph_system),
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
                    update_graph,
                    update_phylo_tree,
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
pub struct GraphText;

#[derive(Component)]
pub struct PhyloText;

/// Shared mesh handles to avoid creating thousands of identical meshes
#[derive(Resource, Default)]
pub struct SharedMeshes {
    pub circle: Option<Handle<Mesh>>,
    pub food_circle: Option<Handle<Mesh>>,
    pub food_material: Option<Handle<ColorMaterial>>,
    pub outline_material: Option<Handle<ColorMaterial>>,
}

#[derive(Component)]
pub struct OrganismOutline;

fn setup_shared_meshes(
    mut shared: ResMut<SharedMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    shared.circle = Some(meshes.add(Circle::new(1.0)));
    shared.food_circle = Some(meshes.add(Circle::new(1.0)));
    shared.food_material = Some(materials.add(ColorMaterial::from(Color::srgb(0.2, 0.8, 0.2))));
    shared.outline_material = Some(materials.add(ColorMaterial::from(Color::srgba(0.0, 0.0, 0.0, 0.6))));
}

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

    // Population graph (bottom-left)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 11.0,
            ..default()
        },
        TextColor(Color::srgba(0.8, 0.9, 1.0, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            bottom: Val::Px(10.0),
            ..default()
        },
        GraphText,
    ));

    // Phylogenetic tree (bottom-right)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 11.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 0.95, 0.8, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(10.0),
            bottom: Val::Px(10.0),
            ..default()
        },
        PhyloText,
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
    shared_meshes: Res<SharedMeshes>,
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
        let is_plant = genome.photosynthesis_rate > 0.2 && genome.has_photo_surface();
        let z_level = if is_plant { 0.3 } else { 1.0 };

        if use_detailed && !body_plan.parts.is_empty() {
            let first = &body_plan.parts[0];
            let mesh = segment_mesh(first.segment_type, first.size, &mut meshes);
            let base_color = segment_color(first.segment_type, genome);
            let rgba = base_color.to_srgba();
            let alpha = 1.0; // all opaque now
            let color = Color::srgba(rgba.red, rgba.green, rgba.blue, alpha);
            let material = materials.add(ColorMaterial::from(color));

            commands.entity(entity).insert((
                Mesh2d(mesh),
                MeshMaterial2d(material),
                Transform::from_xyz(pos.0.x, pos.0.y, z_level),
                OrganismSprite,
            ));

            for part in body_plan.parts.iter().skip(1) {
                let mesh = segment_mesh(part.segment_type, part.size, &mut meshes);
                let base_color = segment_color(part.segment_type, genome);
                let rgba = base_color.to_srgba();
                let color = Color::srgba(rgba.red, rgba.green, rgba.blue, alpha);
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
            let base_color = species_colors.get_or_create(species_id.0);
            let base_rgba = base_color.to_srgba();

            let photo = genome.photosynthesis_rate;
            let predator = genome.claw_power().min(1.0);
            let is_plant = photo > 0.2 && genome.has_photo_surface();

            let (r, g, b, z_level, scale_mult) = if is_plant {
                // Plants: bright yellow-green, distinct from terrain, behind active organisms
                let bright = 0.5 + photo * 0.5;
                (0.5 * bright, 0.9 * bright, 0.15, 0.3, 1.5)
            } else {
                // Active organisms: species colour with predator red shift
                let r = (base_rgba.red * (1.0 - photo * 0.6) + predator * 0.4).clamp(0.1, 1.0);
                let g = (base_rgba.green * (1.0 - predator * 0.4) + photo * 0.4).clamp(0.1, 1.0);
                let b = (base_rgba.blue * (1.0 - photo * 0.3 - predator * 0.3)).clamp(0.05, 1.0);
                (r, g, b, 1.0, 2.0)
            };

            let mesh = shared_meshes.circle.clone().unwrap();
            let material = materials.add(ColorMaterial::from(Color::srgb(r, g, b)));

            commands.entity(entity).insert((
                Mesh2d(mesh.clone()),
                MeshMaterial2d(material),
                Transform::from_xyz(pos.0.x, pos.0.y, z_level)
                    .with_scale(Vec3::splat(genome.body_size * scale_mult)),
                OrganismSprite,
            ));

            // Only active organisms get outlines
            if !is_plant {
                if let Some(outline_mat) = &shared_meshes.outline_material {
                    let outline = commands.spawn((
                        Mesh2d(mesh),
                        MeshMaterial2d(outline_mat.clone()),
                        Transform::from_xyz(0.0, 0.0, -0.1)
                            .with_scale(Vec3::splat(1.3)),
                        OrganismOutline,
                    )).id();
                    commands.entity(entity).add_child(outline);
                }
            }
        }
    }

    // Update existing transforms (preserve z-level set at spawn)
    for (pos, energy, body_size, mut transform) in &mut organisms_with_sprite {
        transform.translation.x = pos.0.x;
        transform.translation.y = pos.0.y;
        // z preserved from spawn — photosynthesizers at 0.3, active at 1.0

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
    shared_meshes: Res<SharedMeshes>,
    food_without_sprite: Query<(Entity, &Position), (With<Food>, Without<FoodSprite>)>,
) {
    let Some(mesh) = &shared_meshes.food_circle else { return };
    let Some(material) = &shared_meshes.food_material else { return };

    for (entity, pos) in &food_without_sprite {
        commands.entity(entity).insert((
            Mesh2d(mesh.clone()),
            MeshMaterial2d(material.clone()),
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
    organisms: Query<&Genome, With<Organism>>,
    food: Query<&Food>,
    speed: Res<SimSpeed>,
    mut text_query: Query<&mut Text, With<StatsText>>,
) {
    let Ok(mut text) = text_query.get_single_mut() else {
        return;
    };

    let org_count = organisms.iter().len();
    let food_count = food.iter().len();

    let mut photosynthesizers = 0u32;
    let mut predators = 0u32;
    let mut foragers = 0u32;
    for genome in &organisms {
        if genome.photosynthesis_rate > 0.2 && genome.has_photo_surface() {
            photosynthesizers += 1;
        } else if genome.claw_power() > 0.5 {
            predators += 1;
        } else {
            foragers += 1;
        }
    }

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
         Plants: {}  Predators: {}  Foragers: {}\n\
         Food: {}  |  Generation: {}\n\
         Births: {}  |  Deaths: {}\n\
         \n\
         X=asteroid  I=ice age  V=volcano  G=graph\n\
         Click organism to inspect",
        speed_str, org_count, stats.species_count,
        photosynthesizers, predators, foragers,
        food_count, stats.max_generation,
        stats.total_births, stats.total_deaths,
    );
}

/// Show details about selected organism
fn update_inspect_panel(
    selected: Res<SelectedOrganism>,
    organisms: Query<(&Energy, &Health, &BodySize, &Genome, &SpeciesId, &Position, &Age, &Generation), With<Organism>>,
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

    let Ok((energy, health, body_size, genome, species, pos, age, generation)) = organisms.get(entity) else {
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
         Gen: {}  |  Age: {}\n\
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
        generation.0, age.0,
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

/// Toggle population graph visibility
fn toggle_graph_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut history: ResMut<PopulationHistory>,
) {
    if keys.just_pressed(KeyCode::KeyG) {
        history.visible = !history.visible;
    }
}

/// Render population graph as text-based sparklines
fn update_graph(
    history: Res<PopulationHistory>,
    mut text_query: Query<&mut Text, With<GraphText>>,
) {
    let Ok(mut text) = text_query.get_single_mut() else {
        return;
    };

    if !history.visible || history.snapshots.len() < 2 {
        **text = String::new();
        return;
    }

    let width = 60usize; // characters wide
    let _height = 8usize;
    let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    let snaps = &history.snapshots;
    let display_count = snaps.len().min(width);
    let start = snaps.len() - display_count;
    let display = &snaps[start..];

    let (org_line, org_range) = sparkline_ranged(display, |s| s.organisms as f32, &blocks);
    let (food_line, food_range) = sparkline_ranged(display, |s| s.food as f32, &blocks);
    let (species_line, sp_range) = sparkline_ranged(display, |s| s.species as f32, &blocks);
    // Births/deaths use zero-anchored since they're rates
    let (births_line, _) = sparkline_zero(display, |s| s.births_per_sec as f32, &blocks);
    let (deaths_line, _) = sparkline_zero(display, |s| s.deaths_per_sec as f32, &blocks);

    let latest = snaps.last().unwrap();

    **text = format!(
        "--- Population ({display_count}s, G=toggle) ---\n\
         Organisms {now:>4} ({lo}-{hi}): {line}\n\
         Food      {now_f:>4} ({flo}-{fhi}): {fline}\n\
         Species   {now_s:>4} ({slo}-{shi}): {sline}\n\
         Births/s  {now_b:>4}: {births_line}\n\
         Deaths/s  {now_d:>4}: {deaths_line}",
        now = latest.organisms,
        lo = org_range.0, hi = org_range.1, line = org_line,
        now_f = latest.food,
        flo = food_range.0, fhi = food_range.1, fline = food_line,
        now_s = latest.species,
        slo = sp_range.0, shi = sp_range.1, sline = species_line,
        now_b = latest.births_per_sec,
        now_d = latest.deaths_per_sec,
    );
}

/// Min-max normalized sparkline — shows relative variation within the data range.
/// Good for levels (population, food) where you want to see wobble.
fn sparkline_ranged(data: &[PopSnapshot], extract: impl Fn(&PopSnapshot) -> f32, blocks: &[char; 8]) -> (String, (u32, u32)) {
    if data.is_empty() {
        return (String::new(), (0, 0));
    }

    let values: Vec<f32> = data.iter().map(&extract).collect();
    let min = values.iter().cloned().fold(f32::MAX, f32::min);
    let max = values.iter().cloned().fold(f32::MIN, f32::max);
    let range = (max - min).max(1.0);

    let line: String = values.iter().map(|&v| {
        let normalized = ((v - min) / range).clamp(0.0, 1.0);
        let idx = (normalized * 7.0) as usize;
        blocks[idx.min(7)]
    }).collect();

    (line, (min as u32, max as u32))
}

/// Zero-anchored sparkline — good for rates (births/s, deaths/s) that spike from zero.
fn sparkline_zero(data: &[PopSnapshot], extract: impl Fn(&PopSnapshot) -> f32, blocks: &[char; 8]) -> (String, (u32, u32)) {
    if data.is_empty() {
        return (String::new(), (0, 0));
    }

    let values: Vec<f32> = data.iter().map(&extract).collect();
    let max = values.iter().cloned().fold(1.0f32, f32::max);

    let line: String = values.iter().map(|&v| {
        let normalized = (v / max).clamp(0.0, 1.0);
        let idx = (normalized * 7.0) as usize;
        blocks[idx.min(7)]
    }).collect();

    (line, (0, max as u32))
}

/// Render the phylogenetic tree as text
fn update_phylo_tree(
    phylo: Res<PhyloTree>,
    tick: Res<TickCounter>,
    mut text_query: Query<&mut Text, With<PhyloText>>,
) {
    let Ok(mut text) = text_query.get_single_mut() else {
        return;
    };

    **text = phylo.render_text(tick.0);
}
