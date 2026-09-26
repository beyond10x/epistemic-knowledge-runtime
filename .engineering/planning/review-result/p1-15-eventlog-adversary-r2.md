---
format: aep.planning-md/2
id: review-result:p1-15-eventlog-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p1-15 eventlog unit
relations:
- reviews: story:eventlog-0-4-batched-reads
revision: 1
---
Adversary pass 2 on unit p1-15-eventlog (wave p1-15), `aep-drive:adversary`, against `04b6b8d`.

Owners: 2 findings, 1 implementor (introduced), 1 pre-existing.

Verdict: CONFIRMED (1 warning, 1 note). cases: executed 390→391, red 1.

Pass-1 check: F1 resolved by the coordinator decision; F2 resolved for streams, carried for blobs (F-A); F3 resolved.

Case: `crates/ekr-store/tests/adversary2_p1_15_blob_batch_order.rs` (red: a later blob provider failure replaces an earlier object refusal on SQLite).

Routing (coordinator): F-A no-op to the code: the coordinator documented the blob-batch ordering in `required()` and rewrote the case to today's documented state (`d195e77`). F-B is pre-existing: filed as `task:selected-revision-loads-read-one-event-per-call`.

```findings
[
  {"file": "crates/ekr-store/src/eventlog.rs", "line": 356, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The blob batch is one read_many, so on SQLite a later object blob provider failure replaces the earlier object per-object refusal, an ordering exception the doc comment does not name (adversary2_p1_15_blob_batch_order.rs:165)."},
  {"file": "crates/ekr-store/src/eventlog.rs", "line": 582, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "history_at and replay load with page limit 1, costing N+5 provider calls (measured 7 at 2 proposals, 29 at 24), so the five-calls-whatever-its-length claim holds only for loads at MAX_READ_LIMIT."}
]
```
