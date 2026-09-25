//! The command-line flag table and the check that runs before anything else.
//!
//! `main` still reads each flag's value where it always did; this module only
//! decides whether the argument list is acceptable. It runs before the app is
//! built, so `--help`, an unknown flag, a missing or unparsable value, or a
//! repeated flag exits with usage text instead of opening a window. That
//! matters most for headless runs: a typo in `--headless` used to launch the
//! GUI as though no flags had been given.
//!
//! `FLAGS` is the one list of known flags. A test checks it against the
//! README's flag list in both directions so the two cannot drift.

use clauvolution_core::MIN_DIET_EFFICIENCY_EXPONENT;

/// What a flag expects after it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    /// A bare switch, no value.
    None,
    /// One non-negative integer that fits a `u64`.
    Int,
    /// One non-negative integer that fits a `u32`. The check must use the
    /// same width as the flag's reader in `main`, or an out-of-range value
    /// passes here and is then silently dropped there.
    U32,
    /// One finite number within a range.
    Float(Range),
    /// One path or name.
    Text,
    /// One or more paths, up to the next `--` argument.
    Paths,
}

/// The values a `Value::Float` flag accepts. Each tuning flag's range comes
/// from what its `SimConfig` field means, so a value the sim would misread
/// (a bite larger than the plant, a clock running backwards) is refused here,
/// before the app starts, rather than clamped downstream.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Range {
    /// Zero or more.
    NonNegative,
    /// Strictly more than zero.
    Positive,
    /// A fraction: zero to one inclusive.
    Unit,
    /// At least the given floor.
    AtLeast(f32),
}

impl Range {
    fn contains(self, v: f32) -> bool {
        match self {
            Range::NonNegative => v >= 0.0,
            Range::Positive => v > 0.0,
            Range::Unit => (0.0..=1.0).contains(&v),
            Range::AtLeast(min) => v >= min,
        }
    }

    fn describe(self) -> String {
        match self {
            Range::NonNegative => "a number >= 0".to_string(),
            Range::Positive => "a number > 0".to_string(),
            Range::Unit => "a number from 0 to 1".to_string(),
            Range::AtLeast(min) => format!("a number >= {min}"),
        }
    }
}

pub struct Flag {
    pub name: &'static str,
    pub value: Value,
    /// Placeholder shown in the usage text.
    pub metavar: &'static str,
    pub help: &'static str,
}

const fn flag(name: &'static str, value: Value, metavar: &'static str, help: &'static str) -> Flag {
    Flag {
        name,
        value,
        metavar,
        help,
    }
}

/// Every flag the binary accepts, in usage order.
pub const FLAGS: &[Flag] = &[
    flag("--help", Value::None, "", "print this usage and exit"),
    flag(
        "--load",
        Value::Text,
        "DIR",
        "load DIR/save.json, a saved session directory",
    ),
    flag(
        "--seed-with",
        Value::Paths,
        "FILE...",
        "add exported creatures to the founders (repeatable; ignored with --load)",
    ),
    flag("--seed", Value::Int, "N", "terrain and simulation seed"),
    flag(
        "--script",
        Value::Text,
        "FILE",
        "run a scripted tour with egui-aware screenshots",
    ),
    flag(
        "--screenshot",
        Value::None,
        "",
        "legacy fixed screenshot tour",
    ),
    flag(
        "--headless",
        Value::Int,
        "TICKS",
        "run TICKS ticks without a window and print a summary",
    ),
    flag(
        "--speed",
        Value::Float(Range::Positive),
        "N",
        "headless ticks per frame (default 10)",
    ),
    flag(
        "--save-as",
        Value::Text,
        "NAME",
        "headless: write sessions/NAME/save.json at the end",
    ),
    flag(
        "--dump-history",
        Value::Text,
        "FILE",
        "headless: write 1 Hz population history as CSV",
    ),
    flag(
        "--species-threshold",
        Value::Float(Range::NonNegative),
        "T",
        "override species_compat_threshold",
    ),
    flag(
        "--bite-fraction",
        Value::Float(Range::Unit),
        "F",
        "override bite_fraction",
    ),
    flag(
        "--bite-reach",
        Value::Float(Range::NonNegative),
        "R",
        "override bite_reach",
    ),
    flag(
        "--mouthless-bite",
        Value::Float(Range::Unit),
        "B",
        "override mouthless_bite_bonus",
    ),
    flag(
        "--kill-transfer",
        Value::Float(Range::Unit),
        "K",
        "override both kill shares (animal and plant victims)",
    ),
    flag(
        "--kill-transfer-animal",
        Value::Float(Range::Unit),
        "K",
        "override kill_transfer_animal",
    ),
    flag(
        "--kill-transfer-plant",
        Value::Float(Range::Unit),
        "K",
        "override kill_transfer_plant",
    ),
    flag(
        "--strike-cost",
        Value::Float(Range::NonNegative),
        "C",
        "override strike_cost",
    ),
    flag(
        "--photo-drag",
        Value::Float(Range::NonNegative),
        "D",
        "override photo_drag",
    ),
    flag(
        "--leaf-capacity",
        Value::Float(Range::NonNegative),
        "C",
        "override leaf_capacity",
    ),
    flag(
        "--founder-diet-spread",
        Value::Float(Range::Unit),
        "S",
        "override founder_diet_spread",
    ),
    flag(
        "--animal-efficiency",
        Value::Float(Range::NonNegative),
        "M",
        "override animal_efficiency (0 = nobody can live by hunting)",
    ),
    flag(
        "--diet-exponent",
        Value::Float(Range::AtLeast(MIN_DIET_EFFICIENCY_EXPONENT)),
        "X",
        "override diet_efficiency_exponent (at least 1; 2 = squared curve)",
    ),
    flag(
        "--max-energy",
        Value::Float(Range::Positive),
        "E",
        "override max_energy",
    ),
    flag(
        "--max-food-density",
        Value::Float(Range::NonNegative),
        "D",
        "override max_food_density",
    ),
    flag(
        "--population-ceiling",
        Value::U32,
        "N",
        "override population_ceiling",
    ),
];

