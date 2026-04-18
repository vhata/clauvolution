# Creature Portrait — Detailed Inspect Visualization

A dedicated rendering area in the inspect panel that shows a large, detailed, visually pleasing portrait of the selected organism — its body and its brain. The map sprites stay simple (circles/blobs); this is only rendered when you click to inspect.

Pairs naturally with **Brain Activation Heatmap** (see [ROADMAP.md](../ROADMAP.md) Theme 1) — the portrait shows the creature, the brain DAG shows its thinking, both animated live.

## Body visualization: modular sprite stacking

Build on the existing `BodyPlan` system but render at much larger scale with better art.

**Approach:**
- Library of body part shapes (SVG-style or procedural meshes), each with anchor points for attachment
- Parts snap onto predefined coordinates around the torso, respecting bilateral symmetry
- Proper z-ordering: armor plates behind torso, limbs to sides, eyes and mouth in front, claws at edges
- Only 10-20 distinct part shapes needed to create thousands of unique combinations

**Part representations:**
- **Torso** — smooth organic ellipse, size reflects genome body_size
- **PhotoSurface** — leaf-like structures growing from the back, subtle green glow/particle effect
- **Claws/Fangs** — sharp triangles at the mouth area, size reflects attack_power
- **Fins** — semi-transparent veined shapes on the sides
- **Eyes** — prominent circles with pupils, count reflects eye_count
- **Armor** — layered plate shapes behind the body, opacity reflects armor_value
- **Limbs** — jointed segments radiating from torso
- **Mouth** — opening on the front, size reflects foraging ability

## Trait-based color coding

Colors communicate biology at a glance:
- **Energy source**: green tint = photosynthetic, red/dark tint = predator
- **Environment**: blue/sleek = aquatic adapted, tan/rough = land/desert adapted
- **Health**: brightness reflects current energy level
- **Species**: hue from species colour, so related organisms look related

## Brain visualization: NEAT topology graph

Render the actual neural network as a node graph below or beside the body portrait.

- **Nodes as circles**: inputs on the left, outputs on the right, hidden neurons in the middle
- **Connections as lines**: thickness proportional to weight, colour = positive (blue) vs negative (red)
- **Disabled connections**: shown as faint dotted lines
- **Labels**: input nodes labelled with what they sense (food dir, energy, group size, etc.), output nodes with what they do (move, eat, attack, etc.)
- **Live activation**: nodes brighten when firing, connections pulse when transmitting (this is the Brain Activation Heatmap feature merging with the portrait)
- Shows brain complexity at a glance — a simple forager has few connections, a sophisticated predator has a dense web

## Procedural generation options (future)

For even more organic-looking creatures:

- **Metaballs**: 2D organic blobs that merge into each other. Adding a "tail" means adding a new metaball at the rear. Looks like a living, squishy organism. Requires a custom shader.
- **L-Systems**: Branching fractal structures — particularly good for photosynthesizers. Genome parameters control branching angle, depth, and leaf density.

## Visual polish

- Consistent line weight (2px) and limited colour palette for coherent aesthetic
- Simple "squash and stretch" animation — breathing/pulsing idle animation
- Winged creatures get slight bobbing motion
- Photosynthesizers sway gently
- Scale the portrait to fit the inspect panel regardless of organism body_size

## Implementation notes

- Render as a separate Bevy camera/layer or an egui canvas (pairs well with the bevy_egui UI)
- Only rendered for the single selected organism — no performance concern
- Portrait updates live as the organism moves, eats, takes damage, etc.
- Could eventually support "compare two organisms" side-by-side — which dovetails into the Genome Diff View feature
