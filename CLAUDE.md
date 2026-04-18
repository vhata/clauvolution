# Clauvolution — Project Context

An evolution simulator built in Rust + Bevy 0.15. Organisms with NEAT neural network brains evolve in a procedural world. No behaviours are programmed — everything emerges from selection pressure. The user watches evolution happen in real time.

**Core motivation:** pure curiosity and joy in watching evolution unfold. This is a personal project — no research question to answer, no audience to ship to. Priorities should serve that motivation. See `docs/ROADMAP.md` for the full framing.

## Where to find what

- **`docs/FEATURES.md`** — everything the sim currently does, grouped and one-lined
- **`docs/ARCHITECTURE.md`** — crate layout, the simulation tick, ECS patterns, where-to-find-X table
- **`docs/DECISIONS.md`** — non-obvious design choices and their tradeoffs (the "why did we do that?" doc)
- **`docs/ROADMAP.md`** — what's next, themed by priority; ongoing concerns; backlog
- **`docs/design/`** — detailed design docs for bigger features (creature portrait etc.)

Start there before reading code for any meaningful change. The decisions doc in particular captures nuance that would otherwise only live in chat history.

## Working agreements

- Commit regularly in discrete, well-sized, single-purpose commits. No leaving work uncommitted.
- No `Co-Authored-By` trailers in commit messages — credits live in the README.
- When adding a new simulation dynamic, budget a follow-up tuning pass. Instrument first (ensure Graphs tab can show the dynamic's effect), then tune.
- Prefer editing existing files to creating new ones.

## Build & run

```bash
cargo run --release                              # normal
cargo run --release -- --screenshot              # scripted tour + screenshot
cargo run --release -- --load sessions/<name>    # load saved session
```

Incremental release builds are enabled in Cargo.toml for fast iteration after the first compile.

## Controls summary

Space=pause, [/]=speed, WASD=pan, scroll=zoom, click=inspect, Shift+S=screenshot, F5=save, M=heatmap, Shift+M=hide minimap, T=trails, X=asteroid, I=ice age, V=volcano, B=solar bloom, N=nutrient rain, J=Cambrian spark, 1–6=switch right-panel tab. Graphs/chronicle/phylo/help are in egui tabs in the right panel.

## Known rough edges

- **LOD at close zoom**: body part meshes are created per-organism (not shared). Could be optimised with shared mesh handles per segment type.
- **Terrain not saved in save files**: terrain regenerated from seed on load. Niche construction changes (vegetation density, moisture, nutrients modified by organisms) are lost on save/load.
- **Species naming collisions**: similar traits + similar species ID modulo → same name. Not a bug, just a limitation of the word-list approach.
- **Convergent evolution detection**: checks every classification tick by scanning all species. Could be noisy early on.
- **Session RNG is not seeded** — only terrain uses the seed; other randomness is `thread_rng()`. Session seeds are on the roadmap.
