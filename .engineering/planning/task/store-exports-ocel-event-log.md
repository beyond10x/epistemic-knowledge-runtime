---
format: aep.planning-md/3
id: task:store-exports-ocel-event-log
kind: task
status: implemented
title: A store exports as an OCEL 2.0 event log
relations:
- serves: vision:o5
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T08:12:45Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-30T08:12:45Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-30T12:13:41Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

A consumer designs a replay of its organisation's work and process maps over one event log in the
OCEL 2.0 format, and assigns the generic part to the engine (reported 2026-09-30): exporting any EKR
store as an OCEL 2.0 log. Until the engine does, the consumer reads a snapshot and derives the log
itself.

## Build

- A read verb (and session verb) that exports a revision's store as an OCEL 2.0 JSON log: node types
  whose nodes carry valid times become event types, other node types object types, edges object
  relations, properties attributes; derived from the store's shape, never from names. Specified in
  ESS first.
- Deterministic: the same revision gives the same bytes.

## Acceptance

- A fixture store gives a byte-stable OCEL file that an OCEL 2.0 reader (for example the `process_mining`
  crate) reads back with the expected event and object counts.
- Two reads of one revision are byte-identical, on both providers.
