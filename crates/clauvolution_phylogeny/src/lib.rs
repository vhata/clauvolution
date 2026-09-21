use bevy::prelude::*;
use std::collections::HashMap;

/// Traits used to generate a species name
pub struct SpeciesTraits {
    pub strategy: SpeciesStrategy,
    pub aquatic: f32,
    pub body_size: f32,
    pub speed: f32,
    pub armor: f32,
    pub has_fins: bool,
    pub has_eyes: bool,
    pub has_claws: bool,
    pub has_armor_plates: bool,
}

/// Which word of the three-word name is being chosen. Each slot mixes the
/// species id differently so the three words vary independently instead
/// of moving in lockstep as consecutive ids arrive.
#[derive(Clone, Copy)]
enum NameSlot {
    Habitat = 1,
    Descriptor = 2,
    Noun = 3,
}

/// Deterministically pick one entry from a word list for a species id.
/// `attempt` steps to the next word in the list, so the caller can walk
/// the whole list looking for a name no living species already has.
fn pick<'a>(words: &[&'a str], id: u64, slot: NameSlot, attempt: usize) -> &'a str {
    // splitmix64 finalizer: consecutive ids land on unrelated words.
    let mut z = id ^ ((slot as u64) << 56);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    let base = (z % words.len() as u64) as usize;
    words[(base + attempt) % words.len()]
}

/// The descriptors a species can carry, chosen by its most notable trait.
/// This is the only word a child name varies, so the lists are the longest.
fn body_descriptors(traits: &SpeciesTraits) -> &'static [&'static str] {
    if traits.has_armor_plates && traits.armor > 0.5 {
        &[
            "Plated", "Armored", "Shelled", "Ironclad", "Crusted", "Scaled", "Ridged", "Horned",
            "Mailed", "Stony",
        ]
    } else if traits.has_claws {
        &[
            "Clawed", "Hooked", "Barbed", "Serrated", "Fanged", "Taloned", "Spiked", "Thorned",
            "Pincered", "Bladed",
        ]
    } else if traits.has_eyes {
        &[
            "Keen",
            "Sharp-eyed",
            "Watchful",
            "Bright",
            "Alert",
            "Wide-eyed",
            "Gazing",
            "Vigilant",
            "Peering",
            "Owl-eyed",
        ]
    } else if traits.body_size > 1.3 {
        &[
            "Greater", "Giant", "Massive", "Hulking", "Towering", "Broad", "Stout", "Heavy",
            "Grand", "Colossal",
        ]
    } else if traits.body_size < 0.5 {
        &[
            "Lesser", "Dwarf", "Tiny", "Minute", "Pygmy", "Slight", "Little", "Slender", "Small",
            "Fine",
        ]
    } else if traits.speed > 1.2 {
        &[
            "Swift",
            "Fleet",
            "Darting",
            "Quick",
            "Racing",
            "Nimble",
            "Rapid",
            "Brisk",
            "Flitting",
            "Skittering",
        ]
    } else {
        &[
            "Common", "Spotted", "Banded", "Pale", "Dusky", "Mottled", "Striped", "Grey", "Russet",
            "Freckled",
        ]
    }
}

/// Colour and pattern words that claim nothing about a species' traits.
/// Walked after the trait bucket is exhausted, before a name gets a number.
const OVERFLOW_DESCRIPTORS: &[&str] = &[
    "Amber", "Ashen", "Umber", "Slate", "Ochre", "Sable", "Ivory", "Tawny", "Rusty", "Sooty",
    "Chalky", "Dappled", "Brindled", "Flecked", "Ringed", "Speckled", "Marbled", "Piebald",
    "Copper", "Silver", "Golden", "Olive", "Violet", "Crimson",
];

/// How many distinct descriptors `body_descriptor` can offer a species
/// before its name needs a number.
fn descriptor_attempts(traits: &SpeciesTraits) -> usize {
    body_descriptors(traits).len() + OVERFLOW_DESCRIPTORS.len()
}

