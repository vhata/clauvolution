# Clauvolution: agent contract

An evolution simulator built in Rust and Bevy 0.15. Organisms with NEAT neural network brains evolve in a procedural world. No behaviours are programmed; everything emerges from selection pressure. The user watches evolution happen in real time.

**Core motivation:** pure curiosity and joy in watching evolution unfold. This is a personal project with no research question to answer and no audience to ship to. Priorities serve that motivation. See `docs/ROADMAP.md` for the full framing.

<!-- docket:begin -->

## Docket

This project is run with docket. Read [docket/CONTRACT.md](docket/CONTRACT.md) before any work; it is the authoritative workflow, and this section records only this project's settings. The installer keeps this section current: the text above the list is rewritten on every run, the list keeps the values this project has set, and the section may sit anywhere in this file.

- Queue areas: `[SIM]` (organism biology, body plans, metabolism, predation, disease, symbiosis, ecology dynamics), `[BRAIN]` (NEAT networks, genome encoding, mutation, speciation, species classification), `[WORLD]` (terrain, climate, seasons, world events such as asteroids and ice ages), `[UI]` (egui panels, inspect, graphs, chronicle, phylogeny, input controls), `[RENDER]` (rendering, camera, meshes, trails, level of detail), `[PERF]` (throughput, parallelism, memory), `[PERSIST]` (save and load, sessions, seeds, reproducibility), `[TOOLING]` (build, CLI flags, headless mode, tests, scripts), `[DOCS]` (documentation or research). One area per entry; an idea that spans areas is split into independently actionable entries connected with `Related`. A new area is added here only when none of these fits.
- Concurrency cap: 4 heavy builds or headless runs at once.
- Merging and releases: no standing merge delegation. Nothing is published without the user's explicit sign-off; release tags are already covered by the contract.
- Optional pieces in force: all of them. The review ledger under `review/`; the roadmap queue (`docs/ROADMAP.md`, the third queue next to `TODO.md` and `review/BACKLOG.md`; "grab something from the roadmap" names it and nothing else); the scheduled validation of `main` (`.github/workflows/probe.yml`, long headless runs after each merge and weekly); stacked PRs when a shared interface must land first.
- Environment: warn the user before anything that opens a window (any non-headless run). Sub-agents never open a GUI; they use headless mode (`--headless N --seed S`). GUI runs, including review screenshots and release-build reproductions, are done by the coordinating agent after warning the user. GitHub runners have no GPU, so anything that needs a window is verified by a human in a release build.
- Roadmap claims: a roadmap item is claimed directly, with no `TODO.md` entry, under a kebab-case slug derived from its heading (`### Species range heatmap` becomes `species-range-heatmap`); the branch is `roadmap/<slug>` and the marker `Claims roadmap: <slug>`. A step of a plan in `plans/` uses the step's own slug if the plan names one, otherwise the kebab-case of the step heading without its number (`### 4. Continents and shallow shelves` becomes `continents-and-shallow-shelves`). When the work lands, the same PR updates the roadmap section, or the step's status in the plan, and changes the marker to `Resolves roadmap: <slug>`. A concrete follow-up spun off a roadmap theme goes to `TODO.md` with `Source` naming the theme.
- Plans: `plans/YYYY-MM-DD-<slug>.md`. Write it, commit it, and stop; the user reads it locally and pushes it. A plan sets out why, the ordered steps, what done looks like for each, what is out of scope, and open questions.
- Simulation dynamics: when adding a new simulation dynamic, budget a follow-up tuning pass. Instrument first so the Graphs tab can show the dynamic's effect, then tune. Tuning-constant changes carry a headless before-and-after summary in the PR body (see `docs/QUALITY.md`).
- Review record: documentation-only and measurement-only PRs get a lighter review of their claims. Codebase review snapshots are opened from a `code-review/<type>-<date>` branch, not a `review/` branch, which the marker check reserves for backlog work.
- Commits: no generated-with footers; credits live in the README.
- Priorities: an active crash or a save that will not load is P0 here, alongside the data loss, security exposure and release blocker that the vendored TODO guide names.

<!-- docket:end -->

## Working with the user

