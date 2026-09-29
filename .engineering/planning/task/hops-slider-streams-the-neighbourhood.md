---
format: aep.planning-md/3
id: task:hops-slider-streams-the-neighbourhood
kind: task
status: draft
title: Moving the hops slider streams the neighbourhood it names
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
revision: 1
---
## Context

Since 0adab98a7 ("load /overview, stream /expand", 2026-09-28) the page holds only the overview's
top nodes plus what was streamed. The hops slider (`hopsR`, 1–3, `crates/ekr/src/cli/viewer/index.html`
~line 1683) is shown only while a neighbourhood focus is active, and `hopsFrom` (~line 1647) walks
the loaded `graph` only; moving it triggers no `/expand`. An operator reported the slider as gone
(consumer instance, 2026-09-29; read from code, not run).

## Build

Moving the slider to N while focused streams `/expand {seeds: [focus], depth: N}`, as "Expand 2 hops"
does, and computes the set when the stream ends. With nothing focused, the breadcrumb bar says how to
reach the slider.

## Acceptance

- With a focus on a node whose 2-hop neighbourhood is not loaded, moving the slider to 2 issues one
  `/expand` with `depth=2` and the drawn set equals that neighbourhood.
- With nothing focused, the page shows how to get a depth control.
