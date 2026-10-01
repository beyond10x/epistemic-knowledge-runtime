---
format: aep.planning-md/3
id: task:viewer-compact-follow-ups
kind: task
status: implemented
title: Compact mode shows a hidden selection; chips and tabs work from the keyboard; narrow windows
relations:
- serves: vision:o5
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T08:12:46Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-30T08:12:46Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-30T12:13:42Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

Wave reads-04 unit V's adversary pass on the viewer's compact mode and shift+click type solo
(`crates/ekr/src/cli/viewer/index.html`) left three notes:

- With the right sidebar collapsed, choosing a node renders its detail into the hidden panel, and the
  strip gives no sign of it (`index.html:2325`).
- Type chips are plain `div`s with no role or tabindex, so hiding or soloing a type has no keyboard
  path (pre-existing, `index.html:1878`); the fold tabs' accessible name is the arrow character and
  the Compact button has no pressed state.
- Below 650 px with both sidebars shown, the graph has no width and the right sidebar runs off-screen
  (`index.html:21`).

## Build

- A chosen node while the right panel is collapsed marks the strip (or opens the panel; decide and
  document).
- Chips are buttons (or carry role and tabindex) with keyboard activation, shift for solo; fold tabs
  and the Compact button have accessible names and `aria-pressed`.
- Under a width threshold the page starts compact, or the sidebars shrink, so the graph keeps a
  usable width.

## Acceptance

- `view_page` cases for each of the three.
