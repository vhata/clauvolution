use bevy::prelude::*;
use std::collections::HashMap;

pub struct PhylogenyPlugin;

impl Plugin for PhylogenyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PhyloTree::default())
            .insert_resource(WorldChronicle::default());
    }
}

/// A log of significant evolutionary events
#[derive(Resource)]
pub struct WorldChronicle {
    pub entries: Vec<ChronicleEntry>,
    pub max_display: usize,
}

#[derive(Clone)]
pub struct ChronicleEntry {
    pub tick: u64,
    pub text: String,
}

impl Default for WorldChronicle {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            max_display: 20,
        }
    }
}

impl WorldChronicle {
    pub fn log(&mut self, tick: u64, text: String) {
        self.entries.push(ChronicleEntry { tick, text });
    }

    pub fn render_text(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        let mut lines = vec!["--- World Chronicle (C=toggle) ---".to_string()];
        let start = if self.entries.len() > self.max_display {
            self.entries.len() - self.max_display
        } else {
            0
        };

        for entry in &self.entries[start..] {
            let time_secs = entry.tick / 30;
            let time_str = if time_secs >= 60 {
                format!("{}m{:02}s", time_secs / 60, time_secs % 60)
            } else {
                format!("{:3}s", time_secs)
            };
            lines.push(format!("[{}] {}", time_str, entry.text));
        }

        lines.join("\n")
    }
}

/// A node in the phylogenetic tree representing a species
#[derive(Clone, Debug)]
pub struct PhyloNode {
    pub species_id: u64,
    pub parent_id: Option<u64>,
    pub born_tick: u64,
    pub extinct_tick: Option<u64>,
    pub peak_population: u32,
    pub current_population: u32,
    pub strategy: SpeciesStrategy,
    pub color: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpeciesStrategy {
    Photosynthesizer,
    Predator,
    Forager,
}

/// The full phylogenetic tree
#[derive(Resource, Default)]
pub struct PhyloTree {
    pub nodes: HashMap<u64, PhyloNode>,
    pub root_species: Vec<u64>,
}

impl PhyloTree {
    /// Record that a new species exists. If it came from an existing species, set parent_id.
    pub fn record_species(
        &mut self,
        species_id: u64,
        parent_id: Option<u64>,
        tick: u64,
        color: Color,
        strategy: SpeciesStrategy,
    ) {
        if self.nodes.contains_key(&species_id) {
            return;
        }

        let node = PhyloNode {
            species_id,
            parent_id,
            born_tick: tick,
            extinct_tick: None,
            peak_population: 1,
            current_population: 1,
            strategy,
            color,
        };

        if parent_id.is_none() {
            self.root_species.push(species_id);
        }

        self.nodes.insert(species_id, node);
    }

    /// Update population counts for all species. Mark species with 0 members as extinct.
    pub fn update_populations(&mut self, species_counts: &HashMap<u64, u32>, tick: u64) {
        for (id, node) in &mut self.nodes {
            let count = species_counts.get(id).copied().unwrap_or(0);
            node.current_population = count;
            if count > node.peak_population {
                node.peak_population = count;
            }
            if count == 0 && node.extinct_tick.is_none() {
                node.extinct_tick = Some(tick);
            }
            // Species can come back from the dead if reclassification reassigns members
            if count > 0 && node.extinct_tick.is_some() {
                node.extinct_tick = None;
            }
        }
    }

    /// Get all living species
    pub fn living_species(&self) -> Vec<&PhyloNode> {
        self.nodes.values().filter(|n| n.extinct_tick.is_none() && n.current_population > 0).collect()
    }

    /// Get children of a species
    pub fn children_of(&self, species_id: u64) -> Vec<&PhyloNode> {
        self.nodes.values().filter(|n| n.parent_id == Some(species_id)).collect()
    }

    /// Build a text representation of the tree for display
    pub fn render_text(&self, current_tick: u64) -> String {
        if self.nodes.is_empty() {
            return "No species yet".to_string();
        }

        let mut lines = Vec::new();
        lines.push("--- Living Species ---".to_string());

        let mut living: Vec<&PhyloNode> = self.living_species();
        living.sort_by(|a, b| b.current_population.cmp(&a.current_population));

        let max_display = 12;
        for node in living.iter().take(max_display) {
            let strategy = match node.strategy {
                SpeciesStrategy::Photosynthesizer => "Plant",
                SpeciesStrategy::Predator => "Predator",
                SpeciesStrategy::Forager => "Forager",
            };
            let age_secs = (current_tick.saturating_sub(node.born_tick)) / 30;
            let age_str = if age_secs >= 60 {
                format!("{}m{}s", age_secs / 60, age_secs % 60)
            } else {
                format!("{}s", age_secs)
            };
            let ancestry = self.ancestry_depth(node.species_id);
            let tree_prefix = if ancestry == 0 {
                "".to_string()
            } else {
                format!("{}\u{2514} ", "  ".repeat((ancestry - 1).min(4) as usize))
            };

            // Population bar — visual indicator of size
            let bar_len = ((node.current_population as f32 / 50.0).ceil() as usize).clamp(1, 15);
            let bar: String = "\u{2588}".repeat(bar_len);

            lines.push(format!(
                "{}{} ({}) {} [{}] {}",
                tree_prefix, strategy, node.current_population,
                bar, age_str,
                if node.current_population < node.peak_population / 2 { "declining" } else { "" },
            ));
        }

        if living.len() > max_display {
            lines.push(format!("  ...and {} more", living.len() - max_display));
        }

        // Recently extinct (last 3)
        let mut recently_extinct: Vec<&PhyloNode> = self.nodes.values()
            .filter(|n| n.extinct_tick.is_some())
            .collect();
        recently_extinct.sort_by(|a, b| b.extinct_tick.cmp(&a.extinct_tick));

        if !recently_extinct.is_empty() {
            lines.push(String::new());
            lines.push("Recently extinct:".to_string());
            for node in recently_extinct.iter().take(3) {
                let strategy = match node.strategy {
                    SpeciesStrategy::Photosynthesizer => "Plant",
                    SpeciesStrategy::Predator => "Predator",
                    SpeciesStrategy::Forager => "Forager",
                };
                let ago = current_tick.saturating_sub(node.extinct_tick.unwrap_or(0)) / 30;
                lines.push(format!(
                    "  {} - peak {} - died {}s ago",
                    strategy, node.peak_population, ago,
                ));
            }
        }

        let total_ever = self.nodes.len();
        let total_living = living.len();
        let total_extinct = total_ever - total_living;
        lines.push(String::new());
        lines.push(format!(
            "{} alive / {} extinct / {} total species",
            total_living, total_extinct, total_ever
        ));

        lines.join("\n")
    }

    /// How deep is this species in the tree (distance from root)
    fn ancestry_depth(&self, species_id: u64) -> u32 {
        let mut depth = 0;
        let mut current = species_id;
        while let Some(node) = self.nodes.get(&current) {
            if let Some(parent) = node.parent_id {
                depth += 1;
                current = parent;
                if depth > 20 {
                    break; // safety
                }
            } else {
                break;
            }
        }
        depth
    }
}
