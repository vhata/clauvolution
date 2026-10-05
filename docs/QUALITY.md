# Quality assurance

How this project keeps itself honest: which checks exist, where each one runs, what blocks a merge, and what stays a human judgement call. Companion to `docs/CODE_REVIEW_GUIDE.md` (periodic whole-codebase review) and `docs/TODO_GUIDE.md` (how deferred work is tracked). This document describes the gates as they are; the scripts, hooks, and workflows implement it. The choices behind the setup are recorded in `docs/DECISIONS.md` under "Tooling & workflow".

## What quality means here

Clauvolution is a solo hobby project whose code is mostly written by agents and whose correctness is largely emergent. Nobody can unit test "predators evolve claws". That shapes what assurance is worth having:

- **The mechanical layer should be free.** Formatting, lints, compilation of every target, and the unit tests that do exist must pass on every change without anyone thinking about it. Agents write most of the code, and agents drift toward whatever the tooling tolerates. Tolerate nothing mechanical.
- **The simulation is the product, so the simulation gets exercised, not just compiled.** A headless run is the cheapest end-to-end check available. A short one runs on every pull request, and longer runs happen regularly on `main` so that a silent change in dynamics is noticed within days rather than at the next audit.
- **Tests guard accounting, not behaviour.** Energy ledgers, genome operations, NEAT crossover, spatial hashing, serialization round trips, and terrain generation are deterministic functions with right answers. Emergent behaviour is watched, audited, and graphed, not asserted.
- **No coverage target.** A percentage would push tests into the render and UI crates where they cost the most and catch the least.
- **The GUI stays manual.** GitHub runners have no GPU. Anything that needs a window is verified by a human in a release build, as the code review guide requires.

## The gates

Every check is a standalone shell script in `scripts/`, runnable on its own and exiting non-zero on failure. The hooks, CI, the review guide, and an agent typing a command all run the same scripts. `scripts/check.sh` is the entry point that runs the local gates in order; there is no Makefile.

| Script | What it runs | Pre-commit | Pre-push | CI on PR | CI on `main` |
| --- | --- | --- | --- | --- | --- |
| `scripts/fmt-check.sh` | `cargo fmt --all -- --check` | yes | no | yes | yes |
| `scripts/lint.sh` | `cargo clippy --workspace --all-targets -- -D warnings` | yes | no | yes | yes |
| `scripts/test.sh` | `cargo test --workspace` | no | yes, skipped for docs-only pushes | yes | yes |
| `scripts/pre-push-test.sh` | the pre-push wrapper: `scripts/test.sh` unless the push changes only docs | | | | |
| `scripts/smoke.sh` | release build, then `--headless 500 --seed 42`, must exit 0 with a living population | no | no | yes | yes |
| `scripts/probe.sh SEED TICKS OUT_DIR [LABEL]` | one long headless run, described below | no | no | only when `probe.yml` or `scripts/probe.sh` changes, advisory | after merge and weekly, advisory |
| `scripts/check.sh` | `fmt-check` + `lint` + `test` | | | | |
| `scripts/setup.sh` | installs lefthook's hooks into the repository | | | | |

