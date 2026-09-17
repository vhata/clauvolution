# Code review guide

This guide defines how whole-codebase reviews are performed, recorded in [`review/`](../review/), kept current through cheaper incremental reviews, and promoted into the tracked [`review/BACKLOG.md`](../review/BACKLOG.md) work queue. It is separate from pull request review, which continues to happen on each change.

Read this guide when asked to perform a full or incremental code review, when resolving a review finding, or when deciding whether a review finding should enter the review backlog.

## Why two kinds of review

A pull request review sees one change. Some problems are properties of the whole codebase rather than of any change: two copies of a rule that drift apart, code that becomes dead when its last caller is removed elsewhere, a component that crosses a size threshold two hundred lines at a time, a test that becomes vacuous when the element it targets is removed, or a latent bug that a later rename makes live. A **full review** reads everything and finds these. An **incremental review** starts from the last review, examines only what changed since, re-checks the standing findings, and runs cheap whole-repo checks. Incremental reviews are the default; full reviews are a periodic reset.

## Artifacts

### The `review/` directory

- `review/README.md` is the index: one row per review, newest first, with type, reviewed commit, and the count of findings open when the review closed. The top row is the baseline for the next incremental review.
- `review/BACKLOG.md` is the mutable queue of review-derived work approved for a separate workspace. It is not a review snapshot.
- `review/YYYY-MM-DD-HHMM-full.md` or `review/YYYY-MM-DD-HHMM-incremental.md` is one review. The timestamp is the UTC time the review was written, so several reviews on one day sort correctly and a later incremental review can be told apart from the one it started from. Review files are snapshots: once merged, they are not edited except to fix a factual error in the review itself. Status changes are recorded by the next review, not by the pull request that fixes a finding.

Snapshots written before the review backlog was split from `TODO.md` use `TODO` labels for their backlog mappings. Preserve that historical wording; resolve those mapped slugs against `review/BACKLOG.md`.

### Review file structure

Every review file has, in order:

1. **Header table** with `Review type`, `Reviewed commit` (the short hash of the code that was reviewed, not the commit that lands the review), `Previous review` (file name and its reviewed commit, or None), and `Baseline` (which checks were run and their results).
2. **Summary**: a few sentences on the overall state and where the problems concentrate.
3. **Invariants**: properties the codebase should keep, carried forward and amended by each review. Incremental reviews check changed hunks against this list.
4. **Findings**, grouped by kind.
5. **Findings filed in the review backlog**: a table mapping review backlog slugs to the finding slugs they cover.
6. **Suggested order of work**.

### Finding format

```md
- `finding-slug` — **One-sentence title.** Kind · Status · Verification.
  - Where: `path:lines` (`symbol`); another `path` (`symbol`).
  - Detail, failure scenario, and suggested fix in one to four sentences.
  - Review backlog: `backlog-slug`
```

- **Slug**: unique, descriptive kebab-case, immutable once published. Later reviews refer to the finding by slug.
- **Kind**: `Bug`, `Design`, `Duplication`, `Performance`, `Test`, `Style`, `Tooling`, or `Docs`.
- **Status**: `Open`, `Fixed`, `Moved`, `Accepted`, `Invalid`, or `Superseded`. Every status other than `Open` carries evidence:
  - `Fixed`: name the commit or pull request, give the `Where` at the reviewed commit that shows the fix (file, lines, symbol), and state how it was confirmed. For a finding that was `Verified`, re-run the original reproduction and record that it no longer reproduces; closing a verified bug by reading alone is not sufficient.
  - `Moved`: still open; give the new `Where`.
  - `Accepted`: will not fix; give the reason.
  - `Invalid`: the finding was wrong; quote or cite the code that shows why.
  - `Superseded`: name the replacing finding slug.
- **Verification**: `Verified` when reproduced in a browser against the production build or confirmed by running a suite; `Read` when established by inspection only.
- **Where**: file paths with line numbers valid at the reviewed commit, plus the enclosing symbol so the reference survives edits. To see a reference as it was, run `git show <reviewed-commit>:<path>`; to map it forward, run `git blame -L <line>,<line> <reviewed-commit> -- <path>` and follow the change.
- **Review backlog**: present only when the finding is covered by an entry in `review/BACKLOG.md`.

### Closed finding format

Findings that leave `Open` appear in the closed list as one line each, with all three parts: the reference, the location at the reviewed commit, and the confirmation. A closed entry missing any part is malformed, in the same way a finding without a slug is.

