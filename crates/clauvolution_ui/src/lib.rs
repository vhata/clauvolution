use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use clauvolution_core::*;
use clauvolution_phylogeny::{WorldChronicle};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .init_resource::<UiState>()
            .add_systems(Update, (header_bar_system, right_panel_system));
    }
}

/// Which tab the right panel is showing
#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum RightTab {
    #[default]
    Inspect,
    Phylo,
    Graphs,
    Chronicle,
    Events,
    Help,
}

#[derive(Resource)]
pub struct UiState {
    pub right_tab: RightTab,
    pub egui_wants_keyboard: bool,
    pub chronicle_hide_seasons: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            right_tab: RightTab::default(),
            egui_wants_keyboard: false,
            chronicle_hide_seasons: false,
        }
    }
}

fn help_tab(ui: &mut egui::Ui) {
    ui.heading("Clauvolution");
    ui.label("Every dot is a living organism with its own evolved brain. They sense the world, decide what to do, and pass their genes to offspring. No behaviour is programmed — everything emerges from evolution.");
    ui.separator();

    egui::CollapsingHeader::new("Organism colours").default_open(true).show(ui, |ui| {
        ui.label("• Bright circles with outlines — active organisms (foragers, predators)");
        ui.label("• Faded circles without outlines — photosynthesizers (plants)");
        ui.label("• Colour varies by species — related organisms share colours");
        ui.label("• Red tint — predator (has claws)");
        ui.label("• Green tint — photosynthesizer");
    });

    egui::CollapsingHeader::new("Body parts").show(ui, |ui| {
        ui.label("Torso — main body, everyone has one");
        ui.label("Limb — helps move on land");
        ui.label("Fin — helps swim in water");
        ui.label("Eye — extends sensing range");
        ui.label("Mouth — improves food eating efficiency");
        ui.label("PhotoSurface — absorbs light for energy");
        ui.label("Claw — weapon, used to attack");
        ui.label("ArmorPlate — defence, reduces damage");
    });

    egui::CollapsingHeader::new("Controls").default_open(true).show(ui, |ui| {
        egui::Grid::new("controls_grid").striped(true).show(ui, |ui| {
            for (key, desc) in [
                ("Space", "pause / unpause"),
                ("[  ]", "slow down / speed up"),
                ("Scroll", "zoom in / out"),
                ("Click", "inspect organism"),
                ("Right-drag", "pan camera"),
                ("WASD", "pan camera"),
                ("M", "toggle minimap heatmap"),
                ("F5", "save world"),
                ("S", "take screenshot"),
            ] {
                ui.monospace(key);
                ui.label(desc);
                ui.end_row();
            }
        });
    });

    egui::CollapsingHeader::new("Mass extinction events").show(ui, |ui| {
        ui.label("X — asteroid impact (kills 70%)");
        ui.label("I — ice age (halves temperature)");
        ui.label("V — volcano (kills area, boosts nutrients)");
    });

    egui::CollapsingHeader::new("Bloom events").show(ui, |ui| {
        ui.label("B — solar bloom (double light for 30s)");
        ui.label("N — nutrient rain (massive food burst)");
        ui.label("J — Cambrian spark (triple mutation for 30s)");
    });
}

/// Top bar: season, population, species, speed/pause — always visible
fn header_bar_system(
    mut contexts: EguiContexts,
    stats: Res<SimStats>,
    season: Res<Season>,
    speed: Res<SimSpeed>,
    mut ui_state: ResMut<UiState>,
) {
    let ctx = contexts.ctx_mut();

    egui::TopBottomPanel::top("header_bar")
        .exact_height(28.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let season_name = match season.name() {
                    SeasonName::Spring => "Spring",
                    SeasonName::Summer => "Summer",
                    SeasonName::Autumn => "Autumn",
                    SeasonName::Winter => "Winter",
                };
                let light_pct = (season.light_multiplier() * 100.0) as u32;

                let speed_str = if speed.paused {
                    "PAUSED".to_string()
                } else if speed.multiplier == 1.0 {
                    "1x".to_string()
                } else if speed.multiplier < 1.0 {
                    format!("{:.2}x", speed.multiplier)
                } else {
                    format!("{}x", speed.multiplier as u32)
                };

                ui.label(format!("{} (light {}%)", season_name, light_pct));
                ui.separator();
                ui.label(format!("Pop: {}", stats.total_organisms));
                ui.separator();
                ui.label(format!("Species: {}", stats.species_count));
                ui.separator();
                ui.label(format!("Gen: {}", stats.max_generation));
                ui.separator();
                ui.label(format!("Speed: {}", speed_str));
            });
        });

    // Track whether egui is consuming keyboard so hotkeys can gate themselves
    ui_state.egui_wants_keyboard = ctx.wants_keyboard_input();
}

/// Right side panel with tabs — one content area switched via tab bar
fn right_panel_system(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    chronicle: Res<WorldChronicle>,
) {
    let ctx = contexts.ctx_mut();

    egui::SidePanel::right("right_panel")
        .resizable(true)
        .default_width(380.0)
        .min_width(280.0)
        .show(ctx, |ui| {
            // Tab strip
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut ui_state.right_tab, RightTab::Inspect, "Inspect");
                ui.selectable_value(&mut ui_state.right_tab, RightTab::Phylo, "Phylo");
                ui.selectable_value(&mut ui_state.right_tab, RightTab::Graphs, "Graphs");
                ui.selectable_value(&mut ui_state.right_tab, RightTab::Chronicle, "Chronicle");
                ui.selectable_value(&mut ui_state.right_tab, RightTab::Events, "Events");
                ui.selectable_value(&mut ui_state.right_tab, RightTab::Help, "Help");
            });
            ui.separator();

            match ui_state.right_tab {
                RightTab::Inspect => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading("Inspect");
                        ui.label("(migrating — old panel still active)");
                    });
                }
                RightTab::Phylo => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading("Phylogenetic tree");
                        ui.label("(migrating — old panel still active)");
                    });
                }
                RightTab::Graphs => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading("Population graphs");
                        ui.label("(migrating — old panel still active)");
                    });
                }
                RightTab::Chronicle => {
                    chronicle_tab(ui, &chronicle, &mut ui_state.chronicle_hide_seasons);
                }
                RightTab::Events => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading("Events");
                        ui.label("(migrating — old panel still active)");
                    });
                }
                RightTab::Help => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        help_tab(ui);
                    });
                }
            }
        });
}

fn chronicle_tab(ui: &mut egui::Ui, chronicle: &WorldChronicle, hide_seasons: &mut bool) {
    ui.horizontal(|ui| {
        ui.heading("Chronicle");
        ui.add_space(8.0);
        ui.checkbox(hide_seasons, "Hide seasons");
    });
    ui.separator();

    let hide = *hide_seasons;
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for entry in &chronicle.entries {
                // Simple filter: skip season change entries if hidden
                if hide {
                    let t = &entry.text;
                    if t.starts_with("Spring") || t.starts_with("Summer")
                        || t.starts_with("Autumn") || t.starts_with("Winter") {
                        continue;
                    }
                }
                let time_secs = entry.tick / 30;
                let time_str = if time_secs >= 60 {
                    format!("{}m{:02}s", time_secs / 60, time_secs % 60)
                } else {
                    format!("{:3}s", time_secs)
                };
                ui.horizontal(|ui| {
                    ui.monospace(format!("[{}]", time_str));
                    ui.label(&entry.text);
                });
            }
        });
}
