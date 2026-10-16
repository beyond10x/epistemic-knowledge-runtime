---
format: aep.planning-md/3
id: story:stage-copy-streams
kind: story
status: draft
title: A stage begin copies the store in bounded memory and linear time
relations:
- serves: vision:o5
revision: 1
---
## Outcome

`ekr stage begin` (and `ekr migrate`) copies a store into its destination in bounded memory and in
time linear in the store's history: the writer's resident memory during the copy does not grow with
the number of retained revisions and objects, and each copied revision is validated against state
the copy carries forward rather than a fresh read and replay of the destination.

## Why

A stage begins with a preserving copy of the whole store (`crates/ekr-kernel/src/migrate.rs`,
`Commit::capture`, `CapturedStore::copy_into`, `publish_into`). Today the capture holds the
entire retained history with every object's bytes in memory, and `publish_into` reads the
destination's whole history and reconstructs its state again for every copied occurrence, so the
copy is quadratic in revisions. Measured by a consumer on PostgreSQL: a stage begin of minutes, and
a writer killed for memory at a 2 GiB limit during the copy that completed at 6 GiB.

## Acceptance

- A copy of a store with N revisions reads the destination's history a number of times that does
  not grow with N (counted through the store's read calls, not timed).
- The copy holds at most a bounded number of objects' bytes at once (counted by an instrumented
  object store), on SQLite and on PostgreSQL where its tests run.
- The copied store replays to the same head, roots and retained bytes as before
  (`migrate-verification-disagrees` never fires on the existing suites).