fn body_descriptor(traits: &SpeciesTraits, id: u64, attempt: usize) -> &'static str {
    let bucket = body_descriptors(traits);
    if attempt < bucket.len() {
        pick(bucket, id, NameSlot::Descriptor, attempt)
    } else {
        pick(
            OVERFLOW_DESCRIPTORS,
            id,
            NameSlot::Descriptor,
            attempt - bucket.len(),
        )
    }
}

fn habitat_word(traits: &SpeciesTraits, id: u64) -> &'static str {
    let words: &[&str] = if traits.aquatic > 0.5 {
        if traits.has_fins {
            &[
                "Reef", "Deep", "Tidal", "Pelagic", "Abyssal", "Open-sea", "Trench", "Shoal",
            ]
        } else {
            &[
                "Shore", "Marsh", "Coastal", "Brackish", "Littoral", "Lagoon", "Mudflat", "Cove",
            ]
        }
    } else if traits.aquatic > 0.2 {
        &[
            "Riparian",
            "Swamp",
            "Estuary",
            "Delta",
            "Wetland",
            "Fen",
            "Bayou",
            "Floodplain",
        ]
    } else {
        &[
            "Plains", "Ridge", "Upland", "Steppe", "Highland", "Valley", "Heath", "Moor",
        ]
    };
    pick(words, id, NameSlot::Habitat, 0)
}

fn strategy_noun(traits: &SpeciesTraits, id: u64) -> &'static str {
    let aquatic = traits.aquatic > 0.5;
    let words: &[&str] = match traits.strategy {
        SpeciesStrategy::Photosynthesizer => {
            if aquatic {
                &[
                    "Kelp", "Algae", "Seagrass", "Coral", "Lichen", "Wrack", "Sponge", "Reedmace",
                ]
            } else {
                &[
                    "Fern", "Moss", "Vine", "Shrub", "Bloom", "Sedge", "Thistle", "Bramble",
                ]
            }
        }
        SpeciesStrategy::Hunter => {
            if aquatic {
                &[
                    "Shark",
                    "Eel",
                    "Hunter",
                    "Stalker",
                    "Lurker",
                    "Pike",
                    "Lamprey",
                    "Barracuda",
                ]
            } else {
                &[
                    "Raptor", "Prowler", "Striker", "Ambusher", "Mauler", "Stalker", "Pouncer",
                    "Harrier",
                ]
            }
        }
        SpeciesStrategy::Grazer => {
            if aquatic {
                &[
                    "Drifter", "Grazer", "Filter", "Crawler", "Nibbler", "Limpet", "Sifter",
                    "Wader",
                ]
            } else {
                &[
                    "Grazer", "Browser", "Gleaner", "Rooter", "Cropper", "Nibbler", "Mower",
                    "Chewer",
                ]
            }
        }
        SpeciesStrategy::Omnivore => {
            if aquatic {
                &[
                    "Scavenger",
                    "Dabbler",
                    "Rover",
                    "Scrounger",
                    "Skimmer",
                    "Dredger",
                    "Prober",
                    "Bottom-feeder",
                ]
            } else {
                &[
                    "Forager",
                    "Wanderer",
                    "Rummager",
                    "Scavenger",
                    "Rover",
                    "Rooter",
                    "Picker",
                    "Snuffler",
                ]
            }
        }
    };
    pick(words, id, NameSlot::Noun, 0)
}

/// A full name from traits, for a root species with no parent. `attempt`
/// steps through the descriptor list.
fn generate_species_name(traits: &SpeciesTraits, species_id: u64, attempt: usize) -> String {
    format!(
        "{} {} {}",
        habitat_word(traits, species_id),
        body_descriptor(traits, species_id, attempt),
        strategy_noun(traits, species_id),
    )
}

/// A child name inherits its parent's habitat and strategy noun and only
/// changes the body descriptor to reflect what's new. `attempt` steps
/// through the descriptor list.
fn generate_child_name(
    traits: &SpeciesTraits,
    species_id: u64,
    parent_name: &str,
    attempt: usize,
) -> String {
    let new_body = body_descriptor(traits, species_id, attempt);
    let parts: Vec<&str> = parent_name.splitn(3, ' ').collect();
    // The parent's noun may carry a disambiguating number; drop it.
    match (
        parts.first(),
        parts.get(2).and_then(|p| p.split(' ').next()),
    ) {
        (Some(habitat), Some(noun)) if parts.len() == 3 => {
            format!("{habitat} {new_body} {noun}")
        }
        _ => generate_species_name(traits, species_id, attempt),
    }
}

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
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
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
    pub name: String,
}

