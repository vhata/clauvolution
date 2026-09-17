# Code review guide

This guide defines how whole-codebase reviews are performed, recorded in [`review/`](../review/), kept current through cheaper incremental reviews, and promoted into the tracked [`review/BACKLOG.md`](../review/BACKLOG.md) work queue. It is separate from pull request review, which continues to happen on each change.

Read this guide when asked to perform a full or incremental code review, when resolving a review finding, or when deciding whether a review finding should enter the review backlog.

## Why two kinds of review

A pull request review sees one change. Some problems are properties of the whole codebase rather than of any change: two copies of a tuning rule that drift apart, a helper that becomes dead when its last caller is removed elsewhere, a crate's `lib.rs` that crosses a size threshold fifty lines at a time, a documented control or CLI flag that no longer matches the code behind it, or a latent bug that a later rename or schedule reorder makes live. A **full review** reads everything and finds these. An **incremental review** starts from the last review, examines only what changed since, re-checks the standing findings, and runs cheap whole-repo checks. Incremental reviews are the default; full reviews are a periodic reset.

## Artifacts

### The `review/` directory

- `review/README.md` is the index: one row per review, newest first, with type, reviewed commit, and the count of findings open when the review closed. The top row is the baseline for the next incremental review.
- `review/BACKLOG.md` is the mutable queue of review-derived work approved for a separate branch. It is not a review snapshot.
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

### Invariants

Invariants are derived from this codebase, not imposed on it. Record one when a review finds the same class of problem twice, or when a choice recorded in [`DECISIONS.md`](DECISIONS.md) only holds as long as code keeps to a rule that nothing enforces. Properties this project has repeatedly depended on, offered as examples of the kind of thing worth recording rather than as a fixed checklist:

- Simulation crates do not import from `clauvolution_render` or `clauvolution_ui`, which is what keeps headless mode possible.
- A system that depends on another running first says so through an explicit ordering constraint rather than relying on registration order.
- Hot `FixedUpdate` systems do not allocate per organism per tick.
- `Commands` is used where deferred structural change is actually needed, not where a direct mutation through a query would do.
- Work handed to Rayon through `par_iter` or `par_iter_mut` does not touch `SimRng`, `Commands`, or any other shared mutable state.
- Every control, CLI flag, and feature described in the documentation has code behind it.

### Finding format

```md
- `finding-slug` — **One-sentence title.** Kind · Status · Verification.
  - Where: `path:lines` (`symbol`); another `path` (`symbol`).
  - Detail, failure scenario, and suggested fix in one to four sentences.
  - Review backlog: `backlog-slug`
```

- **Slug**: unique, descriptive kebab-case, immutable once published. Later reviews refer to the finding by slug.
- **Kind**: `Bug`, `Design`, `Duplication`, `Performance`, `Test`, `Style`, `Tooling`, or `Docs`. `Design` covers ECS structure and system ordering: a system that reads state a later system in the same tick writes, a resource two crates mutate with no ordering constraint between them, a schedule placement that works only by accident.
- **Status**: `Open`, `Fixed`, `Moved`, `Accepted`, `Invalid`, or `Superseded`. Every status other than `Open` carries evidence:
  - `Fixed`: name the commit or pull request, give the `Where` at the reviewed commit that shows the fix (file, lines, symbol), and state how it was confirmed. For a finding that was `Verified`, re-run the original reproduction and record that it no longer reproduces; closing a verified bug by reading alone is not sufficient.
  - `Moved`: still open; give the new `Where`.
  - `Accepted`: will not fix; give the reason.
  - `Invalid`: the finding was wrong; quote or cite the code that shows why.
  - `Superseded`: name the replacing finding slug.
- **Verification**: `Verified` when reproduced in a running release build of the sim, reproduced in a headless run, or confirmed by a test in the workspace suite; `Read` when established by inspection only.
- **Where**: file paths with line numbers valid at the reviewed commit, plus the enclosing symbol so the reference survives edits. To see a reference as it was, run `git show <reviewed-commit>:<path>`; to map it forward, run `git blame -L <line>,<line> <reviewed-commit> -- <path>` and follow the change.
- **Review backlog**: present only when the finding is covered by an entry in `review/BACKLOG.md`.

### Closed finding format

Findings that leave `Open` appear in the closed list as one line each, with all three parts: the reference, the location at the reviewed commit, and the confirmation. A closed entry missing any part is malformed, in the same way a finding without a slug is.

```md
- `finding-slug` — Fixed in #45 (f10a8e2). `path:lines` (`symbol`): one sentence on what the code now does. Re-reproduced: no longer reproduces.
```

A pull request number alone is not enough. It says where to look, not what to look at, and without the location and confirmation the next review has to re-derive the closure from scratch. Pull requests are squash merged, so the hash in the reference is the single commit the pull request became on `main` and the pull request description is that commit's message. Running `git show f10a8e2` therefore shows both the change and the motivation it was merged for.

One finding is one thing that can be independently fixed and independently verified. Group many small instances of the same smell into one finding (for example, one finding for a repeated unchecked `unwrap()` pattern, listing every site) rather than one finding per site.

