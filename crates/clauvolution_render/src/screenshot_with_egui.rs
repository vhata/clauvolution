//! Screenshot path that captures camera + egui overlays together.
//!
//! Why not `bevy::render::view::screenshot::Screenshot`? Because bevy_egui
//! renders directly to `window.swap_chain_texture_view`, bypassing Bevy's
//! `ViewTarget`, while Bevy's `Screenshot` swaps the `ViewTarget`'s output
//! attachment — so egui draws to the real swap chain while the camera
//! draws to the capture texture. The saved PNG has no panel on it.
//! See DECISIONS.md ("Screenshotting egui overlays …") for the full story.
//!
//! Approach here:
//! 1. Create an `Image` asset with `RENDER_ATTACHMENT | COPY_SRC | TEXTURE_BINDING`.
//! 2. Spawn a secondary `Camera2d` whose target is that image, mirroring the
//!    main camera's transform and projection. It renders the main scene to
//!    the image in parallel with the main camera rendering to the window.
//! 3. Attach `EguiRenderToImage { handle }` to the primary window entity.
//!    bevy_egui's setup_new_render_to_image_nodes_system picks this up and
//!    creates a second render graph node that renders the same egui context
//!    onto the image. So the image ends up with camera-output + egui.
//! 4. A frame or two later, spawn a `Readback::texture(handle)` and observe
//!    `ReadbackComplete`. Save the PNG, clean up the camera and the
//!    EguiRenderToImage component.

use bevy::prelude::*;
use bevy::render::camera::{ClearColorConfig, RenderTarget};
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_graph::RenderGraph;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::{Extract, ExtractSchedule, RenderApp};
use bevy::window::PrimaryWindow;
use bevy_egui::{
    render_systems::{setup_new_render_to_image_nodes_system, EguiPass},
    EguiRenderToImage,
};
use std::path::PathBuf;

use crate::MainCamera;

/// Installs render-graph patches that make `EguiRenderToImage` composite
/// *after* the camera, not before. See comment on `flip_egui_image_edge`.
pub struct ScreenshotWithEguiPlugin;

impl Plugin for ScreenshotWithEguiPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                ExtractSchedule,
                flip_egui_image_edge.after(setup_new_render_to_image_nodes_system),
            );
        }
    }
}

/// bevy_egui's default wiring for `EguiRenderToImage` adds the edge
/// `egui_pass → CameraDriverLabel`, i.e. it renders UI to the image
/// *before* any camera does. That makes sense for "use egui output as a
/// texture in the 3D scene" — you render the UI first, then the camera
/// samples it. For OUR use (screenshotting camera output with UI on top)
/// it's exactly wrong: the camera runs afterwards and clears the image,
/// erasing the UI we just drew.
///
/// Flip it: remove the `egui_pass → CameraDriverLabel` edge and add the
/// reverse so the camera renders first and egui composites on top (with
/// `LoadOp::Load`).
fn flip_egui_image_edge(
    targets: Extract<Query<Entity, Added<EguiRenderToImage>>>,
    mut graph: ResMut<RenderGraph>,
) {
    for entity in targets.iter() {
        let egui_pass = EguiPass::from_render_to_image_entity(entity);
        // Ignore the error if the edge doesn't exist — different bevy_egui
        // versions might change the default wiring.
        let _ = graph.remove_node_edge(egui_pass.clone(), bevy::render::graph::CameraDriverLabel);
        graph.add_node_edge(bevy::render::graph::CameraDriverLabel, egui_pass);
    }
}

/// Marks the temporary camera we spawn to render the main scene into the
/// screenshot image. Despawned after capture completes.
#[derive(Component)]
pub struct ScreenshotCamera;

/// Resource driving in-flight screenshot captures. At most one pending
/// capture at a time — additional requests are dropped with a warning.
#[derive(Resource, Default)]
pub struct ScreenshotState {
    pub pending: Option<PendingScreenshot>,
}

pub struct PendingScreenshot {
    pub handle: Handle<Image>,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub camera_entity: Entity,
    pub window_entity: Entity,
    pub state: CaptureState,
}

/// Capture goes through two phases: first wait a few frames for the scene
/// + egui to have rendered into the image, then spawn a Readback and
/// wait for its observer to fire. The second phase is tracked so callers
/// (e.g. the script runner) can tell a capture is "still in flight" right
/// up until the PNG actually hits disk.
pub enum CaptureState {
    WaitingFrames(u32),
    AwaitingReadback,
}

