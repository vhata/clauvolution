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

/// What a flag expects after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    /// A bare switch, no value.
    None,
    /// One non-negative integer.
    Int,
    /// One number.
    Float,
    /// One path or name.
    Text,
    /// One or more paths, up to the next `--` argument.
    Paths,
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
        Value::Float,
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
        Value::Float,
        "T",
        "override species_compat_threshold",
    ),
    flag(
        "--bite-fraction",
        Value::Float,
        "F",
        "override bite_fraction",
    ),
    flag(
        "--kill-transfer",
        Value::Float,
        "K",
        "override kill_transfer",
    ),
    flag("--photo-drag", Value::Float, "D", "override photo_drag"),
    flag(
        "--leaf-capacity",
        Value::Float,
        "C",
        "override leaf_capacity",
    ),
    flag(
        "--founder-diet-spread",
        Value::Float,
        "S",
        "override founder_diet_spread",
    ),
    flag(
        "--animal-efficiency",
        Value::Float,
        "M",
        "override animal_efficiency (0 = nobody can live by hunting)",
    ),
    flag("--max-energy", Value::Float, "E", "override max_energy"),
    flag(
        "--max-food-density",
        Value::Float,
        "D",
        "override max_food_density",
    ),
    flag(
        "--population-ceiling",
        Value::Int,
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
                    Value::Float => value.parse::<f32>().is_ok_and(f32::is_finite),
                    _ => true,
                };
                if !ok {
                    let what = if kind == Value::Int {
                        "a non-negative integer"
                    } else {
                        "a number"
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
