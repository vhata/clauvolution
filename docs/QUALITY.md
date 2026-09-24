# Quality assurance

How this project keeps itself honest: which checks exist, where each one runs, what blocks a merge, and what stays a human judgement call. Companion to `docs/CODE_REVIEW_GUIDE.md` (periodic whole-codebase review) and `docs/TODO_GUIDE.md` (how deferred work is tracked). This document is the contract; the scripts, hooks, and workflows implement it.

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

Every check is a standalone shell script in `scripts/`, runnable on its own with no arguments and exiting non-zero on failure. The hooks, CI, the review guide, and an agent typing a command all run the same scripts. `scripts/check.sh` is the entry point that runs the local gates in order; there is no Makefile.

| Script | What it runs | Pre-commit | Pre-push | CI on PR | CI on `main` |
| --- | --- | --- | --- | --- | --- |
| `scripts/fmt-check.sh` | `cargo fmt --all -- --check` | yes | no | yes | yes |
| `scripts/lint.sh` | `cargo clippy --workspace --all-targets -- -D warnings` | yes | no | yes | yes |
| `scripts/test.sh` | `cargo test --workspace` | no | yes, skipped for docs-only pushes | yes | yes |
| `scripts/pre-push-test.sh` | the pre-push wrapper: `scripts/test.sh` unless the push changes only docs | | | | |
| `scripts/smoke.sh` | release build, then `--headless 500 --seed 42`, must exit 0 with a living population | no | no | yes | yes |
| `scripts/probe.sh SEED TICKS OUT_DIR` | one long headless run, described below | no | no | no | scheduled |
| `scripts/check.sh` | `fmt-check` + `lint` + `test` | | | | |
| `scripts/setup.sh` | installs lefthook's hooks into the repository | | | | |

"Yes" in a CI column means the job blocks the merge. Everything in the pre-commit and pre-push columns is a fast local mirror of the same gate, so a red CI run should be a surprise, not a discovery. Each local gate runs at one hook, not both: the format check and clippy already run on every commit, so a push normally carries only commits that already passed them, and repeating them at push time would add wait for little gain. A commit made with the hooks bypassed (`LEFTHOOK=0` or `--no-verify`) is caught by CI instead. The authoritative list of what runs where is `lefthook.yml`.

Timings measured on the development machine with a warm cache, which decide what runs at commit time: the format check takes well under a second, and clippy after editing a root crate takes about one second. Both are under the two-second budget for a commit hook. A cold cache (after a toolchain change or `cargo clean`) takes minutes once; that is the price of the first commit after this lands.

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

A `rust-toolchain.toml` pinning an exact stable release, `channel = "1.98.1"` (the current stable at the time of writing), with `components = ["rustfmt", "clippy"]`. Clippy's lint set changes between releases, so the lint gate is only meaningful if everyone runs the same toolchain, and a new stable release must never turn an unrelated PR red. Bumping the pin is its own small PR: edit the file, run the lint script, fix whatever the new release flags.

This changes the local toolchain from nightly to stable. Nothing in the code needs nightly today (the workspace builds on the nightly only because that is what was installed). `rustup` reads the file and installs the pinned toolchain on first use.

### Tests

`cargo test --workspace` runs on every push and every PR. Policy for what needs a test, applied by the author and checked in review:

- **A bug fix in a simulation crate** (`core`, `genome`, `brain`, `body`, `world`, `sim`, `phylogeny`) comes with a regression test when the behaviour is reachable from a unit test. The five accounting defects fixed in PR #6 are the model: each was arithmetic with a right answer (child energy against parent cost, one payout per kill) that a small test could have pinned.
- **A new pure function or data structure** in those crates comes with tests for its edge cases. Genome mutation, crossover, compatibility distance, spatial hash queries, save and load round trips, and terrain classification all qualify.
- **Render, UI, and app-wiring changes** need no tests. They are verified by hand in a release build.
- **Tuning constant changes** need no tests. They need a headless before-and-after summary in the PR body, which the code review guide already asks for.