## Baseline checks

Both kinds of review start from the same checks. Run them at the reviewed commit and record each result in the header table.

```bash
cargo build --release
cargo clippy --all-targets
cargo test --workspace
cargo run --release -- --headless 1000 --seed 42
cargo run --release -- --screenshot
```

Record the clippy warning count as a number, not just pass or fail, so the next review can compare against it:

```bash
cargo clippy --all-targets --message-format=short 2>&1 | grep '^warning: ' | grep -vc generated
```

The headless run prints a summary: final population, species count, max generation, births, deaths by cause, strategy breakdown, trait averages, and the predation funnel. Record that summary in full. Comparing it against the previous review's is how a silent change in simulation dynamics shows up, and a run that dies out or hangs is itself a finding. `--screenshot` runs the scripted tour and exits; it must complete and write its images under `sessions/<name>/`.

`cargo test --workspace` currently runs no tests, so it confirms that every crate still compiles as a test target and nothing more. Record that plainly rather than reporting a green suite. A `Test` finding that adds real coverage changes what this check is worth.

A check that passed at the previous review and fails now is a finding.

## Performing a full review

1. Record the reviewed commit before reading anything. Run the baseline checks above and record their results.
2. Read everything. Delegate large files to parallel subagents where that helps, but verify their claims against the source before recording them. Do not record a claim you could not confirm.
3. Reproduce the most serious bugs and mark them `Verified`: in a running release build for anything involving rendering, input, or the egui panels, and in a headless run for simulation behaviour. Bugs that cannot be reproduced stay `Read`.
4. **Triage into the work queue.** Apply the mandatory review-close triage below to every open finding. File or update review backlog entries and complete the review's backlog mapping.
5. Write the review file and invariants, then add the row to `review/README.md`.
6. Open the review as its own pull request. One pull request does one thing, so the review never carries a code fix. Small documentation fixes made under the triage rule below are their own commits on the same branch and land with the review.

## Performing an incremental review

Inputs: this guide, the most recent review file, and the repository. The reviewed commit of the previous review is the **base**; the current `HEAD` is the new reviewed commit.

1. **Baseline.** Run the baseline checks above and record them. A clippy warning count that rose since the previous review with no finding to explain the rise is itself a finding.
2. **Triage standing findings.** For each finding in the previous review, determine its new status. Read the relevant code; do not infer status from pull request titles, and for `Verified` findings re-run the reproduction before closing. Check merged pull requests for `Resolves review finding: <slug>` lines. Carry every finding forward with its slug: `Fixed`, `Accepted`, `Invalid`, and `Superseded` findings appear in the closed list using the closed finding format; `Open` and `Moved` findings appear in full with any updated `Where`.
3. **Review the delta.** Run `git log --oneline <base>..HEAD` and `git diff <base>..HEAD --stat`, then read every hunk. Judge each hunk against the invariants and against the standing findings: a change that adds another copy of something already listed as duplicated is a finding even though the hunk is locally fine. Record new findings with new slugs.
4. **Run whole-repo mechanical checks.** These are cheap and catch drift that no hunk shows. They are listed below. Compare every number against the previous review's, and record the new numbers even when nothing moved.
5. **Sweep hot files.** Any file changed by most of the pull requests in the delta gets a full re-read, not just its hunks. Interaction bugs form where changes concentrate.
6. **Re-verify.** Re-run the reproductions for `Verified` bugs that are still open, in the release build or headless as the original reproduction required, and reproduce new serious bugs.
7. **Triage into the work queue.** Apply the mandatory review-close triage below to every `Open` or `Moved` finding, including findings carried from the previous review. File or update review backlog entries and complete the review's backlog mapping.
8. **Write the file** as `review/YYYY-MM-DD-HHMM-incremental.md` with `Previous review` filled in, update the invariants, update `review/README.md`, and open it as its own pull request.

A prompt that starts an incremental review: "Read `docs/CODE_REVIEW_GUIDE.md` and the most recent review in `review/`, then do an incremental review."

### Whole-repo mechanical checks

Keep this list to checks that need nothing beyond `cargo`, `git`, and the shell. Do not add one that requires installing a tool.

```bash
# Suppressed lints. Each needs a stated reason and the count should not drift up.
grep -rn '#\[allow(' crates --include='*.rs'

# Largest source files.
find crates -name '*.rs' -exec wc -l {} + | sort -rn | head

# Systems defined per file, a proxy for how much each crate is carrying.
grep -rc 'fn [a-z_0-9]*_system' crates/*/src/*.rs

# Deferred work left in code rather than in a tracked queue.
grep -rnE 'TODO|FIXME|HACK' crates

# Panic sites in non-test code.
grep -rnE '\.unwrap\(\)|\.expect\(' crates
```

Alongside those, read for the checks no one-liner covers:

