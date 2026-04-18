//! CLI-driven automation for reproducible captures (README screenshots,
//! demo reels, regression checks).
//!
//! Pass `--script PATH` to load a JSON file containing a list of actions
//! keyed to virtual-time seconds. At each trigger time, the runner applies
//! the action: switch tab, set zoom, move the camera, take a screenshot,
//! exit. Screenshots go through `clauvolution_render::capture_window_screenshot`
//! which shells out to macOS `screencapture` so egui overlays are included.
//!
//! Example script (`tours/demo.json`):
//! ```json
//! {
//!   "actions": [
//!     { "at_seconds": 3.0, "kind": "set_speed", "multiplier": 4.0 },
//!     { "at_seconds": 6.0, "kind": "set_tab",   "tab": "graphs"   },
//!     { "at_seconds": 8.0, "kind": "screenshot", "name": "01_graphs" },
//!     { "at_seconds": 9.0, "kind": "exit" }
//!   ]
//! }
//! ```

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use clauvolution_core::{Session, SimSpeed};
use clauvolution_render::{begin_screenshot, MainCamera, ScreenshotState};
use clauvolution_ui::{RightTab, UiState};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Script {
    pub actions: Vec<ScriptAction>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScriptAction {
    pub at_seconds: f32,
    #[serde(flatten)]
    pub kind: ScriptActionKind,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScriptActionKind {
    SetTab { tab: TabName },
    SetSpeed { multiplier: f32 },
    SetZoom { zoom: f32 },
    CameraAt { x: f32, y: f32 },
    Screenshot { name: String },
    Exit,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TabName {
    Inspect,
    Phylo,
    Graphs,
    Chronicle,
    Events,
    Help,
}

impl From<TabName> for RightTab {
    fn from(t: TabName) -> RightTab {
        match t {
            TabName::Inspect => RightTab::Inspect,
            TabName::Phylo => RightTab::Phylo,
            TabName::Graphs => RightTab::Graphs,
            TabName::Chronicle => RightTab::Chronicle,
            TabName::Events => RightTab::Events,
            TabName::Help => RightTab::Help,
        }
    }
}

#[derive(Resource)]
pub struct ScriptState {
    pub script: Script,
    pub next_action: usize,
}

pub fn load_script(path: &Path) -> Result<Script, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("reading script {:?}: {}", path, e))?;
    serde_json::from_str(&content).map_err(|e| format!("parsing script {:?}: {}", path, e))
}

/// Per-frame check: fire every action whose trigger time has passed.
pub fn script_runner_system(
    mut state: ResMut<ScriptState>,
    time: Res<Time<Virtual>>,
    mut ui_state: ResMut<UiState>,
    mut sim_speed: ResMut<SimSpeed>,
    mut camera: Query<(&mut Transform, &mut OrthographicProjection), With<MainCamera>>,
    primary_window: Query<(Entity, &Window), With<PrimaryWindow>>,
    session: Res<Session>,
    mut images: ResMut<Assets<Image>>,
    mut screenshot_state: ResMut<ScreenshotState>,
    mut exit: EventWriter<AppExit>,
    mut commands: Commands,
) {
    let elapsed = time.elapsed_secs();
    while let Some(action) = state.script.actions.get(state.next_action) {
        if elapsed < action.at_seconds {
            break;
        }
        info!("Script @{:.2}s: {:?}", action.at_seconds, action.kind);
        match &action.kind {
            ScriptActionKind::SetTab { tab } => {
                ui_state.right_tab = tab.clone().into();
            }
            ScriptActionKind::SetSpeed { multiplier } => {
                sim_speed.multiplier = *multiplier;
                sim_speed.paused = false;
            }
            ScriptActionKind::SetZoom { zoom } => {
                if let Ok((_, mut proj)) = camera.get_single_mut() {
                    proj.scale = *zoom;
                }
            }
            ScriptActionKind::CameraAt { x, y } => {
                if let Ok((mut t, _)) = camera.get_single_mut() {
                    t.translation.x = *x;
                    t.translation.y = *y;
                }
            }
            ScriptActionKind::Screenshot { name } => {
                let path = session.screenshot_path(name);
                let Ok((cam_transform, cam_projection)) = camera.get_single() else {
                    error!("No main camera; can't screenshot");
                    state.next_action += 1;
                    continue;
                };
                let Ok((window_entity, window)) = primary_window.get_single() else {
                    error!("No primary window; can't screenshot");
                    state.next_action += 1;
                    continue;
                };
                let cam_transform = *cam_transform;
                let cam_projection = cam_projection.clone();
                let window_width = window.physical_width();
                let window_height = window.physical_height();
                begin_screenshot(
                    path,
                    &mut commands,
                    &mut images,
                    &mut screenshot_state,
                    window_entity,
                    window_width,
                    window_height,
                    &cam_transform,
                    &cam_projection,
                );
            }
            ScriptActionKind::Exit => {
                exit.send(AppExit::Success);
            }
        }
        state.next_action += 1;
    }
}
