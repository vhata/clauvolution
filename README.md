# Clauvolution

An evolution simulator where life emerges, adapts, competes, and speciates — all without programmer-designed species or behaviours.

Watch organisms evolve in real time. Each creature has its own neural network brain, inherited from its parents with mutations. Natural selection does the rest. Predators evolve claws. Prey evolves armor. Plants terraform the landscape. And sometimes an asteroid wipes the slate clean.

## Running

```bash
cargo run --release
```

Each run gets a unique cosmic name (e.g. "pale-fading-shard"). Session data — chronicle log, screenshots, save files — lives in `sessions/<name>/`.

```bash
# Save: press F5 during gameplay
cargo run --release -- --load sessions/pale-fading-shard   # load a saved session
cargo run --release -- --screenshot                        # scripted verification tour
```

Requires Rust (latest stable) and a GPU with Metal / Vulkan / DX12 support.

## Controls

| Key | Action |
|-----|--------|
| **WASD / Arrows** | Pan camera |
| **Q / E** or **- / +** or **Scroll** | Zoom |
| **Right-click drag** | Pan camera |
| **Left click** | Select organism (inspect panel) |
| **F** | Focus camera on selected organism |
| **,** / **.** | Cycle through living members of the selected organism's species |
| **R** | Select a random living organism |
| **Space** | Pause / unpause |
| **[** / **]** | Slow down / speed up (0.125× to 16×) |
| **M** | Toggle minimap heatmap |
| **T** | Toggle trail for selected organism |
| **X** / **I** / **V** | Asteroid / Ice age / Volcano |
| **B** / **N** / **J** | Solar bloom / Nutrient rain / Cambrian spark |
| **S** | Screenshot |
| **F5** | Save world |

The right-side panel has tabs for Inspect, Phylo, Graphs, Chronicle, Events, and Help.

## Documentation

- **[docs/FEATURES.md](docs/FEATURES.md)** — one-liner list of everything the simulation does
- **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** — how the code is organised, what runs when, where to find things
- **[docs/DECISIONS.md](docs/DECISIONS.md)** — the non-obvious design choices and their tradeoffs
- **[docs/ROADMAP.md](docs/ROADMAP.md)** — what's next, ongoing concerns, backlog
- **[docs/design/](docs/design/)** — detailed design docs for bigger features

## Built with

Built collaboratively by [Jonathan Hitchcock](https://github.com/vhata) and [Claude Code](https://claude.ai/claude-code) (Anthropic's Claude, various versions).

## License

MIT
