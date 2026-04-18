// Bevy system signatures legitimately nest multiple generics (Query with
// tuple data + tuple filters, etc.). Clippy flags most of them as
// "very complex type" but there's no meaningful readability win from
// aliasing each one individually — Bevy idiomatic code accepts this.
#![allow(clippy::type_complexity)]
// Similarly, Bevy systems often take >7 params (queries, resources, events);
// clippy's too_many_arguments lint misfires constantly on system signatures.
#![allow(clippy::too_many_arguments)]

use bevy::prelude::*;
use bevy::image::{Image, ImageSampler};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use clauvolution_body::BodyPlan;
use clauvolution_core::*;
use clauvolution_genome::{Genome, SegmentType};
use clauvolution_world::TileMap;

mod screenshot_with_egui;

pub use screenshot_with_egui::{begin_screenshot, capture_now, ScreenshotState};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(screenshot_with_egui::ScreenshotWithEguiPlugin)
            .init_resource::<CameraDragState>()
            .init_resource::<SharedMeshes>()
            .init_resource::<LodState>()
            .init_resource::<MinimapMode>()
            .init_resource::<ScreenshotState>()
            .add_systems(Startup, (setup_camera, setup_shared_meshes, setup_minimap))
            .add_systems(
                Update,
                (
                    speed_control_system,
                    click_select_system,
                    toggle_minimap_mode_system,
                    toggle_trails_system,
                    lod_change_system,
                    manual_screenshot_system,
                    screenshot_with_egui::drive_screenshot_capture,
                    minimap_click_system,
                    cycle_species_member_system,
                    random_select_system,
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    spawn_terrain_sprites,
                    sync_organism_transforms,
                    sync_food_transforms,
                    update_death_markers,
                    draw_trails_system,
                    draw_infection_indicators_system,
                    camera_control_system,
                    update_minimap,
                )
                    .chain(),
            );
    }
}

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct OrganismSprite;

#[derive(Component)]
pub struct SelectionRing;

#[derive(Component)]
pub struct FoodSprite;

#[derive(Component)]

/// Tracks whether we're in detailed or simple rendering mode
#[derive(Resource)]
pub struct LodState {
    pub detailed: bool,
}

impl Default for LodState {
    fn default() -> Self {
        Self { detailed: false }
    }
}


#[derive(Resource)]
pub struct UiFont(pub Handle<Font>);

#[derive(Component)]
pub struct MinimapNode;

#[derive(Resource, Default, PartialEq, Eq)]
pub enum MinimapMode {
    #[default]
    Normal,
    Heatmap,
}

#[derive(Resource)]
pub struct MinimapData {
    pub image_handle: Handle<Image>,
    pub size: u32, // pixels per side
    pub timer: Timer,
}

#[derive(Component)]

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