/// How a species makes its living, read off the genome by
/// `classify_strategy`. Plants photosynthesise; the other three are split
/// by the `diet` trait. See `docs/design/simulation-rules.md`, phase 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpeciesStrategy {
    Photosynthesizer,
    /// `diet` at or below `-DIET_SPECIALIST_THRESHOLD`: lives on plant tissue.
    Grazer,
    /// `diet` at or above `DIET_SPECIALIST_THRESHOLD`: lives on animal tissue.
    Hunter,
    /// `diet` between the two: digests a little of each.
    Omnivore,
}

/// `|diet|` from which an organism counts as a specialist rather than an
/// omnivore. At a third, a specialist digests at least 44% of its preferred
/// food and at most 11% of the other (`Genome::plant_efficiency`).
pub const DIET_SPECIALIST_THRESHOLD: f32 = 1.0 / 3.0;

impl SpeciesStrategy {
    /// Every strategy, in display order.
    pub const ALL: [SpeciesStrategy; 4] = [
        SpeciesStrategy::Photosynthesizer,
        SpeciesStrategy::Grazer,
        SpeciesStrategy::Hunter,
        SpeciesStrategy::Omnivore,
    ];

    /// Short display name.
    pub fn label(self) -> &'static str {
        match self {
            SpeciesStrategy::Photosynthesizer => "Plant",
            SpeciesStrategy::Grazer => "Grazer",
            SpeciesStrategy::Hunter => "Hunter",
            SpeciesStrategy::Omnivore => "Omnivore",
        }
    }

    /// The activity the strategy names, for chronicle text
    /// ("3 lineages evolved grazing").
    pub fn activity(self) -> &'static str {
        match self {
            SpeciesStrategy::Photosynthesizer => "photosynthesis",
            SpeciesStrategy::Grazer => "grazing",
            SpeciesStrategy::Hunter => "hunting",
            SpeciesStrategy::Omnivore => "omnivory",
        }
    }
}

