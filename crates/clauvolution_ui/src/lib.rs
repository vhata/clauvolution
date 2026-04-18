// Bevy systems have complex Query signatures by design — clippy's
// type_complexity and too_many_arguments lints misfire constantly here.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use clauvolution_brain::Brain;
use clauvolution_core::*;
use clauvolution_genome::{Genome, NUM_INPUTS, NUM_OUTPUTS};
use clauvolution_phylogeny::{PhyloTree, PhyloNode, SpeciesStrategy, WorldChronicle};
use clauvolution_world::TileMap;
use egui_plot::{Line, Plot, PlotPoints, Legend};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .init_resource::<UiState>()
            // Read egui's capture state from the PREVIOUS frame's panel layout
            // so Update/PostUpdate systems this frame can gate on it.
            // egui carries layout state across frames (immediate mode builds
            // rects during show() and they remain queryable until next show()).
            .add_systems(PreUpdate, update_input_capture_system)
            .add_systems(Update, (header_bar_system, right_panel_system).chain());
    }
}

fn update_input_capture_system(
    mut contexts: EguiContexts,
    mut input_state: ResMut<UiInputState>,
) {
    let ctx = contexts.ctx_mut();
    input_state.wants_keyboard = ctx.wants_keyboard_input();
    input_state.pointer_over_ui = ctx.is_pointer_over_area()
        || ctx.wants_pointer_input()
        || ctx.is_using_pointer();
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
                ("F", "focus camera on selected organism"),
                (", / .", "prev / next living member of same species"),
                ("R", "select a random living organism"),
                ("Right-drag", "pan camera"),
                ("WASD", "pan camera"),
                ("M", "toggle minimap heatmap"),
                ("T", "toggle trail for selected organism"),
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
    bloom: Res<BloomEffects>,
    infected: Query<(), (With<Organism>, With<Infection>)>,
    tick: Res<TickCounter>,
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

                // Sim time in mm:ss — lets you see at a glance how long this run has been going
                let secs = tick.0 / 30;
                let time_str = if secs >= 60 {
                    format!("{}m{:02}s", secs / 60, secs % 60)
                } else {
                    format!("{}s", secs)
                };
                ui.label(format!("⏱ {}", time_str));
                ui.separator();
                ui.label(format!("{} (light {}%)", season_name, light_pct));
                ui.separator();
                ui.label(format!("Pop: {}", stats.total_organisms));
                ui.separator();
                ui.label(format!("Species: {}", stats.species_count));
                ui.separator();
                ui.label(format!("Gen: {}", stats.max_generation));
                ui.separator();
                ui.label(format!("Speed: {}", speed_str));

                // Infection count — only shown when disease is circulating, coloured to match halos.
                let infection_count = infected.iter().count();
                if infection_count > 0 {
                    ui.separator();
                    ui.colored_label(
                        egui::Color32::from_rgb(200, 120, 230),
                        format!("⚠ {} sick", infection_count),
                    );
                }

                // Active bloom effects — coloured so they stand out, showing seconds remaining.
                // Only appear while ticking down, never intrude when nothing's active.
                if bloom.solar_ticks > 0 {
                    ui.separator();
                    let secs = bloom.solar_ticks / 30;
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 210, 80),
                        format!("☀ solar {}s", secs),
                    );
                }
                if bloom.mutation_ticks > 0 {
                    ui.separator();
                    let secs = bloom.mutation_ticks / 30;
                    ui.colored_label(
                        egui::Color32::from_rgb(200, 140, 255),
                        format!("✦ spark {}s", secs),
                    );
                }
            });
        });

    ui_state.egui_wants_keyboard = ctx.wants_keyboard_input();
}

