// Bevy system signatures legitimately nest multiple generics (Query with
// tuple data + tuple filters, etc.). Clippy flags most of them as
// "very complex type" but there's no meaningful readability win from
// aliasing each one individually — Bevy idiomatic code accepts this.
#![allow(clippy::type_complexity)]
// Similarly, Bevy systems often take >7 params (queries, resources, events);
// clippy's too_many_arguments lint misfires constantly on system signatures.
#![allow(clippy::too_many_arguments)]

use bevy::image::{Image, ImageSampler};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiUserTextures};
use clauvolution_body::BodyPlan;
use clauvolution_core::*;
use clauvolution_genome::{Genome, SegmentType};
use clauvolution_phylogeny::{classify_strategy, SpeciesStrategy};
use clauvolution_world::TileMap;

// -----------------------------------------------------------------------------
// View tuning constants
//
// Screen-space sizes are in pixels at zoom 1.0; each use multiplies by the
// camera's `OrthographicProjection::scale` so it stays the same on screen
// however far the camera is zoomed.
// -----------------------------------------------------------------------------

/// Minimum hit radius (pixels at zoom 1.0) for click-to-select. Larger
/// organisms use their body radius instead, so small ones stay clickable.
const CLICK_RADIUS_PX: f32 = 5.0;
/// Margin (pixels at zoom 1.0) added around the viewport when culling
/// organism sprites so they do not pop in and out at the screen edge.
const SPRITE_CULL_MARGIN_PX: f32 = 20.0;
/// Margin (pixels at zoom 1.0) added around the viewport when culling
/// infection indicator rings, which are drawn larger than the body.
const INDICATOR_CULL_MARGIN_PX: f32 = 40.0;

/// Selection ring radius as a multiple of the selected organism's body size.
const SELECTION_RING_SCALE: f32 = 3.5;
/// Selection ring depth: just behind active organisms (z 1.0), in front of
/// plants (z 0.3) and food (z 0.5).
const SELECTION_RING_Z: f32 = 0.9;

/// Uniform `Transform` scale for an organism sprite. The simple LOD draws a
/// unit circle, so it scales by body size. The detailed LOD draws
/// `BodyPlan` parts whose sizes already include body size, so it must not
/// scale by it again. Both give a radius of `2 × body_size` for a unit
/// torso, so the two LODs agree and switching between them does not pop.
fn organism_sprite_scale(body_size: f32, detailed: bool, energy_factor: f32, flash: f32) -> f32 {
    let size = if detailed { 1.0 } else { body_size };
    size * 2.0 * energy_factor * flash
}

/// Logical window size assumed when the primary window cannot be read. It is
/// the resolution the app requests at startup.
const FALLBACK_WINDOW_SIZE: Vec2 = Vec2::new(1920.0, 1080.0);

/// Logical size of the primary window, which is what the 2D camera's default
/// `ScalingMode::WindowSize` projection maps one world unit per pixel onto.
fn window_logical_size(windows: &Query<&Window, With<PrimaryWindow>>) -> Vec2 {
    windows
        .get_single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(FALLBACK_WINDOW_SIZE)
}

/// World-space rectangle the camera shows, grown by `margin_px` screen pixels
/// on every side. `zoom` is `OrthographicProjection::scale`. Culling and the
/// minimap viewport box both derive from this so they follow the real window
/// size rather than a fixed one.
fn visible_world_rect(camera_center: Vec2, window_size: Vec2, zoom: f32, margin_px: f32) -> Rect {
    let half = (window_size * 0.5 + Vec2::splat(margin_px)) * zoom;
    Rect::from_center_half_size(camera_center, half)
}

/// Minimap pixel bounds `(left, right, top, bottom)` of a world rectangle on a
/// `size`-pixel square minimap of a `world` sized map. Minimap y grows
/// downwards, world y upwards. Values may fall outside the image; the painter
/// clips them.
fn minimap_rect_px(view: Rect, world: Vec2, size: usize) -> (i32, i32, i32, i32) {
    let s = size as f32;
    let left = (view.min.x / world.x * s) as i32;
    let right = (view.max.x / world.x * s) as i32;
    let top = size as i32 - 1 - (view.max.y / world.y * s) as i32;
    let bottom = size as i32 - 1 - (view.min.y / world.y * s) as i32;
    (left, right, top, bottom)
}

/// Minimap dot colour per strategy (normal mode, heatmap blend, legend).
fn strategy_rgb(strategy: SpeciesStrategy) -> [u8; 3] {
    match strategy {
        SpeciesStrategy::Photosynthesizer => [100, 255, 100],
        SpeciesStrategy::Grazer => [255, 210, 80],
        SpeciesStrategy::Hunter => [255, 60, 60],
        SpeciesStrategy::Omnivore => [255, 255, 255],
    }
}

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
            .init_resource::<MinimapVisible>()
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
                    draw_minimap_egui,
                    cycle_species_member_system,
                    random_select_system,
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    spawn_terrain_sprites,
                    sync_organism_transforms,
                    sync_selection_ring,
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

