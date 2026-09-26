//! Offline species-distance report for step 1 of
//! `plans/2026-09-24-innovation-keying.md`.
//!
//! Reads a save written at the end of a run and, optionally, a save written
//! before the first classification pass (`--headless 149`: the founders and
//! their first children, all still unclassified), re-keys every genome with
//! `clauvolution_genome::rekey_population`, and prints the species
//! instruments under the current identity (legacy innovation numbers) and the
//! keyed one, each with both structural normalisations. It changes nothing.
//!
//! ```text
//! cargo run --release -p clauvolution_sim --example species_keying_report -- \
//!     sessions/run-5000/save.json [sessions/run-149/save.json]
//! ```
//!
//! Species ids come from the save, so the keyed columns ask how the keyed
//! distance would see the species the legacy distance formed, not what a
//! keyed run would form. Each species' representative is its first member in
//! save order, as `species_classification_system` takes the first member in
//! query order.

use clauvolution_genome::{rekey_population, CompatibilityTerms, Genome, StructuralNorm};
use clauvolution_sim::save::{save_to_genome, SaveState};

const HYSTERESIS: f32 = 1.3;
const THRESHOLDS: [f32; 10] = [0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.8];

struct Population {
    genomes: Vec<Genome>,
    species: Vec<u64>,
}

fn load(path: &str) -> SaveState {
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn population(state: &SaveState) -> Population {
    Population {
        genomes: state
            .organisms
            .iter()
            .map(|o| save_to_genome(&o.genome))
            .collect(),
        species: state.organisms.iter().map(|o| o.species_id).collect(),
    }
}

/// `(species id, index of its first member)` in save order.
fn representatives(species: &[u64]) -> Vec<(u64, usize)> {
    let mut reps: Vec<(u64, usize)> = Vec::new();
    for (i, &s) in species.iter().enumerate() {
        if s > 0 && !reps.iter().any(|&(id, _)| id == s) {
            reps.push((s, i));
        }
    }
    reps
}

fn label(norm: StructuralNorm) -> &'static str {
    match norm {
        StructuralNorm::Larger => "larger",
        StructuralNorm::Mean => "mean",
    }
}

fn percentile(sorted: &[f32], p: f64) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[i]
}

/// Per-organism terms against every representative, for one identity.
fn rep_terms(genomes: &[Genome], reps: &[(u64, usize)]) -> Vec<Vec<CompatibilityTerms>> {
    genomes
        .iter()
        .map(|g| {
            reps.iter()
                .map(|&(_, r)| g.compatibility_terms(&genomes[r]))
                .collect()
        })
        .collect()
}

fn population_report(name: &str, pop: &Population, genomes: &[Genome]) {
    let reps = representatives(&pop.species);
    let terms = rep_terms(genomes, &reps);
    let n = genomes.len();

    // Unrelated pairs: organism against another species' representative.
    let mut pairs = 0u64;
    let mut sharing = 0u64;
    let mut weight_sum = 0.0f64;
    for (i, row) in terms.iter().enumerate() {
        for (k, t) in row.iter().enumerate() {
            if reps[k].0 == pop.species[i] {
                continue;
            }
            pairs += 1;
            if t.matching > 0 {
                sharing += 1;
                weight_sum += t.weight_term() as f64;
            }
        }
    }
    println!(
        "  {name}: {} organisms, {} species; organism-to-other-rep pairs sharing a gene {:.1}% (weight term when shared: mean {:.3})",
        n,
        reps.len(),
        100.0 * sharing as f64 / pairs.max(1) as f64,
        weight_sum / sharing.max(1) as f64
    );

    for norm in [StructuralNorm::Larger, StructuralNorm::Mean] {
        let own: Vec<f32> = (0..n)
            .filter_map(|i| {
                reps.iter()
                    .position(|&(id, _)| id == pop.species[i])
                    .map(|k| terms[i][k].distance(norm))
            })
            .collect();
        let mut nearest_other: Vec<f32> = (0..n)
            .map(|i| {
                terms[i]
                    .iter()
                    .enumerate()
                    .filter(|&(k, _)| reps[k].0 != pop.species[i])
                    .map(|(_, t)| t.distance(norm))
                    .fold(f32::MAX, f32::min)
            })
            .collect();
        let mut own_sorted = own.clone();
        own_sorted.sort_by(|a, b| a.total_cmp(b));
        nearest_other.sort_by(|a, b| a.total_cmp(b));
        println!(
            "    {name}/{}: distance to own rep median {:.3} (p90 {:.3}); to nearest other rep median {:.3} (p10 {:.3}, p90 {:.3})",
            label(norm),
            percentile(&own_sorted, 0.5),
            percentile(&own_sorted, 0.9),
            percentile(&nearest_other, 0.5),
            percentile(&nearest_other, 0.1),
            percentile(&nearest_other, 0.9),
        );
        println!("      join  near-other%  drifting  isolated");
        for join in THRESHOLDS {
            let stay = join * HYSTERESIS;
            let mut near = 0;
            let mut drifting = 0;
            let mut isolated = 0;
            for (row, &species) in terms.iter().zip(&pop.species) {
                let mut own_d = None;
                let mut other_min = f32::MAX;
                for (k, t) in row.iter().enumerate() {
                    let d = t.distance(norm);
                    if reps[k].0 == species {
                        own_d = Some(d);
                    } else {
                        other_min = other_min.min(d);
                    }
                }
                if other_min < join {
                    near += 1;
                }
                if own_d.is_some_and(|d| d >= stay) {
                    drifting += 1;
                    if other_min >= join {
                        isolated += 1;
                    }
                }
            }
            println!(
                "      {:>4.2}  {:>10.1}  {:>8}  {:>8}",
                join,
                100.0 * near as f64 / n as f64,
                drifting,
                isolated
            );
        }
    }
}

