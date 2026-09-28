---
format: aep.planning-md/3
id: task:view-load-replays-once
kind: task
status: implemented
title: ekr_views::load replays once, not once per schema version
relations:
- serves: vision:o5
- blocks: story:view-streams-overview-and-expansion
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T02:45:11Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-28T02:45:11Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-28T06:19:10Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Context

`ekr view`'s first request to a revision takes 11.25 s on the bench store (2,288 nodes, 6,314
edges, 76 revisions, 5 schema versions, file provider): `ekr_views::load`
(`crates/ekr-views/src/lib.rs:185-250`) calls `runtime.replay(n)` for every earlier schema-version
boundary, and each replay starts from revision 0. Measured on the same store: `ekr snapshot` 1.05 s,
`ekr snapshot --at 40` 2.6 s, a full replay 3.4 s. The plan's target for a first `/overview` is
under 2 s (story:view-streams-overview-and-expansion).

## Build

One pass: collect the ontology at each schema-version boundary while replaying once up to the
requested revision, through a kernel read that returns the schema history (for example
`Runtime::schema_history(at)`), instead of one replay per boundary. The loaded revision stays
byte-identical in what it renders (the ekr-views conformance suite and determinism tests hold).

## Acceptance

On the bench store, the first `/overview` of the head answers in under 2 s (release build), and
`ekr_views::load` performs one replay whatever the number of schema versions (a test counts it).
