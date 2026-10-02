---
format: aep.planning-md/3
id: task:ontology-at-reads-only-the-schema
kind: task
status: draft
title: ekr ontology --at reads only the schema history
relations:
- decomposes: epic:read-and-storage-cost
- serves: vision:o5
revision: 1
---
## What is wrong

A consumer reported on 2026-10-01 that `ekr ontology --at 405` on a SQLite store of about 148,000
assertions takes 486 s with ekr 0.0.25, while `ekr ontology` at the head and `ekr head` answer
quickly. A caller that needs only a past revision's `schema_version` and `schema_version_number`
pays a full replay of the graph up to that revision. Not blocking the consumer now (it keeps its
own copy record), but older runs and its check still pay it.

Related: the perf audit's "an overview at an old revision loads the full history"
(`task:perf-audit-2026-09-29-remaining`, Views bullet; `crates/ekr-kernel/src/read.rs`
`schema_history`) and `task:rebuilt-revision-reuses-retained-verdicts` (rebuilding a released
revision replays from the seed).

## Build

A probe first: where the 486 s goes (replaying data operations, revalidation, or loading history).
Then a schema-only read: the schema at a revision is a function of the seed and the schema
transactions up to it, so `ontology --at` replays only those, or reads a revision → schema-version
index kept with the history. The answer stays byte-identical.

## Acceptance

- `ekr ontology --at <revision>` answers in time that does not grow with the number of data
  transactions before that revision: a counting test shows no data operation is replayed for it.
- Its output is byte-identical to today's for every revision of the views and kernel fixtures, on
  both providers.
