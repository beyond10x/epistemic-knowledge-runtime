---
format: aep.planning-md/3
id: task:held-bytes-notice-deleted-blobs
kind: task
status: active
title: Decide whether a live store handle notices a retained blob deleted under it
relations:
- serves: vision:o2
- decomposes: epic:p6-maintenance-observability
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T18:42:14Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-01T19:15:31Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":2}}}
---
## Context

Wave perf-01's adversary pass on unit R (2026-09-29): a store handle that has already read keeps its
verified copies of retained objects (`crates/ekr-store/src/eventlog.rs:883`). After a seed payload's
blob is deleted on disk, that handle still answers `read`, while a fresh handle refuses with
`object-integrity: native blob missing`. The kernel's seed-input guard `payloads_held`
(`crates/ekr-kernel/src/read.rs:266`) is correct but cannot see the deletion through either
provider. The held-bytes path predates the wave (`a094b53b`); reaching it needs outside damage to a
store under a long-lived session or MCP runtime. The adversary's case
`a_seed_payload_deleted_after_a_read_is_answered_as_a_fresh_handle_answers_it` pins it and is marked
ignored with this task's id.

Wave extract-06 adversary pass on unit C (2026-10-01, `review-result:adversary-extract-06-c-pass-1`): since
`story:commit-cost-flat-with-store-size`, a handle re-reads a held non-canonical object only when its
stream gains an event. On SQLite, an in-place redaction of that event moves no feed position, so a
held handle keeps serving a Provenance payload a fresh handle refuses (`stream-envelope-disagrees`).
Nothing calls `redact` yet; roadmap D1 and P6 route deletion requests through it. The case
`a_held_evidence_payload_whose_event_is_redacted_is_refused_as_a_fresh_handle_refuses_it_on_sqlite`
(`crates/ekr-store/tests/adversary_x6_c.rs`) pins it, ignored with this task's id. The same decision
covers it: a deletion or redaction path either appends an event to the object's stream or drops held
entries, or a live handle re-verifies.

## Build

Decide in the ESS store domain whether a live handle must notice a retained blob deleted under it
(re-verify presence per request, or per commit, or never); implement that decision; un-ignore the case
if the answer is "notice".

## Acceptance

- The decision is written in `systems/ekr/domains/store.yaml` and design § 100 or a new section.
- The two pinned cases pass or are replaced by cases asserting the decided behaviour, on both providers.

## Decision

The decision this task needs is recorded once, in `task:sqlite-store-replaced-in-place` § Decision (2026-10-01): a live handle never answers from bytes a fresh handle would refuse, and any operation that withdraws retained bytes appends an event to the object's stream. Both tasks are implemented together as unit S of wave correct-07 (`release-plan:next-waves-2026-10-01`).