An integration test crate that spawns a headless app, runs a few hundred ticks, and asserts invariants (no negative energy, population above zero, ledger balances) is on the roadmap and belongs here. Same-seed headless runs are bit-identical since 2026-09-20 (see "Headless runs are deterministic" in `DECISIONS.md`), so such tests can assert exact values for a given seed rather than loose bounds; a test that pins an exact value will need updating whenever a simulation rule changes, which is the intended signal. The `scripts/smoke.sh` gate covers the "does it still run" half of that idea now.

### Pre-commit and pre-push hooks

Hooks are managed by [lefthook](https://github.com/evilmartians/lefthook), configured in a versioned `lefthook.yml` at the repository root. Lefthook is the current standard for polyglot repositories: a single Go binary with no runtime dependency (husky needs Node, the `pre-commit` framework needs Python), a declarative config that lives in git, and `lefthook install` to wire it up. Its config here does nothing clever: each hook calls the matching script in `scripts/`, so the hook and the CI job are the same code. `scripts/setup.sh` runs `lefthook install` and tells you how to get lefthook if it is missing (`brew install lefthook`).

The generated hooks live in `.git/hooks`, which is shared by every worktree of this repository, so one install covers all of them.

- **pre-commit** runs `scripts/fmt-check.sh` and `scripts/lint.sh`, in parallel. Both finish in about a second with a warm cache. The hook does not auto-format: a hook that rewrites the files being committed hides the change from the author, and `cargo fmt --all` is one command away.
- **pre-push** runs `scripts/test.sh` through `scripts/pre-push-test.sh`. Test compilation is much slower than clippy, so it sits at push time. Pushes here open or update a PR, which is the moment the result matters.

The pre-push wrapper skips the tests when a push changes only documentation. Git hands the hook one line per pushed ref; the wrapper diffs each ref against the remote's old tip, or against its merge-base with `origin/main` when the ref is new, and exits early only when every changed path is on the allowlist: `*.md` anywhere under `docs/`, `plans/` or `review/`, plus the top-level `TODO.md`. `README.md` is not on the list, because `crates/clauvolution_app/src/cli.rs` reads it with `include_str!` for the test that keeps the README's flag table and the parser in step. Anything it cannot account for runs the tests: a path outside the list (including `README.md`, `CLAUDE.md`, `AGENTS.md`, and markdown inside a crate), a remote tip it does not have locally, a new ref with no `origin/main` to measure from, or empty or malformed input. The diff lists both sides of a rename, so moving a source file into `docs/` still counts as a code change. `FORCE_TESTS=1 git push` always runs the tests, and `scripts/test.sh` runs them directly. Separately, lefthook itself skips a pre-push job when `HEAD` has no changes against its push target; that is lefthook's behaviour, not this wrapper's.

Skipping fits the gate policy because the pre-push run is a local mirror of a CI gate, not the gate itself. CI runs `scripts/test.sh` on every pull request and on every push to `main`, including plans pushed straight to `main`, so every commit on `main` still gets a full test run either way. A docs-only push leaves the code identical to a commit that has already been through the gates, so the local run would compile and test exactly what was tested before; its only effect was close to a minute of waiting on pushes that are mostly plans and TODO edits. The allowlist is safe only while no crate reads an allowlisted file at compile time (`include_str!`, `include_bytes!`, or `#[doc = include_str!(...)]`). The wrapper checks this itself before it skips: it greps the crate sources for those macros, resolves each string literal against the including file's directory, and runs the tests if any target is on the allowlist or if a call has no literal on the same line to resolve (a multi-line call, or one built with `concat!` or `env!`). The check prints the offending file, so the fix is to drop the matching pattern from `is_docs_path`. It exists because the README's flag-table test (#59) and the wrapper's README entry (#61) landed separately, and until this check a README-only push skipped a test that CI then failed.

`--no-verify` is for recovering from a broken toolchain, not for deferring a fix. If a hook is wrong, fix the hook.

### CI on pull requests

One workflow, `.github/workflows/ci.yml`, triggered on `pull_request` and on `push` to `main`. Jobs:

1. **check**: `scripts/fmt-check.sh`, `scripts/lint.sh`, `scripts/test.sh`, in that order so the cheapest failure reports first.
2. **smoke**: `scripts/smoke.sh`, which builds release and runs the short headless check. Runs in parallel with `check` because the release build shares nothing with the debug test build.

Both jobs run on `ubuntu-latest` with `dtolnay/rust-toolchain` reading the pinned toolchain file and `Swatinem/rust-cache` caching `target/`. Bevy 0.15 needs `libasound2-dev`, `libudev-dev`, `libwayland-dev`, and `libxkbcommon-dev` installed with `apt` before the build; the headless binary still links the audio and windowing stacks. A cold Bevy build on a two-core runner is slow (expect ten minutes or more the first time); with the cache warm it should be a few minutes. Concurrency is set so a new push to a PR cancels the previous run.

The smoke run uses 500 ticks rather than 1000 because runner cores are slower than the development machine and the check is "does the world still come up and run", not "does it reach the attractor". It asserts exit code 0 and parses the final population from the summary, failing on zero. It uploads the summary as an artifact so a PR reviewer can compare it against `main` without running anything.

The user merges. CI is what makes "the user merges" safe rather than ceremonial: a PR with a red check does not get merged. Whether to enforce that with branch protection (require the two jobs to pass before the merge button works) is a repository setting and the user's call; see open questions.

### Scheduled runs on `main`

This is the one place the project needs more than a standard Rust CI, because the failure mode that matters most is "a merged change quietly moved the attractor" and nothing above catches that. A separate workflow, `.github/workflows/probe.yml`:

- **Trigger**: `push` to `main`, `schedule` weekly (Sunday night), `workflow_dispatch` for on-demand runs, and `pull_request` when the probe workflow or its script changes, so edits to the probe are exercised before merge. Main moves only by squash merge, so the push trigger fires a few times a day at most.
- **Two shapes, one script.** Each matrix job runs `scripts/probe.sh SEED TICKS OUT_DIR`, which does one headless run with `--dump-history` and writes the summary and CSV to the output directory. The eight seeds are the audit seeds from `scripts/attractor_audit.sh` (1, 2, 3, 42, 314, 7, 99, 1000). Runner jobs are independent machines, so the matrix gives the parallelism the audit script gets from background processes.
  - **On push**: eight seeds, one run each, 5000 ticks. A compromise between time and signal: long enough to be past the early transient the audits describe, short enough to report within the hour on runner hardware. The first run on the PR that added the workflow took about five minutes per 5000-tick run on `ubuntu-latest`, plus around eight minutes for the cold release build, so the whole push-triggered probe finished in about fifteen minutes.
  - **Weekly**: the full audit shape, eight seeds, two runs each, 15000 ticks, sixteen jobs. This is the "as long as necessary for best signal" run. It matches the audit protocol, so its artifacts are directly comparable to the entries under `docs/audits/`, and it catches toolchain and dependency drift when main has been idle. At the measured rate a 15000-tick run should take around fifteen minutes per job, well inside GitHub's six-hour job limit, and the runner minutes are free on a public repository.
- **Pass criteria**: each job exits 0 and ends with a living population. Nothing more. Asserting on strategy mix or species count would encode today's attractor as correct, which is the opposite of what the project wants.
- **Output**: per-run summary and history CSV uploaded as artifacts, retained for 90 days, so that when a change in dynamics is suspected the evidence is already there. A short job-summary table (final population, plants, foragers, predators, species per seed) is written to the workflow run page.
- **Determinism probe**: one extra job runs seed 42 twice, back to back, and diffs the two summaries. It reports match or mismatch in the job summary and never fails the workflow. Headless runs are deterministic since 2026-09-20, so a mismatch here is a regression: something has let wall-clock time, thread scheduling, or hash iteration order back into the simulation.

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
4. **Scripts and hooks.** The scripts listed in the gates table, `lefthook.yml`, and `scripts/setup.sh`. README gains a short "Development" section pointing at `scripts/setup.sh` and `scripts/check.sh`.
5. **CI workflow.** `ci.yml` with the `check` and `smoke` jobs. This is the first point at which the PR itself shows a green check.
6. **Probe workflow.** `probe.yml` with the seed matrix and determinism job. Its `pull_request` trigger fires on this PR because the workflow file is new, which is how it gets tested before merge.
7. **Docs.** `AGENTS.md` gets one line under Workflow: run `scripts/check.sh` before opening or updating a PR. `docs/CODE_REVIEW_GUIDE.md` baseline section is updated: the clippy count is no longer recorded because it is enforced at zero, and `cargo test --workspace` is described as it actually is. `TODO.md` entry `clippy-baseline-toolchain` is resolved and a low-priority entry for the scripted tour under a software renderer is added.

### Effect on open branches

Four roadmap branches are open in worktrees. The format sweep will conflict with all of them wherever they touch a reformatted line. The cheapest recovery: after this PR merges, in each worktree run `cargo fmt --all` on the branch, commit, then rebase onto `main`. Two formatted sides of the same file mostly merge cleanly. Their pre-existing clippy warnings will also fail the new gate on rebase; that is a handful of `format!` inlines per branch.

Once `scripts/setup.sh` has installed the hooks, they apply to every worktree, so a commit on one of those branches fails the format check until the branch has been formatted. For the rebase window itself, `LEFTHOOK=0 git commit` skips the hooks for one command; that is the sanctioned use of a bypass, not a habit.

## Done looks like

- `scripts/check.sh` passes on `main` and the hooks are installed by `scripts/setup.sh`.
- Every PR shows two required checks and a smoke summary artifact.
- The probe workflow has run at least once on the branch and produced eight seed artifacts and a determinism verdict.
- `docs/CODE_REVIEW_GUIDE.md`, `AGENTS.md`, and `README.md` describe the workflow as built.

## Out of scope

- GUI or screenshot testing in CI. No GPU on runners; software rendering under Bevy is possible but brittle. Filed as `ci-scripted-tour-software-renderer` in `TODO.md` at low priority.
- A coverage tool or coverage threshold.
- Integration tests with tight bounds on simulation outcomes. Unblocked by headless determinism (2026-09-20); still on the roadmap rather than in this work.
- Release packaging or the macOS app bundle script. Unchanged by this work.
- Reformatting or lint-fixing the four open roadmap branches. Each branch handles its own rebase.

## Decisions

Settled with the user on 2026-09-18, after the first draft of this document.

- **Standalone scripts, no Makefile.** Each check is its own script; `scripts/check.sh` is the entry point.
- **Lefthook for hooks** rather than raw `.githooks` or husky.
- **Exact stable version pinned** in `rust-toolchain.toml`, bumped deliberately.
- **Clippy at commit time**, because it measured under the two-second budget with a warm cache. If a later toolchain or a much larger workspace pushes it past that, it moves to pre-push.
- **Probe length**: weekly runs use the full audit shape for best signal; push runs use 5000 ticks as the time-versus-benefit compromise.
- **Scripted tour in CI**: not now. Filed as a low-priority TODO.

## Branch protection

Enabled on `main` on 2026-09-18, as a classic branch protection rule: the `check` and `smoke` jobs must pass before a pull request can be merged, the rule is not enforced for administrators, and there are no push restrictions, review requirements, or "branch must be up to date" requirement.

Two consequences follow from how GitHub applies required status checks, and both are intended:

- Required checks also apply to commits pushed directly, so without an exemption a plan committed straight to `main` would be rejected. Not enforcing the rule for administrators is what keeps direct pushes working for the repository owner, whose credentials every push here uses.
- The same exemption means the merge button offers an administrator a "merge without waiting for requirements" bypass on a red pull request. It is an extra deliberate click with a warning, not the default, which is the right amount of friction for a solo project.

Force pushes and branch deletion on `main` are refused for everyone.
