use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use clauvolution_core::*;
use clauvolution_genome::Genome;
use clauvolution_phylogeny::{PhyloTree, WorldChronicle};
use clauvolution_world::TileMap;

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
    mut event_writer: EventWriter<WorldEventRequest>,
    bloom: Res<BloomEffects>,
    selected: Res<SelectedOrganism>,
    organisms: Query<(&Energy, &Health, &BodySize, &Genome, &SpeciesId, &Position, &Age, &Generation, &Signal, &GroupSize, &ParentInfo), With<Organism>>,
    tile_map: Option<Res<TileMap>>,
    config: Res<SimConfig>,
    phylo: Res<PhyloTree>,
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
                    inspect_tab(ui, &selected, &organisms, tile_map.as_deref(), &config, &phylo);
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
                    events_tab(ui, &mut event_writer, &bloom);
                }
                RightTab::Help => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        help_tab(ui);
                    });
                }
            }
        });
}

fn inspect_tab(
    ui: &mut egui::Ui,
    selected: &SelectedOrganism,
    organisms: &Query<(&Energy, &Health, &BodySize, &Genome, &SpeciesId, &Position, &Age, &Generation, &Signal, &GroupSize, &ParentInfo), With<Organism>>,
    tile_map: Option<&TileMap>,
    config: &SimConfig,
    phylo: &PhyloTree,
) {
    let Some(entity) = selected.entity else {
        ui.heading("Inspect");
        ui.label("Click an organism to inspect it.");
        return;
    };

    let Ok((energy, health, body_size, genome, species, pos, age, generation, signal, group_size, parent_info)) = organisms.get(entity) else {
        ui.heading("Inspect");
        ui.colored_label(egui::Color32::LIGHT_RED, "Selected organism died.");
        return;
    };

    let species_name = phylo.nodes.get(&species.0)
        .map(|n| n.name.as_str())
        .unwrap_or("Unknown");
    let parent_name = parent_info.parent_species_id
        .and_then(|pid| phylo.nodes.get(&pid))
        .map(|n| n.name.as_str())
        .unwrap_or("(origin)");

    let terrain_name = tile_map
        .map(|tm| format!("{:?}", tm.tile_at_pos(pos.0).terrain))
        .unwrap_or_else(|| "?".to_string());

    let strategy = if genome.photosynthesis_rate > 0.2 && genome.has_photo_surface() {
        ("Photosynthesizer", egui::Color32::from_rgb(120, 200, 100))
    } else if genome.claw_power() > 0.5 {
        ("Predator", egui::Color32::from_rgb(220, 100, 100))
    } else {
        ("Forager", egui::Color32::from_rgb(230, 230, 230))
    };

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading(species_name);
            ui.colored_label(strategy.1, strategy.0);
        });
        ui.separator();

        egui::Grid::new("inspect_overview").num_columns(2).striped(true).show(ui, |ui| {
            ui.label("Parent species");
            ui.label(parent_name);
            ui.end_row();

            ui.label("Generation");
            ui.label(generation.0.to_string());
            ui.end_row();

            ui.label("Age (ticks)");
            ui.label(age.0.to_string());
            ui.end_row();

            ui.label("Energy");
            let ratio = (energy.0 / config.max_organism_energy).clamp(0.0, 1.0);
            ui.add(egui::ProgressBar::new(ratio)
                .text(format!("{:.1} / {:.0}", energy.0, config.max_organism_energy)));
            ui.end_row();

            ui.label("Health");
            ui.add(egui::ProgressBar::new(health.0.clamp(0.0, 1.0))
                .text(format!("{:.0}%", health.0 * 100.0)));
            ui.end_row();

            ui.label("Position");
            ui.label(format!("({:.0}, {:.0})", pos.0.x, pos.0.y));
            ui.end_row();

            ui.label("Terrain");
            ui.label(terrain_name);
            ui.end_row();

            ui.label("Group nearby");
            ui.label(group_size.0.to_string());
            ui.end_row();
        });

        ui.add_space(8.0);
        egui::CollapsingHeader::new("Body traits").default_open(true).show(ui, |ui| {
            egui::Grid::new("inspect_body").num_columns(2).striped(true).show(ui, |ui| {
                ui.label("Size");
                ui.label(format!("{:.2}", body_size.0));
                ui.end_row();

                ui.label("Speed factor");
                ui.label(format!("{:.2}", genome.speed_factor));
                ui.end_row();

                ui.label("Sense range");
                ui.label(format!("{:.1}", genome.effective_sense_range()));
                ui.end_row();

                ui.label("Aquatic adaptation");
                ui.label(format!("{:.0}%", genome.aquatic_adaptation * 100.0));
                ui.end_row();

                ui.label("Photosynthesis rate");
                ui.label(format!("{:.0}%", genome.photosynthesis_rate * 100.0));
                ui.end_row();

                ui.label("Attack (claw power)");
                ui.label(format!("{:.2}", genome.claw_power()));
                ui.end_row();

                ui.label("Armor value");
                ui.label(format!("{:.2}", genome.armor_value()));
                ui.end_row();

                ui.label("Signal");
                ui.label(format!("{:.2}", signal.0));
                ui.end_row();
            });
        });

        egui::CollapsingHeader::new("Body segments").show(ui, |ui| {
            for seg in &genome.body_segments {
                ui.label(format!("• {:?} (size {:.2})", seg.segment_type, seg.size));
            }
        });

        egui::CollapsingHeader::new("Brain").show(ui, |ui| {
            ui.label(format!("Neurons: {}", genome.neurons.len()));
            ui.label(format!("Connections: {} enabled / {} total",
                genome.connections.iter().filter(|c| c.enabled).count(),
                genome.connections.len()));
        });
    });
}

