# Quality assurance

How this project keeps itself honest: which checks exist, where each one runs, what blocks a merge, and what stays a human judgement call. Companion to `docs/CODE_REVIEW_GUIDE.md` (periodic whole-codebase review) and `docs/TODO_GUIDE.md` (how deferred work is tracked). This document is the contract; the Makefile, hooks, and workflows implement it.

## What quality means here

Clauvolution is a solo hobby project whose code is mostly written by agents and whose correctness is largely emergent. Nobody can unit test "predators evolve claws". That shapes what assurance is worth having:

- **The mechanical layer should be free.** Formatting, lints, compilation of every target, and the unit tests that do exist must pass on every change without anyone thinking about it. Agents write most of the code, and agents drift toward whatever the tooling tolerates. Tolerate nothing mechanical.
- **The simulation is the product, so the simulation gets exercised, not just compiled.** A headless run is the cheapest end-to-end check available and it already exists. It should run on every pull request, and longer runs should happen regularly on `main` so that a silent change in dynamics is noticed within days rather than at the next audit.
- **Tests guard accounting, not behaviour.** Energy ledgers, genome operations, NEAT crossover, spatial hashing, serialization round trips, and terrain generation are deterministic functions with right answers. Emergent behaviour is watched, audited, and graphed, not asserted.
- **No coverage target.** A percentage would push tests into the render and UI crates where they cost the most and catch the least.
- **The GUI stays manual.** GitHub runners have no GPU. Anything that needs a window is verified by a human in a release build, as the code review guide already requires.

## Where things stand (measured 2026-09-18 on `main` at 737c825)

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Fails. 12 of the 13 source files have formatting diffs. |
| `cargo clippy --workspace --all-targets` | 74 warnings. 42 are `uninlined_format_args`; 5 are `too_many_arguments` on Bevy systems; the rest are one-offs. |
| `cargo test --workspace` | Compiles every crate as a test target and runs one test (biome counts in `clauvolution_world`). |
| Toolchain | No `rust-toolchain.toml`. README says latest stable; the local toolchain is a nightly from June 2025. Clippy's lint set differs between the two. |
| Hooks | None. |
| CI | None. The repository is public and GitHub Actions is enabled. |
| Headless run | `--headless 1000 --seed 42` takes about 48 seconds wall clock in release on the development machine. |

The clippy count problem is already tracked as `clippy-baseline-toolchain` in `TODO.md`; this work resolves it by making the count zero and enforced.

## The gates

Every check below has one canonical entry point in a `Makefile` at the repository root, so that the hook, CI, the review guide, and an agent typing a command all run the same thing. `make` with no target lists them.

| Target | What it runs | Pre-commit | Pre-push | CI on PR | CI on `main` |
| --- | --- | --- | --- | --- | --- |
| `make fmt-check` | `cargo fmt --all -- --check` | yes | yes | yes | yes |
| `make lint` | `cargo clippy --workspace --all-targets -- -D warnings` | yes | yes | yes | yes |
| `make test` | `cargo test --workspace` | no | yes | yes | yes |
| `make build` | `cargo build --release` | no | no | yes | yes |
| `make smoke` | release binary, `--headless 500 --seed 42`, must exit 0 with a living population | no | no | yes | yes |
| `make probe` | the long headless probe described below | no | no | no | scheduled |
| `make check` | `fmt-check` + `lint` + `test` | | | | |
| `make fmt` | `cargo fmt --all` (fixes, does not check) | | | | |
| `make setup` | installs the hooks (`git config core.hooksPath .githooks`) | | | | |

"Yes" in a CI column means the job blocks the merge. Everything in the pre-commit and pre-push columns is a fast local mirror of the same gate, so a red CI run should be a surprise, not a discovery.

### Formatting

Default `rustfmt` settings, plus a `rustfmt.toml` that pins `edition = "2021"` so that the formatter does not depend on which directory it is invoked from. No style opinions beyond the defaults; the point is zero diffs, not a house style.

The initial format sweep touches almost every file. Its commit hash goes in `.git-blame-ignore-revs` so that `git blame` skips it.

### Lints

Clippy with warnings denied, configured once in `Cargo.toml` under `[workspace.lints]` and inherited by every crate with `[lints] workspace = true`. This makes the configuration part of the code rather than part of the command line, so `cargo clippy` in an editor matches CI.

Lint set: `clippy::all` at warn (denied by `-D warnings`), no `pedantic`. Pedantic on a Bevy codebase produces mostly noise about casts and `must_use`. Two adjustments to the default set:

- `clippy::too_many_arguments` allowed. Bevy systems take one parameter per query or resource and routinely exceed seven. Wrapping them in `SystemParam` structs to satisfy a lint would make the code worse.
- `unsafe_code = "forbid"` under `[workspace.lints.rust]`. There is no `unsafe` today and no reason to expect any.

Rustdoc warnings are not gated. `cargo doc` is not part of the workflow and the lint would mostly complain about intra-doc links to Bevy types.

### Toolchain