/// Marks an organism whose sprite was built at the detailed LOD (body-plan
/// parts rather than a circle). `lod_change_system` strips it with the
/// sprite. Read by `sync_organism_transforms` to pick the sprite scale.
#[derive(Component)]
pub struct DetailedSprite;

#[derive(Component)]
pub struct FoodSprite;

#[derive(Component)]

/// Tracks whether we're in detailed or simple rendering mode
#[derive(Resource, Default)]
pub struct LodState {
    pub detailed: bool,
}

#[derive(Resource)]
pub struct UiFont(pub Handle<Font>);

#[derive(Resource, Default, PartialEq, Eq)]
pub enum MinimapMode {
    #[default]
    Normal,
    Heatmap,
    /// Fades terrain and organism dots, paints members of the currently
    /// selected organism's species bright. Reveals where a species lives.
    Range,
}

#[derive(Resource)]
pub struct MinimapVisible(pub bool);

impl Default for MinimapVisible {
    fn default() -> Self {
        Self(true)
    }
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
    pub selection_material: Option<Handle<ColorMaterial>>,
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
    shared.outline_material =
        Some(materials.add(ColorMaterial::from(Color::srgba(0.0, 0.0, 0.0, 0.6))));
    shared.selection_material =
        Some(materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 0.0, 0.5))));
}

#[derive(Component)]
pub struct TerrainRendered;

fn setup_camera(mut commands: Commands, config: Res<SimConfig>, asset_server: Res<AssetServer>) {
    let font: Handle<Font> = asset_server.load("fonts/JetBrainsMono-Regular.ttf");
    commands.insert_resource(UiFont(font));
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
    // Legend is drawn inline with the minimap by `draw_minimap_egui` now
    // that the minimap lives in an egui Area (so it ends up in screenshots
    // along with the rest of the panels).
}

/// True while either Shift key is held. Every Shift-modified binding (drag
/// pan, click suppression, Shift+S, Shift+M, WASD suppression) goes through
/// this so left and right Shift behave the same.
fn shift_held(keys: &ButtonInput<KeyCode>) -> bool {
    keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
}

