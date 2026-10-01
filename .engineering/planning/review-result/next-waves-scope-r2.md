---
format: aep.planning-md/3
id: review-result:next-waves-scope-r2
kind: review-result
status: active
title: Scope critic, next waves 2026-09-30, round 2
relations:
- reviews: epic:read-and-storage-cost
- reviews: release-plan:next-waves-2026-09-30
- reviews: story:explain-reads-an-index
- reviews: story:preparation-blobs-are-reclaimed
- reviews: task:divergence-is-a-typed-store-error
revision: 1
---
`approve`

Round 1's findings are resolved: `task:held-bytes-notice-deleted-blobs` now decomposes `epic:p6-maintenance-observability` and left the release plan's 10 items; `task:divergence-is-a-typed-store-error` now decomposes `epic:p1-kernel-ontology-core`, whose Outcome names the eventlog-backed store with the SQLite and file providers; `story:explain-reads-an-index` now bounds MCP too.

7 promise-sentences extracted from `epic:read-and-storage-cost` (Outcome: explain, snapshot, views reads, store size; Acceptance: explain's bound, store size halved, each story keeps its audit measurement). All 7 trace: explain and store size by `story:explain-reads-an-index` and `story:preparation-blobs-are-reclaimed`; snapshot and views reads are named as not yet scheduled in `task:perf-audit-2026-09-29-remaining` (an honest omission); both drafted stories cite the 2026-09-29 audit numbers in their acceptance.

What I read: the epic and its 6 current children; `release-plan:next-waves-2026-09-30` and its 10 items; `epic:p1-kernel-ontology-core`, `epic:p3-incubation-integration`, `epic:p4-operator-surface`, `epic:consumer-sdk`, `epic:p6-maintenance-observability`, `vision:o5`, `vision:o2`; `review-result:next-waves-scope-r1`; `aep plan artifact relations`/`kinds`; grep of `decomposes:` for every release-plan item and every p1 child.

What I could not establish: whether the implemented `epic:p1-kernel-ontology-core` is a legitimate parent for a new draft task (lifecycle question, outside this lane); whether `story:reads-served-from-a-persisted-read-model`'s snapshot bound and the audit task's unscheduled snapshot bullet are one fix or two.

```findings
[]
```
