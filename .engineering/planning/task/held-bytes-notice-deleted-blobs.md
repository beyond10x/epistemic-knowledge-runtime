---
format: aep.planning-md/3
id: task:held-bytes-notice-deleted-blobs
kind: task
status: draft
title: Decide whether a live store handle notices a retained blob deleted under it
relations:
- serves: vision:o2
- decomposes: epic:p6-maintenance-observability
revision: 1
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

## Build

Decide in the ESS store domain whether a live handle must notice a retained blob deleted under it
(re-verify presence per request, or per commit, or never); implement that decision; un-ignore the case
if the answer is "notice".

## Acceptance

- The decision is written in `systems/ekr/domains/store.yaml` and design § 100 or a new section.
- The pinned case passes or is replaced by a case asserting the decided behaviour, on both providers.
