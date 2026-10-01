---
format: aep.planning-md/3
id: task:migrate-reads-a-current-store
kind: task
status: draft
title: ekr migrate reads a store the current release wrote
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o2
revision: 1
---
## What is wrong

A consumer reported on 2026-10-01: `ekr migrate` from 0.0.25 fails on any store that 0.0.25 itself
seeded, with `ekr: a stored document could not be read: required-object-missing`. The consumer
reproduced it with a plain store: one seed, then a few facts, then `ekr migrate`. A store with
envelope /2, written by 0.0.14, migrates fine. Its own test `migrate_keeps_every_schema_version`
fails, and the consumer offers to extract a smaller reproduction from it.

Not established: whether a store seeded by 0.0.24 fails the same way, which makes 0.0.25 the
regression; and which change of wave extract-06 is the cause. Candidates to check, unverified:
- unit C's held-object memo (`crates/ekr-store/src/eventlog.rs`), where an object-stream read was
  counted at `inventory.rs:102`;
- unit X's explain index;
- unit J's per-type property definitions in the graph document.

## Build

A failing test of seed → facts → `ekr migrate` on both providers, at 0.0.25. Bisect the extract-06
merges, then fix.

## Acceptance

- A store seeded and written by the current release migrates on both providers. So does a store
  written by 0.0.24, and a store with envelope /2.
- The test runs in the default gate.