/// Right side panel with tabs — one content area switched via tab bar
fn right_panel_system(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    chronicle: Res<WorldChronicle>,
    mut event_writer: EventWriter<WorldEventRequest>,
    bloom: Res<BloomEffects>,
    mut selected: ResMut<SelectedOrganism>,
    organisms: Query<(&Energy, &Health, &BodySize, &Genome, &SpeciesId, &Position, &Age, &Generation, &Signal, &GroupSize, &ParentInfo, Option<&Infection>, &Brain, &BrainActivations), With<Organism>>,
    species_members: Query<(Entity, &SpeciesId), With<Organism>>,
    tile_map: Option<Res<TileMap>>,
    config: Res<SimConfig>,
    phylo: Res<PhyloTree>,
    history: Res<PopulationHistory>,
    tick: Res<TickCounter>,
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
                    inspect_tab(ui, &mut selected, &organisms, &species_members, tile_map.as_deref(), &config, &phylo);
                }
                RightTab::Phylo => {
                    phylo_tab(ui, &phylo, tick.0, &mut selected, &species_members);
                }
                RightTab::Graphs => {
                    graphs_tab(ui, &history);
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

fn phylo_tab(
    ui: &mut egui::Ui,
    phylo: &PhyloTree,
    current_tick: u64,
    selected: &mut SelectedOrganism,
    species_members: &Query<(Entity, &SpeciesId), With<Organism>>,
) {
    if phylo.nodes.is_empty() {
        ui.heading("Phylogenetic tree");
        ui.label("No species yet.");
        return;
    }

    // If a species name gets clicked below, we'll remember its id and resolve
    // the first living-member lookup after the tree renders.
    let mut clicked_species: Option<u64> = None;

    let living: Vec<&PhyloNode> = phylo.nodes.values()
        .filter(|n| n.extinct_tick.is_none() && n.current_population > 0)
        .collect();

    let total_living = living.len();
    let total_ever = phylo.nodes.len();
    let total_extinct = total_ever - total_living;

    ui.horizontal(|ui| {
        ui.heading("Phylogeny");
        ui.small(format!("{} alive · {} extinct · {} total", total_living, total_extinct, total_ever));
    });
    ui.separator();

    // Group living species by lineage root (walk parent chain up to 10 steps)
    use std::collections::HashMap;
    let mut lineages: HashMap<u64, Vec<&PhyloNode>> = HashMap::new();
    for node in &living {
        let mut root = node.species_id;
        let mut current = node.species_id;
        for _ in 0..10 {
            if let Some(n) = phylo.nodes.get(&current) {
                if let Some(pid) = n.parent_id {
                    root = pid;
                    current = pid;
                } else { break; }
            } else { break; }
        }
        lineages.entry(root).or_default().push(node);
    }

    // Sort lineages by total population (largest first)
    let mut sorted_lineages: Vec<(u64, Vec<&PhyloNode>)> = lineages.into_iter().collect();
    sorted_lineages.sort_by(|a, b| {
        let pop_a: u32 = a.1.iter().map(|n| n.current_population).sum();
        let pop_b: u32 = b.1.iter().map(|n| n.current_population).sum();
        pop_b.cmp(&pop_a).then(a.0.cmp(&b.0))
    });

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for (_root_id, mut members) in sorted_lineages {
            members.sort_by(|a, b| b.current_population.cmp(&a.current_population).then(a.species_id.cmp(&b.species_id)));

            let first = members[0];
            let lineage_total: u32 = members.iter().map(|n| n.current_population).sum();

            let header_text = if members.len() == 1 {
                format!("{} — pop {}", first.name, first.current_population)
            } else {
                format!("{} — {} species, {} total", first.name, members.len(), lineage_total)
            };

            let id = egui::Id::new(("lineage", first.species_id));
            egui::CollapsingHeader::new(header_text)
                .id_salt(id)
                .default_open(true)
                .show(ui, |ui| {
                    for node in &members {
                        if species_row(ui, node, current_tick) {
                            clicked_species = Some(node.species_id);
                        }
                    }
                });
        }

        // Recently extinct
        let mut recently_extinct: Vec<&PhyloNode> = phylo.nodes.values()
            .filter(|n| n.extinct_tick.is_some())
            .collect();
        recently_extinct.sort_by(|a, b| b.extinct_tick.cmp(&a.extinct_tick).then(a.species_id.cmp(&b.species_id)));

        if !recently_extinct.is_empty() {
            ui.add_space(8.0);
            ui.separator();
            egui::CollapsingHeader::new(format!("Recently extinct ({})", recently_extinct.len().min(10))).show(ui, |ui| {
                for node in recently_extinct.iter().take(10) {
                    let age_secs = current_tick.saturating_sub(node.extinct_tick.unwrap_or(0)) / 30;
                    let lived = node.extinct_tick.unwrap_or(0).saturating_sub(node.born_tick) / 30;
                    ui.horizontal(|ui| {
                        ui.small(format!("✝ {}", node.name));
                        ui.small(format!("peak {} · lived {}s · died {}s ago", node.peak_population, lived, age_secs));
                    });
                }
            });
        }
    });

    // Resolve a click on a species name into a selection of a living member.
    // First match wins — arbitrary but deterministic given query ordering.
    if let Some(sp_id) = clicked_species {
        for (entity, species) in species_members.iter() {
            if species.0 == sp_id {
                selected.entity = Some(entity);
                break;
            }
        }
    }
}

