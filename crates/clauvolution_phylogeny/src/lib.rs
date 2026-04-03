use bevy::prelude::*;
use std::collections::HashMap;

pub struct PhylogenyPlugin;

impl Plugin for PhylogenyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PhyloTree::default());
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
        lines.push("--- Phylogenetic Tree ---".to_string());

        // Collect living species sorted by population
        let mut living: Vec<&PhyloNode> = self.living_species();
        living.sort_by(|a, b| b.current_population.cmp(&a.current_population));

        // Show top species with tree structure
        let max_display = 15;
        for node in living.iter().take(max_display) {
            let strategy_char = match node.strategy {
                SpeciesStrategy::Photosynthesizer => 'P',
                SpeciesStrategy::Predator => 'X',
                SpeciesStrategy::Forager => 'F',
            };
            let age = (current_tick - node.born_tick) / 30; // approximate seconds
            let ancestry = self.ancestry_depth(node.species_id);
            let indent: String = "  ".repeat(ancestry.min(5) as usize);

            lines.push(format!(
                "{}{} Sp:{} pop:{} peak:{} age:{}s",
                indent, strategy_char,
                node.species_id, node.current_population,
                node.peak_population, age,
            ));
        }

        // Summary
        let total_ever = self.nodes.len();
        let total_living = living.len();
        let total_extinct = total_ever - total_living;
        lines.push(format!(
            "\nLiving: {}  Extinct: {}  Total: {}",
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