/// The founding population (every organism alive before the first
/// classification pass): pairwise distances, gene sharing, and the species
/// the founding pass (greedy, in save order, as `choose_species` does for
/// species 0) would form at each join threshold.
fn founder_report(name: &str, genomes: &[Genome]) {
    let n = genomes.len();
    let mut terms = Vec::with_capacity(n * (n - 1) / 2);
    for i in 0..n {
        for j in (i + 1)..n {
            terms.push(genomes[i].compatibility_terms(&genomes[j]));
        }
    }
    let sharing: Vec<&CompatibilityTerms> = terms.iter().filter(|t| t.matching > 0).collect();
    let weight_mean =
        sharing.iter().map(|t| t.weight_term() as f64).sum::<f64>() / sharing.len().max(1) as f64;
    println!(
        "  {name}: {} founders, {} pairs; pairs sharing a gene {:.1}% (weight term when shared: mean {:.3})",
        n,
        terms.len(),
        100.0 * sharing.len() as f64 / terms.len().max(1) as f64,
        weight_mean
    );
    for norm in [StructuralNorm::Larger, StructuralNorm::Mean] {
        let mut d: Vec<f32> = terms.iter().map(|t| t.distance(norm)).collect();
        d.sort_by(|a, b| a.total_cmp(b));
        let mut s: Vec<f32> = terms.iter().map(|t| t.structural_term(norm)).collect();
        s.sort_by(|a, b| a.total_cmp(b));
        let founding: Vec<String> = THRESHOLDS
            .iter()
            .map(|&join| {
                let mut reps: Vec<usize> = Vec::new();
                for i in 0..n {
                    let fits = reps.iter().any(|&r| {
                        genomes[i].compatibility_terms(&genomes[r]).distance(norm) < join
                    });
                    if !fits {
                        reps.push(i);
                    }
                }
                format!("{join:.1}:{}", reps.len())
            })
            .collect();
        println!(
            "    {name}/{}: pair distance median {:.3} (p10 {:.3}, p90 {:.3}); structural median {:.3}; pairs under 1.0 {:.1}%",
            label(norm),
            percentile(&d, 0.5),
            percentile(&d, 0.1),
            percentile(&d, 0.9),
            percentile(&s, 0.5),
            100.0 * d.iter().filter(|&&x| x < 1.0).count() as f64 / d.len().max(1) as f64
        );
        println!(
            "      founding-pass species by join: {}",
            founding.join(" ")
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(end_path) = args.first() else {
        eprintln!("usage: species_keying_report <end save.json> [<pre-founding save.json>]");
        std::process::exit(2);
    };

    let state = load(end_path);
    let pop = population(&state);
    let (keyed, table, report) = rekey_population(&pop.genomes);
    let hidden: usize = pop
        .genomes
        .iter()
        .flat_map(|g| &g.neurons)
        .filter(|n| n.neuron_type == clauvolution_genome::NeuronType::Hidden)
        .count();
    let genes: usize = pop.genomes.iter().map(|g| g.connections.len()).sum();
    let legacy_numbers: std::collections::HashSet<u64> = pop
        .genomes
        .iter()
        .flat_map(|g| g.connections.iter().map(|c| c.innovation))
        .collect();
    println!("== {end_path} (tick {})", state.tick);
    println!(
        "  genes {genes}, distinct legacy innovation numbers {}, distinct keys {} connection / {} split; hidden neurons {hidden}: {:?}, fresh ids {}",
        legacy_numbers.len(),
        table.connection_keys(),
        table.split_keys(),
        report,
        table.fresh_hidden_count()
    );
    population_report("legacy", &pop, &pop.genomes);
    population_report("keyed", &pop, &keyed);

    if let Some(founder_path) = args.get(1) {
        let founders = population(&load(founder_path));
        let (keyed, table, _) = rekey_population(&founders.genomes);
        println!(
            "== founders {founder_path}: distinct keys {}",
            table.connection_keys()
        );
        founder_report("legacy", &founders.genomes);
        founder_report("keyed", &keyed);
    }
}