/// Kick off a screenshot capture. Returns immediately; saving happens
/// asynchronously via the readback observer. Safe to call from any
/// Bevy system that has `Commands + Assets<Image> + ScreenshotState`.
pub fn begin_screenshot(
    path: PathBuf,
    commands: &mut Commands,
    images: &mut Assets<Image>,
    state: &mut ScreenshotState,
    primary_window_entity: Entity,
    window_width: u32,
    window_height: u32,
    main_camera_transform: &Transform,
    main_camera_projection: &OrthographicProjection,
) {
    if state.pending.is_some() {
        warn!("Screenshot already in progress; dropping request for {}", path.display());
        return;
    }

    // Swap-chain-srgb-matching format so the rendered result looks like
    // what's on screen. Bgra8UnormSrgb is the format macOS Metal surfaces
    // usually expose.
    let format = TextureFormat::Bgra8UnormSrgb;

    let size = Extent3d {
        width: window_width,
        height: window_height,
        depth_or_array_layers: 1,
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 255],
        format,
        RenderAssetUsages::all(),
    );
    image.texture_descriptor.usage = TextureUsages::COPY_SRC
        | TextureUsages::TEXTURE_BINDING
        | TextureUsages::RENDER_ATTACHMENT;
    let handle = images.add(image);

    // Secondary camera — renders the main scene into our image. Copies
    // transform and projection from the main camera so the composition
    // matches what the user sees. `ClearColor::Custom` matches the scene
    // background; ordering before the main camera doesn't matter since
    // they write to different targets.
    let camera_entity = commands
        .spawn((
            Camera2d,
            Camera {
                target: RenderTarget::Image(handle.clone()),
                clear_color: ClearColorConfig::Default,
                order: -1,
                ..default()
            },
            *main_camera_transform,
            main_camera_projection.clone(),
            ScreenshotCamera,
        ))
        .id();

    // Attach EguiRenderToImage to the primary window entity. bevy_egui's
    // setup system sees `Added<EguiRenderToImage>` on the next extract
    // phase and creates a second render graph node that renders this
    // entity's egui content to our image. LoadOp::Load preserves what the
    // camera drew underneath.
    commands.entity(primary_window_entity).insert(EguiRenderToImage {
        handle: handle.clone(),
        load_op: bevy::render::render_resource::LoadOp::Load,
    });

    state.pending = Some(PendingScreenshot {
        handle,
        path,
        width: window_width,
        height: window_height,
        camera_entity,
        window_entity: primary_window_entity,
        // Wait a handful of frames so (a) the Image asset gets uploaded to
        // the GPU, (b) Added<EguiRenderToImage> fires on extract and adds
        // the render node, (c) that node actually runs at least once with
        // the image in RenderAssets<GpuImage>. Empirically 2 is too few.
        state: CaptureState::WaitingFrames(6),
    });
}

/// Every frame, if a capture is in `WaitingFrames`, decrement it. When it
/// reaches zero, spawn a Readback and flip to `AwaitingReadback`. The
/// observer attached to the Readback entity is what finally clears
/// `state.pending` — so callers can treat `state.pending.is_some()` as
/// "capture still in flight" right up to disk write.
pub fn drive_screenshot_capture(
    mut commands: Commands,
    mut state: ResMut<ScreenshotState>,
) {
    let Some(pending) = state.pending.as_mut() else { return };

    match &mut pending.state {
        CaptureState::WaitingFrames(n) if *n > 0 => {
            *n -= 1;
            return;
        }
        CaptureState::AwaitingReadback => {
            // Observer will clear state.pending — nothing to do here.
            return;
        }
        CaptureState::WaitingFrames(_) => {
            // Countdown exhausted — fall through to spawn the Readback.
        }
    }

    let handle = pending.handle.clone();
    let path = pending.path.clone();
    let width = pending.width;
    let height = pending.height;
    let camera_entity = pending.camera_entity;
    let window_entity = pending.window_entity;

    commands
        .spawn(Readback::texture(handle))
        .observe(
            move |trigger: Trigger<ReadbackComplete>,
                  mut cmds: Commands,
                  mut state: ResMut<ScreenshotState>| {
                let bytes = &trigger.event().0;
                match save_bgra_as_png(&path, width, height, bytes) {
                    Ok(_) => info!("Screenshot saved: {}", path.display()),
                    Err(e) => error!("Failed to save screenshot: {}", e),
                }
                cmds.entity(camera_entity).try_despawn_recursive();
                cmds.entity(trigger.entity()).try_despawn_recursive();
                cmds.entity(window_entity).remove::<EguiRenderToImage>();
                state.pending = None;
            },
        );

    pending.state = CaptureState::AwaitingReadback;
}

fn save_bgra_as_png(
    path: &std::path::Path,
    width: u32,
    height: u32,
    bgra: &[u8],
) -> Result<(), String> {
    if bgra.len() as u32 != width * height * 4 {
        return Err(format!(
            "unexpected byte count: got {}, expected {}",
            bgra.len(),
            width * height * 4
        ));
    }
    // image::save_buffer expects RGBA8. Our texture format was BGRA
    // (matching the typical macOS swap chain format), so swap R↔B.
    let mut rgba = vec![0u8; bgra.len()];
    for (src, dst) in bgra.chunks_exact(4).zip(rgba.chunks_exact_mut(4)) {
        dst[0] = src[2];
        dst[1] = src[1];
        dst[2] = src[0];
        dst[3] = src[3];
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {}", e))?;
    }
    image::save_buffer(path, &rgba, width, height, image::ColorType::Rgba8)
        .map_err(|e| format!("{}", e))
}

/// Convenience for systems that want to capture the current frame.
/// Queries the main camera + primary window, allocates the image, and
/// begins the capture. Returns true if a capture was started.
pub fn capture_now(
    path: PathBuf,
    commands: &mut Commands,
    images: &mut Assets<Image>,
    state: &mut ScreenshotState,
    main_camera: &Query<(&Transform, &OrthographicProjection), With<MainCamera>>,
    primary_window: &Query<(Entity, &Window), With<PrimaryWindow>>,
) -> bool {
    let Ok((window_entity, window)) = primary_window.get_single() else {
        error!("No primary window; can't screenshot");
        return false;
    };
    let Ok((cam_transform, cam_projection)) = main_camera.get_single() else {
        error!("No main camera; can't screenshot");
        return false;
    };
    begin_screenshot(
        path,
        commands,
        images,
        state,
        window_entity,
        window.physical_width(),
        window.physical_height(),
        cam_transform,
        cam_projection,
    );
    true
}