"Yes" in the CI on PR column means the job is a required check that must pass before the PR can merge (see [Branch protection](#branch-protection)). "Yes" in the CI on `main` column means the job runs again on each push to `main`; by then the change is merged, so a red run there is a signal to act on, not a block. Everything in the pre-commit and pre-push columns is a fast local mirror of the same gate, so a red CI run should be a surprise, not a discovery. Each local gate runs at one hook, not both: the format check and clippy already run on every commit, so a push normally carries only commits that already passed them, and repeating them at push time would add wait for little gain. A commit made with the hooks bypassed (`LEFTHOOK=0` or `--no-verify`) is caught by CI instead. The authoritative list of what runs where is `lefthook.yml`.

With a warm cache the format check takes well under a second and clippy after editing a root crate takes about one second, which is what keeps both inside the commit hook's budget. A cold cache (after a toolchain change or `cargo clean`) takes minutes once.

### Formatting

Default `rustfmt` settings, plus a `rustfmt.toml` that pins `edition = "2021"` so that the formatter does not depend on which directory it is invoked from. No style opinions beyond the defaults; the point is zero diffs, not a house style.

The one-time format sweep (PR #11) is listed in `.git-blame-ignore-revs` so that `git blame` skips it.

### Lints

Clippy with warnings denied, configured once in `Cargo.toml` under `[workspace.lints]` and inherited by every crate with `[lints] workspace = true`. This makes the configuration part of the code rather than part of the command line, so `cargo clippy` in an editor matches CI.

Lint set: `clippy::all` at warn (denied by `-D warnings`), no `pedantic`. Pedantic on a Bevy codebase produces mostly noise about casts and `must_use`. Two adjustments to the default set:

- `clippy::too_many_arguments` allowed. Bevy systems take one parameter per query or resource and routinely exceed seven. Wrapping them in `SystemParam` structs to satisfy a lint would make the code worse.
- `unsafe_code = "forbid"` under `[workspace.lints.rust]`. There is no `unsafe` and no reason to expect any.

Rustdoc warnings are not gated. `cargo doc` is not part of the workflow and the lint would mostly complain about intra-doc links to Bevy types.

### Toolchain

`rust-toolchain.toml` pins an exact stable release with `components = ["rustfmt", "clippy"]`. Clippy's lint set changes between releases, so the lint gate is only meaningful if everyone runs the same toolchain, and a new stable release must never turn an unrelated PR red. `rustup` reads the file and installs the pinned toolchain on first use, locally and in CI. Bumping the pin is its own small PR: edit the file, run the lint script, fix whatever the new release flags.

### Tests

`cargo test --workspace` runs before every push that changes code and on every PR. Policy for what needs a test, applied by the author and checked in review:

- **A bug fix in a simulation crate** (`core`, `genome`, `brain`, `body`, `world`, `sim`, `phylogeny`) comes with a regression test when the behaviour is reachable from a unit test. The five accounting defects fixed in PR #6 are the model: each was arithmetic with a right answer (child energy against parent cost, one payout per kill) that a small test could have pinned.
- **A new pure function or data structure** in those crates comes with tests for its edge cases. Genome mutation, crossover, compatibility distance, spatial hash queries, save and load round trips, and terrain classification all qualify.
- **Render, UI, and app-wiring changes** need no tests. They are verified by hand in a release build.
- **Tuning constant changes** need no tests. They need a headless before-and-after summary in the PR body, which the code review guide asks for.

There is no integration test crate yet. One that spawns a headless app, runs a few hundred ticks, and asserts invariants (no negative energy, population above zero, ledger balances) is on the roadmap. Same-seed headless runs are bit-identical (see "Headless runs are deterministic" in `DECISIONS.md`), so such tests can assert exact values for a given seed rather than loose bounds; a test that pins an exact value will need updating whenever a simulation rule changes, which is the intended signal. Until then `scripts/smoke.sh` covers the "does it still run" half.

### Pre-commit and pre-push hooks

Hooks are managed by [lefthook](https://github.com/evilmartians/lefthook), configured in a versioned `lefthook.yml` at the repository root. It is a single Go binary with no runtime dependency (husky needs Node, the `pre-commit` framework needs Python), a declarative config that lives in git, and `lefthook install` to wire it up. Its config here does nothing clever: each hook calls the matching script in `scripts/`, so the hook and the CI job are the same code. `scripts/setup.sh` runs `lefthook install` and says how to get lefthook if it is missing (`brew install lefthook`).

The generated hooks live in `.git/hooks`, which is shared by every worktree of this repository, so one install covers all of them.

- **pre-commit** runs `scripts/fmt-check.sh` and `scripts/lint.sh`, in parallel. The hook does not auto-format: a hook that rewrites the files being committed hides the change from the author, and `cargo fmt --all` is one command away.
- **pre-push** runs `scripts/test.sh` through `scripts/pre-push-test.sh`. Test compilation is much slower than clippy, so it sits at push time. Pushes here open or update a PR, which is the moment the result matters.

The pre-push wrapper skips the tests when a push changes only documentation. Git hands the hook one line per pushed ref; the wrapper diffs each ref against the remote's old tip, or against its merge-base with `origin/main` when the ref is new, and exits early only when every changed path is on the allowlist: `*.md` anywhere under `docs/`, `plans/` or `review/`, plus the top-level `TODO.md`. `README.md` is not on the list, because `crates/clauvolution_app/src/cli.rs` reads it with `include_str!` for the test that keeps the README's flag table and the parser in step. Anything it cannot account for runs the tests: a path outside the list (including `README.md`, `CLAUDE.md`, `AGENTS.md`, and markdown inside a crate), a remote tip it does not have locally, a new ref with no `origin/main` to measure from, or empty or malformed input. The diff lists both sides of a rename, so moving a source file into `docs/` still counts as a code change. `FORCE_TESTS=1 git push` always runs the tests, and `scripts/test.sh` runs them directly. Separately, lefthook itself skips a pre-push job when `HEAD` has no changes against its push target; that is lefthook's behaviour, not this wrapper's.

Skipping fits the gate policy because the pre-push run is a local mirror of a CI gate, not the gate itself. CI runs `scripts/test.sh` on every pull request and on every push to `main`, including plans pushed straight to `main`. A docs-only push leaves the code identical to a commit that has already been through the gates, so the local run would compile and test exactly what was tested before, and cost close to a minute on pushes that are mostly plans and TODO edits. The allowlist is safe only while no crate reads an allowlisted file at compile time (`include_str!`, `include_bytes!`, or `#[doc = include_str!(...)]`). The wrapper checks this itself before it skips: it greps the crate sources for those macros, resolves each string literal against the including file's directory, and runs the tests if any target is on the allowlist or if a call has no literal on the same line to resolve (a multi-line call, or one built with `concat!` or `env!`). The check prints the offending file, so the fix is to drop the matching pattern from `is_docs_path`. It exists because the README's flag-table test (#59) and the wrapper's README entry (#61) landed separately, and before the check a README-only push skipped a test that CI then failed.

`--no-verify` is for recovering from a broken toolchain, not for deferring a fix. If a hook is wrong, fix the hook.

### CI on pull requests

One workflow, `.github/workflows/ci.yml`, triggered on `pull_request` and on `push` to `main`. Jobs:

1. **check**: `scripts/fmt-check.sh`, `scripts/lint.sh`, `scripts/test.sh`, in that order so the cheapest failure reports first.
2. **smoke**: `scripts/smoke.sh`, which builds release and runs the short headless check. Runs in parallel with `check` because the release build shares nothing with the debug test build.

Both jobs run on `ubuntu-latest` with a 60-minute timeout. Each installs the toolchain with `rustup toolchain install`, which reads `rust-toolchain.toml`, so the version lives in one file and no toolchain action is involved. `Swatinem/rust-cache` caches `target/`, keyed `debug` for `check` and `release` for `smoke`. Bevy 0.15 needs `libasound2-dev`, `libudev-dev`, `libwayland-dev`, and `libxkbcommon-dev` installed with `apt` before the build; the headless binary still links the audio and windowing stacks. A cold Bevy build on a two-core runner is slow (ten minutes or more); with the cache warm it is a few minutes.

Concurrency is grouped by workflow and ref with `cancel-in-progress: true`, for every ref. A new push to a PR branch cancels that branch's previous run, and a new push to `main` cancels an in-progress run on `main`, so when two merges land close together only the later commit gets a complete CI run.

The smoke run uses 500 ticks because runner cores are slower than the development machine and the check is "does the world still come up and run", not "does it reach the attractor". It asserts exit code 0 and parses the final population from the summary, failing on zero. It writes the summary to the job summary page and uploads it as the `smoke-summary` artifact (kept 30 days), so a PR reviewer can compare it against `main` without running anything.

### Scheduled runs on `main`

The failure mode that matters most is "a merged change quietly moved the attractor", and nothing above catches it. A separate workflow, `.github/workflows/probe.yml`, covers it:

- **Triggers**: `push` to `main`; `schedule` weekly (Sunday 03:00 UTC); `workflow_dispatch` with optional `ticks` and `runs` inputs (defaults 15000 and 2); and `pull_request` when `probe.yml` or `scripts/probe.sh` changes, so edits to the probe are exercised before merge.
- **Shape.** A `plan` job picks the tick count and runs per seed from the trigger. Each matrix job then builds release and runs `scripts/probe.sh SEED TICKS OUT_DIR LABEL`, which does one headless run with `--dump-history` and writes the summary and CSV to the output directory. The eight seeds are the audit seeds from `scripts/attractor_audit.sh` (1, 2, 3, 42, 314, 7, 99, 1000). Jobs set `CLAU_WORKERS=2`, run independently with `fail-fast: false`, and have a six-hour timeout.
  - **On push and pull request**: eight seeds, one run each, 5000 ticks. Long enough to be past the early transient the audits describe, short enough to report within the hour; measured when the workflow was added, a run took about five minutes on `ubuntu-latest` after a cold release build of about eight minutes.
  - **Weekly**: the full audit shape, eight seeds, two runs each, 15000 ticks, sixteen jobs. Its artifacts are directly comparable to the entries under `docs/audits/`, and it catches toolchain and dependency drift when `main` has been idle.
- **Pass criteria**: each job exits 0 and ends with a living population. Nothing more. Asserting on strategy mix or species count would encode today's attractor as correct, which is the opposite of what the project wants.
- **Output**: per-run summary and history CSV uploaded as `probe-seed<S>-run<R>` artifacts, kept for 90 days, so that when a change in dynamics is suspected the evidence is already there. Each job writes a one-row table (population, plants, foragers, predators, species, verdict) to its run page.
- **Determinism job**: runs on the same `plan`, so it uses the same tick count as the matrix (5000 on push, 15000 weekly). It builds release, runs seed 42 twice back to back on one runner, drops the lines that legitimately differ (the wall-clock timing line and the history dump path), and diffs the two summaries. It also compares the two history CSVs byte for byte. The verdict (summaries match or mismatch, CSVs identical or differ) and any diff go to the job summary, and the outputs are uploaded as `probe-determinism`. Both probe runs share one `continue-on-error` step and the comparison step always exits 0, so a mismatch, or a probe run that fails, never fails the job. A failure in setup or the release build still does. Headless runs are deterministic, so a mismatch is a regression: something has let wall-clock time, thread scheduling, or hash iteration order back into the simulation.

Failure of the probe workflow is a notification, not a block, because by the time it runs the change is already merged. The response to a red probe is a `TODO.md` entry or a revert, decided by a human.

### Dependency hygiene

Not gated. `Cargo.lock` is committed, which is correct for a binary. No `cargo audit` runs: a hobby simulator with no network surface does not need advisories blocking merges. There is no Dependabot: Bevy upgrades are project work, not something to accept from a bot.

### Pull request shape

`.github/pull_request_template.md` carries the structure the agent contract requires: `## Why` first, then what changed, claim and resolution markers, validation, and the `## Review` record. The template makes the rule visible to a human opening a PR from the web UI and gives the squash-merge commit body a consistent shape.

## Branch protection

`main` has a classic branch protection rule:

- Required status checks: `check` and `smoke`.
- The branch must be up to date with `main` before merging.
- Linear history is required.
- All conversations must be resolved before merging.
- Force pushes and deletions are blocked.
- The rule is not enforced for administrators.
- Zero approving reviews are required.

Required checks also apply to commits pushed directly, so without an exemption a plan committed straight to `main` would be rejected. Not enforcing the rule for administrators is what keeps the direct-to-main exceptions in `AGENTS.md` working for the repository owner, whose credentials every push here uses. The same exemption means the merge button offers an administrator a bypass on a red pull request; it is an extra deliberate click with a warning, not the default.

No approvals are required because the reviewers are agents the forge cannot see. The `## Review` section of the PR body is the record of review.

Repository merge settings: squash merging only, with the commit title taken from the PR title and the commit body from the PR body. The head branch is deleted on merge.

## What stays manual

- Anything involving the window, egui panels, input, or screenshots. Verified in a release build per the code review guide. Running the scripted tour in CI under a software renderer is filed as `ci-scripted-tour-software-renderer` in `TODO.md`.
- Judgement about whether simulation dynamics changed for the better. The probe surfaces data; the Graphs tab and the audits are where the judgement happens.
- Whole-codebase code reviews, on the cadence the review guide sets.