/// Keyboard speed controls: Space = pause, [ = slower, ] = faster
fn speed_control_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut speed: ResMut<SimSpeed>,
    ui_input: Res<UiInputState>,
) {
    if ui_input.wants_keyboard {
        return;
    }
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
    ui_input: Res<UiInputState>,
) {
    // Don't select when dragging or when pointer is over an egui panel
    if shift_held(&keys) || ui_input.pointer_over_ui {
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

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

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
    let click_radius = CLICK_RADIUS_PX * projection.scale;

    for (entity, pos, body_size) in &organisms {
        let dist = (pos.0 - world_pos).length();
        let hit_radius = (body_size.0 * 2.0).max(click_radius);
        if dist < hit_radius && dist < nearest_dist {
            nearest_dist = dist;
            nearest = Some(entity);
        }
    }

    // The ring follows from `sync_selection_ring`.
    selected.entity = nearest;
}

/// Keep exactly one selection ring on the selected organism, whichever path
/// selected it (click, R, `,`/`.`, a panel link), and remove it when nothing
/// is selected or the selected organism has died.
fn sync_selection_ring(
    mut commands: Commands,
    selected: Res<SelectedOrganism>,
    shared_meshes: Res<SharedMeshes>,
    organisms: Query<(&Position, &BodySize), With<Organism>>,
    mut rings: Query<(Entity, &mut Transform), With<SelectionRing>>,
) {
    let Some((pos, body_size)) = selected.entity.and_then(|e| organisms.get(e).ok()) else {
        for (ring, _) in &rings {
            commands.entity(ring).try_despawn();
        }
        return;
    };
    let ring_transform = Transform::from_xyz(pos.0.x, pos.0.y, SELECTION_RING_Z)
        .with_scale(Vec3::splat(body_size.0 * SELECTION_RING_SCALE));

    let mut have_ring = false;
    for (ring, mut transform) in &mut rings {
        if have_ring {
            commands.entity(ring).try_despawn();
        } else {
            *transform = ring_transform;
            have_ring = true;
        }
    }
    if have_ring {
        return;
    }
    let (Some(mesh), Some(material)) = (
        shared_meshes.circle.clone(),
        shared_meshes.selection_material.clone(),
    ) else {
        warn_once!("sync_selection_ring: shared meshes missing — no selection ring");
        return;
    };
    commands.spawn((
        Mesh2d(mesh),
        MeshMaterial2d(material),
        ring_transform,
        SelectionRing,
    ));
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
        (
            &Position,
            &Energy,
            &BodySize,
            &ActionFlash,
            &mut Transform,
            &mut Visibility,
            Has<DetailedSprite>,
        ),
        (With<Organism>, With<OrganismSprite>),
    >,
    camera: Query<
        (&Transform, &OrthographicProjection),
        (With<MainCamera>, Without<Organism>, Without<SelectionRing>),
    >,
    config: Res<SimConfig>,
    mut species_colors: ResMut<SpeciesColors>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let (zoom_scale, view) = if let Ok((cam_t, proj)) = camera.get_single() {
        (
            proj.scale,
            visible_world_rect(
                cam_t.translation.truncate(),
                window_logical_size(&windows),
                proj.scale,
                SPRITE_CULL_MARGIN_PX,
            ),
        )
    } else {
        (
            1.0,
            Rect::new(
                0.0,
                0.0,
                config.world_width as f32,
                config.world_height as f32,
            ),
        )
    };

    let use_detailed = zoom_scale < 0.6;

    for (entity, pos, genome, body_plan, species_id) in &organisms_without_sprite {
        let is_plant = genome.is_photosynthesiser();
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
                Transform::from_xyz(pos.0.x, pos.0.y, z_level).with_scale(Vec3::splat(
                    organism_sprite_scale(genome.body_size, true, 1.0, 1.0),
                )),
                OrganismSprite,
                DetailedSprite,
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
            let is_plant = genome.is_photosynthesiser();

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
                    let outline = commands
                        .spawn((
                            Mesh2d(mesh),
                            MeshMaterial2d(outline_mat.clone()),
                            Transform::from_xyz(0.0, 0.0, -0.1).with_scale(Vec3::splat(1.3)),
                            OrganismOutline,
                        ))
                        .id();
                    commands.entity(entity).add_child(outline);
                }
            }
        }
    }

    // Update existing transforms — frustum cull off-screen organisms
    for (pos, energy, body_size, flash, mut transform, mut vis, detailed) in
        &mut organisms_with_sprite
    {
        if !view.contains(pos.0) {
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
        transform.scale = Vec3::splat(organism_sprite_scale(
            body_size.0,
            detailed,
            energy_factor,
            flash_pulse,
        ));
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
        *vis = if food_visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Don't spawn new food sprites if zoomed out
    if !food_visible {
        return;
    }

    let Some(mesh) = &shared_meshes.food_circle else {
        return;
    };
    let Some(material) = &shared_meshes.food_material else {
        return;
    };

    for (entity, pos) in &food_without_sprite {
        commands.entity(entity).insert((
            Mesh2d(mesh.clone()),
            MeshMaterial2d(material.clone()),
            Transform::from_xyz(pos.0.x, pos.0.y, 0.5).with_scale(Vec3::splat(1.5)),
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
    mut existing_markers: Query<
        (Entity, &mut DeathMarker, &mut Transform),
        With<DeathMarkerSprite>,
    >,
) {
    let Some(mesh) = &shared_meshes.circle else {
        return;
    };

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
            Transform::from_xyz(pos.0.x, pos.0.y, 0.8).with_scale(Vec3::splat(2.0)),
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
    mut focus_requests: EventReader<CameraFocusRequest>,
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

    // UI-driven focus (a clicked chronicle location, for instance). The last
    // request in a frame wins.
    for CameraFocusRequest(target) in focus_requests.read() {
        transform.translation.x = target.x;
        transform.translation.y = target.y;
    }

    let dt = time.delta_secs();

    // Keyboard pan and zoom stand down while egui has keyboard focus, like
    // every other hotkey.
    if !ui_input.wants_keyboard {
        let pan = keyboard_pan_direction(&keys);
        let speed = 200.0 * projection.scale * dt;
        transform.translation.x += pan.x * speed;
        transform.translation.y += pan.y * speed;

        let zoom_speed = 2.0 * dt;
        if keys.pressed(KeyCode::KeyE) || keys.pressed(KeyCode::Equal) {
            projection.scale *= 1.0 - zoom_speed;
        }
        if keys.pressed(KeyCode::KeyQ) || keys.pressed(KeyCode::Minus) {
            projection.scale *= 1.0 + zoom_speed;
        }
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

    let dragging = drag_pan_held(&mouse_buttons, &keys);

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

/// Direction the keyboard asks the camera to pan in, one unit per held axis
/// (not normalised, matching the original per-key speed). WASD is
/// suppressed while Shift is held so Shift+S (screenshot) and Shift+M
/// (minimap toggle) don't also pan. Arrow keys are unaffected because they
/// are not used as modifier targets anywhere.
fn keyboard_pan_direction(keys: &ButtonInput<KeyCode>) -> Vec2 {
    let letters = !shift_held(keys);
    let held =
        |letter: KeyCode, arrow: KeyCode| (letters && keys.pressed(letter)) || keys.pressed(arrow);
    let mut dir = Vec2::ZERO;
    if held(KeyCode::KeyW, KeyCode::ArrowUp) {
        dir.y += 1.0;
    }
    if held(KeyCode::KeyS, KeyCode::ArrowDown) {
        dir.y -= 1.0;
    }
    if held(KeyCode::KeyA, KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if held(KeyCode::KeyD, KeyCode::ArrowRight) {
        dir.x += 1.0;
    }
    dir
}

/// Mouse-drag pan: middle or right button, or Shift plus left button.
fn drag_pan_held(mouse_buttons: &ButtonInput<MouseButton>, keys: &ButtonInput<KeyCode>) -> bool {
    mouse_buttons.pressed(MouseButton::Middle)
        || mouse_buttons.pressed(MouseButton::Right)
        || (mouse_buttons.pressed(MouseButton::Left) && shift_held(keys))
}

fn toggle_minimap_mode_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<MinimapMode>,
    mut visible: ResMut<MinimapVisible>,
    ui_input: Res<UiInputState>,
) {
    if ui_input.wants_keyboard || !keys.just_pressed(KeyCode::KeyM) {
        return;
    }
    if shift_held(&keys) {
        // Shift+M: toggle minimap visibility. draw_minimap_egui respects
        // the flag; legend follows the image automatically.
        visible.0 = !visible.0;
    } else {
        // Plain M: cycle minimap mode (normal ↔ heatmap)
        *mode = match *mode {
            MinimapMode::Normal => MinimapMode::Heatmap,
            MinimapMode::Heatmap => MinimapMode::Range,
            MinimapMode::Range => MinimapMode::Normal,
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
    camera: Query<
        (&Transform, &OrthographicProjection),
        (With<MainCamera>, Without<Organism>, Without<SelectionRing>),
    >,
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok((cam_t, proj)) = camera.get_single() else {
        return;
    };
    let view = visible_world_rect(
        cam_t.translation.truncate(),
        window_logical_size(&windows),
        proj.scale,
        INDICATOR_CULL_MARGIN_PX,
    );

    // Pulse rate: slow (about 1Hz). Breath-like.
    let pulse = (time.elapsed_secs() * std::f32::consts::TAU).sin() * 0.15 + 0.85;

    for (pos, body_size, infection) in &infected {
        if !view.contains(pos.0) {
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
    let Some(entity) = selected.entity else {
        return;
    };
    let Ok((_pos, trail, species)) = organisms.get(entity) else {
        return;
    };
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
    let zoom_scale = camera.get_single().map(|p| p.scale).unwrap_or(1.0);

    let should_be_detailed = zoom_scale < 0.6;

    if should_be_detailed == lod_state.detailed {
        return;
    }

    lod_state.detailed = should_be_detailed;

    // Strip OrganismSprite, Mesh2d, MeshMaterial2d from all organisms
    // so sync_organism_transforms re-creates them at the new LOD level.
    // Also despawn child entities (body parts, outlines).
    for (entity, children) in &organisms {
        commands
            .entity(entity)
            .remove::<OrganismSprite>()
            .remove::<DetailedSprite>()
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
    ui_input: Res<UiInputState>,
) {
    // Shift+S takes a screenshot. Plain S is WASD camera-south; the
    // camera_control_system skips panning when shift is pressed so the
    // two don't fight.
    if ui_input.wants_keyboard {
        return;
    }
    if shift_held(&keys) && keys.just_pressed(KeyCode::KeyS) {
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
    mut egui_user_textures: ResMut<EguiUserTextures>,
    _config: Res<SimConfig>,
) {
    let size = 160u32;

    // Create a blank RGBA image
    let mut image = Image::new_fill(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[40, 40, 40, 255],
        TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::all(),
    );
    image.sampler = ImageSampler::nearest();
    let image_handle = images.add(image);

    // Register the paint image as an egui texture so `draw_minimap_egui`
    // can render it inside an egui Area. This replaces the old Bevy-UI
    // ImageNode path — the egui Area ends up inside the render target
    // that our screenshot pipeline captures, so the minimap now shows up
    // in saved PNGs alongside the other overlays.
    egui_user_textures.add_image(image_handle.clone());

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
    organisms: Query<(&Position, &Genome, &SpeciesId), With<Organism>>,
    camera: Query<(&Transform, &OrthographicProjection), With<MainCamera>>,
    minimap_mode: Res<MinimapMode>,
    selected: Res<SelectedOrganism>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    minimap.timer.tick(time.delta());
    if !minimap.timer.just_finished() {
        return;
    }

    let Some(tile_map) = tile_map else { return };
    let Some(image) = images.get_mut(&minimap.image_handle) else {
        return;
    };

    let size = minimap.size as usize;
    let world_w = config.world_width as f32;
    let world_h = config.world_height as f32;

    match *minimap_mode {
        MinimapMode::Normal => {
            paint_minimap_normal(
                image, size, world_w, world_h, &tile_map, &config, &organisms,
            );
        }
        MinimapMode::Heatmap => {
            paint_minimap_heatmap(image, size, world_w, world_h, &organisms);
        }
        MinimapMode::Range => {
            // Resolve which species to highlight — the one the selected
            // organism belongs to. No selection → fall back to Normal.
            let focus_species = selected
                .entity
                .and_then(|e| organisms.get(e).ok().map(|(_, _, s)| s.0));
            match focus_species {
                Some(sp) => paint_minimap_range(
                    image, size, world_w, world_h, &tile_map, &config, &organisms, sp,
                ),
                None => paint_minimap_normal(
                    image, size, world_w, world_h, &tile_map, &config, &organisms,
                ),
            }
        }
    }

    // Paint camera viewport rectangle (both modes)
    if let Ok((cam_transform, projection)) = camera.get_single() {
        let view = visible_world_rect(
            cam_transform.translation.truncate(),
            window_logical_size(&windows),
            projection.scale,
            0.0,
        );
        let (left, right, top, bottom) = minimap_rect_px(view, Vec2::new(world_w, world_h), size);

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
        if let Ok((pos, _genome, _species)) = organisms.get(sel_entity) {
            let cx = (pos.0.x / world_w * size as f32) as i32;
            let cy = size as i32 - 1 - (pos.0.y / world_h * size as f32) as i32;
            // Draw plus shape: center + 2 pixels each direction
            for &(dx, dy) in &[
                (0, 0),
                (-2, 0),
                (-1, 0),
                (1, 0),
                (2, 0),
                (0, -2),
                (0, -1),
                (0, 1),
                (0, 2),
            ] {
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
    organisms: &Query<(&Position, &Genome, &SpeciesId), With<Organism>>,
) {
    // Paint terrain
    for py in 0..size {
        for px in 0..size {
            let wx = (px as f32 / size as f32 * world_w) as u32;
            let wy = ((size - 1 - py) as f32 / size as f32 * world_h) as u32;
            let tile = tile_map.get(
                wx.min(config.world_width - 1),
                wy.min(config.world_height - 1),
            );

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
    for (pos, genome, _species) in organisms {
        let px = (pos.0.x / world_w * size as f32) as usize;
        let py = size - 1 - (pos.0.y / world_h * size as f32) as usize;

        if px < size && py < size {
            let idx = (py * size + px) * 4;
            let [r, g, b] = strategy_rgb(classify_strategy(genome));
            image.data[idx] = r;
            image.data[idx + 1] = g;
            image.data[idx + 2] = b;
        }
    }
}

/// Heatmap minimap: density gradient coloured by dominant strategy
fn paint_minimap_heatmap(
    image: &mut Image,
    size: usize,
    world_w: f32,
    world_h: f32,
    organisms: &Query<(&Position, &Genome, &SpeciesId), With<Organism>>,
) {
    // Grid cells — each cell covers a region of the minimap
    let cell_size = 4usize; // pixels per cell
    let grid_w = size / cell_size;
    let grid_h = size / cell_size;
    let grid_len = grid_w * grid_h;

    // Count organisms per cell, one histogram per strategy (indexed as in
    // `SpeciesStrategy::ALL`).
    let mut counts = vec![[0u32; SpeciesStrategy::ALL.len()]; grid_len];

    for (pos, genome, _species) in organisms {
        let gx = (pos.0.x / world_w * grid_w as f32) as usize;
        let gy = (pos.0.y / world_h * grid_h as f32) as usize;

        if gx < grid_w && gy < grid_h {
            let gi = gy * grid_w + gx;
            let strategy = classify_strategy(genome);
            let slot = SpeciesStrategy::ALL
                .iter()
                .position(|s| *s == strategy)
                .unwrap_or(0);
            counts[gi][slot] += 1;
        }
    }

    // Find max density for normalization
    let max_density = counts
        .iter()
        .map(|c| c.iter().sum::<u32>())
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

            let cell = &counts[gi];
            let total = cell.iter().sum::<u32>() as f32;

            let idx = (py * size + px) * 4;

            if total < 0.5 {
                // Empty cell — dark background
                image.data[idx] = 15;
                image.data[idx + 1] = 15;
                image.data[idx + 2] = 20;
            } else {
                // Blend the strategy colours by proportion, intensity by density
                let intensity = (total / max_density).sqrt().clamp(0.15, 1.0);
                let mut rgb = [0.0f32; 3];
                for (strategy, &n) in SpeciesStrategy::ALL.iter().zip(cell.iter()) {
                    let share = n as f32 / total;
                    for (acc, c) in rgb.iter_mut().zip(strategy_rgb(*strategy)) {
                        *acc += share * c as f32;
                    }
                }
                for (k, c) in rgb.iter().enumerate() {
                    image.data[idx + k] = (c * intensity).min(255.0) as u8;
                }
            }
            image.data[idx + 3] = 255;
        }
    }
}

/// Range minimap: faded terrain + only members of `focus_species`
/// painted bright. Reveals where a species lives at a glance —
/// shows niche partitioning you can't see from the normal view
/// where every organism is painted the same brightness.
fn paint_minimap_range(
    image: &mut Image,
    size: usize,
    world_w: f32,
    world_h: f32,
    tile_map: &TileMap,
    config: &SimConfig,
    organisms: &Query<(&Position, &Genome, &SpeciesId), With<Organism>>,
    focus_species: u64,
) {
    // Faded terrain (same colours as Normal but dimmed to 35%)
    for py in 0..size {
        for px in 0..size {
            let wx = (px as f32 / size as f32 * world_w) as u32;
            let wy = ((size - 1 - py) as f32 / size as f32 * world_h) as u32;
            let tile = tile_map.get(
                wx.min(config.world_width - 1),
                wy.min(config.world_height - 1),
            );
            let (r, g, b) = match tile.terrain {
                clauvolution_world::TerrainType::DeepWater => (20, 40, 120),
                clauvolution_world::TerrainType::ShallowWater => (40, 80, 160),
                clauvolution_world::TerrainType::Sand => (180, 170, 120),
                clauvolution_world::TerrainType::Grassland => (60, 130, 50),
                clauvolution_world::TerrainType::Forest => (30, 90, 30),
                clauvolution_world::TerrainType::Rock => (120, 120, 110),
            };
            let idx = (py * size + px) * 4;
            image.data[idx] = (r as f32 * 0.35) as u8;
            image.data[idx + 1] = (g as f32 * 0.35) as u8;
            image.data[idx + 2] = (b as f32 * 0.35) as u8;
            image.data[idx + 3] = 255;
        }
    }

    // Faint grey dots for every other organism so the empty vs.
    // occupied-by-other-species distinction still reads.
    for (pos, _genome, species) in organisms {
        if species.0 == focus_species {
            continue;
        }
        let px = (pos.0.x / world_w * size as f32) as usize;
        let py = size - 1 - (pos.0.y / world_h * size as f32) as usize;
        if px < size && py < size {
            let idx = (py * size + px) * 4;
            image.data[idx] = 80;
            image.data[idx + 1] = 80;
            image.data[idx + 2] = 90;
        }
    }

    // Bright highlight for the focus species — paint a 3×3 block so
    // single organisms are actually visible at low pixel counts.
    for (pos, _genome, species) in organisms {
        if species.0 != focus_species {
            continue;
        }
        let cx = (pos.0.x / world_w * size as f32) as i32;
        let cy = size as i32 - 1 - (pos.0.y / world_h * size as f32) as i32;
        for dy in -1..=1i32 {
            for dx in -1..=1i32 {
                let x = cx + dx;
                let y = cy + dy;
                if x >= 0 && (x as usize) < size && y >= 0 && (y as usize) < size {
                    let idx = (y as usize * size + x as usize) * 4;
                    image.data[idx] = 255;
                    image.data[idx + 1] = 240;
                    image.data[idx + 2] = 90;
                }
            }
        }
    }
}

/// Draws the minimap (paint image + legend) as an egui Area in the
/// top-left corner. Being egui means it flows through the same render
/// path as the right-side panel and header bar, so the screenshot
/// pipeline catches it automatically. Click-to-teleport is handled via
/// the egui image Response rather than cursor coordinates.
fn draw_minimap_egui(
    mut contexts: EguiContexts,
    minimap: Option<Res<MinimapData>>,
    visible: Res<MinimapVisible>,
    config: Res<SimConfig>,
    mut camera: Query<&mut Transform, With<MainCamera>>,
) {
    if !visible.0 {
        return;
    }
    let Some(minimap) = minimap else { return };

    let Some(tex_id) = contexts.image_id(&minimap.image_handle) else {
        // Image not yet registered with egui this frame — try again later.
        return;
    };

    let ctx = contexts.ctx_mut();
    let size = minimap.size as f32;

    egui::Area::new(egui::Id::new("minimap"))
        .fixed_pos([10.0, 38.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 160))
                .inner_margin(egui::Margin::same(4))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        let image = egui::Image::new(egui::load::SizedTexture::new(
                            tex_id,
                            egui::vec2(size, size),
                        ));
                        let response = ui.add(image.sense(egui::Sense::click()));

                        if response.clicked() {
                            if let Some(pos) = response.interact_pointer_pos() {
                                let local = pos - response.rect.min;
                                let nx = (local.x / size).clamp(0.0, 1.0);
                                let ny = (local.y / size).clamp(0.0, 1.0);
                                let world_x = nx * config.world_width as f32;
                                let world_y = (1.0 - ny) * config.world_height as f32;
                                if let Ok(mut t) = camera.get_single_mut() {
                                    t.translation.x = world_x;
                                    t.translation.y = world_y;
                                }
                            }
                        }

                        ui.add_space(2.0);
                        for strategy in SpeciesStrategy::ALL {
                            let [r, g, b] = strategy_rgb(strategy);
                            ui.colored_label(
                                egui::Color32::from_rgb(r, g, b),
                                format!("● {}", strategy.label().to_lowercase()),
                            );
                        }
                    });
                });
        });
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

    let Some(cur_entity) = selected.entity else {
        return;
    };
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

#[cfg(test)]
mod tests {
    use super::*;
    use clauvolution_genome::InnovationCounter;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn keys_with(pressed: &[KeyCode]) -> ButtonInput<KeyCode> {
        let mut keys = ButtonInput::<KeyCode>::default();
        for &k in pressed {
            keys.press(k);
        }
        keys
    }

    fn assert_vec2_eq(a: Vec2, b: Vec2) {
        assert!((a - b).length() < 1e-4, "{a} != {b}");
    }

    // --- shift-modifier-checks-inconsistent ---

    #[test]
    fn either_shift_key_counts_as_shift() {
        assert!(shift_held(&keys_with(&[KeyCode::ShiftLeft])));
        assert!(shift_held(&keys_with(&[KeyCode::ShiftRight])));
        assert!(!shift_held(&keys_with(&[KeyCode::KeyS])));
    }

    #[test]
    fn right_shift_plus_left_drag_pans() {
        let mut mouse = ButtonInput::<MouseButton>::default();
        mouse.press(MouseButton::Left);
        assert!(drag_pan_held(&mouse, &keys_with(&[KeyCode::ShiftRight])));
        assert!(drag_pan_held(&mouse, &keys_with(&[KeyCode::ShiftLeft])));
        assert!(!drag_pan_held(&mouse, &keys_with(&[])));
    }

    #[test]
    fn shift_suppresses_wasd_but_not_arrows() {
        assert_vec2_eq(
            keyboard_pan_direction(&keys_with(&[KeyCode::ShiftRight, KeyCode::KeyS])),
            Vec2::ZERO,
        );
        assert_vec2_eq(
            keyboard_pan_direction(&keys_with(&[KeyCode::ShiftLeft, KeyCode::ArrowDown])),
            Vec2::new(0.0, -1.0),
        );
        assert_vec2_eq(
            keyboard_pan_direction(&keys_with(&[KeyCode::KeyW, KeyCode::KeyD])),
            Vec2::new(1.0, 1.0),
        );
    }

    // --- hotkeys-ignore-egui-keyboard-focus ---

    fn app_with_keys(pressed: &[KeyCode], wants_keyboard: bool) -> App {
        let mut app = App::new();
        app.insert_resource(keys_with(pressed));
        app.insert_resource(UiInputState {
            wants_keyboard,
            pointer_over_ui: false,
        });
        app
    }

    #[test]
    fn space_and_brackets_act_only_without_egui_focus() {
        for (wants_keyboard, expect_acted) in [(false, true), (true, false)] {
            let mut app = app_with_keys(&[KeyCode::Space, KeyCode::BracketRight], wants_keyboard);
            app.insert_resource(SimSpeed {
                paused: false,
                multiplier: 1.0,
            });
            app.add_systems(Update, speed_control_system);
            app.update();
            let speed = app.world().resource::<SimSpeed>();
            assert_eq!(speed.paused, expect_acted);
            let expect_multiplier = if expect_acted { 2.0 } else { 1.0 };
            assert_eq!(speed.multiplier, expect_multiplier);
        }
    }

    #[test]
    fn m_cycles_minimap_only_without_egui_focus() {
        for (wants_keyboard, expect_heatmap) in [(false, true), (true, false)] {
            let mut app = app_with_keys(&[KeyCode::KeyM], wants_keyboard);
            app.init_resource::<MinimapMode>();
            app.init_resource::<MinimapVisible>();
            app.add_systems(Update, toggle_minimap_mode_system);
            app.update();
            let heatmap = *app.world().resource::<MinimapMode>() == MinimapMode::Heatmap;
            assert_eq!(heatmap, expect_heatmap);
        }
    }

    #[test]
    fn right_shift_m_hides_minimap() {
        let mut app = app_with_keys(&[KeyCode::ShiftRight, KeyCode::KeyM], false);
        app.init_resource::<MinimapMode>();
        app.init_resource::<MinimapVisible>();
        app.add_systems(Update, toggle_minimap_mode_system);
        app.update();
        assert!(!app.world().resource::<MinimapVisible>().0);
        assert!(*app.world().resource::<MinimapMode>() == MinimapMode::Normal);
    }

    // --- hardcoded-half-viewport ---

    #[test]
    fn visible_rect_matches_old_constants_at_default_window() {
        let rect = visible_world_rect(Vec2::new(100.0, 50.0), FALLBACK_WINDOW_SIZE, 1.0, 0.0);
        assert_vec2_eq(rect.min, Vec2::new(100.0 - 960.0, 50.0 - 540.0));
        assert_vec2_eq(rect.max, Vec2::new(100.0 + 960.0, 50.0 + 540.0));
    }

    #[test]
    fn visible_rect_follows_window_size_zoom_and_margin() {
        let rect = visible_world_rect(Vec2::ZERO, Vec2::new(1280.0, 720.0), 2.0, 0.0);
        assert_vec2_eq(rect.half_size(), Vec2::new(1280.0, 720.0));

        // The margin is in screen pixels, so it scales with zoom too.
        let rect = visible_world_rect(Vec2::ZERO, Vec2::new(800.0, 600.0), 0.5, 20.0);
        assert_vec2_eq(rect.half_size(), Vec2::new(210.0, 160.0));
    }

    #[test]
    fn minimap_rect_spans_the_minimap_when_the_whole_world_is_visible() {
        let world = Vec2::new(512.0, 256.0);
        let view = Rect::new(0.0, 0.0, world.x, world.y);
        assert_eq!(minimap_rect_px(view, world, 160), (0, 160, -1, 159));
    }

    #[test]
    fn minimap_rect_shrinks_with_the_window() {
        let world = Vec2::new(1000.0, 1000.0);
        let center = Vec2::new(500.0, 500.0);
        let wide = visible_world_rect(center, Vec2::new(400.0, 200.0), 1.0, 0.0);
        let narrow = visible_world_rect(center, Vec2::new(200.0, 200.0), 1.0, 0.0);
        let (wl, wr, wt, wb) = minimap_rect_px(wide, world, 100);
        let (nl, nr, nt, nb) = minimap_rect_px(narrow, world, 100);
        assert_eq!((wl, wr), (30, 70));
        assert_eq!((nl, nr), (40, 60));
        // Same height, so the same rows.
        assert_eq!((wt, wb), (nt, nb));
    }

    // --- detailed-lod-double-scale ---

    #[test]
    fn detailed_torso_matches_simple_circle_radius() {
        let mut rng = StdRng::seed_from_u64(3);
        let mut innovation = InnovationCounter(0);
        for body_size in [0.5, 1.0, 2.0] {
            let mut genome = Genome::new_minimal(&mut innovation, &mut rng);
            genome.body_size = body_size;
            genome.body_segments[0].size = 1.0;
            let plan = BodyPlan::from_genome(&genome);

            // The torso ellipse's semi-major axis is the part size; the
            // simple LOD is a unit circle.
            let detailed_radius =
                plan.parts[0].size * organism_sprite_scale(body_size, true, 1.0, 1.0);
            let simple_radius = organism_sprite_scale(body_size, false, 1.0, 1.0);
            assert!(
                (detailed_radius - simple_radius).abs() < 1e-5,
                "body_size {body_size}: detailed {detailed_radius} vs simple {simple_radius}"
            );
        }
    }

    #[test]
    fn sprite_scale_applies_energy_and_flash_in_both_lods() {
        let simple = organism_sprite_scale(1.5, false, 0.5, 1.2);
        let detailed = organism_sprite_scale(1.5, true, 0.5, 1.2);
        assert!((simple - 1.5 * 2.0 * 0.5 * 1.2).abs() < 1e-6);
        assert!((detailed - 2.0 * 0.5 * 1.2).abs() < 1e-6);
    }

    // --- selection-ring-only-on-click ---

    fn ring_app() -> App {
        let mut app = App::new();
        app.init_resource::<SelectedOrganism>();
        app.insert_resource(SharedMeshes {
            circle: Some(Handle::default()),
            selection_material: Some(Handle::default()),
            ..default()
        });
        app.add_systems(Update, sync_selection_ring);
        app
    }

    fn rings(app: &mut App) -> Vec<Vec3> {
        let world = app.world_mut();
        world
            .query_filtered::<&Transform, With<SelectionRing>>()
            .iter(world)
            .map(|t| t.translation)
            .collect()
    }

    fn spawn_organism(app: &mut App, at: Vec2) -> Entity {
        app.world_mut()
            .spawn((Organism, Position(at), BodySize(1.0)))
            .id()
    }

    fn select(app: &mut App, entity: Option<Entity>) {
        app.world_mut().resource_mut::<SelectedOrganism>().entity = entity;
    }

    #[test]
    fn ring_appears_for_any_selection_and_follows_it() {
        let mut app = ring_app();
        let a = spawn_organism(&mut app, Vec2::new(10.0, 20.0));
        let b = spawn_organism(&mut app, Vec2::new(30.0, 40.0));

        app.update();
        assert!(rings(&mut app).is_empty(), "no selection, no ring");

        // Selection set directly, as R, `,`/`.`, and the panel links do.
        select(&mut app, Some(a));
        app.update();
        assert_eq!(
            rings(&mut app),
            vec![Vec3::new(10.0, 20.0, SELECTION_RING_Z)]
        );

        select(&mut app, Some(b));
        app.update();
        assert_eq!(
            rings(&mut app),
            vec![Vec3::new(30.0, 40.0, SELECTION_RING_Z)]
        );

        // The ring tracks movement.
        app.world_mut().get_mut::<Position>(b).unwrap().0 = Vec2::new(35.0, 45.0);
        app.update();
        assert_eq!(
            rings(&mut app),
            vec![Vec3::new(35.0, 45.0, SELECTION_RING_Z)]
        );
    }

    #[test]
    fn ring_goes_when_the_selected_organism_dies_or_selection_clears() {
        let mut app = ring_app();
        let a = spawn_organism(&mut app, Vec2::new(10.0, 20.0));
        select(&mut app, Some(a));
        app.update();
        assert_eq!(rings(&mut app).len(), 1);

        app.world_mut().despawn(a);
        app.update();
        assert!(rings(&mut app).is_empty(), "ring outlived its organism");

        let b = spawn_organism(&mut app, Vec2::new(1.0, 2.0));
        select(&mut app, Some(b));
        app.update();
        assert_eq!(rings(&mut app).len(), 1);

        select(&mut app, None);
        app.update();
        assert!(rings(&mut app).is_empty());
    }
}
