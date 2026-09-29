---
format: aep.planning-md/3
id: task:retire-alt-viewer-page
kind: task
status: active
title: Remove the earlier viewer page served at /alt
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T15:13:56Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-09-29T15:13:56Z", actor: "human:timo", revision: 5}
---
## Context

`ekr view` still serves the pre-rewrite page at `GET /alt` (`crates/ekr/src/cli/view.rs:50`, `:103`,
`:623`), "kept while the new one is accepted". It is 65,297 bytes beside the 199,766-byte main page.

## Build

Once the operator accepts the streamed page, remove `/alt`, `viewer/alt.html`, its route and its test
entries, and the `docs/cli.md` row.

## Acceptance

- `GET /alt` answers 404; the route list test names the remaining routes; `docs_cli` passes.

## Operator decision (relayed)

Relayed 2026-09-29 by the consumer instance's session: the operator approved retiring `/alt` once `story:viewer-3d-draws-in-batches` and `task:hops-slider-streams-the-neighbourhood` (wave sdk-01) are merged, with no further acceptance step. Confirmed by the operator in the coordinator session the same day ("jsut continuze now", answering that question). Scheduled in wave sdk-01 after unit G.