fn setup_camera(mut commands: Commands, config: Res<SimConfig>, asset_server: Res<AssetServer>) {
    let font: Handle<Font> = asset_server.load("fonts/JetBrainsMono-Regular.ttf");
    commands.insert_resource(UiFont(font.clone()));
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

    // Minimap legend — sits directly below the 160px minimap (top:38, size:160 → 203 for 5px gap).
    // Top offset of 38 clears the 28px egui header bar plus a 10px gap.
    // Colours mirror the minimap organism dots and heatmap bins.
    let legend_entries: [(Color, &str); 3] = [
        (Color::srgb(0.39, 1.0, 0.39), "plants"),
        (Color::srgb(1.0, 1.0, 1.0), "foragers"),
        (Color::srgb(1.0, 0.24, 0.24), "predators"),
    ];
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(203.0),
                width: Val::Px(160.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(5.0)),
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ))
        .with_children(|parent| {
            for (color, label) in legend_entries {
                parent.spawn((
                    Text::new(format!("● {}", label)),
                    TextFont { font: font.clone(), font_size: 11.0, ..default() },
                    TextColor(color),
                ));
            }
        });
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
    ui_input: Res<UiInputState>,
) {
    // Don't select when dragging or when pointer is over an egui panel
    if keys.pressed(KeyCode::ShiftLeft) || ui_input.pointer_over_ui {
        return;
    }

    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.get_single() else {
        warn_once!("click_select: primary window missing — clicks will do nothing");
        return;
    };
    let Ok((cam_transform, projection)) = camera.get_single() else {
        warn_once!("click_select: MainCamera missing — clicks will do nothing");
        return;
    };

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
        commands.entity(ring).try_despawn();
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

    let chunk_size = 8u32;
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
        (&Position, &Energy, &BodySize, &ActionFlash, &mut Transform, &mut Visibility),
        (With<Organism>, With<OrganismSprite>),
    >,
    camera: Query<(&Transform, &OrthographicProjection), (With<MainCamera>, Without<Organism>, Without<SelectionRing>)>,
    config: Res<SimConfig>,
    mut species_colors: ResMut<SpeciesColors>,
    selected: Res<SelectedOrganism>,
    mut selection_rings: Query<&mut Transform, (With<SelectionRing>, Without<Organism>, Without<MainCamera>)>,
) {
    let (zoom_scale, cam_left, cam_right, cam_bottom, cam_top) = if let Ok((cam_t, proj)) = camera.get_single() {
        let half_w = 960.0 * proj.scale;
        let half_h = 540.0 * proj.scale;
        let margin = 20.0 * proj.scale; // slight margin so entities don't pop in/out at edges
        (
            proj.scale,
            cam_t.translation.x - half_w - margin,
            cam_t.translation.x + half_w + margin,
            cam_t.translation.y - half_h - margin,
            cam_t.translation.y + half_h + margin,
        )
    } else {
        (1.0, 0.0, config.world_width as f32, 0.0, config.world_height as f32)
    };

    let use_detailed = zoom_scale < 0.6;

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
                Transform::from_xyz(pos.0.x, pos.0.y, z_level)
                    .with_scale(Vec3::splat(genome.body_size * 2.0)),
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

    // Update existing transforms — frustum cull off-screen organisms
    for (pos, energy, body_size, flash, mut transform, mut vis) in &mut organisms_with_sprite {
        let in_view = pos.0.x >= cam_left && pos.0.x <= cam_right
            && pos.0.y >= cam_bottom && pos.0.y <= cam_top;

        if !in_view {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Inherited;

        transform.translation.x = pos.0.x;
        transform.translation.y = pos.0.y;

        let energy_factor = (energy.0 / config.max_organism_energy).clamp(0.5, 1.0);
        // Flash pulse — organisms briefly grow when eating/attacking/reproducing
        let flash_pulse = if flash.timer > 0.0 {
            1.0 + flash.timer * 1.5 // up to 1.45x size
        } else {
            1.0
        };
        transform.scale = Vec3::splat(body_size.0 * 2.0 * energy_factor * flash_pulse);
    }

    // Update selection ring position
    if let Some(sel_entity) = selected.entity {
        if let Ok((pos, _, body_size, _, _, _)) = organisms_with_sprite.get(sel_entity) {
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
    mut food_with_sprite: Query<&mut Visibility, (With<Food>, With<FoodSprite>)>,
    camera: Query<&OrthographicProjection, With<MainCamera>>,
) {
    let zoom = camera.get_single().map(|p| p.scale).unwrap_or(1.0);
    let food_visible = zoom < 2.0;

    // Toggle visibility on existing food sprites
    for mut vis in &mut food_with_sprite {
        *vis = if food_visible { Visibility::Inherited } else { Visibility::Hidden };
    }

    // Don't spawn new food sprites if zoomed out
    if !food_visible {
        return;
    }

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

#[derive(Component)]
pub struct DeathMarkerSprite;

/// Spawn visuals for new death markers, fade and despawn existing ones
fn update_death_markers(
    mut commands: Commands,
    time: Res<Time>,
    shared_meshes: Res<SharedMeshes>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    new_markers: Query<(Entity, &Position, &DeathMarker), Without<DeathMarkerSprite>>,
    mut existing_markers: Query<(Entity, &mut DeathMarker, &mut Transform), With<DeathMarkerSprite>>,
) {
    let Some(mesh) = &shared_meshes.circle else { return };

    // Spawn visuals for new markers
    for (entity, pos, marker) in &new_markers {
        let color = if marker.was_predated {
            Color::srgba(1.0, 0.2, 0.1, 0.8) // red for predation
        } else {
            Color::srgba(0.6, 0.5, 0.2, 0.6) // dim amber for starvation/old age
        };
        let material = materials.add(ColorMaterial::from(color));
        commands.entity(entity).insert((
            Mesh2d(mesh.clone()),
            MeshMaterial2d(material),
            Transform::from_xyz(pos.0.x, pos.0.y, 0.8)
                .with_scale(Vec3::splat(2.0)),
            DeathMarkerSprite,
        ));
    }

    // Fade and despawn existing markers
    let dt = time.delta_secs();
    for (entity, mut marker, mut transform) in &mut existing_markers {
        marker.timer -= dt;
        if marker.timer <= 0.0 {
            commands.entity(entity).try_despawn();
            continue;
        }
        // Expand and fade out
        let progress = 1.0 - marker.timer / 0.5;
        transform.scale = Vec3::splat(2.0 + progress * 3.0);
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
    ui_input: Res<UiInputState>,
    selected: Res<SelectedOrganism>,
    organism_positions: Query<&Position, With<Organism>>,
) {
    let Ok((mut transform, mut projection)) = camera.get_single_mut() else {
        warn_once!("camera_control: MainCamera missing — pan/zoom disabled");
        return;
    };

    // F — snap camera to selected organism (if any)
    if !ui_input.wants_keyboard && keys.just_pressed(KeyCode::KeyF) {
        if let Some(sel_entity) = selected.entity {
            if let Ok(pos) = organism_positions.get(sel_entity) {
                transform.translation.x = pos.0.x;
                transform.translation.y = pos.0.y;
            }
        }
    }

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

    // Only zoom with scroll when pointer is over the world, not when over egui panels
    if ui_input.pointer_over_ui {
        scroll_events.clear();
    } else {
        for event in scroll_events.read() {
            let zoom_factor = 1.0 + (-event.y * 0.02).clamp(-0.15, 0.15);
            projection.scale *= zoom_factor;
        }
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

fn toggle_minimap_mode_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<MinimapMode>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        *mode = match *mode {
            MinimapMode::Normal => MinimapMode::Heatmap,
            MinimapMode::Heatmap => MinimapMode::Normal,
        };
    }
}

fn toggle_trails_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut trails: ResMut<TrailsVisible>,
    mut organisms: Query<&mut TrailHistory, With<Organism>>,
    ui_input: Res<UiInputState>,
) {
    if ui_input.wants_keyboard {
        return;
    }
    if keys.just_pressed(KeyCode::KeyT) {
        trails.0 = !trails.0;
        // When turning off, clear all existing trail buffers so turning on
        // again starts fresh instead of showing stale teleport-lines
        if !trails.0 {
            for mut trail in &mut organisms {
                trail.positions.clear();
            }
        }
    }
}

/// Draw a pulsing purple halo around infected organisms using gizmos.
/// Stateless — no entity bookkeeping, batched into a single draw call.
fn draw_infection_indicators_system(
    mut gizmos: Gizmos,
    infected: Query<(&Position, &BodySize, &Infection), With<Organism>>,
    camera: Query<(&Transform, &OrthographicProjection), (With<MainCamera>, Without<Organism>, Without<SelectionRing>)>,
    time: Res<Time>,
) {
    let Ok((cam_t, proj)) = camera.get_single() else { return };
    let half_w = 960.0 * proj.scale;
    let half_h = 540.0 * proj.scale;
    let margin = 40.0 * proj.scale;
    let cam_left = cam_t.translation.x - half_w - margin;
    let cam_right = cam_t.translation.x + half_w + margin;
    let cam_bottom = cam_t.translation.y - half_h - margin;
    let cam_top = cam_t.translation.y + half_h + margin;

    // Pulse rate: slow (about 1Hz). Breath-like.
    let pulse = (time.elapsed_secs() * std::f32::consts::TAU).sin() * 0.15 + 0.85;

    for (pos, body_size, infection) in &infected {
        if pos.0.x < cam_left || pos.0.x > cam_right
            || pos.0.y < cam_bottom || pos.0.y > cam_top {
            continue;
        }
        // Purple, opacity scales with severity
        let alpha = (0.35 + 0.35 * infection.severity).clamp(0.2, 0.8);
        let color = Color::srgba(0.7, 0.2, 0.9, alpha);
        let radius = body_size.0 * 2.2 * pulse;
        gizmos.circle_2d(pos.0, radius, color);
    }
}

/// Draw trail linestrips behind each visible organism using Bevy gizmos.
/// Gizmos batch into a single draw call so even 2000 trails are cheap.
/// Only draws trails for organisms in the camera viewport (frustum cull).
fn draw_trails_system(
    mut gizmos: Gizmos,
    trails: Res<TrailsVisible>,
    selected: Res<SelectedOrganism>,
    organisms: Query<(&Position, &TrailHistory, &SpeciesId), With<Organism>>,
    mut species_colors: ResMut<SpeciesColors>,
) {
    if !trails.0 {
        return;
    }

    // Only draw the trail of the currently-selected organism. At 2000 organisms,
    // drawing every trail produced unreadable visual noise; limiting to the
    // selected one turns the feature into a focused inspection tool.
    let Some(entity) = selected.entity else { return };
    let Ok((_pos, trail, species)) = organisms.get(entity) else { return };
    if trail.positions.len() < 2 {
        return;
    }

    let base = species_colors.get_or_create(species.0);
    let rgba = base.to_srgba();
    // Brighter alpha than the old all-trails mode (0.35) since it's now
    // a single focused line, not an ambient smear.
    let color = Color::srgba(rgba.red, rgba.green, rgba.blue, 0.75);

    gizmos.linestrip_2d(trail.positions.iter().copied(), color);
}


/// Detect zoom crossing the LOD threshold and strip sprites so they re-render
fn lod_change_system(
    mut commands: Commands,
    camera: Query<&OrthographicProjection, With<MainCamera>>,
    mut lod_state: ResMut<LodState>,
    organisms: Query<(Entity, &Children), (With<Organism>, With<OrganismSprite>)>,
    _outlines: Query<Entity, With<OrganismOutline>>,
) {
    let zoom_scale = camera
        .get_single()
        .map(|p| p.scale)
        .unwrap_or(1.0);

    let should_be_detailed = zoom_scale < 0.6;

    if should_be_detailed == lod_state.detailed {
        return;
    }

    lod_state.detailed = should_be_detailed;

    // Strip OrganismSprite, Mesh2d, MeshMaterial2d from all organisms
    // so sync_organism_transforms re-creates them at the new LOD level.
    // Also despawn child entities (body parts, outlines).
    for (entity, children) in &organisms {
        commands.entity(entity)
            .remove::<OrganismSprite>()
            .remove::<Mesh2d>()
            .remove::<MeshMaterial2d<ColorMaterial>>();

        for &child in children.iter() {
            if let Some(mut cmd) = commands.get_entity(child) {
                cmd.try_despawn();
            }
        }
    }
}

/// S key takes a manual screenshot, saved to session directory.
/// Uses the egui-aware capture path so the side panel, header bar, and
/// minimap legend are all included in the saved PNG.
fn manual_screenshot_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    session: Res<Session>,
    tick: Res<TickCounter>,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<ScreenshotState>,
    main_camera: Query<(&Transform, &OrthographicProjection), With<MainCamera>>,
    primary_window: Query<(Entity, &Window), With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::KeyS) {
        let time_secs = tick.0 / 30;
        let label = format!("screenshot_{}s", time_secs);
        let path = session.screenshot_path(&label);
        capture_now(
            path,
            &mut commands,
            &mut images,
            &mut state,
            &main_camera,
            &primary_window,
        );
    }
}

fn setup_minimap(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    _config: Res<SimConfig>,
) {
    let size = 160u32;

    // Create a blank RGBA image
    let mut image = Image::new_fill(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[40, 40, 40, 255],
        TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::all(),
    );
    image.sampler = ImageSampler::nearest();
    let image_handle = images.add(image);

    // Spawn UI node for the minimap.
    // Placed top-LEFT below the 28px egui header bar so the egui side panel
    // (which always hugs the right edge) can't hide it.
    commands.spawn((
        ImageNode::new(image_handle.clone()),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(38.0),
            width: Val::Px(size as f32),
            height: Val::Px(size as f32),
            ..default()
        },
        MinimapNode,
    ));

    commands.insert_resource(MinimapData {
        image_handle,
        size,
        timer: Timer::from_seconds(0.5, TimerMode::Repeating),
    });
}

fn update_minimap(
    time: Res<Time>,
    mut minimap: ResMut<MinimapData>,
    mut images: ResMut<Assets<Image>>,
    tile_map: Option<Res<TileMap>>,
    config: Res<SimConfig>,
    organisms: Query<(&Position, &Genome), With<Organism>>,
    camera: Query<(&Transform, &OrthographicProjection), With<MainCamera>>,
    minimap_mode: Res<MinimapMode>,
    selected: Res<SelectedOrganism>,
) {
    minimap.timer.tick(time.delta());
    if !minimap.timer.just_finished() {
        return;
    }

    let Some(tile_map) = tile_map else { return };
    let Some(image) = images.get_mut(&minimap.image_handle) else { return };

    let size = minimap.size as usize;
    let world_w = config.world_width as f32;
    let world_h = config.world_height as f32;

    match *minimap_mode {
        MinimapMode::Normal => {
            paint_minimap_normal(image, size, world_w, world_h, &tile_map, &config, &organisms);
        }
        MinimapMode::Heatmap => {
            paint_minimap_heatmap(image, size, world_w, world_h, &organisms);
        }
    }

    // Paint camera viewport rectangle (both modes)
    if let Ok((cam_transform, projection)) = camera.get_single() {
        let cam_x = cam_transform.translation.x;
        let cam_y = cam_transform.translation.y;
        let half_w = 960.0 * projection.scale;
        let half_h = 540.0 * projection.scale;

        let left = ((cam_x - half_w) / world_w * size as f32) as i32;
        let right = ((cam_x + half_w) / world_w * size as f32) as i32;
        let top = size as i32 - 1 - ((cam_y + half_h) / world_h * size as f32) as i32;
        let bottom = size as i32 - 1 - ((cam_y - half_h) / world_h * size as f32) as i32;

        for x in left.max(0)..=right.min(size as i32 - 1) {
            for &y in &[top, bottom] {
                if y >= 0 && (y as usize) < size {
                    let idx = (y as usize * size + x as usize) * 4;
                    image.data[idx] = 255;
                    image.data[idx + 1] = 255;
                    image.data[idx + 2] = 0;
                }
            }
        }
        for y in top.max(0)..=bottom.min(size as i32 - 1) {
            for &x in &[left, right] {
                if x >= 0 && (x as usize) < size {
                    let idx = (y as usize * size + x as usize) * 4;
                    image.data[idx] = 255;
                    image.data[idx + 1] = 255;
                    image.data[idx + 2] = 0;
                }
            }
        }
    }

    // Paint selected organism as a bright yellow plus marker.
    // Plus (crosshair) shape is more legible than a square at 1-2 pixel scale
    // and doesn't obscure the organism dot beneath it.
    if let Some(sel_entity) = selected.entity {
        if let Ok((pos, _genome)) = organisms.get(sel_entity) {
            let cx = (pos.0.x / world_w * size as f32) as i32;
            let cy = size as i32 - 1 - (pos.0.y / world_h * size as f32) as i32;
            // Draw plus shape: center + 2 pixels each direction
            for &(dx, dy) in &[(0, 0), (-2, 0), (-1, 0), (1, 0), (2, 0),
                               (0, -2), (0, -1), (0, 1), (0, 2)] {
                let x = cx + dx;
                let y = cy + dy;
                if x >= 0 && (x as usize) < size && y >= 0 && (y as usize) < size {
                    let idx = (y as usize * size + x as usize) * 4;
                    image.data[idx] = 255;
                    image.data[idx + 1] = 230;
                    image.data[idx + 2] = 50;
                    image.data[idx + 3] = 255;
                }
            }
        }
    }
}

/// Normal minimap: terrain background with organism dots
fn paint_minimap_normal(
    image: &mut Image,
    size: usize,
    world_w: f32,
    world_h: f32,
    tile_map: &TileMap,
    config: &SimConfig,
    organisms: &Query<(&Position, &Genome), With<Organism>>,
) {
    // Paint terrain
    for py in 0..size {
        for px in 0..size {
            let wx = (px as f32 / size as f32 * world_w) as u32;
            let wy = ((size - 1 - py) as f32 / size as f32 * world_h) as u32;
            let tile = tile_map.get(wx.min(config.world_width - 1), wy.min(config.world_height - 1));

            let (r, g, b) = match tile.terrain {
                clauvolution_world::TerrainType::DeepWater => (20, 40, 120),
                clauvolution_world::TerrainType::ShallowWater => (40, 80, 160),
                clauvolution_world::TerrainType::Sand => (180, 170, 120),
                clauvolution_world::TerrainType::Grassland => (60, 130, 50),
                clauvolution_world::TerrainType::Forest => (30, 90, 30),
                clauvolution_world::TerrainType::Rock => (120, 120, 110),
            };

            let idx = (py * size + px) * 4;
            image.data[idx] = r;
            image.data[idx + 1] = g;
            image.data[idx + 2] = b;
            image.data[idx + 3] = 255;
        }
    }

    // Paint organisms as bright dots
    for (pos, genome) in organisms {
        let px = (pos.0.x / world_w * size as f32) as usize;
        let py = size - 1 - (pos.0.y / world_h * size as f32) as usize;

        if px < size && py < size {
            let idx = (py * size + px) * 4;
            let is_plant = genome.photosynthesis_rate > 0.2 && genome.has_photo_surface();
            let is_predator = genome.claw_power() > 0.5;

            if is_plant {
                image.data[idx] = 100;
                image.data[idx + 1] = 255;
                image.data[idx + 2] = 100;
            } else if is_predator {
                image.data[idx] = 255;
                image.data[idx + 1] = 60;
                image.data[idx + 2] = 60;
            } else {
                image.data[idx] = 255;
                image.data[idx + 1] = 255;
                image.data[idx + 2] = 255;
            }
        }
    }
}

/// Heatmap minimap: density gradient coloured by dominant strategy
fn paint_minimap_heatmap(
    image: &mut Image,
    size: usize,
    world_w: f32,
    world_h: f32,
    organisms: &Query<(&Position, &Genome), With<Organism>>,
) {
    // Grid cells — each cell covers a region of the minimap
    let cell_size = 4usize; // pixels per cell
    let grid_w = size / cell_size;
    let grid_h = size / cell_size;
    let grid_len = grid_w * grid_h;

    // Count organisms per cell, tracking strategy breakdown
    let mut plants = vec![0u32; grid_len];
    let mut predators = vec![0u32; grid_len];
    let mut foragers = vec![0u32; grid_len];

    for (pos, genome) in organisms {
        let gx = (pos.0.x / world_w * grid_w as f32) as usize;
        let gy = (pos.0.y / world_h * grid_h as f32) as usize;

        if gx < grid_w && gy < grid_h {
            let gi = gy * grid_w + gx;
            let is_plant = genome.photosynthesis_rate > 0.2 && genome.has_photo_surface();
            let is_predator = genome.claw_power() > 0.5;

            if is_plant {
                plants[gi] += 1;
            } else if is_predator {
                predators[gi] += 1;
            } else {
                foragers[gi] += 1;
            }
        }
    }

    // Find max density for normalization
    let max_density = plants.iter().zip(predators.iter()).zip(foragers.iter())
        .map(|((&p, &pr), &f)| p + pr + f)
        .max()
        .unwrap_or(1)
        .max(1) as f32;

    // Paint each pixel based on its grid cell
    for py in 0..size {
        for px in 0..size {
            // Map pixel to grid cell (minimap y is flipped)
            let gy = grid_h - 1 - (py / cell_size).min(grid_h - 1);
            let gx = (px / cell_size).min(grid_w - 1);
            let gi = gy * grid_w + gx;

            let p = plants[gi] as f32;
            let pr = predators[gi] as f32;
            let f = foragers[gi] as f32;
            let total = p + pr + f;

            let idx = (py * size + px) * 4;

            if total < 0.5 {
                // Empty cell — dark background
                image.data[idx] = 15;
                image.data[idx + 1] = 15;
                image.data[idx + 2] = 20;
            } else {
                // Blend colour by strategy proportion, intensity by density
                let intensity = (total / max_density).sqrt().clamp(0.15, 1.0);
                let r = (pr / total) * intensity;
                let g = (p / total) * intensity;
                let b = (f / total) * intensity * 0.6;
                // Add white component for foragers so they're visible
                let forager_white = (f / total) * intensity * 0.4;

                image.data[idx] = ((r + forager_white) * 255.0).min(255.0) as u8;
                image.data[idx + 1] = ((g + forager_white) * 255.0).min(255.0) as u8;
                image.data[idx + 2] = ((b + forager_white) * 255.0).min(255.0) as u8;
            }
            image.data[idx + 3] = 255;
        }
    }
}

/// Click on minimap to teleport camera
fn minimap_click_system(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    minimap_node: Query<(&Node, &ComputedNode), With<MinimapNode>>,
    minimap: Option<Res<MinimapData>>,
    config: Res<SimConfig>,
    mut camera: Query<&mut Transform, With<MainCamera>>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(minimap) = minimap else { return };
    let Ok(window) = windows.get_single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };

    // Get minimap screen position and size
    let Ok((_node, _computed)) = minimap_node.get_single() else { return };

    // The minimap is positioned at left:10, top:38 with fixed size
    // (38 = 28px egui header + 10px gap).
    let map_size = minimap.size as f32;
    let map_left = 10.0;
    let map_top = 38.0;

    // Check if click is within minimap bounds
    let local_x = cursor_pos.x - map_left;
    let local_y = cursor_pos.y - map_top;

    if local_x < 0.0 || local_x > map_size || local_y < 0.0 || local_y > map_size {
        return;
    }

    // Convert minimap pixel to world coordinate
    let world_x = local_x / map_size * config.world_width as f32;
    let world_y = (1.0 - local_y / map_size) * config.world_height as f32;

    if let Ok(mut cam_transform) = camera.get_single_mut() {
        cam_transform.translation.x = world_x;
        cam_transform.translation.y = world_y;
    }
}

