---
format: aep.planning-md/3
id: story:reads-served-from-a-persisted-read-model
kind: story
status: draft
title: One-shot reads are served from a persisted, verified read model instead of a rebuild
relations:
- serves: vision:o5
- decomposes: epic:ingestion-throughput
revision: 1
---
## Context

Measured by a consumer instance on 2026-09-29 (ekr 0.0.13, SQLite provider, one store of 4,127
nodes, 14,373 edges and 67,558 assertions): `store.sqlite` is 467 MB, of which `ekr_blobs` holds
420 MB in 392 rows and `ekr_events` 0.5 MB in 741 rows; `ekr_snapshots` is empty; the only indexes
are the primary keys and `ekr_events_feed (tenant_id, global_seq)`. One-shot `ekr head` takes
0.29 s, `ekr snapshot` 3.7 s and `ekr ontology` 9.9 s. The consumer infers, without a trace, that
each process rebuilds state from the blobs.

## Build

Profile the three reads first and name where the time goes. Then persist what a read needs so a
one-shot read does not replay or decode the whole history: at least the ontology at each schema
version and the head graph, kept by the store beside the history, verified on open like a replay
checkpoint (invariant 1: nothing it holds is trusted without the kernel's verdict). Declared in
the ESS store domain first.

## Acceptance

- On a store of that size, one-shot `ekr ontology` and `ekr snapshot` each answer in under 1 s on
  both providers, measured the same way before and after.
- Answers are byte-identical to today's for every revision on a fixture store.
- A persisted read model that does not match the history is refused or rebuilt, never served.