/// Returns true if the species name was clicked (caller resolves the selection).
fn species_row(ui: &mut egui::Ui, node: &PhyloNode, current_tick: u64) -> bool {
    let age_secs = current_tick.saturating_sub(node.born_tick) / 30;
    let age_str = if age_secs >= 60 {
        format!("{}m{:02}s", age_secs / 60, age_secs % 60)
    } else {
        format!("{}s", age_secs)
    };

    let strategy_badge = match node.strategy {
        SpeciesStrategy::Photosynthesizer => ("🌱", egui::Color32::from_rgb(120, 200, 100)),
        SpeciesStrategy::Predator => ("🦷", egui::Color32::from_rgb(220, 100, 100)),
        SpeciesStrategy::Forager => ("🍂", egui::Color32::from_rgb(220, 200, 120)),
    };

    let declining = node.current_population < node.peak_population / 2;

    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.colored_label(strategy_badge.1, strategy_badge.0);
        // Clickable name — selects a living member of this species
        if ui.link(&node.name).on_hover_text("Click to select a living member").clicked() {
            clicked = true;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if declining {
                ui.small(egui::RichText::new("↓").color(egui::Color32::LIGHT_RED));
            }
            ui.small(age_str);
            ui.separator();
            ui.small(format!("pop {}", node.current_population));
        });
    });
    clicked
}

fn inspect_tab(
    ui: &mut egui::Ui,
    selected: &mut SelectedOrganism,
    organisms: &Query<(&Energy, &Health, &BodySize, &Genome, &SpeciesId, &Position, &Age, &Generation, &Signal, &GroupSize, &ParentInfo, Option<&Infection>, &Brain, &BrainActivations), With<Organism>>,
    species_members: &Query<(Entity, &SpeciesId), With<Organism>>,
    tile_map: Option<&TileMap>,
    config: &SimConfig,
    phylo: &PhyloTree,
) {
    let Some(entity) = selected.entity else {
        ui.heading("Inspect");
        ui.label("Click an organism to inspect it.");
        return;
    };

    let Ok((energy, health, body_size, genome, species, pos, age, generation, signal, group_size, parent_info, infection, brain, activations)) = organisms.get(entity) else {
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
            if let Some(inf) = infection {
                ui.colored_label(
                    egui::Color32::from_rgb(180, 80, 220),
                    format!("⚠ INFECTED ({}s, sev {:.0}%)",
                        inf.ticks_remaining / 30, inf.severity * 100.0),
                );
            }
        });
        ui.separator();

        // Parent species clickable if it has living members — lets you walk
        // a lineage backward by clicking parent → member → parent → member ...
        let parent_pid = parent_info.parent_species_id;
        let parent_living_member: Option<Entity> = parent_pid.and_then(|pid| {
            species_members.iter()
                .find(|(_, sp)| sp.0 == pid)
                .map(|(e, _)| e)
        });

        egui::Grid::new("inspect_overview").num_columns(2).striped(true).show(ui, |ui| {
            ui.label("Parent species");
            if let Some(pid_entity) = parent_living_member {
                if ui.link(parent_name)
                    .on_hover_text("Click to select a living member of the parent species")
                    .clicked()
                {
                    selected.entity = Some(pid_entity);
                }
            } else if parent_pid.is_some() {
                ui.weak(format!("{} (extinct)", parent_name));
            } else {
                ui.weak(parent_name);  // "(origin)"
            }
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

                ui.label("Disease resistance");
                ui.label(format!("{:.0}%", genome.disease_resistance * 100.0));
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

        egui::CollapsingHeader::new("Brain").default_open(true).show(ui, |ui| {
            ui.label(format!("Neurons: {} ({} hidden)",
                genome.neurons.len(),
                genome.neurons.len().saturating_sub(NUM_INPUTS + NUM_OUTPUTS)));
            ui.label(format!("Connections: {} enabled / {} total",
                genome.connections.iter().filter(|c| c.enabled).count(),
                genome.connections.len()));
            ui.add_space(4.0);
            draw_brain_viz(ui, genome, brain, activations);
            ui.small("Inputs left, outputs right, hidden middle. Node colour = activation, line colour = weight sign, line alpha = signal this tick.");
        });
    });
}