```md
- `finding-slug` — Fixed in #45 (f10a8e2). `path:lines` (`symbol`): one sentence on what the code now does. Re-reproduced: no longer reproduces.
```

A pull request number alone is not enough. It says where to look, not what to look at, and without the location and confirmation the next review has to re-derive the closure from scratch.

One finding is one thing that can be independently fixed and independently verified. Group many small instances of the same smell into one finding (for example, one finding for dead CSS selectors listing them all) rather than one finding per selector.

## Performing a full review

1. Record the reviewed commit before reading anything. Run typecheck, lint, unit tests, build, and the end-to-end suite; record results as the baseline.
2. Read everything. Delegate large files to parallel reviewers if available, but verify their claims against the source before recording them. Do not record a claim you could not confirm.
3. Reproduce the most serious bugs against the production build in a browser and mark them `Verified`. Bugs that cannot be reproduced stay `Read`.
4. **Triage into the work queue.** Apply the mandatory review-close triage below to every open finding. File or update review backlog entries and complete the review's backlog mapping.
5. Write the review file and invariants, then add the row to `review/README.md`.
6. Commit the review on its own branch. The review commit is separate from any code fix; small documentation fixes made under the triage rule below are their own commits on the same branch.

## Performing an incremental review

Inputs: this guide, the most recent review file, and the repository. The reviewed commit of the previous review is the **base**; the current `HEAD` is the new reviewed commit.

1. **Baseline.** Run the same checks as a full review and record them. A check that passed before and fails now is a finding.
2. **Triage standing findings.** For each finding in the previous review, determine its new status. Read the relevant code; do not infer status from pull request titles, and for `Verified` findings re-run the reproduction before closing. Check merged pull requests for `Resolves review finding: <slug>` lines. Carry every finding forward with its slug: `Fixed`, `Accepted`, `Invalid`, and `Superseded` findings appear in the closed list using the closed finding format; `Open` and `Moved` findings appear in full with any updated `Where`.
3. **Review the delta.** Run `git log --oneline <base>..HEAD` and `git diff <base>..HEAD --stat`, then read every hunk. Judge each hunk against the invariants and against the standing findings: a change that adds another copy of something already listed as duplicated is a finding even though the hunk is locally fine. Record new findings with new slugs.
4. **Run whole-repo mechanical checks.** These are cheap and catch drift that no hunk shows: unused exports, CSS classes with no TSX reference, file and component sizes and hook counts for the large components, test selectors with no match in `src/`, product-name and copy consistency. Compare against the previous review's numbers.
5. **Sweep hot files.** Any file changed by most of the pull requests in the delta gets a full re-read, not just its hunks. Interaction bugs form where changes concentrate.
6. **Re-verify.** Re-run the browser reproductions for `Verified` bugs that are still open, and reproduce new serious bugs.
7. **Triage into the work queue.** Apply the mandatory review-close triage below to every `Open` or `Moved` finding, including findings carried from the previous review. File or update review backlog entries and complete the review's backlog mapping.
8. **Write the file** as `review/YYYY-MM-DD-HHMM-incremental.md` with `Previous review` filled in, update the invariants, and update `review/README.md`.

A prompt that starts an incremental review: "Read `docs/CODE_REVIEW_GUIDE.md` and the most recent review in `review/`, then do an incremental review."

## When a full review is due

Not on a calendar. Trigger a full review when any of these holds:

- A large refactor landed, so standing findings no longer map onto the structure and the delta is large but low-signal.
- The cumulative delta since the last full review touches more than about a third of the source lines.
- A hot file has been substantially rewritten.
- Most standing findings are closed and a clean baseline is wanted.
- Two or more consecutive incremental reviews have each added many new findings, which suggests the incremental view is missing whole-program drift.

Each incremental review inherits the blind spots of the review it started from. The full review is the reset that clears them.

## Turning findings into review backlog entries

Findings are observations. Review backlog entries are decisions to do work. Most findings never become backlog entries; they are recorded in the review and picked up wholesale when a refactor claims the area.

### Selection hierarchy

`review/BACKLOG.md` is the work queue for review-derived changes; the phrases that select it are defined in the queue boundaries of [`TODO_GUIDE.md`](TODO_GUIDE.md). When the user requests review work, select it in this order:

1. Follow an explicit user or maintainer assignment, including an assignment of a raw finding slug.
2. Otherwise choose a review backlog entry by priority.
3. Treat raw findings without a review backlog entry as evidence and inventory, not as an independently claimable queue.

