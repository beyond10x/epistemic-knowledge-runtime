---
format: aep.planning-md/3
id: story:session-keeps-only-the-head-graph
kind: story
status: draft
title: A session keeps the head graph, not one graph per revision
relations:
- serves: vision:o5
- decomposes: epic:ingestion-throughput
scope:
- confidence: cited
  path: crates/ekr-kernel/src/replay.rs
revision: 2
---
## Context

Performance audit of 2026-09-29. A session keeps a full graph copy for every revision it commits
(`crates/ekr-kernel/src/replay.rs:477`, `:774`): +73 MB per commit at 1×; peak 4.5 GB after 10 commits
at 3×, 14.6 GB after 6 at 10×. A long ingest session runs out of memory before it runs out of work.

## Build

The replay cache keeps the head graph and the checkpointed graphs only; an older revision a read asks
for is reconstructed as today's fallback already does. Structurally shared maps are out of scope.

## Acceptance

- At the 3× shape, a session's resident memory after 20 commits is at most 1.5× its memory after 1
  (measured).
- Reads of past revisions answer byte-identically (views suite, historical-projection tests).

## Scope (cited from the audit)

`crates/ekr-kernel/src/replay.rs`.
