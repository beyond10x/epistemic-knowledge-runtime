---
format: aep.planning-md/3
id: story:seed-envelope-v3-references-payloads
kind: story
status: implemented
title: The seed envelope names evidence payloads by hash instead of embedding them
relations:
- serves: vision:o5
- decomposes: epic:ingestion-throughput
- informed_by: task:object-payloads-belong-in-provider-blobs
- depends_on: story:seed-envelope-decoded-once
- depends_on: story:history-loaded-once-per-process
scope:
- confidence: inferred
  path: crates/ekr-kernel/src/seed.rs
- confidence: inferred
  path: crates/ekr-store/src/eventlog.rs
- confidence: inferred
  path: crates/ekr-store/tests/payload_blobs.rs
- confidence: inferred
  path: systems/ekr/domains/store.yaml
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T23:00:44Z", actor: "agent:claude-coordinator", revision: 8, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-09-28T23:00:44Z", actor: "agent:claude-coordinator", revision: 9, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-09-29T07:07:33Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
## Context

The seed envelope embeds every evidence payload as a JSON integer array. On the 112 MB store of
`epic:ingestion-throughput`, 11 MB of corpus evidence becomes a 39,055,886-byte envelope blob, and
the largest blob, 66,545,772 bytes, is an `ekr.publication-preparation/2` record carrying the same
input again (`ls -S -l store/blobs` on that store, 2026-09-28). The payloads are also retained as
their own content-addressed blobs. `task:object-payloads-belong-in-provider-blobs` (draft) records
the same defect for object payloads in event bodies.

This story rewrites code that wave ingest-01 changes (`crates/ekr-kernel/src/seed.rs` in
`story:seed-envelope-decoded-once`, `crates/ekr-store/src/eventlog.rs` in
`story:history-loaded-once-per-process`), so it starts after both are merged.

## Build

`ekr-seed-envelope/3`: the envelope names each evidence payload by its content hash instead of
embedding it, and the publication-preparation record names the envelope by hash instead of
carrying its input. Replay reads the payload blobs the envelope names. `/2` envelopes keep
replaying unchanged; a preserving migration verb rewrites an `/2` store to `/3` without deleting or
reinterpreting any retained object. Declared in the ESS store and kernel domains first.

## Acceptance

- A store seeded with `/3` from a seed with 10 MB of evidence holds an envelope blob under 1 MB
  (both providers).
- In that store no retained blob other than the payload blobs contains the payload bytes (a test
  over the store's blobs, both providers).
- A `/2` store written by 0.0.11 reopens and replays with identical roots under the new code.
- The migration verb turns a `/2` store into a `/3` store whose snapshot (nodes, edges,
  assertions, evidence ids and payload hashes) equals the `/2` store's; every original object is
  still retained.
- A `/3` envelope naming a payload the store does not hold is refused by name.
- The kernel conformance suite passes on both providers.