/// Strategy label used by population history, species records, rendering,
/// and the founder log line. Plants first, then the diet axis.
pub fn classify_strategy(genome: &clauvolution_genome::Genome) -> SpeciesStrategy {
    if genome.is_photosynthesiser() {
        SpeciesStrategy::Photosynthesizer
    } else if genome.diet <= -DIET_SPECIALIST_THRESHOLD {
        SpeciesStrategy::Grazer
    } else if genome.diet >= DIET_SPECIALIST_THRESHOLD {
        SpeciesStrategy::Hunter
    } else {
        SpeciesStrategy::Omnivore
    }
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
        traits: Option<&SpeciesTraits>,
    ) {
        if self.nodes.contains_key(&species_id) {
            return;
        }

        let name = match traits {
            Some(t) => self.unused_name(t, species_id, parent_id),
            None => format!("Species {}", species_id),
        };

        let node = PhyloNode {
            species_id,
            parent_id,
            born_tick: tick,
            extinct_tick: None,
            peak_population: 1,
            current_population: 1,
            strategy,
            color,
            name,
        };

        if parent_id.is_none() {
            self.root_species.push(species_id);
        }

        self.nodes.insert(species_id, node);
    }

    /// A name for a new species that no living species already carries. A
    /// child inherits its parent's habitat and noun, so the descriptor is
    /// walked first; when every descriptor is taken by a living relative,
    /// the first candidate gets the lowest unused number appended.
    fn unused_name(
        &self,
        traits: &SpeciesTraits,
        species_id: u64,
        parent_id: Option<u64>,
    ) -> String {
        let parent_name = parent_id
            .and_then(|pid| self.nodes.get(&pid))
            .map(|parent| parent.name.as_str());
        let candidate = |attempt: usize| match parent_name {
            Some(parent_name) => generate_child_name(traits, species_id, parent_name, attempt),
            None => generate_species_name(traits, species_id, attempt),
        };
        let taken = |name: &str| {
            self.nodes
                .values()
                .any(|n| n.extinct_tick.is_none() && n.name == name)
        };
        let first = candidate(0);
        if !taken(&first) {
            return first;
        }
        for attempt in 1..descriptor_attempts(traits) {
            let name = candidate(attempt);
            if !taken(&name) {
                return name;
            }
        }
        (2..)
            .map(|n| format!("{first} {n}"))
            .find(|name| !taken(name))
            .expect("the numbered names are unbounded")
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
        self.nodes
            .values()
            .filter(|n| n.extinct_tick.is_none() && n.current_population > 0)
            .collect()
    }

    /// Get children of a species
    pub fn children_of(&self, species_id: u64) -> Vec<&PhyloNode> {
        self.nodes
            .values()
            .filter(|n| n.parent_id == Some(species_id))
            .collect()
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

        let mut results = Vec::new();

        for &strat in &SpeciesStrategy::ALL {
            let species_with_strat: Vec<&PhyloNode> = living
                .iter()
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
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
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
            let root = lineage_root
                .get(&node.species_id)
                .copied()
                .unwrap_or(node.species_id);
            lineages.entry(root).or_default().push(node);
        }

        // Sort lineages by total population, show each group
        let mut sorted_lineages: Vec<(u64, Vec<&PhyloNode>)> = lineages.into_iter().collect();
        sorted_lineages.sort_by(|a, b| {
            let pop_a: u32 = a.1.iter().map(|n| n.current_population).sum();
            let pop_b: u32 = b.1.iter().map(|n| n.current_population).sum();
            pop_b.cmp(&pop_a).then(a.0.cmp(&b.0))
        });

        for (_root_id, mut members) in sorted_lineages.into_iter().take(max_display) {
            members.sort_by(|a, b| {
                b.current_population
                    .cmp(&a.current_population)
                    .then(a.species_id.cmp(&b.species_id))
            });

            // Show the biggest member as the root line
            let first = members[0];
            lines.push(self.format_species_line(first, 0, current_tick));

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
        let mut recently_extinct: Vec<&PhyloNode> = self
            .nodes
            .values()
            .filter(|n| n.extinct_tick.is_some())
            .collect();
        recently_extinct.sort_by(|a, b| {
            b.extinct_tick
                .cmp(&a.extinct_tick)
                .then(a.species_id.cmp(&b.species_id))
        });

        if !recently_extinct.is_empty() {
            lines.push(String::new());
            lines.push("Recently extinct:".to_string());
            for node in recently_extinct.iter().take(3) {
                let strategy = node.strategy.label();
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
        let age_secs = current_tick.saturating_sub(node.born_tick) / 30;
        let age_str = if age_secs >= 60 {
            format!("{}m{:02}s", age_secs / 60, age_secs % 60)
        } else {
            format!("{}s", age_secs)
        };

        let indent = if depth == 0 {
            "".to_string()
        } else {
            format!("  {}\u{2514} ", "\u{2502} ".repeat((depth - 1).min(3)))
        };

        // Truncate name to fit — shorter for indented children
        let max_name = 22 - indent.chars().count().min(10);
        let name: String = node.name.chars().take(max_name).collect();

        let bar_len = ((node.current_population as f32 / 50.0).ceil() as usize).clamp(1, 10);
        let bar: String = "\u{2588}".repeat(bar_len);
        let bar = format!("{:<15}", bar);

        let declining = if node.current_population < node.peak_population / 2 {
            " declining"
        } else {
            ""
        };

        let padded_name = format!("{}{}", indent, name);
        format!(
            "{:<24} {:>4} {} {:>6}{}",
            padded_name, node.current_population, bar, age_str, declining,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clauvolution_genome::{
        BodySegmentGene, Genome, InnovationCounter, SegmentType, Symmetry,
        PHOTOSYNTHESISER_RATE_THRESHOLD,
    };
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn genome(seed: u64) -> Genome {
        let mut innovation = InnovationCounter(0);
        let mut rng = StdRng::seed_from_u64(seed);
        Genome::new_minimal(&mut innovation, &mut rng)
    }

    #[test]
    fn plants_are_classified_before_diet() {
        let mut g = genome(1);
        g.body_segments.push(BodySegmentGene {
            segment_type: SegmentType::PhotoSurface,
            size: 1.0,
            attachment_angle: 0.0,
            attachment_slot: 0,
            symmetry: Symmetry::None,
        });
        g.photosynthesis_rate = PHOTOSYNTHESISER_RATE_THRESHOLD + 0.1;
        g.diet = 1.0;
        assert_eq!(classify_strategy(&g), SpeciesStrategy::Photosynthesizer);
    }

    #[test]
    fn diet_splits_the_rest_into_three() {
        let mut g = genome(2);
        g.body_segments
            .retain(|s| s.segment_type != SegmentType::PhotoSurface);
        g.diet = -1.0;
        assert_eq!(classify_strategy(&g), SpeciesStrategy::Grazer);
        g.diet = -DIET_SPECIALIST_THRESHOLD;
        assert_eq!(classify_strategy(&g), SpeciesStrategy::Grazer);
        g.diet = 0.0;
        assert_eq!(classify_strategy(&g), SpeciesStrategy::Omnivore);
        g.diet = DIET_SPECIALIST_THRESHOLD;
        assert_eq!(classify_strategy(&g), SpeciesStrategy::Hunter);
        g.diet = 1.0;
        assert_eq!(classify_strategy(&g), SpeciesStrategy::Hunter);
    }

    /// What the naming measurement found for one simulated tree.
    struct NameReport {
        /// Species sharing their name with another (every node is alive,
        /// the strictest case).
        colliding: usize,
        /// The same count using each species' first candidate, before the
        /// walk past living names: what the hash and word lists alone do.
        colliding_without_walk: usize,
        /// Species whose name needed a number because every descriptor
        /// was taken by a living relative.
        numbered: usize,
        shared: Vec<String>,
    }

    fn count_shared(names: impl Iterator<Item = String>) -> (usize, Vec<String>) {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for name in names {
            *counts.entry(name).or_default() += 1;
        }
        let colliding = counts.values().filter(|&&c| c > 1).sum();
        let mut shared: Vec<String> = counts
            .iter()
            .filter(|(_, &c)| c > 1)
            .map(|(n, c)| format!("{n} x{c}"))
            .collect();
        shared.sort();
        (colliding, shared)
    }

    /// Build a tree the way `species_classification_system` does:
    /// sequential ids, a dozen founders spread over the strategies, then
    /// children of random living species whose traits drift a little from
    /// their parent's. Once more than `living_cap` species are alive, each
    /// new species sends a random older one extinct, as a run does.
    fn name_collisions(seed: u64, species: u64, living_cap: usize) -> NameReport {
        use rand::Rng;
        let mut rng = StdRng::seed_from_u64(seed);
        let mut phylo = PhyloTree::default();
        let mut traits_by_id: HashMap<u64, SpeciesTraits> = HashMap::new();
        let mut first_candidates: HashMap<u64, String> = HashMap::new();
        let mut living: Vec<u64> = Vec::new();
        let strategies = SpeciesStrategy::ALL;
        for id in 1..=species {
            if living.len() > living_cap {
                let gone = living.swap_remove(rng.gen_range(0..living.len()));
                phylo.nodes.get_mut(&gone).unwrap().extinct_tick = Some(id);
            }
            let parent = if id <= 12 {
                None
            } else {
                Some(living[rng.gen_range(0..living.len())])
            };
            living.push(id);
            let traits = match parent.and_then(|p| traits_by_id.get(&p)) {
                None => SpeciesTraits {
                    strategy: strategies[(id as usize) % strategies.len()],
                    aquatic: rng.gen_range(0.0..0.5),
                    body_size: rng.gen_range(0.5..1.5),
                    speed: rng.gen_range(0.5..1.5),
                    armor: rng.gen_range(0.0..0.3),
                    has_fins: false,
                    has_eyes: rng.gen_bool(0.3),
                    has_claws: rng.gen_bool(0.1),
                    has_armor_plates: rng.gen_bool(0.1),
                },
                Some(p) => SpeciesTraits {
                    strategy: p.strategy,
                    aquatic: (p.aquatic + rng.gen_range(-0.1..0.1)).clamp(0.0, 1.0),
                    body_size: (p.body_size + rng.gen_range(-0.2..0.2)).clamp(0.3, 2.0),
                    speed: (p.speed + rng.gen_range(-0.2..0.2)).clamp(0.2, 2.0),
                    armor: (p.armor + rng.gen_range(-0.1..0.1)).clamp(0.0, 1.0),
                    has_fins: p.has_fins || rng.gen_bool(0.05),
                    has_eyes: p.has_eyes || rng.gen_bool(0.1),
                    has_claws: p.has_claws || rng.gen_bool(0.05),
                    has_armor_plates: p.has_armor_plates || rng.gen_bool(0.05),
                },
            };
            let first = match parent.and_then(|p| first_candidates.get(&p)) {
                Some(parent_name) => generate_child_name(&traits, id, parent_name, 0),
                None => generate_species_name(&traits, id, 0),
            };
            first_candidates.insert(id, first);
            phylo.record_species(id, parent, id, Color::WHITE, traits.strategy, Some(&traits));
            traits_by_id.insert(id, traits);
        }
        let alive: Vec<&PhyloNode> = phylo
            .nodes
            .values()
            .filter(|n| n.extinct_tick.is_none())
            .collect();
        let (colliding, shared) = count_shared(alive.iter().map(|n| n.name.clone()));
        let (colliding_without_walk, _) = count_shared(
            alive
                .iter()
                .map(|n| first_candidates[&n.species_id].clone()),
        );
        let numbered = alive
            .iter()
            .filter(|n| n.name.ends_with(|c: char| c.is_ascii_digit()))
            .count();
        NameReport {
            colliding,
            colliding_without_walk,
            numbered,
            shared,
        }
    }

    /// 200 species with every one still alive: more than any run keeps,
    /// so a stress test of the walk itself.
    #[test]
    fn species_names_do_not_collide_even_with_200_living_species() {
        for seed in 1..=5 {
            let report = name_collisions(seed, 200, usize::MAX);
            println!(
                "seed {seed}: {} of 200 living share a name ({} without the walk past living \
                 names), {} numbered: {:?}",
                report.colliding, report.colliding_without_walk, report.numbered, report.shared
            );
            assert_eq!(report.colliding, 0, "seed {seed}: {:?}", report.shared);
        }
    }

    /// 200 species over the run with about 40 alive at a time, like a
    /// headless run at 3000 ticks. Names exist to tell species apart while
    /// watching, so a number must be the rare last resort.
    #[test]
    fn species_names_rarely_need_a_number_at_realistic_living_counts() {
        for seed in 1..=5 {
            let report = name_collisions(seed, 200, 40);
            println!(
                "seed {seed}: {} of ~40 living share a name ({} without the walk past living \
                 names), {} numbered: {:?}",
                report.colliding, report.colliding_without_walk, report.numbered, report.shared
            );
            assert_eq!(report.colliding, 0, "seed {seed}: {:?}", report.shared);
            assert!(
                report.numbered <= 2,
                "seed {seed}: {} numbered",
                report.numbered
            );
        }
    }

    #[test]
    fn child_of_a_numbered_parent_drops_the_number() {
        let traits = SpeciesTraits {
            strategy: SpeciesStrategy::Grazer,
            aquatic: 0.0,
            body_size: 1.0,
            speed: 1.0,
            armor: 0.0,
            has_fins: false,
            has_eyes: false,
            has_claws: false,
            has_armor_plates: false,
        };
        let name = generate_child_name(&traits, 7, "Plains Common Grazer 3", 0);
        let words: Vec<&str> = name.split(' ').collect();
        assert_eq!(words.len(), 3, "{name}");
        assert_eq!(words[0], "Plains");
        assert_eq!(words[2], "Grazer");
    }

    #[test]
    fn every_strategy_has_a_label_and_activity() {
        for s in SpeciesStrategy::ALL {
            assert!(!s.label().is_empty());
            assert!(!s.activity().is_empty());
        }
    }
}
