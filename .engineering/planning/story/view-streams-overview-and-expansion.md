---
format: aep.planning-md/3
id: story:view-streams-overview-and-expansion
kind: story
status: implemented
title: ekr view loads an overview and streams expansions instead of the whole projection
relations:
- serves: vision:o5
- depends_on: story:data-free-graph-viewer
- decomposes: epic:p4-operator-surface
scope:
- confidence: cited
  path: crates/ekr/src/cli/view.rs
- confidence: cited
  path: crates/ekr/src/cli/viewer
- confidence: cited
  path: crates/ekr/tests/view_page.rs
- confidence: cited
  path: crates/ekr/tests/view_stream.rs
- confidence: cited
  path: docs/cli.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T02:33:53Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-28T02:33:53Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-28T06:19:10Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Context

The operator's prototype viewer is the `ekr view` page (PR #36), but it loaded the whole
`ekr.graph-projection/1` (5.6 MB, about 12 s per first render on a store of 2,288 nodes and 6,314
edges). Operator-approved plan (2026-09-28): the page loads a small overview first and expands the
graph on demand through a streamed read, with the data shapes specified in ESS (`ekr.views`
ProjectOverview, ExpandNeighbourhood, DescribeNode, SearchNodes, merged in PR #36) and the
transport outside ESS.

## What is built

- Server (`crates/ekr/src/cli/view.rs`): `/overview`, `/expand` as HTTP/1.1 chunked NDJSON
  (meta, records, progress, end), `/node/<id>`, `/search`, on `ekr-views`' `IndexCache`; the
  existing Host, body, connection-cap and deadline limits hold.
- Page (`crates/ekr/src/cli/viewer/index.html`): the prototype UI's data layer on those endpoints;
  nothing loads the full projection; the sidebar carries the assertion cards and the text-only
  evidence panel; a render budget of 20,000 nodes.

## Acceptance

On the bench store, the page's first load fetches the overview only (under 300 KB), a 1-hop
expansion of the top node streams its first bytes in under 200 ms from a warm index, opening a node
fetches only its detail, and the page names no type, edge type, property or entity. One adversary
pass on the whole (XSS, limits, data-free, determinism) is answered before the merge.

## Known

- The first request to a revision takes about 11 s on the bench store: the engine loads and
  indexes the revision. The plan's target was under 2 s; this is open.
