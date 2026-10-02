---
format: aep.planning-md/3
id: task:selected-revision-loads-read-one-event-per-call
kind: task
status: draft
title: Selected-revision history loads read the revision stream one event per call
relations:
- serves: vision:o2
- derived_from: story:eventlog-0-4-batched-reads
- decomposes: epic:read-and-storage-cost
revision: 2
---
## What is wrong

Measured by adversary pass 2 on wave p1-15 unit p1-15-eventlog (`review-result:p1-15-eventlog-adversary-r2`): `history_at` (`crates/ekr-store/src/eventlog.rs:582`) and `replay` (`:778`) load history with a page limit of 1, so a selected-revision read costs N+5 provider calls (7 at 2 proposals, 29 at 24, counted through a counting provider). On the file provider each call verifies the log. Reached by `Commit::read(Some(revision))` (`crates/ekr-kernel/src/read.rs:60`), i.e. `ekr snapshot --at`, and `Commit::replay` (`commit.rs:196`). The base had the same shape (`load_history(1, …)`).

## What closes this

Selected-revision loads read the revision stream in pages up to `MAX_READ_LIMIT` and stop at the requested revision, so their provider calls stay constant up to one page; a counting-provider case holds it.

## Refined scope and acceptance

Cited at released main: crates/ekr-store/src/eventlog.rs history_at calls load_history with a
single-event page, and replay calls admitted with the same limit. StreamRead::absorb stops at
the selected event. Inferred edit surface: those entry points, the page reader if required,
and existing counting-provider tests under crates/ekr-store/tests. No native dependency,
wire format, durability, checkpoint cadence or authority rule changes.

Use bounded pages up to MAX_READ_LIMIT while retaining the exact selected boundary. Count
revision-stream provider calls below and across page boundaries, separating unrelated stream
calls. Compare selected roots and refusal results on file and SQLite providers. Cover first
and last events of a page, missing revisions, stalled cursors and replacement of a held prefix.

Mandatory safety cases place invalid envelopes, malformed domain events and missing later
record blobs after a valid selected revision within the same provider page. Earlier selected
reads retain their baseline result; reads reaching damage refuse. Damage before the selected
boundary still refuses. Provider-level batch decoding must not accidentally widen the selected
read's validation boundary. Every case asserts the preserved contract after the fix.
