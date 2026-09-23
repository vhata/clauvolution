# Review backlog

Work promoted from whole-codebase reviews because it merits its own branch belongs here. A request to “grab something from the review backlog” means this file; a request to “grab something from the TODO” or “grab something from the roadmap” does not. Every entry is ready for separate work and is prioritized directly rather than grouped by workflow stage.

See the [`code review guide`](../docs/CODE_REVIEW_GUIDE.md) for how findings enter and leave this backlog and the [`backlog workflow guide`](../docs/TODO_GUIDE.md) before claiming or resolving an entry.

## P0 Critical

## P1 High

## P2 Normal

- [TOOLING] `screenshot-tour-egui-path` — **Make `--screenshot` use the egui-aware capture path and wait for its last image.** The legacy tour exits before the sixth capture is written and its images have no header, panel, or minimap, so the baseline check in the code review guide proves only that a window opens.
  - Starting point: Drive `--screenshot` through `clauvolution_render::begin_screenshot` (or load a bundled tour JSON) and gate `AppExit` on `ScreenshotState.pending` being clear. Validate: `cargo run --release -- --screenshot` writes six PNGs and each shows the side panel.
  - Source: review/2026-09-17-0756-full.md, 2026-09-17
  - Findings: `screenshot-tour-drops-last-capture`

## P3 Low

## Unprioritized