- **History.** Branches are updated by `git rebase origin/main`, keeping every commit. Never squash or soft-reset a branch to tidy it or before a rebase; the only squash is the squash merge that lands the PR. No merge commits anywhere, including PR branches.
- **When the user steps away and asks for PRs**, fill the time: take independent entries from the queue they named, one worktree agent per entry with disjoint files, each ending in its own reviewed PR, within the concurrency cap. Keep the main thread on the current task. Without an explicit ask for PRs, the unattended rules in `docket/CONTRACT.md` apply instead.
- **In chat**, write every pull request as a clickable link, `[#N](https://github.com/vhata/clauvolution/pull/N)`. Commit messages, PR bodies and docs keep plain `#N`.
- **Clever code is a smell, not a veto.** When a solution needs a multi-paragraph justification, put the simpler alternative beside it and name the cost, so the user chooses knowingly. If the clever path is taken, say so in the change description.
- **Capture learnings where they belong, when they happen.** When an earlier claim turns out wrong, correct the doc that holds it, not just the commit message. A tradeoff that was not obvious at plan time goes in `docs/DECISIONS.md`; what a tuning pass teaches about the sim's behaviour goes in the "Ongoing concerns" section of `docs/ROADMAP.md`.

## Machine and tooling notes

- **Shared repository.** Several Conductor workspaces share this repository's `.git`, so branches and worktrees from other workspaces show up in claim checks, and the main checkout keeps local `main` checked out. From a worktree, compare against `origin/main` (for example `check-pr-markers.sh --base origin/main`), and make direct-to-main commits from the main checkout after `git pull --ff-only`.
- **Warm builds for new worktrees.** A fresh worktree compiles Bevy from scratch. On APFS, `cp -c -R <warm-worktree>/target <new-worktree>/target` clones an existing target directory almost instantly, and cargo then rebuilds only the workspace crates. Do this before dispatching parallel agents.
- **Assets resolve from the crate directory.** Run the GUI with `cargo run --release` from the repository root. Running `target/release/clauvolution` directly looks for `assets/` next to the binary and logs a missing-font error, because the assets live in `crates/clauvolution_app/assets`.
- **What an agent can check in the GUI.** `--script` tours can set the speed, zoom, tab and camera position and take egui-aware screenshots, but cannot pause, press keys or drag. Agents also cannot inject keystrokes (the terminal has no macOS Accessibility permission). Acceptance checks that need input, such as "pause, then WASD still pans", go to the user. A tour's `exit` action does not currently quit the app (`script-exit-leaves-app-running` in `TODO.md`), so stop a tour process once its screenshots are written.
- **rerere is on.** The user's git configuration enables `rerere` with `autoupdate`, so a rebase replays and stages any conflict resolution recorded on an earlier rebase, and the conflict can look already resolved. After any rebase that touched `review/BACKLOG.md`, `TODO.md` or the docs, check the staged result against the current `main` before continuing.
- **Workflow files need the `workflow` scope.** Pushing a change under `.github/workflows/` fails unless the `gh` token has the `workflow` scope (`gh auth refresh -s workflow`). CI changes need the user's word anyway.

## Where to find what

- **`README.md`**: build and run commands, CLI flags, controls.
- **`docket/`**: the vendored workflow contract, the queue and review guides, and the scripts that check and operate them. Never edited here; see `docket/README.md`.
- **`docs/ARCHITECTURE.md`**: crate layout, the simulation tick, ECS patterns, where-to-find-X table.
- **`docs/DECISIONS.md`**: non-obvious design choices and their tradeoffs. Read before changing a tuning constant or a mechanism that looks odd.
- **`docs/QUALITY.md`**: what quality means here, the gates (format, lint, tests, headless smoke, scheduled probe), where each runs, the test policy, and the inputs a codebase review reads.
- **`docs/FEATURES.md`**: everything the sim currently does, grouped and one-lined.
- **`docs/ROADMAP.md`**: guiding motivation, themes, ongoing concerns.
- **`docs/design/`**: detailed design docs for bigger features.
- **`TODO.md`**, **`review/`**: deferred work and the review ledger.
- **`plans/`**: dated plans for upcoming stretches of work, one file per plan, newest last by filename.

Start with ARCHITECTURE and DECISIONS before reading code for any meaningful change. The decisions doc captures nuance that would otherwise only live in chat history.