/// R — pick a random living organism and select it. Frictionless "show me
/// something alive" shortcut when you don't have anything in mind or want
/// to break out of a stuck-on-one-creature mental loop.
fn random_select_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<SelectedOrganism>,
    organisms: Query<Entity, With<Organism>>,
    ui_input: Res<UiInputState>,
) {
    if ui_input.wants_keyboard {
        return;
    }
    if !keys.just_pressed(KeyCode::KeyR) {
        return;
    }
    let candidates: Vec<Entity> = organisms.iter().collect();
    if candidates.is_empty() {
        return;
    }
    let mut rng = rand::thread_rng();
    let idx = rand::Rng::gen_range(&mut rng, 0..candidates.len());
    selected.entity = Some(candidates[idx]);
}

/// , / . — cycle backward/forward through living members of the selected
/// organism's species. Lets you browse siblings of an interesting specimen
/// without having to click each dot on the map. Complements F (focus camera).
fn cycle_species_member_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<SelectedOrganism>,
    species_query: Query<(Entity, &SpeciesId), With<Organism>>,
    ui_input: Res<UiInputState>,
) {
    if ui_input.wants_keyboard {
        return;
    }
    let next = keys.just_pressed(KeyCode::Period);
    let prev = keys.just_pressed(KeyCode::Comma);
    if !next && !prev {
        return;
    }

    let Some(cur_entity) = selected.entity else { return };
    let Ok((_, cur_species)) = species_query.get(cur_entity) else {
        // selected organism died; nothing sensible to cycle within
        return;
    };

    // Collect members of the current species; sort by Entity for a stable order
    let mut members: Vec<Entity> = species_query
        .iter()
        .filter(|(_, sp)| sp.0 == cur_species.0)
        .map(|(e, _)| e)
        .collect();
    if members.len() < 2 {
        return;
    }
    members.sort();

    let cur_idx = members.iter().position(|&e| e == cur_entity).unwrap_or(0);
    let new_idx = if next {
        (cur_idx + 1) % members.len()
    } else {
        (cur_idx + members.len() - 1) % members.len()
    };
    selected.entity = Some(members[new_idx]);
}
