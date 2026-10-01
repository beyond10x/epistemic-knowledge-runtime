---
format: aep.planning-md/3
id: review-result:next-waves-design-r2
kind: review-result
status: active
title: Design critic, next waves 2026-09-30, round 2
relations:
- reviews: release-plan:next-waves-2026-09-30
- reviews: story:extraction-document-applies-to-a-store
- reviews: story:fact-quality-by-judged-sample
- reviews: story:explain-reads-an-index
- reviews: task:one-event-type-rule
- reviews: task:projection-carries-per-type-property-definitions
- reviews: story:extraction-verb-shares-the-sdk-path
- reviews: story:sdk-store-checks
- reviews: story:preparation-blobs-are-reclaimed
- reviews: task:divergence-is-a-typed-store-error
- reviews: task:seed-document-bounds-alias-expansion
- reviews: epic:read-and-storage-cost
revision: 1
---
`approve`

Round 1's finding is closed: `story:preparation-blobs-are-reclaimed` now carries `depends_on: story:explain-reads-an-index`, so the shared `explain.rs:227,381,401-406` readers are an edged ordering constraint. `task:one-event-type-rule` and `task:projection-carries-per-type-property-definitions` gained `decomposes: epic:p4-operator-surface` (placement only). `story:extraction-document-applies-to-a-store` dropped a stale collision note with `story:fact-quality-by-judged-sample`, which adds a `Command` variant, not an `ExampleFormat` one.

Defect check on the revised set:
- Cycle: none. `depends_on` closure reaches 9 outside artifacts, all implemented, none pointing back into the set.
- Serialising chain: three independent two-node pairs (extraction-verb → extraction-document; sdk-store-checks → fact-quality; preparation-blobs → explain-index) plus four unconnected tasks.
- Split abstraction: extraction-document and extraction-verb each carry independently checkable acceptance and are edged.
- Hidden dependency: none beyond the one round 1 caught.
- Horizontal slice: each item is demonstrable through its own CLI, SDK or MCP surface.
- Two items, one surface: `views.yaml`, `session.rs`, `eventlog.rs`, `conformance.rs` shared cases are narrated in the release plan as different-section collisions; parallel-safety's lane.

What I read: the 10 items, `epic:read-and-storage-cost` and `release-plan:next-waves-2026-09-30` via `aep plan artifact show`; `aep plan artifact relations`; `aep plan artifact graph --format json`; `aep plan artifact validate` (valid, 14 pre-existing prose-only-findings notices outside this set); `review-result:next-waves-design-r1`; `git diff` against `ec6bbf38`.

```findings
[]
```
