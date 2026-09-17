# Backlog workflow guide

This guide defines how work is captured, classified, claimed, and resolved across the three tracked queues: [`docs/ROADMAP.md`](ROADMAP.md), [`TODO.md`](../TODO.md), and [`review/BACKLOG.md`](../review/BACKLOG.md).

Read this guide only when evaluating a possible follow-up, or when adding, claiming, moving, or resolving work in one of those queues.

## Decide whether work belongs in the current branch

Compare each new suggestion with the current goal and acceptance criteria before acting on it.

- Keep work in scope when it is required for the requested outcome, correctness, regression prevention, tests, or another explicit acceptance criterion.
- Treat a separately shippable feature, broader refactor, optional optimization, or unrelated polish as a follow-up.
- Ask the user when the boundary is ambiguous.
- For a clear follow-up, capture it in the queue named under [Queue boundaries](#queue-boundaries), tell the user where it was recorded, and continue the original task without investigating or implementing it.
- For an unrelated P0 discovery, such as a crash, a corrupted save file, or lost session data, record it, alert the user prominently, and pause for direction.

## Capture an idea

Search all three queues for the idea and related slugs first. Update an existing entry instead of adding a duplicate.

Infer the workflow stage from the available evidence, but use Unprioritized unless the user supplied a priority or the issue objectively qualifies as P0 Critical.

### Entry format

Keep the main line concise and easy to scan: one area marker, an immutable slug, a short title, and a one-sentence rationale. Put supporting context on indented lines:

```md
- [UI] `graphs-axis-labels` — **Label the Graphs tab axes.** Curves are unreadable without knowing the units and the time window.
  - Starting point: The plotting helpers in `clauvolution_ui` already know the ranges.
  - Source: species-threshold tuning branch, 2026-09-14
  - Related: `graphs-tab-y-autoscale`
```

- `Starting point` optionally tells a future worker where to begin. It is not an instruction for the agent that records the idea.
- `Source` is required and names the task, branch, issue, or pull request where the idea arose, followed by the date in `YYYY-MM-DD` form. For a follow-up spun off a roadmap theme, name the theme.
- `Related` optionally cross-references closely connected entry slugs.
- `Split from` links a new entry created by a backlog-only split to the original batch slug.
- `Remaining from` preserves the original slug when a partial implementation leaves residual work from a removed entry.
- Slugs must be unique, descriptive kebab-case and must not change when an entry moves.

### Workflow stages

- **Needs triage:** The value, intended outcome, dependencies, or appropriate follow-up still needs a decision.
- **Needs proof of concept:** A focused experiment is needed to establish feasibility or choose an approach.
- **Ready for separate work:** The outcome is understood well enough to plan and implement on its own branch.

Move an entry when its readiness changes.

### Priorities

- **P0 Critical:** An active crash, data-loss, or save-corruption risk that requires immediate direction.
- **P1 High:** Important work that should be scheduled promptly.
- **P2 Normal:** Worthwhile work without immediate urgency.
- **P3 Low:** Optional or speculative improvement.
- **Unprioritized:** Priority has not been assigned.

### Area markers

- `[SIM]`: Organism biology, body plans, metabolism, predation, disease, symbiosis, and ecology dynamics.
- `[BRAIN]`: NEAT networks, genome encoding, mutation, speciation, and species classification.
- `[WORLD]`: Terrain, climate, seasons, and world events such as asteroids and ice ages.
- `[UI]`: egui panels, inspect, graphs, chronicle, phylogeny, and input controls.
- `[RENDER]`: Rendering, camera, meshes, trails, and level of detail.
- `[PERF]`: Throughput, parallelism, and memory.
- `[PERSIST]`: Save and load, sessions, seeds, and reproducibility.
- `[TOOLING]`: Build, CLI flags, headless mode, tests, and scripts.
- `[DOCS]`: Documentation or research.

Each entry has exactly one area marker. If an idea spans areas, split it into independently actionable entries and connect their slugs with `Related`. Add a new marker to this legend only when none of the existing areas fits.

### Queue boundaries

[`docs/ROADMAP.md`](ROADMAP.md) holds work that serves the project's vision and is big enough to belong to a theme. Roadmap items are prose sections under a theme heading; they do not use the entry format above and do not need a matching `TODO.md` entry.

[`TODO.md`](../TODO.md) holds concrete work that is not theme-sized: ideas that arise during ordinary work. An ordinary idea still goes there when it is a bug or refactor; the source of the work, rather than its technical kind, determines the queue. A concrete follow-up spun off a roadmap theme also goes here, with `Source` naming the theme. Its entries use the workflow stages above.

[`review/BACKLOG.md`](../review/BACKLOG.md) holds only work promoted from a whole-codebase review: bugs, duplication, structural refactors, and tooling gaps worth their own branch. Entries use the standard format plus a `Findings` line listing every raw review finding slug they cover and a `Source` naming the review file. They keep priority sections but have no workflow stages because each is understood well enough to start. A batch entry may cover many findings; each finding names at most one review backlog entry. See [`CODE_REVIEW_GUIDE.md`](CODE_REVIEW_GUIDE.md) for which findings qualify.

```md
- [SIM] `energy-math-duplication` — **Collapse the duplicated energy accounting.** Three call sites recompute metabolic drain and have already drifted apart.
  - Starting point: Extract one drain helper in `clauvolution_sim` and route the three call sites through it.
  - Source: review/2026-09-14-0450-full.md, 2026-09-14
  - Findings: `metabolism-drain-dup`, `predation-drain-dup`
```

Natural-language selection follows the file boundary:

- "Grab something from the roadmap" means `docs/ROADMAP.md` only.
- "Grab something from the TODO," "grab a TODO," and "grab a feature" mean `TODO.md` only.
- "Grab something from the review backlog" and "grab a review finding" mean `review/BACKLOG.md`; a specifically named raw finding follows the direct-selection exception in `CODE_REVIEW_GUIDE.md`.
- For implementation work from `TODO.md`, prefer the highest-priority suitable unclaimed entry in **Ready for separate work**. Ask before selecting from **Needs proof of concept** or **Needs triage** when no suitable entry is ready.
- Never switch queues because the requested queue lacks a suitable, available, or small entry. State the selected queue, section, and slug before claiming it.

## Claim and resolve an entry

Every unit of work gets its own branch, its own git worktree, and its own pull request.

Before starting, check both places where another worker may already have claimed the work:

1. Search open pull requests for the exact slug: `gh pr list --state open --search "<slug>"`.
2. Search worktrees and branches for the slug and the entry title: `git worktree list` and `git branch -a`. A worktree or branch whose name contains the slug counts as a provisional claim even when it has not opened a pull request yet.

For an entry in `review/BACKLOG.md`, repeat both checks for every slug in its `Findings` line. A worker may have been explicitly assigned one raw finding without claiming the mapped batch entry. Also follow each raw finding to any other review backlog entry that names it, then search for that entry's slug; the mapped backlog branch is the normal provisional claim even when its name does not contain every underlying finding slug.

If either search finds another claim, pause and ask the user before duplicating the work. After selecting an unclaimed entry, immediately create the branch and its worktree, before investigating or implementing, so later workers can discover the provisional claim. Name the branch after the queue and the slug:

- `todo/<slug>` for a `TODO.md` entry.
- `review/<slug>` for a `review/BACKLOG.md` entry.
- `roadmap/<slug>` for a roadmap item.

Recheck worktrees and branches after creating yours; if another one took the same slug concurrently, pause before either proceeds.

### Claim a roadmap item

A roadmap item is claimed directly and needs no `TODO.md` entry. Derive a short kebab-case slug from its heading, so `### Species range heatmap` becomes `species-range-heatmap`, then run the same claim checks against that slug and branch as `roadmap/<slug>`. When the work lands, update the roadmap section in the same pull request so the roadmap reflects what is now true.

### Write the pull request description as a commit message

Pull requests are squash merged: the title becomes the commit subject and the description becomes the commit body. The description is therefore the permanent record of the change, not a note to a reviewer that disappears on merge. Never merge with a one-line body.

One pull request does one thing. It may contain several commits, which the squash collapses, so the description carries the weight.

Order the description as follows.

1. `## Why`, always first. Explain why the work is worth doing and the outcome it is meant to achieve, not merely what code changed: describe the unmet need, the problem or opportunity, and its impact. For a feature, state the need and the capability it adds; for a bug, state the original behavior and why it was wrong; for internal work, state the concrete risk, limitation, or recurring cost it removes. Write it so a reader can understand the motivation without opening a backlog, its source, or a review ledger. If a pull request covers several entries or findings, address each one's motivation.
2. What changed, at the level of behavior and structure rather than a file-by-file diff summary.
3. Claim or resolution markers.
4. Validation instructions: what to run or watch to confirm the change works.

Keep `## Why` first as the draft evolves. The user merges. The agent merges only when told to in that same turn, using a squash merge that preserves the full description as the commit body, and confirms afterwards that the resulting commit body is not just the subject line.

### Claim markers

Once the branch has its first meaningful commit, open a draft pull request. For a `TODO.md` entry, include `Claims TODO: <slug>`. For a `review/BACKLOG.md` entry, include `Claims review backlog: <slug>` plus `Claims review finding: <finding-slug>` for each raw finding in scope. For a raw finding selected under the direct-selection exception in [`CODE_REVIEW_GUIDE.md`](CODE_REVIEW_GUIDE.md), include only `Claims review finding: <finding-slug>`. Leave the source entry intact while work is underway. If a draft pull request cannot be created, report that the claim is not globally visible and do not remove the entry. If the work is abandoned, close the draft pull request so the entry is visibly available again.

### Focus a batched review backlog entry

A batched review backlog entry is a scheduling unit, not a requirement to mix unrelated fixes into one pull request. To take a focused subset:

1. Claim the existing review backlog slug and check every raw finding slug in the entry before editing. This temporarily reserves the batch and prevents two workers from producing conflicting remainder entries.
2. State the selected subset with `Claims review finding: <slug>` lines in the draft pull request. Do not claim findings the pull request will not address.
3. Keep the implementation limited to that coherent subset.
4. At resolution, use the partial-resolution flow below: replace the original batch entry with a newly assessed remainder entry that lists only the open findings. The new entry gets a new backlog slug and `Remaining from: <original-slug>`; raw finding slugs remain immutable.

If parallel work on pieces of a batch is important, first land a backlog-only split. Keep the original slug on one focused entry, narrow its `Findings` line, and add independently claimable entries with new slugs and `Split from: <original-slug>` for the other subsets. Do not use resolution markers for this queue-only change, and do not let multiple implementation branches each invent a different remainder from the same original entry.

### Resolution markers

Before marking the pull request ready for review, verify the implementation against the complete source entry:

- For a full `TODO.md` resolution, remove the entry in the same pull request and change the description to `Resolves TODO: <slug>`.
- For a partial `TODO.md` resolution, remove the original entry and add a new entry describing only the remaining work. Give it a new slug, reassess its workflow stage, priority, and area, and add `Remaining from: <original-slug>`. Include `Partially resolves TODO: <original-slug>` and `Remaining TODO: <new-slug>` in the pull request description.
- For a full `review/BACKLOG.md` resolution, remove the entry and change its claim marker to `Resolves review backlog: <slug>`.
- For a partial `review/BACKLOG.md` resolution, replace the original with a newly prioritized remainder that lists only the open findings, has a new slug, and includes `Remaining from: <original-slug>`. Use `Partially resolves review backlog: <original-slug>` and `Remaining review backlog: <new-slug>` in the pull request description.
- Remove a rejected or obsolete entry only when the reason is documented in the associated pull request.
- Whenever an entry is removed, search both backlogs for its slug and update or remove the `Related` lines that name it. Deleted entries otherwise leave dangling references that only an incremental review catches.

For review-derived work, also add the validation instructions required by [`CODE_REVIEW_GUIDE.md`](CODE_REVIEW_GUIDE.md) before marking the pull request ready for review. Replace each `Claims review finding` line for completed work with `Resolves review finding: <finding-slug>`; a direct-selection pull request with no backlog entry uses only that marker. Findings left in a partial-resolution remainder stay listed only in the new review backlog entry; do not mark them resolved. The merged pull request and its markers are the handoff to the next incremental review, which owns closure as described in that guide.

Claiming an entry never removes it. The default branch keeps the source entry if the implementation branch or draft pull request is abandoned.