const BRAIN_INPUT_LABELS: [&str; NUM_INPUTS] = [
    "energy", "food dx", "food dy", "food near",
    "org dx", "org dy", "org near", "org size",
    "in water", "nutrients", "light", "aquatic",
    "health", "same sp",
    "mem 0", "mem 1", "mem 2",
    "photo hint", "org signal",
    "group sz", "group sig", "bias",
];

const BRAIN_OUTPUT_LABELS: [&str; NUM_OUTPUTS] = [
    "move x", "move y", "eat", "reproduce",
    "attack", "signal",
    "mem out 0", "mem out 1", "mem out 2",
];

fn brain_node_color(activation: f32) -> egui::Color32 {
    // Activations mostly land in [-1, 1] thanks to sigmoid/tanh, but linear
    // activations can exceed that. Clamp for visualisation.
    let a = activation.clamp(-1.5, 1.5);
    let mag = (a.abs()).min(1.0);
    let base: u8 = 55;
    let intensity = (mag * 200.0) as u8;
    if a >= 0.0 {
        egui::Color32::from_rgb(base, base.saturating_add(intensity), base.saturating_add(intensity / 2))
    } else {
        egui::Color32::from_rgb(base.saturating_add(intensity), base, base.saturating_add(intensity / 3))
    }
}