fn events_tab(
    ui: &mut egui::Ui,
    events: &mut EventWriter<WorldEventRequest>,
    bloom: &BloomEffects,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Mass extinction");
        ui.label("Immediate, destructive. 2s global cooldown between events.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("☄  Asteroid  (X)").on_hover_text("Kill 70% of organisms randomly").clicked() {
                events.send(WorldEventRequest::Asteroid);
            }
            if ui.button("❄  Ice age  (I)").on_hover_text("Halve temperature, reduce moisture").clicked() {
                events.send(WorldEventRequest::IceAge);
            }
            if ui.button("🌋 Volcano  (V)").on_hover_text("Kill zone + nutrient boost").clicked() {
                events.send(WorldEventRequest::Volcano);
            }
        });

        ui.add_space(12.0);
        ui.separator();
        ui.heading("Bloom events");
        ui.label("Positive stimuli. Boom now, crash later.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("☀  Solar bloom  (B)").on_hover_text("Double light for 30 seconds").clicked() {
                events.send(WorldEventRequest::SolarBloom);
            }
            if ui.button("🌧 Nutrient rain  (N)").on_hover_text("Massive food burst across the world").clicked() {
                events.send(WorldEventRequest::NutrientRain);
            }
            if ui.button("✦ Cambrian spark  (J)").on_hover_text("Triple mutation rate for 30 seconds").clicked() {
                events.send(WorldEventRequest::CambrianSpark);
            }
        });

        // Active effects readout
        ui.add_space(12.0);
        ui.separator();
        ui.heading("Active effects");
        if bloom.solar_ticks == 0 && bloom.mutation_ticks == 0 {
            ui.label("(none)");
        } else {
            if bloom.solar_ticks > 0 {
                let secs = bloom.solar_ticks / 30;
                ui.label(format!("Solar bloom: {}s remaining (light × {:.1})", secs, bloom.solar_bloom));
            }
            if bloom.mutation_ticks > 0 {
                let secs = bloom.mutation_ticks / 30;
                ui.label(format!("Cambrian spark: {}s remaining (mutation × {:.1})", secs, bloom.mutation_boost));
            }
        }

        ui.add_space(12.0);
        ui.separator();
        ui.heading("Persistence");
        if ui.button("💾 Save world  (F5)").on_hover_text("Save to sessions/<name>/save.json").clicked() {
            events.send(WorldEventRequest::Save);
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
