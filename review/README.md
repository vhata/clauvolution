# Code reviews

Whole-codebase and incremental review ledgers. Review snapshots are indexed below; the mutable queue of findings promoted to their own branch is [`BACKLOG.md`](BACKLOG.md). The process is defined in [`docket/CODE_REVIEW_GUIDE.md`](../docket/CODE_REVIEW_GUIDE.md); the baseline commands and drift checks every review runs are listed under Review inputs in [`docs/QUALITY.md`](../docs/QUALITY.md#review-inputs). Each review lands as its own pull request. Newest review first; the top row is the baseline for the next incremental review.

| Review                                                           | Type        | Reviewed commit | Open findings at close |
| ---------------------------------------------------------------- | ----------- | --------------- | ---------------------- |
| [2026-10-24-0257-full.md](2026-10-24-0257-full.md) | Full | `f3b6b38` | 70 |
| [2026-10-09-0906-full.md](2026-10-09-0906-full.md)               | Full        | `ab0fd98`       | 74                     |
| [2026-09-17-0756-full.md](2026-09-17-0756-full.md)               | Full        | `e6bcf49`       | 47                     |

<!-- This section must stay below the index table: review-due.sh reads the first table row with a backticked commit as the latest review. -->
## Pending reconciliation

Findings whose fix has landed with independent verification but whose closure the next review has not yet recorded. The next incremental review verifies each and clears the list.

| Finding | Fix PR and commit | Reviewer | Evidence |
| --- | --- | --- | --- |
