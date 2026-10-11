# Clauvolution: agent contract

An evolution simulator built in Rust and Bevy 0.15. Organisms with NEAT neural network brains evolve in a procedural world. No behaviours are programmed; everything emerges from selection pressure. The user watches evolution happen in real time.

**Core motivation:** pure curiosity and joy in watching evolution unfold. This is a personal project with no research question to answer and no audience to ship to. Priorities serve that motivation. See `docs/ROADMAP.md` for the full framing.

<!-- docket:begin -->
## Docket

This project is run with docket. Read [docket/CONTRACT.md](docket/CONTRACT.md) before any work; it is the authoritative workflow, and this section records only this project's settings. The installer keeps this section current: the text above the list is rewritten on every run, the list keeps the values this project has set.

- Queue areas: `[SIM]` (organism biology, body plans, metabolism, predation, disease, symbiosis, ecology dynamics), `[BRAIN]` (NEAT networks, genome encoding, mutation, speciation, species classification), `[WORLD]` (terrain, climate, seasons, world events such as asteroids and ice ages), `[UI]` (egui panels, inspect, graphs, chronicle, phylogeny, input controls), `[RENDER]` (rendering, camera, meshes, trails, level of detail), `[PERF]` (throughput, parallelism, memory), `[PERSIST]` (save and load, sessions, seeds, reproducibility), `[TOOLING]` (build, CLI flags, headless mode, tests, scripts), `[DOCS]` (documentation or research). One area per entry; an idea that spans areas is split into independently actionable entries connected with `Related`. A new area is added here only when none of these fits.
- Concurrency cap: 4 heavy builds or headless runs at once.
- Merging and releases: the user merges unless they delegate it in that turn; there is no standing delegation. Nothing is published or tagged without the user's explicit sign-off.
- Optional pieces in force: all of them. The review ledger under `review/`; the roadmap queue (`docs/ROADMAP.md`, the third queue next to `TODO.md` and `review/BACKLOG.md`; "grab something from the roadmap" names it and nothing else); the scheduled validation of `main` (`.github/workflows/probe.yml`, long headless runs after each merge and weekly); stacked PRs when a shared interface must land first.
- Environment: warn the user before anything that opens a window (any non-headless run). Sub-agents never open a GUI; they use headless mode (`--headless N --seed S`). GUI runs, including review screenshots and release-build reproductions, are done by the coordinating agent after warning the user. GitHub runners have no GPU, so anything that needs a window is verified by a human in a release build.
- Roadmap claims: a roadmap item is claimed directly, with no `TODO.md` entry, under a kebab-case slug derived from its heading (`### Species range heatmap` becomes `species-range-heatmap`); the branch is `roadmap/<slug>` and the marker `Claims roadmap: <slug>`. A step of a plan in `plans/` uses the step's own slug if the plan names one, otherwise the kebab-case of the step heading without its number (`### 4. Continents and shallow shelves` becomes `continents-and-shallow-shelves`). When the work lands, the same PR updates the roadmap section, or the step's status in the plan, and changes the marker to `Resolves roadmap: <slug>`.
- Plans: `plans/YYYY-MM-DD-<slug>.md`, committed directly to `main` as the contract allows. Write it, commit it, and stop; the user reads it locally and pushes it. A plan sets out why, the ordered steps, what done looks like for each, what is out of scope, and open questions.
- Simulation dynamics: when adding a new simulation dynamic, budget a follow-up tuning pass. Instrument first so the Graphs tab can show the dynamic's effect, then tune. Tuning-constant changes carry a headless before-and-after summary in the PR body (see `docs/QUALITY.md`).
- Review record: the `## Review` section names the reviewer, the commit reviewed, the verdict, each finding with its disposition, and what was not checked. Documentation-only and measurement-only PRs get a lighter review of their claims. Codebase review snapshots are opened from a `code-review/<type>-<date>` branch, not a `review/` branch, which the marker check reserves for backlog work.
- Commits: no `Co-Authored-By` trailers or generated-with footers; credits live in the README. Prefer editing existing files to creating new ones.
<!-- docket:end -->

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