/// What `main` should do after the check.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Run,
    Help,
}

fn lookup(name: &str) -> Option<&'static Flag> {
    FLAGS.iter().find(|f| f.name == name)
}

/// Check the full argument list, program name included. Returns the error
/// message for the first problem found.
pub fn check(args: &[String]) -> Result<Outcome, String> {
    let mut seen: Vec<&str> = Vec::new();
    let mut help = false;
    let mut i = 1;
    while i < args.len() {
        let arg = args[i].as_str();
        i += 1;
        // Older macOS LaunchServices passes a process serial number to apps
        // opened from Finder. It is not ours to reject.
        if arg.starts_with("-psn_") {
            continue;
        }
        let Some(flag) = lookup(arg) else {
            return Err(if arg.starts_with('-') {
                format!("unknown flag {arg}")
            } else {
                format!("unexpected argument {arg}")
            });
        };
        if flag.value != Value::Paths && seen.contains(&flag.name) {
            return Err(format!("{} given more than once", flag.name));
        }
        seen.push(flag.name);
        match flag.value {
            Value::None => {
                if flag.name == "--help" {
                    help = true;
                }
            }
            Value::Paths => {
                let start = i;
                while i < args.len() && !args[i].starts_with("--") {
                    i += 1;
                }
                if i == start {
                    return Err(format!("{} needs at least one {}", flag.name, flag.metavar));
                }
            }
            kind => {
                let Some(value) = args.get(i).filter(|v| !v.starts_with("--")) else {
                    return Err(format!("{} needs a value {}", flag.name, flag.metavar));
                };
                i += 1;
                let ok = match kind {
                    Value::Int => value.parse::<u64>().is_ok(),
                    Value::U32 => value.parse::<u32>().is_ok(),
                    Value::Float(range) => value
                        .parse::<f32>()
                        .is_ok_and(|v| v.is_finite() && range.contains(v)),
                    _ => true,
                };
                if !ok {
                    let what = match kind {
                        Value::Int => "a non-negative integer".to_string(),
                        Value::U32 => format!("a non-negative integer up to {}", u32::MAX),
                        Value::Float(range) => range.describe(),
                        _ => unreachable!("only numeric kinds are checked"),
                    };
                    return Err(format!("{} expects {what}, got {value:?}", flag.name));
                }
            }
        }
    }
    Ok(if help { Outcome::Help } else { Outcome::Run })
}

