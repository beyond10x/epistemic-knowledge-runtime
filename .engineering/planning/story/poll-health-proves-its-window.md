---
format: aep.planning-md/3
id: story:poll-health-proves-its-window
kind: story
status: draft
title: A poll records its status and proves only its window
relations:
- serves: vision:o5
- decomposes: epic:p2-observation-layer
- depends_on: story:source-adapter-contract
revision: 1
---
## Context

Roadmap P2 carries poll health (A8): `checked_through`, `attempt | complete | partial | failed`,
and the rule that a successful poll proves only its window. `story:source-adapter-contract` says
the engine "reports poll health and coverage" but does not specify these semantics. A consumer
instance handles a truncated page as a partial poll on its own.

## Build

In `ekr.observe`, spec first: a poll result carries its status and the window it proves;
`checked_through` for a unit advances only over complete windows; a partial or failed poll leaves
it where it was and is reported in coverage.

## Acceptance

- A fixture adapter returning a truncated page records `partial` and does not advance
  `checked_through`.
- A failed poll after a complete one leaves `checked_through` at the complete poll's window end.
- Coverage names the last status and `checked_through` per declared unit.