A `rust-toolchain.toml` pinning `channel = "stable"` with `components = ["rustfmt", "clippy"]`. Clippy's lint set changes between releases, so the lint gate is only meaningful if everyone runs the same toolchain. Stable rather than a specific version: a hobby project should ride the release train, and when a new stable adds a lint that fires, the fix is a small PR. If that churn turns out to be annoying, pin to a version and bump deliberately.

This changes the local toolchain from nightly to stable. Nothing in the code needs nightly today (the workspace builds on the nightly only because that is what was installed).

### Tests

`cargo test --workspace` runs on every push and every PR. Policy for what needs a test, applied by the author and checked in review:

- **A bug fix in a simulation crate** (`core`, `genome`, `brain`, `body`, `world`, `sim`, `phylogeny`) comes with a regression test when the behaviour is reachable from a unit test. The five accounting defects fixed in PR #6 are the model: each was arithmetic with a right answer (child energy against parent cost, one payout per kill) that a small test could have pinned.
- **A new pure function or data structure** in those crates comes with tests for its edge cases. Genome mutation, crossover, compatibility distance, spatial hash queries, save and load round trips, and terrain classification all qualify.
- **Render, UI, and app-wiring changes** need no tests. They are verified by hand in a release build.
- **Tuning constant changes** need no tests. They need a headless before-and-after summary in the PR body, which the code review guide already asks for.

An integration test crate that spawns a headless app, runs a few hundred ticks, and asserts invariants (no negative energy, population above zero, ledger balances) is on the roadmap and belongs here once same-seed runs are known to be reproducible. Until `determinism-claim-recheck` in `TODO.md` is resolved, such tests can only assert loose bounds, and loose-bound tests that flake are worse than no tests. The `make smoke` gate covers the "does it still run" half of that idea now.

### Pre-commit and pre-push hooks

Hooks live in a versioned `.githooks/` directory and are activated by `make setup`, which sets `core.hooksPath`. That setting lives in the shared `.git/config`, so it applies to every worktree at once. No hook framework; two shell scripts are enough.

- **pre-commit** runs `make fmt-check` and `make lint`. With a warm incremental cache clippy on this workspace takes tens of seconds, which is acceptable for a commit. It does not auto-format: a hook that rewrites the files being committed hides the change from the author, and `make fmt` is one command away.
- **pre-push** runs `make test`. Test compilation is slower than clippy, so it sits at push time rather than commit time. Pushes here open or update a PR, which is the moment the result matters.

`--no-verify` is for recovering from a broken toolchain, not for deferring a fix. If a hook is wrong, fix the hook.

### CI on pull requests

One workflow, `.github/workflows/ci.yml`, triggered on `pull_request` and on `push` to `main`. Jobs:

1. **check**: `make fmt-check`, `make lint`, `make test`, in that order so the cheapest failure reports first.
2. **smoke**: `make build` then `make smoke`. Runs in parallel with `check` because the release build shares nothing with the debug test build.

Both jobs run on `ubuntu-latest` with `dtolnay/rust-toolchain` reading the pinned toolchain file and `Swatinem/rust-cache` caching `target/`. Bevy 0.15 needs `libasound2-dev`, `libudev-dev`, `libwayland-dev`, and `libxkbcommon-dev` installed with `apt` before the build; the headless binary still links the audio and windowing stacks. A cold Bevy build on a two-core runner is slow (expect ten minutes or more the first time); with the cache warm it should be a few minutes. Concurrency is set so a new push to a PR cancels the previous run.

The smoke run uses 500 ticks rather than 1000 because runner cores are slower than the development machine and the check is "does the world still come up and run", not "does it reach the attractor". It asserts exit code 0 and parses the final population from the summary, failing on zero. It uploads the summary as an artifact so a PR reviewer can compare it against `main` without running anything.

The user merges. CI is what makes "the user merges" safe rather than ceremonial: a PR with a red check does not get merged. Whether to enforce that with branch protection (require the two jobs to pass before the merge button works) is a repository setting and the user's call; see open questions.

### Scheduled runs on `main`

This is the one place the project needs more than a standard Rust CI, because the failure mode that matters most is "a merged change quietly moved the attractor" and nothing above catches that. A separate workflow, `.github/workflows/probe.yml`:

- **Trigger**: `push` to `main`, `schedule` weekly (Sunday night), and `workflow_dispatch` for on-demand runs. Main moves only by squash merge, so the push trigger fires a few times a day at most. The weekly run exists to catch toolchain and dependency drift when main is idle.
- **Matrix**: the eight audit seeds from `scripts/attractor_audit.sh` (1, 2, 3, 42, 314, 7, 99, 1000), one job each, 5000 ticks with `--dump-history`. Not the full 15000-tick, two-runs-per-seed audit: that is a deliberate, documented event with a summary written by a human, and it would take many hours on runner hardware. The probe is a canary, not an audit.
- **Pass criteria**: each job exits 0 and ends with a living population. Nothing more. Asserting on strategy mix or species count would encode today's attractor as correct, which is the opposite of what the project wants.
- **Output**: per-seed summary and history CSV uploaded as artifacts, retained for 90 days, so that when a change in dynamics is suspected the evidence is already there. A short job-summary table (final population, plants, foragers, predators, species per seed) is written to the workflow run page.
- **Determinism probe**: one extra job runs seed 42 twice, back to back, and diffs the two summaries. It reports match or mismatch in the job summary and never fails the workflow. This gives `determinism-claim-recheck` a steady stream of data points from a different machine class at no cost.