Direct work on a raw finding is an exception reserved for an explicit assignment, explicit direction after a P0 issue is reported, or work required for the correctness or regression prevention of an already scoped change. A separately shippable raw finding discovered during ordinary work should be filed or updated in the review backlog rather than silently expanding the branch.

### Mandatory review-close triage

Every full and incremental review must make one queue decision for each finding whose status at review close is `Open` or `Moved`:

1. **Map it to existing work.** If a review backlog entry already covers the finding, add or retain that backlog slug on the finding and include the mapping in the review table.
2. **Promote it.** If it meets the criteria below and no backlog entry covers it, create an entry in `review/BACKLOG.md`, add its slug to the finding, and include the mapping in the review table. Update an existing coherent batch instead of creating a duplicate.
3. **Keep it as inventory.** If it does not justify its own workspace or belong to an existing batch, leave it unmapped in the review ledger. This is a deliberate queue decision, not forgotten triage; reconsider the finding at every later incremental review while it remains open.
4. **Fix it now.** A `Docs` finding whose fix is a small markdown edit, such as a stale number, a retired name, a dangling slug, or a broken link, is corrected in its own commit on the review branch and recorded as `Fixed` with that commit rather than kept as inventory. Documentation drift compounds cheaply and is cheapest to correct at the moment it is found.

Do this once at review close, after statuses and new findings are settled. Do not continuously promote findings ad hoc between reviews unless the selection hierarchy's explicit-assignment, P0-direction, or current-scope exception applies.

### What qualifies

File a review backlog entry when a finding, or a coherent batch of findings, is worth its own workspace: a verified bug, a bug with user-visible impact, a structural change that unblocks other work, or a set of small defects in one area that can be fixed together. Do not file entries for findings that are small and depend on a listed refactor, or that an existing backlog entry already covers; cross-reference the existing slug in the finding instead.

### Where they go

Review-derived entries live in [`review/BACKLOG.md`](../review/BACKLOG.md), not in `TODO.md`, so that correctness and maintainability work stays distinct from deferred product ideas. The entry format, the `Findings` line, and the reason the file has no workflow stages are defined in the queue boundaries of [`TODO_GUIDE.md`](TODO_GUIDE.md).

### Resolving

Claiming, the `## Why` section, claim and resolution markers, partial resolution, and remainder entries all follow [`TODO_GUIDE.md`](TODO_GUIDE.md). This guide adds only what is specific to review work.

For an explicitly selected raw finding with no backlog entry, use the raw slug as the Conductor workspace name, include it in the branch name, and state in the pull request why the direct-selection exception applies. Search active workspaces and open pull requests for both the raw slug and any review backlog entry that now maps it before proceeding.

#### Validation instructions

Every pull request that includes a `Resolves review finding: <slug>` marker must also include clear validation instructions in its description after the opening motivation section. These are primarily instructions for a human reviewer to follow. Whenever practical, write a short, plain-language product scenario that a reviewer can perform manually: for example, start a game, change the affected setting, return to the main menu, describe what broke there before the fix, and state what the reviewer should now observe instead. Include any setup needed to expose the original failure, the actions that exercise the fix, and the expected result.

Automated checks are supporting evidence, not a substitute for human validation when a manual check is possible. Do not supply a transcription of Playwright steps, a test implementation, a generic statement such as “tests pass,” or only a link to CI and call that validation guidance. If the finding cannot reasonably be checked through the product, provide a human-runnable command, inspection, measurement, or other concrete procedure, explain what it validates, and state what evidence constitutes success. Findings may share one procedure when it validates all of them. The pull request is not ready for review until these instructions are present.

### Closure handoff

Merging a pull request or removing its review backlog entry does not itself close a review finding. The next incremental review owns verified closure:

1. Search merged pull requests since the previous reviewed commit for `Resolves review finding: <slug>` markers.
2. Inspect the resulting code at the new reviewed commit and record its current `Where` location and enclosing symbol.
3. Re-run the original reproduction for a `Verified` finding; for a `Read` finding, repeat the relevant inspection or suite check.
4. Record the finding in the closed list with the pull request or commit, location, and confirmation required by the closed finding format. If verification fails, carry the finding forward as `Open` or `Moved` and restore or update its review backlog coverage.

This preserves review files as historical evidence while making the latest incremental review the authoritative inventory of open and closed findings.

### Accepting a finding without fixing it

If the decision is not to fix, say so in the next review by marking the finding `Accepted` with the reason. An accepted finding does not get a review backlog entry and is not carried forward as open.
