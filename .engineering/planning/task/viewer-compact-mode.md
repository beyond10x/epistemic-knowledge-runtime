---
format: aep.planning-md/3
id: task:viewer-compact-mode
kind: task
status: active
title: The viewer's sidebars collapse to the edge and restore
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T01:20:44Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-30T01:20:44Z", actor: "human:timo", revision: 3}
---
## Context

The operator asked on 2026-09-30 for a "compact" mode in the viewer (`ekr view`): both sidebars
(`<aside class="left">`, the filters and types, and `<aside class="right" id="panel">`, the detail
panel, in `crates/ekr/src/cli/viewer/index.html`) collapse to the window's edge, in a way that can be
undone, so the graph gets the whole width.

## Build

- A compact toggle collapses both sidebars to a thin strip at their edge; each strip stays visible
  and restores its sidebar on click. A keyboard shortcut toggles compact mode for both at once, and
  each sidebar can also be collapsed or restored on its own.
- The graph canvas takes the freed width and re-lays out without losing the camera or the
  selection; in the 3D view the render loop wakes for the resize and rests again.
- The state is part of the page's address (as the other view state is, `pushState`), so a reload or
  a shared link keeps it; default is not compact.
- The page stays data-free and passes `crates/ekr/tests/view_page.rs`.
- The pending shift+click type-solo change (branch `viewer/shift-click-solo`, `4eb71965`) is merged
  into this unit so both are tried in one build.

## Acceptance

- `view_page` cases: the toggle and the strips exist; the compact state is carried by the address
  and restored from it; both sidebars have a restore control while collapsed.
- A headless browser check, as earlier viewer units did, shows the canvas width grows by the sidebars'
  width in compact mode and returns when restored, and no console error.
- The operator tries it (a try-out step), and the unit merges after that.