fn draw_brain_viz(
    ui: &mut egui::Ui,
    genome: &Genome,
    brain: &Brain,
    activations: &BrainActivations,
) {
    use egui::{Color32, Pos2, Sense, Stroke, Vec2 as EVec2};
    use std::collections::{HashMap, HashSet};

    // Clamp width so the painter can never exceed the current UI's clip —
    // prevents the heatmap drifting outside the side panel on wider monitors.
    let width = ui.available_width().clamp(200.0, 360.0);
    let height = 300.0_f32;
    let (response, painter) = ui.allocate_painter(EVec2::new(width, height), Sense::hover());
    let rect = response.rect;
    // Force the painter's clip rect to the allocated area so drawing stays
    // inside the widget regardless of the parent UI's clip state.
    let painter = painter.with_clip_rect(rect);

    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 18, 24));

    let margin = 8.0;
    let label_width = 62.0;
    let node_radius = 4.0;

    let input_set: HashSet<u64> = brain.input_ids().iter().copied().collect();
    let output_set: HashSet<u64> = brain.output_ids().iter().copied().collect();
    let hidden_ids: Vec<u64> = genome
        .neurons
        .iter()
        .filter(|n| !input_set.contains(&n.id) && !output_set.contains(&n.id))
        .map(|n| n.id)
        .collect();

    let input_x = rect.left() + margin + label_width;
    let output_x = rect.right() - margin - label_width;
    let hidden_left = input_x + 24.0;
    let hidden_right = output_x - 24.0;
    let top = rect.top() + margin;
    let bottom = rect.bottom() - margin;
    let col_height = bottom - top;

    let mut positions: HashMap<u64, Pos2> = HashMap::new();

    // Inputs: evenly spaced on the left
    let input_count = brain.input_ids().len().max(1);
    for (i, &id) in brain.input_ids().iter().enumerate() {
        let y = top + (i as f32 + 0.5) * col_height / input_count as f32;
        positions.insert(id, Pos2::new(input_x, y));
    }

    // Outputs: evenly spaced on the right
    let output_count = brain.output_ids().len().max(1);
    for (i, &id) in brain.output_ids().iter().enumerate() {
        let y = top + (i as f32 + 0.5) * col_height / output_count as f32;
        positions.insert(id, Pos2::new(output_x, y));
    }

    // Hidden: grid in the middle. Deterministic from insertion order so
    // the layout doesn't jitter frame-to-frame. NEAT creates hidden neurons
    // with monotonic IDs so iteration order is stable.
    if !hidden_ids.is_empty() {
        let cols = ((hidden_ids.len() as f32).sqrt().ceil() as usize).max(1);
        let rows = hidden_ids.len().div_ceil(cols).max(1);
        let col_w = (hidden_right - hidden_left) / cols as f32;
        let row_h = col_height / rows as f32;
        for (i, &id) in hidden_ids.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let x = hidden_left + (col as f32 + 0.5) * col_w;
            let y = top + (row as f32 + 0.5) * row_h;
            positions.insert(id, Pos2::new(x, y));
        }
    }

    // Connections first (behind nodes)
    for conn in &genome.connections {
        if !conn.enabled {
            continue;
        }
        let Some(&from_pos) = positions.get(&conn.from) else { continue };
        let Some(&to_pos) = positions.get(&conn.to) else { continue };

        let source_act = activations.values.get(&conn.from).copied().unwrap_or(0.0);
        let signal = (source_act * conn.weight).abs().min(2.0) / 2.0;
        let alpha = ((signal * 210.0) as u8).saturating_add(25);
        let thickness = conn.weight.abs().clamp(0.15, 2.5) * 0.9;

        let color = if conn.weight >= 0.0 {
            Color32::from_rgba_unmultiplied(90, 170, 220, alpha)
        } else {
            Color32::from_rgba_unmultiplied(220, 110, 90, alpha)
        };

        painter.line_segment([from_pos, to_pos], Stroke::new(thickness, color));
    }

    // Nodes
    for (id, &pos) in &positions {
        let act = activations.values.get(id).copied().unwrap_or(0.0);
        painter.circle_filled(pos, node_radius, brain_node_color(act));
        painter.circle_stroke(pos, node_radius, Stroke::new(0.8, Color32::from_rgb(90, 90, 100)));
    }

    // Input labels
    for (i, &id) in brain.input_ids().iter().enumerate() {
        let Some(&pos) = positions.get(&id) else { continue };
        let label = BRAIN_INPUT_LABELS.get(i).copied().unwrap_or("?");
        painter.text(
            Pos2::new(pos.x - node_radius - 3.0, pos.y),
            egui::Align2::RIGHT_CENTER,
            label,
            egui::FontId::proportional(9.5),
            Color32::from_rgb(170, 170, 180),
        );
    }

    // Output labels
    for (i, &id) in brain.output_ids().iter().enumerate() {
        let Some(&pos) = positions.get(&id) else { continue };
        let label = BRAIN_OUTPUT_LABELS.get(i).copied().unwrap_or("?");
        painter.text(
            Pos2::new(pos.x + node_radius + 3.0, pos.y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(9.5),
            Color32::from_rgb(170, 170, 180),
        );
    }
}