/// Usage text generated from `FLAGS`.
pub fn usage() -> String {
    let mut out = String::from(
        "Usage: clauvolution [FLAGS]\n\n\
         With no flags, opens the simulator window on a random seed.\n\
         With --headless, runs without a window. See README.md for examples.\n\n\
         Flags:\n",
    );
    let lefts: Vec<String> = FLAGS
        .iter()
        .map(|f| {
            if f.metavar.is_empty() {
                f.name.to_string()
            } else {
                format!("{} {}", f.name, f.metavar)
            }
        })
        .collect();
    let width = lefts.iter().map(String::len).max().unwrap_or(0);
    for (left, f) in lefts.iter().zip(FLAGS) {
        out.push_str(&format!("  {left:width$}  {}\n", f.help));
    }
    out.push_str("\nEnvironment:\n  CLAU_WORKERS=N  cap the compute thread pool (default 6)\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        std::iter::once("clauvolution")
            .chain(list.iter().copied())
            .map(String::from)
            .collect()
    }

    #[test]
    fn accepts_no_flags_and_the_readme_examples() {
        assert_eq!(check(&args(&[])), Ok(Outcome::Run));
        assert_eq!(
            check(&args(&[
                "--headless",
                "1000",
                "--seed",
                "42",
                "--dump-history",
                "h.csv",
                "--bite-fraction",
                "0.2",
                "--population-ceiling",
                "900",
            ])),
            Ok(Outcome::Run)
        );
        assert_eq!(
            check(&args(&[
                "--seed-with",
                "a.json",
                "b.json",
                "--seed",
                "1",
                "--seed-with",
                "c.json",
                "--screenshot",
            ])),
            Ok(Outcome::Run)
        );
    }

    #[test]
    fn help_wins_wherever_it_appears() {
        assert_eq!(check(&args(&["--help"])), Ok(Outcome::Help));
        assert_eq!(
            check(&args(&["--headless", "10", "--help"])),
            Ok(Outcome::Help)
        );
    }

    #[test]
    fn rejects_unknown_flags_and_stray_arguments() {
        assert_eq!(
            check(&args(&["--headles", "10"])),
            Err("unknown flag --headles".into())
        );
        assert_eq!(check(&args(&["-h"])), Err("unknown flag -h".into()));
        assert_eq!(
            check(&args(&["--seed", "1", "2"])),
            Err("unexpected argument 2".into())
        );
        // An unknown flag after --help is still an error, so a typo is never
        // hidden behind a usage screen that exits 0.
        assert!(check(&args(&["--help", "--bogus"])).is_err());
    }

    #[test]
    fn rejects_missing_and_malformed_values() {
        assert!(check(&args(&["--headless"])).is_err());
        assert!(check(&args(&["--headless", "lots"])).is_err());
        assert!(check(&args(&["--headless", "-5"])).is_err());
        assert!(check(&args(&["--speed", "fast"])).is_err());
        assert!(check(&args(&["--speed", "NaN"])).is_err());
        assert!(check(&args(&["--seed-with"])).is_err());
        assert!(check(&args(&["--seed-with", "--seed", "1"])).is_err());
        assert!(check(&args(&["--load"])).is_err());
        assert_eq!(
            check(&args(&["--save-as", "--headless", "10"])),
            Err("--save-as needs a value NAME".into())
        );
    }

    #[test]
    fn integer_checks_match_the_readers_width() {
        // --population-ceiling is read as a u32, so a value that only fits
        // a u64 must fail here rather than be dropped by the reader.
        assert_eq!(
            check(&args(&["--population-ceiling", "4294967295"])),
            Ok(Outcome::Run)
        );
        assert_eq!(
            check(&args(&["--population-ceiling", "4294967296"])),
            Err(
                "--population-ceiling expects a non-negative integer up to 4294967295, \
                 got \"4294967296\""
                    .into()
            )
        );
        // --seed and --headless are read as u64.
        assert_eq!(
            check(&args(&["--seed", "18446744073709551615"])),
            Ok(Outcome::Run)
        );
        assert!(check(&args(&["--seed", "18446744073709551616"])).is_err());
    }

    #[test]
    fn positive_range_refuses_zero_and_below() {
        // A negative --speed used to pass the check and then panic in
        // bevy_time; zero would never advance a headless run.
        assert_eq!(check(&args(&["--speed", "0.001"])), Ok(Outcome::Run));
        assert_eq!(
            check(&args(&["--speed", "-1"])),
            Err("--speed expects a number > 0, got \"-1\"".into())
        );
        assert!(check(&args(&["--speed", "0"])).is_err());
        assert!(check(&args(&["--max-energy", "0"])).is_err());
        assert_eq!(check(&args(&["--max-energy", "500"])), Ok(Outcome::Run));
    }

    #[test]
    fn non_negative_range_accepts_zero_and_refuses_below() {
        // `--animal-efficiency 0` is a documented experiment.
        assert_eq!(
            check(&args(&["--animal-efficiency", "0"])),
            Ok(Outcome::Run)
        );
        assert_eq!(check(&args(&["--strike-cost", "40"])), Ok(Outcome::Run));
        assert_eq!(
            check(&args(&["--strike-cost", "-0.5"])),
            Err("--strike-cost expects a number >= 0, got \"-0.5\"".into())
        );
        assert!(check(&args(&["--species-threshold", "-1"])).is_err());
    }

    #[test]
    fn unit_range_accepts_its_ends_and_refuses_outside() {
        for name in [
            "--bite-fraction",
            "--mouthless-bite",
            "--kill-transfer",
            "--kill-transfer-animal",
            "--kill-transfer-plant",
            "--founder-diet-spread",
        ] {
            assert_eq!(check(&args(&[name, "0"])), Ok(Outcome::Run), "{name} 0");
            assert_eq!(check(&args(&[name, "1"])), Ok(Outcome::Run), "{name} 1");
            assert!(check(&args(&[name, "1.01"])).is_err(), "{name} 1.01");
            assert!(check(&args(&[name, "-0.01"])).is_err(), "{name} -0.01");
        }
        assert_eq!(
            check(&args(&["--bite-fraction", "1.5"])),
            Err("--bite-fraction expects a number from 0 to 1, got \"1.5\"".into())
        );
    }

    #[test]
    fn rejects_a_diet_exponent_below_one() {
        assert_eq!(check(&args(&["--diet-exponent", "1.5"])), Ok(Outcome::Run));
        assert_eq!(check(&args(&["--diet-exponent", "1"])), Ok(Outcome::Run));
        assert_eq!(
            check(&args(&["--diet-exponent", "0.5"])),
            Err("--diet-exponent expects a number >= 1, got \"0.5\"".into())
        );
    }

    #[test]
    fn every_float_flag_accepts_its_shipped_default() {
        let config = clauvolution_core::SimConfig::default();
        let defaults = [
            ("--species-threshold", config.species_compat_threshold),
            ("--bite-fraction", config.bite_fraction),
            ("--bite-reach", config.bite_reach),
            ("--mouthless-bite", config.mouthless_bite_bonus),
            ("--kill-transfer", config.kill_transfer_animal),
            ("--kill-transfer-animal", config.kill_transfer_animal),
            ("--kill-transfer-plant", config.kill_transfer_plant),
            ("--strike-cost", config.strike_cost),
            ("--photo-drag", config.photo_drag),
            ("--leaf-capacity", config.leaf_capacity_per_tile),
            ("--founder-diet-spread", config.founder_diet_spread),
            ("--animal-efficiency", config.animal_efficiency_multiplier),
            ("--diet-exponent", config.diet_efficiency_exponent),
            ("--max-energy", config.max_organism_energy),
            ("--max-food-density", config.max_food_density),
        ];
        for f in FLAGS {
            if !matches!(f.value, Value::Float(_)) || f.name == "--speed" {
                continue;
            }
            let (_, v) = defaults
                .iter()
                .find(|(n, _)| *n == f.name)
                .unwrap_or_else(|| panic!("{} has no default listed here", f.name));
            assert_eq!(
                check(&args(&[f.name, &v.to_string()])),
                Ok(Outcome::Run),
                "{} refuses its own default {v}",
                f.name
            );
        }
    }

    #[test]
    fn rejects_a_repeated_single_value_flag() {
        assert_eq!(
            check(&args(&["--seed", "1", "--seed", "2"])),
            Err("--seed given more than once".into())
        );
    }

    #[test]
    fn ignores_the_macos_process_serial_number() {
        assert_eq!(check(&args(&["-psn_0_12345"])), Ok(Outcome::Run));
    }

    #[test]
    fn usage_lists_every_flag() {
        let text = usage();
        for f in FLAGS {
            assert!(text.contains(f.name), "usage is missing {}", f.name);
        }
    }

    /// Every `--flag` in the README's Running section, except cargo's own.
    fn readme_flags() -> Vec<String> {
        let readme = include_str!("../../../README.md");
        let start = readme
            .find("## Running")
            .expect("README has a Running section");
        let section = &readme[start..];
        let end = section[3..].find("\n## ").map_or(section.len(), |e| e + 3);
        let section = &section[..end];
        let mut found = Vec::new();
        let mut rest = section;
        while let Some(pos) = rest.find("--") {
            let tail = &rest[pos + 2..];
            let len = tail
                .find(|c: char| !(c.is_ascii_lowercase() || c == '-'))
                .unwrap_or(tail.len());
            let name = format!("--{}", &tail[..len]);
            if len > 0 && name != "--release" && !found.contains(&name) {
                found.push(name);
            }
            rest = &tail[len..];
        }
        found
    }

    #[test]
    fn readme_and_flag_table_agree() {
        let documented = readme_flags();
        for name in &documented {
            assert!(
                lookup(name).is_some(),
                "README documents {name}, which the parser does not know"
            );
        }
        for f in FLAGS {
            assert!(
                documented.iter().any(|d| d == f.name),
                "{} is accepted but not documented in README's Running section",
                f.name
            );
        }
    }
}
