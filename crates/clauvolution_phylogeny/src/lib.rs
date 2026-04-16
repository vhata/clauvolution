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
    pub log_path: Option<std::path::PathBuf>,
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
            log_path: None,
        }
    }
}

impl WorldChronicle {
    pub fn log(&mut self, tick: u64, text: String) {
        // Write to file if path is set
        if let Some(ref path) = self.log_path {
            use std::io::Write;
            let time_secs = tick / 30;
            let time_str = if time_secs >= 60 {
                format!("{}m{:02}s", time_secs / 60, time_secs % 60)
            } else {
                format!("{:3}s", time_secs)
            };
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(file, "[{}] {}", time_str, text);
            }
        }
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

    /// Get the lineage (chain of ancestor species IDs) for a species
    fn lineage(&self, species_id: u64) -> Vec<u64> {
        let mut chain = vec![species_id];
        let mut current = species_id;
        for _ in 0..50 {
            if let Some(node) = self.nodes.get(&current) {
                if let Some(parent) = node.parent_id {
                    chain.push(parent);
                    current = parent;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        chain
    }

    /// Check if two species share a recent common ancestor (within depth N)
    pub fn shares_recent_ancestor(&self, a: u64, b: u64, max_depth: usize) -> bool {
        let lineage_a = self.lineage(a);
        let lineage_b = self.lineage(b);
        let check_a: Vec<&u64> = lineage_a.iter().take(max_depth).collect();
        let check_b: Vec<&u64> = lineage_b.iter().take(max_depth).collect();
        check_a.iter().any(|id| check_b.contains(id))
    }

    /// Detect convergent evolution: count independent lineages per strategy.
    /// Returns strategies where 2+ unrelated lineages evolved the same thing.
    pub fn detect_convergence(&self) -> Vec<(SpeciesStrategy, usize)> {
        let living = self.living_species();
        let strategies = [
            SpeciesStrategy::Photosynthesizer,
            SpeciesStrategy::Predator,
            SpeciesStrategy::Forager,
        ];

        let mut results = Vec::new();

        for &strat in &strategies {
            let species_with_strat: Vec<&PhyloNode> = living.iter()
                .filter(|n| n.strategy == strat && n.current_population >= 10)
                .copied()
                .collect();

            if species_with_strat.len() < 2 {
                continue;
            }

            // Count independent lineages: group by shared ancestry
            let mut lineage_roots: Vec<u64> = Vec::new();
            for sp in &species_with_strat {
                let mut root = sp.species_id;
                let mut current = sp.species_id;
                for _ in 0..10 {
                    if let Some(n) = self.nodes.get(&current) {
                        if let Some(pid) = n.parent_id {
                            root = pid;
                            current = pid;
                        } else { break; }
                    } else { break; }
                }
                if !lineage_roots.contains(&root) {
                    lineage_roots.push(root);
                }
            }

            if lineage_roots.len() >= 2 {
                results.push((strat, lineage_roots.len()));
            }
        }
        results
    }

    /// Build a text representation of the tree for display
    pub fn render_text(&self, current_tick: u64) -> String {
        if self.nodes.is_empty() {
            return "No species yet".to_string();
        }

        let mut lines = Vec::new();
        lines.push("--- Living Species ---".to_string());

        let max_display = 30;
        let living = self.living_species();

        // Group living species by lineage: walk each species up to a
        // common ancestor (max 10 steps). Species sharing an ancestor
        // are in the same lineage.
        let mut lineage_root: HashMap<u64, u64> = HashMap::new();
        for node in &living {
            let mut root = node.species_id;
            let mut current = node.species_id;
            for _ in 0..10 {
                if let Some(n) = self.nodes.get(&current) {
                    if let Some(pid) = n.parent_id {
                        root = pid;
                        current = pid;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            lineage_root.insert(node.species_id, root);
        }

        // Group by lineage root
        let mut lineages: HashMap<u64, Vec<&PhyloNode>> = HashMap::new();
        for node in &living {
            let root = lineage_root.get(&node.species_id).copied().unwrap_or(node.species_id);
            lineages.entry(root).or_default().push(node);
        }

        // Sort lineages by total population, show each group
        let mut sorted_lineages: Vec<(u64, Vec<&PhyloNode>)> = lineages.into_iter().collect();
        sorted_lineages.sort_by(|a, b| {
            let pop_a: u32 = a.1.iter().map(|n| n.current_population).sum();
            let pop_b: u32 = b.1.iter().map(|n| n.current_population).sum();
            pop_b.cmp(&pop_a).then(a.0.cmp(&b.0))
        });

        let mut roots_shown = 0;
        for (_root_id, mut members) in sorted_lineages {
            if roots_shown >= max_display { break; }
            members.sort_by(|a, b| b.current_population.cmp(&a.current_population).then(a.species_id.cmp(&b.species_id)));

            // Show the biggest member as the root line
            let first = members[0];
            lines.push(self.format_species_line(first, 0, current_tick));
            roots_shown += 1;

            // Always show ALL children — no cap on children
            for sibling in members.iter().skip(1) {
                lines.push(self.format_species_line(sibling, 1, current_tick));
            }
        }

        let total_shown = lines.len() - 1; // minus the header line
        if living.len() > total_shown {
            lines.push(format!("  ...and {} more", living.len() - total_shown));
        }

        // Recently extinct (last 3)
        let mut recently_extinct: Vec<&PhyloNode> = self.nodes.values()
            .filter(|n| n.extinct_tick.is_some())
            .collect();
        recently_extinct.sort_by(|a, b| b.extinct_tick.cmp(&a.extinct_tick).then(a.species_id.cmp(&b.species_id)));

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


    fn format_species_line(&self, node: &PhyloNode, depth: usize, current_tick: u64) -> String {
        let strategy = match node.strategy {
            SpeciesStrategy::Photosynthesizer => "Plant   ",
            SpeciesStrategy::Predator =>         "Predator",
            SpeciesStrategy::Forager =>          "Forager ",
        };
        let age_secs = current_tick.saturating_sub(node.born_tick) / 30;
        let age_str = if age_secs >= 60 {
            format!("{}m{:02}s", age_secs / 60, age_secs % 60)
        } else {
            format!("{}s", age_secs)
        };

        // Fixed-width indent: 4 chars for depth 0, "└ " prefix for children
        let indent = if depth == 0 {
            "  ".to_string()
        } else {
            format!("{}\u{2514} ", "  ".repeat((depth - 1).min(3)))
        };
        // Pad indent to consistent width (4 chars)
        let indent = format!("{:<4}", indent);

        let bar_len = ((node.current_population as f32 / 50.0).ceil() as usize).clamp(1, 15);
        let bar: String = "\u{2588}".repeat(bar_len);
        // Pad bar to fixed width so age column aligns
        let bar = format!("{:<15}", bar);

        let declining = if node.current_population < node.peak_population / 2 { " declining" } else { "" };

        format!(
            "{}{} {:>4} {} {:>6}{}",
            indent, strategy, node.current_population, bar, age_str, declining,
        )
    }
}