fn graphs_tab(ui: &mut egui::Ui, history: &PopulationHistory) {
    if history.snapshots.len() < 2 {
        ui.heading("Graphs");
        ui.label("Collecting data…");
        return;
    }

    let snaps = &history.snapshots;
    let latest = snaps.last().unwrap();

    ui.horizontal(|ui| {
        ui.heading("Graphs");
        ui.add_space(8.0);
        ui.small(format!("({} samples, 1 per second)", snaps.len()));
    });
    ui.separator();

    // Current snapshot: key ratios and rates at a glance — this is the tuning dashboard
    let infected_pct = if latest.organisms > 0 {
        latest.infected as f32 / latest.organisms as f32 * 100.0
    } else { 0.0 };
    let total_deaths_sample = latest.deaths_starvation + latest.deaths_predation
        + latest.deaths_old_age + latest.deaths_disease;
    let ratio = |n: u32| if total_deaths_sample > 0 { n as f32 / total_deaths_sample as f32 * 100.0 } else { 0.0 };

    egui::Grid::new("graphs_current").num_columns(4).striped(true).show(ui, |ui| {
        ui.label("Organisms");
        ui.monospace(format!("{:>4}", latest.organisms));
        ui.label("Food");
        ui.monospace(format!("{:>5}", latest.food));
        ui.end_row();

        ui.label("Species");
        ui.monospace(format!("{:>4}", latest.species));
        ui.label("Lifespan");
        ui.monospace(format!("{:>5}", latest.avg_lifespan as u32));
        ui.end_row();

        ui.label("Plants");
        ui.monospace(format!("{:>4}", latest.plants));
        ui.label("Foragers");
        ui.monospace(format!("{:>5}", latest.foragers));
        ui.end_row();

        ui.label("Predators");
        ui.monospace(format!("{:>4}", latest.predators));
        ui.label("Infected");
        ui.monospace(format!("{:>3} ({:>2.0}%)", latest.infected, infected_pct));
        ui.end_row();
    });

    ui.add_space(6.0);
    egui::CollapsingHeader::new("Death cause breakdown (this second)").default_open(true).show(ui, |ui| {
        egui::Grid::new("death_causes").num_columns(3).striped(true).show(ui, |ui| {
            ui.monospace("Starvation");
            ui.monospace(format!("{:>3}", latest.deaths_starvation));
            ui.monospace(format!("{:>4.0}%", ratio(latest.deaths_starvation)));
            ui.end_row();

            ui.monospace("Predation");
            ui.monospace(format!("{:>3}", latest.deaths_predation));
            ui.monospace(format!("{:>4.0}%", ratio(latest.deaths_predation)));
            ui.end_row();

            ui.monospace("Old age");
            ui.monospace(format!("{:>3}", latest.deaths_old_age));
            ui.monospace(format!("{:>4.0}%", ratio(latest.deaths_old_age)));
            ui.end_row();

            ui.monospace("Disease");
            ui.monospace(format!("{:>3}", latest.deaths_disease));
            ui.monospace(format!("{:>4.0}%", ratio(latest.deaths_disease)));
            ui.end_row();
        });
    });

    ui.add_space(6.0);
    egui::CollapsingHeader::new("Average traits").show(ui, |ui| {
        egui::Grid::new("avg_traits").num_columns(2).striped(true).show(ui, |ui| {
            ui.label("Disease resistance");
            ui.monospace(format!("{:>4.0}%", latest.avg_disease_resistance * 100.0));
            ui.end_row();
            ui.label("Body size");
            ui.monospace(format!("{:>5.2}", latest.avg_body_size));
            ui.end_row();
            ui.label("Speed factor");
            ui.monospace(format!("{:>5.2}", latest.avg_speed));
            ui.end_row();
            ui.label("Attack (claw)");
            ui.monospace(format!("{:>5.2}", latest.avg_attack));
            ui.end_row();
            ui.label("Armor");
            ui.monospace(format!("{:>5.2}", latest.avg_armor));
            ui.end_row();
            ui.label("Photosynthesis");
            ui.monospace(format!("{:>4.0}%", latest.avg_photo * 100.0));
            ui.end_row();
        });
    });

    ui.separator();

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        // Population by strategy
        ui.label("Population by strategy");
        let plants: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, s.plants as f64]).collect();
        let foragers: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, s.foragers as f64]).collect();
        let predators: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, s.predators as f64]).collect();

        Plot::new("pop_strategy")
            .height(130.0)
            .legend(Legend::default().position(egui_plot::Corner::LeftTop))
            .show(ui, |plot_ui| {
                plot_ui.line(Line::new(plants)
                    .color(egui::Color32::from_rgb(90, 200, 90)).name("Plants"));
                plot_ui.line(Line::new(foragers)
                    .color(egui::Color32::from_rgb(230, 230, 230)).name("Foragers"));
                plot_ui.line(Line::new(predators)
                    .color(egui::Color32::from_rgb(230, 100, 100)).name("Predators"));
            });

        ui.add_space(4.0);

        // Deaths by cause — the main tuning chart
        ui.label("Deaths per second by cause");
        let d_starv: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, s.deaths_starvation as f64]).collect();
        let d_pred: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, s.deaths_predation as f64]).collect();
        let d_old: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, s.deaths_old_age as f64]).collect();
        let d_dis: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, s.deaths_disease as f64]).collect();

        Plot::new("deaths_by_cause")
            .height(130.0)
            .legend(Legend::default().position(egui_plot::Corner::LeftTop))
            .show(ui, |plot_ui| {
                plot_ui.line(Line::new(d_starv)
                    .color(egui::Color32::from_rgb(230, 180, 90)).name("Starvation"));
                plot_ui.line(Line::new(d_pred)
                    .color(egui::Color32::from_rgb(230, 100, 100)).name("Predation"));
                plot_ui.line(Line::new(d_old)
                    .color(egui::Color32::from_rgb(180, 180, 180)).name("Old age"));
                plot_ui.line(Line::new(d_dis)
                    .color(egui::Color32::from_rgb(180, 80, 220)).name("Disease"));
            });

        ui.add_space(4.0);

        // Infection & resistance — disease tuning view
        ui.label("Infection rate & evolved resistance");
        let inf: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, s.infected as f64]).collect();
        let resist: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, (s.avg_disease_resistance * 100.0) as f64]).collect();

        Plot::new("disease_trend")
            .height(120.0)
            .legend(Legend::default().position(egui_plot::Corner::LeftTop))
            .show(ui, |plot_ui| {
                plot_ui.line(Line::new(inf)
                    .color(egui::Color32::from_rgb(180, 80, 220)).name("Infected (count)"));
                plot_ui.line(Line::new(resist)
                    .color(egui::Color32::from_rgb(120, 200, 220)).name("Avg resistance × 100"));
            });

        ui.add_space(4.0);

        // Total population vs species count
        ui.label("Total population vs species count");
        let organisms: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, s.organisms as f64]).collect();
        let species: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, s.species as f64]).collect();

        Plot::new("pop_vs_species")
            .height(120.0)
            .legend(Legend::default().position(egui_plot::Corner::LeftTop))
            .show(ui, |plot_ui| {
                plot_ui.line(Line::new(organisms)
                    .color(egui::Color32::from_rgb(100, 160, 255)).name("Organisms"));
                plot_ui.line(Line::new(species)
                    .color(egui::Color32::from_rgb(255, 200, 100)).name("Species"));
            });

        ui.add_space(4.0);

        // Trait evolution — key genetic trends over time
        ui.label("Key trait evolution (scaled to fit)");
        let t_attack: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, (s.avg_attack * 100.0) as f64]).collect();
        let t_armor: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, (s.avg_armor * 100.0) as f64]).collect();
        let t_photo: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, (s.avg_photo * 100.0) as f64]).collect();
        let t_body: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, (s.avg_body_size * 100.0) as f64]).collect();

        Plot::new("trait_trends")
            .height(130.0)
            .legend(Legend::default().position(egui_plot::Corner::LeftTop))
            .show(ui, |plot_ui| {
                plot_ui.line(Line::new(t_attack)
                    .color(egui::Color32::from_rgb(230, 100, 100)).name("Attack ×100"));
                plot_ui.line(Line::new(t_armor)
                    .color(egui::Color32::from_rgb(180, 180, 180)).name("Armor ×100"));
                plot_ui.line(Line::new(t_photo)
                    .color(egui::Color32::from_rgb(90, 200, 90)).name("Photo ×100"));
                plot_ui.line(Line::new(t_body)
                    .color(egui::Color32::from_rgb(200, 170, 230)).name("Body size ×100"));
            });

        ui.add_space(4.0);

        // Food supply and average lifespan
        ui.label("Food supply and average lifespan");
        let food: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, s.food as f64]).collect();
        let lifespan: PlotPoints = snaps.iter().enumerate()
            .map(|(i, s)| [i as f64, s.avg_lifespan as f64]).collect();

        Plot::new("food_lifespan")
            .height(110.0)
            .legend(Legend::default().position(egui_plot::Corner::LeftTop))
            .show(ui, |plot_ui| {
                plot_ui.line(Line::new(food)
                    .color(egui::Color32::from_rgb(180, 230, 90)).name("Food"));
                plot_ui.line(Line::new(lifespan)
                    .color(egui::Color32::from_rgb(220, 180, 255)).name("Lifespan"));
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
