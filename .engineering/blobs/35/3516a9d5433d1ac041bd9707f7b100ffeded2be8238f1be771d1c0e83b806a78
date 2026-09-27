---
format: aep.planning-md/1
id: task:selected-revision-loads-read-one-event-per-call
kind: task
status: draft
title: Selected-revision history loads read the revision stream one event per call
relations:
- serves: vision:o2
- derived_from: story:eventlog-0-4-batched-reads
revision: 1
---
## What is wrong

Measured by adversary pass 2 on wave p1-15 unit p1-15-eventlog (`review-result:p1-15-eventlog-adversary-r2`): `history_at` (`crates/ekr-store/src/eventlog.rs:582`) and `replay` (`:778`) load history with a page limit of 1, so a selected-revision read costs N+5 provider calls (7 at 2 proposals, 29 at 24, counted through a counting provider). On the file provider each call verifies the log. Reached by `Commit::read(Some(revision))` (`crates/ekr-kernel/src/read.rs:60`), i.e. `ekr snapshot --at`, and `Commit::replay` (`commit.rs:196`). The base had the same shape (`load_history(1, …)`).

## What closes this

Selected-revision loads read the revision stream in pages up to `MAX_READ_LIMIT` and stop at the requested revision, so their provider calls stay constant up to one page; a counting-provider case holds it.
