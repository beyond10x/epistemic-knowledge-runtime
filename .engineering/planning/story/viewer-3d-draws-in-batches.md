---
format: aep.planning-md/3
id: story:viewer-3d-draws-in-batches
kind: story
status: draft
title: The 3D view draws all nodes and edges in a few calls and rests when idle
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
scope:
- confidence: cited
  path: crates/ekr/src/cli/viewer/index.html
revision: 2
---
## Context

The 3D view of `ekr view` draws each node and each edge as its own three.js object through
`3d-force-graph` 1.80.0 (`crates/ekr/src/cli/viewer/index.html`, `create3D`, ~line 1033), and its
render loop runs every frame even when nothing moves.

Measured 2026-09-29 on a consumer instance's store (revision 58: 4,127 nodes, 14,373 edges), served
by `ekr` 0.0.14, headless Brave with the GPU (ANGLE on Vulkan, NVIDIA GeForce RTX 3090), 60 s after
load, 120 frames, draw calls counted by wrapping `drawArrays`/`drawElements`:

| view | draw calls per frame | frame time median | p90 |
|---|---|---|---|
| 3D (`#view=3d`) | 10,300 | 116.7 ms | 183.4 ms |
| 2D (`#view=2d`) | 0 (idle) | 16.7 ms | 16.7 ms |

About 8.6 frames per second at rest. The GPU is not the limit: the cost is one draw call per object
submitted from the CPU, every frame. Whether 0.0.13's page was faster was not measured (`/alt` did
not enter 3D from the URL).

## Build

- Edges drawn as one `THREE.LineSegments` over one buffer geometry, updated in place on each engine
  tick; per-edge colour in a colour attribute. Nodes drawn as one `THREE.InstancedMesh` per glyph
  kind, with per-instance colour and scale.
- Hover and click on nodes by raycasting the instanced meshes; on edges, only where the graph holds
  at most 5,000 edges (today's rule, `noLinkHover`).
- The render loop pauses when the layout has stopped and no control, hover or animation is active,
  and resumes on the next interaction.
- Arrows and particles keep today's thresholds.
- The 2D view is unchanged.

## Acceptance

- On a generated fixture of 4,000 nodes and 14,000 edges, the 3D view at rest draws at most 20 calls
  per frame (the probe's counter), and 0 calls per frame while idle with the loop paused.
- The same fixture renders a frame in under 17 ms median in headless Brave with the GPU, on the
  machine the measurement above came from.
- Clicking a node focuses it, and hovering shows its tip, as today.
- `crates/ekr/tests/view_page.rs` and the viewer's data-free rule pass.
