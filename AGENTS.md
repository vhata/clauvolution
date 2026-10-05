# Clauvolution: agent contract

An evolution simulator built in Rust and Bevy 0.15. Organisms with NEAT neural network brains evolve in a procedural world. No behaviours are programmed; everything emerges from selection pressure. The user watches evolution happen in real time.

**Core motivation:** pure curiosity and joy in watching evolution unfold. This is a personal project with no research question to answer and no audience to ship to. Priorities serve that motivation. See `docs/ROADMAP.md` for the full framing.

## Workflow

- **Every unit of work is a branch and a pull request.** Never commit to `main` directly. One PR does one thing; it may contain several commits.
- **Two exceptions go straight to `main`.** The first is plans. A plan is planning for work, not work, so it is a markdown file in `plans/` named `YYYY-MM-DD-<slug>.md`, committed directly to `main` with no branch or pull request. Write it, commit it, and stop; the user reads it locally and pushes it. A plan sets out why, the ordered steps, what done looks like for each, what is out of scope, and open questions. Everything a plan describes still goes through branches and pull requests. The second exception is housekeeping metadata files such as `.git-blame-ignore-revs`, and only when the user says so for that case. Nothing else goes straight to `main`.
- **PRs are squash merged.** The PR title becomes the commit subject and the PR body becomes the commit body, so write the description as a durable commit message: `## Why` first, then what changed, then any claim or resolution markers, then validation steps, then the `## Review` record. Never merge with a one-line body.
- **The user merges.** The agent merges only when explicitly told to in that turn, with a squash merge that keeps the full PR body.
- **Every code-writing agent gets a separate reviewer agent** before its PR counts as ready, including a coordinating agent that writes code. The reviewer verifies the PR's claims and reports findings; the author fixes them. The PR body records the review in a `## Review` section: reviewer, commit reviewed, verdict, findings and their dispositions, and what was not checked. Documentation-only and measurement-only PRs get a lighter review of their claims.
- **Linear history.** Rebase; never merge `main` into a branch. Rebasing keeps every commit on the branch; commits are squashed only by the squash merge that lands the PR. `git push --force-with-lease` is for PR branches only; never force-push `main`.
- **Concurrent work uses git worktrees**, one per branch, named after the work's slug. Check open PRs, `git worktree list`, and `git branch -a` before claiming anything; `bash scripts/workflow/claim-check.sh <slug>` does all three. `bash scripts/workflow/start-work.sh <queue> <slug>` then creates the branch and its worktree. After a PR lands, `bash scripts/workflow/cleanup-landed.sh --apply` removes its branch and worktree.
- **Parallel agents** may take independent items, each in its own worktree with disjoint file ownership. The coordinating agent serialises edits to the queue files and holds push and merge. At most four heavy builds or headless runs at once.
- **Warn the user before anything that opens a window** (any non-headless run). Sub-agents never open a GUI; they use headless mode. GUI runs, including review screenshots and release-build reproductions, are done by the coordinating agent after warning the user.
- **Unattended work** under a broad autonomy grant, with no explicit ask to open PRs, stays on local branches. Finished PR bodies go in `.feral/pr-<slug>.md`, and each load-bearing decision goes in `AUDIT.md` with an undo line. Both are git-ignored. No pushes, merges, or tags without the user's word.
- Commit regularly within the branch. No leaving work uncommitted.
- **Run `scripts/check.sh` before opening or updating a pull request.** It runs the same queue, link, format, lint, and test gates as CI. `scripts/setup.sh` installs the git hooks that run the format, lint, and test gates automatically. See `docs/QUALITY.md` for what is gated and why.
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
- **`docs/QUALITY.md`**: the quality gates (format, lint, tests, headless smoke, scheduled probe), where each runs, and the test policy.
- **`docs/FEATURES.md`**: everything the sim currently does, grouped and one-lined.
- **`docs/ROADMAP.md`**: guiding motivation, themes, ongoing concerns.
- **`docs/design/`**: detailed design docs for bigger features.
- **`plans/`**: dated plans for upcoming stretches of work, one file per plan, newest last by filename.

Start with ARCHITECTURE and DECISIONS before reading code for any meaningful change. The decisions doc captures nuance that would otherwise only live in chat history.