- **Largest systems.** Find the longest system functions in `clauvolution_sim`, `clauvolution_render`, and `clauvolution_ui` and record their line counts. A system that keeps growing is the usual precursor to a `Design` finding.
- **Controls drift.** Compare the controls table in `README.md` against the `KeyCode` matches in `keyboard_to_events_system` (`crates/clauvolution_sim/src/lib.rs`), `clauvolution_render`, and `clauvolution_ui`. A key bound in code and absent from the table, or documented and no longer bound, is a `Docs` finding.
- **CLI drift.** Compare the flags shown in `README.md` against the argument parsing at the top of `crates/clauvolution_app/src/main.rs`.
- **Feature drift.** Compare `docs/FEATURES.md` against what the code actually does, and the tick listing in `docs/ARCHITECTURE.md` against the order systems are registered in.
- **Link rot.** Confirm that every relative markdown link in `README.md`, `CLAUDE.md`, and `docs/` resolves to a file that exists.

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
3. **Keep it as inventory.** If it does not justify its own branch or belong to an existing batch, leave it unmapped in the review ledger. This is a deliberate queue decision, not forgotten triage; reconsider the finding at every later incremental review while it remains open.
4. **Fix it now.** A `Docs` finding whose fix is a small markdown edit, such as a stale number, a retired name, a dangling slug, or a broken link, is corrected in its own commit on the review branch and recorded as `Fixed` with that commit rather than kept as inventory. Documentation drift compounds cheaply and is cheapest to correct at the moment it is found.

Do this once at review close, after statuses and new findings are settled. Do not continuously promote findings ad hoc between reviews unless the selection hierarchy's explicit-assignment, P0-direction, or current-scope exception applies.

### What qualifies

File a review backlog entry when a finding, or a coherent batch of findings, is worth its own branch: a verified bug, a bug with user-visible impact, a structural change that unblocks other work, or a set of small defects in one area that can be fixed together. Do not file entries for findings that are small and depend on a listed refactor, or that an existing backlog entry already covers; cross-reference the existing slug in the finding instead.

### Where they go

Review-derived entries live in [`review/BACKLOG.md`](../review/BACKLOG.md), not in `TODO.md`, so that correctness and maintainability work stays distinct from deferred product ideas. The entry format, the `Findings` line, and the reason the file has no workflow stages are defined in the queue boundaries of [`TODO_GUIDE.md`](TODO_GUIDE.md).

### Resolving

Claiming, the `## Why` section, claim and resolution markers, partial resolution, and remainder entries all follow [`TODO_GUIDE.md`](TODO_GUIDE.md). This guide adds only what is specific to review work.

For an explicitly selected raw finding with no backlog entry, create a worktree and a branch both named after the raw slug, and state in the pull request why the direct-selection exception applies. Check for an existing claim before starting: search open pull requests, `git worktree list`, and `git branch -a` for both the raw slug and any review backlog entry that now maps it.

#### Validation instructions

Every pull request that includes a `Resolves review finding: <slug>` marker must also include clear validation instructions in its description after the opening motivation section. These are primarily instructions for a human reviewer to follow. Whenever practical, write a short, plain-language scenario the reviewer can perform in the running sim. For example, for a hypothetical fix to an ice age that had no effect: run `cargo run --release -- --seed 42`, let the population settle, press `I` to trigger an ice age, and open the Graphs tab. Before the fix the population curve kept climbing straight through the freeze; the reviewer should now see population fall for the duration of the event and recover once it ends. Include any setup needed to expose the original failure, the actions that exercise the fix, and the expected result.

Where the fix changes simulation behaviour rather than presentation, a headless run serves the same purpose. Give the exact invocation, such as `cargo run --release -- --headless 2000 --seed 42`, name the lines of the printed summary that carry the evidence, and state what those numbers looked like before and what they should look like now.

Automated checks are supporting evidence, not a substitute for human validation when a manual check is possible. Do not supply a test implementation, a generic statement such as "tests pass", or only a link to CI and call that validation guidance. If the finding cannot reasonably be checked through the running sim, provide a human-runnable command, inspection, measurement, or other concrete procedure, explain what it validates, and state what evidence constitutes success. Findings may share one procedure when it validates all of them. The pull request is not ready for review until these instructions are present.

### Closure handoff

Merging a pull request or removing its review backlog entry does not itself close a review finding. The next incremental review owns verified closure:

1. Search merged pull requests since the previous reviewed commit for `Resolves review finding: <slug>` markers.
2. Inspect the resulting code at the new reviewed commit and record its current `Where` location and enclosing symbol.
3. Re-run the original reproduction for a `Verified` finding; for a `Read` finding, repeat the relevant inspection or baseline check.
4. Record the finding in the closed list with the pull request or commit, location, and confirmation required by the closed finding format. If verification fails, carry the finding forward as `Open` or `Moved` and restore or update its review backlog coverage.

This preserves review files as historical evidence while making the latest incremental review the authoritative inventory of open and closed findings.

### Accepting a finding without fixing it

If the decision is not to fix, say so in the next review by marking the finding `Accepted` with the reason. An accepted finding does not get a review backlog entry and is not carried forward as open.
