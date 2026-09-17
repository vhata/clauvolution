# Clauvolution: agent contract

An evolution simulator built in Rust and Bevy 0.15. Organisms with NEAT neural network brains evolve in a procedural world. No behaviours are programmed; everything emerges from selection pressure. The user watches evolution happen in real time.

**Core motivation:** pure curiosity and joy in watching evolution unfold. This is a personal project with no research question to answer and no audience to ship to. Priorities serve that motivation. See `docs/ROADMAP.md` for the full framing.

## Workflow

- **Every unit of work is a branch and a pull request.** Never commit to `main` directly. One PR does one thing; it may contain several commits.
- **PRs are squash merged.** The PR title becomes the commit subject and the PR body becomes the commit body, so write the description as a durable commit message: `## Why` first, then what changed, then any claim or resolution markers, then validation steps. Never merge with a one-line body.
- **The user merges.** The agent merges only when explicitly told to in that turn, with a squash merge that keeps the full PR body.
- **Concurrent work uses git worktrees**, one per branch, named after the work's slug. Check open PRs, `git worktree list`, and `git branch -a` before claiming anything.
- Commit regularly within the branch. No leaving work uncommitted.
- No `Co-Authored-By` trailers in commit messages. Credits live in the README.
- When adding a new simulation dynamic, budget a follow-up tuning pass. Instrument first so the Graphs tab can show the dynamic's effect, then tune.
- Prefer editing existing files to creating new ones.

## Process guides

Three guides define the rest of the workflow. Load each only when its trigger fires.

- **A new idea or suggestion appears** during work, from the user or from your own investigation: read [`docs/TODO_GUIDE.md`](docs/TODO_GUIDE.md) and apply its scope decision before acting. Keep the branch focused on its stated goal.
- **The user asks to grab work from a queue.** There are three: [`docs/ROADMAP.md`](docs/ROADMAP.md) for vision-serving themed work, [`TODO.md`](TODO.md) for concrete deferred work, and [`review/BACKLOG.md`](review/BACKLOG.md) for work promoted from a code review. Phrases such as "grab something from the roadmap", "grab a TODO", or "grab a review finding" each name exactly one queue. Read the queue boundaries in `docs/TODO_GUIDE.md`, never switch queues, and state the chosen queue, section, and slug before claiming.
- **A code review is requested, or a PR resolves a review finding:** read [`docs/CODE_REVIEW_GUIDE.md`](docs/CODE_REVIEW_GUIDE.md). Reviews are recorded as ledgers in [`review/`](review/). Do not load this guide for other work.

## Where to find what

- **`README.md`**: build and run commands, CLI flags, controls.
- **`docs/ARCHITECTURE.md`**: crate layout, the simulation tick, ECS patterns, where-to-find-X table.
- **`docs/DECISIONS.md`**: non-obvious design choices and their tradeoffs. Read before changing a tuning constant or a mechanism that looks odd.
- **`docs/FEATURES.md`**: everything the sim currently does, grouped and one-lined.
- **`docs/ROADMAP.md`**: guiding motivation, themes, ongoing concerns.
- **`docs/design/`**: detailed design docs for bigger features.

Start with ARCHITECTURE and DECISIONS before reading code for any meaningful change. The decisions doc captures nuance that would otherwise only live in chat history.
