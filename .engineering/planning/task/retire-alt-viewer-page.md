---
format: aep.planning-md/3
id: task:retire-alt-viewer-page
kind: task
status: draft
title: Remove the earlier viewer page served at /alt
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
revision: 1
---
## Context

`ekr view` still serves the pre-rewrite page at `GET /alt` (`crates/ekr/src/cli/view.rs:50`, `:103`,
`:623`), "kept while the new one is accepted". It is 65,297 bytes beside the 199,766-byte main page.

## Build

Once the operator accepts the streamed page, remove `/alt`, `viewer/alt.html`, its route and its test
entries, and the `docs/cli.md` row.

## Acceptance

- `GET /alt` answers 404; the route list test names the remaining routes; `docs_cli` passes.