Failure of the probe workflow is a notification, not a block, because by the time it runs the change is already merged. The response to a red probe is a `TODO.md` entry or a revert, decided by a human.

### Dependency hygiene

Not gated. Cargo.lock is committed, which is correct for a binary. A weekly `cargo audit` run would be nearly free and is worth adding as an advisory job in the probe workflow, but a hobby simulator with no network surface does not need advisories blocking merges. Dependabot is not proposed: Bevy upgrades are project work, not something to accept from a bot.

### Pull request shape

A `.github/pull_request_template.md` carrying the structure the agent contract already requires (`## Why` first, then what changed, resolution markers, validation). The template makes the rule visible to a human opening a PR from the web UI and gives the squash-merge commit body a consistent shape.

## What stays manual

- Anything involving the window, egui panels, input, or screenshots. Verified in a release build per the code review guide.
- Judgement about whether simulation dynamics changed for the better. The probe surfaces data; the Graphs tab and the audits are where the judgement happens.
- Whole-codebase code reviews, on the cadence the review guide sets.

## Rollout

All of this lands on the `tooling/quality-gates` branch, in this order, each step green before the next:

1. **This document.** Reviewed by the user before anything else is built.
2. **Toolchain pin and lint configuration.** `rust-toolchain.toml`, `rustfmt.toml`, `[workspace.lints]` in `Cargo.toml`, `[lints] workspace = true` in each crate. No code changes yet; the workspace will fail its own gates at this point.
3. **Mechanical sweep.** `cargo fmt --all`, then `cargo clippy --fix` for the auto-fixable warnings, then hand fixes for the rest. Committed separately from step 2 so that the diff is reviewable as "formatting only" and "lint fixes only", though the squash merge will fold them together. The format commit hash is recorded in `.git-blame-ignore-revs` after the squash, in a follow-up.
4. **Makefile and hooks.** `Makefile`, `.githooks/pre-commit`, `.githooks/pre-push`, `make setup`. README gains a two-line "Development" section pointing at `make setup` and `make check`.
5. **CI workflow.** `ci.yml` with the `check` and `smoke` jobs. This is the first point at which the PR itself shows a green check.
6. **Probe workflow.** `probe.yml` with the seed matrix and determinism job. Tested via `workflow_dispatch` on the branch before merge.
7. **Docs.** `AGENTS.md` gets one line under Workflow: run `make check` before opening or updating a PR. `docs/CODE_REVIEW_GUIDE.md` baseline section is updated: the clippy count is no longer recorded because it is enforced at zero, and `cargo test --workspace` is described as it actually is. `TODO.md` entry `clippy-baseline-toolchain` is resolved.

### Effect on open branches

Four roadmap branches are open in worktrees. The format sweep will conflict with all of them wherever they touch a reformatted line. The cheapest recovery: after this PR merges, in each worktree run `cargo fmt --all` on the branch, commit, then rebase onto `main`. Two formatted sides of the same file mostly merge cleanly. Their pre-existing clippy warnings will also fail the new gate on rebase; that is a handful of `format!` inlines per branch.

## Done looks like

- `make check` passes on `main` and the hooks are installed by `make setup`.
- Every PR shows two required checks and a smoke summary artifact.
- The probe workflow has run at least once from `workflow_dispatch` on the branch and produced eight seed artifacts and a determinism verdict.
- `docs/CODE_REVIEW_GUIDE.md`, `AGENTS.md`, and `README.md` describe the workflow as built.

## Out of scope

- GUI or screenshot testing in CI. No GPU on runners; software rendering under Bevy is possible but brittle and not worth the maintenance.
- A coverage tool or coverage threshold.
- Integration tests with tight bounds on simulation outcomes. Blocked on `determinism-claim-recheck`.
- Release packaging or the macOS app bundle script. Unchanged by this work.
- Reformatting or lint-fixing the four open roadmap branches. Each branch handles its own rebase.

## Open questions

1. **Stable or a pinned version?** The proposal is `channel = "stable"`. A new stable can turn a PR red through no fault of its own, roughly every six weeks at worst. Pinning avoids that at the cost of remembering to bump.
2. **Clippy in the pre-commit hook, or only at push?** Tens of seconds per commit with a warm cache, minutes with a cold one. The proposal keeps it at commit because the gate it mirrors is the one most likely to fail. If it becomes irritating, move it to pre-push alongside tests.
3. **Branch protection.** Requiring the `check` and `smoke` jobs before merge is a repository setting, reversible, and makes the "red does not merge" rule mechanical. Turn it on?
4. **Probe cadence and length.** Push-to-main plus weekly, 5000 ticks, is the proposal. Longer runs on a schedule are cheap on a public repository but slow to report; the number can move once a first run shows how long a runner takes.
5. **Should the smoke job also run the scripted tour** (`--script tours/demo.json`) under a software renderer? Deferred as out of scope above, but listed here in case it is wanted enough to try.
