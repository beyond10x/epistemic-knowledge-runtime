---
format: aep.planning-md/3
id: review-result:next-waves-acceptance-r2
kind: review-result
status: active
title: Acceptance critic, next waves 2026-09-30, round 2
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

Both round-1 findings are resolved. `task:seed-document-bounds-alias-expansion` now carries both requirements under one `## Acceptance` heading, and `story:preparation-blobs-are-reclaimed:66` defines an open proposal as one holding a preparation that was elected and not yet published (`elected` is a real term, `crates/ekr-kernel/src/commands.rs:109`). The round-1 gap on `task:one-event-type-rule` is closed: the `/roles` structural rule (`crates/ekr/src/cli/view_roles.rs:34-39`) and the timeline's threshold rule (`crates/ekr-views/src/index.rs:599`) are different mechanisms, so the acceptance does not pass without the fix.

Noted, not flagged: the blobs story's Scope "Not established" line still quoted the pre-fix phrase after the Acceptance defined it (Scope is not Acceptance); the store-wide convention of joining two sub-checks with "and" in one Acceptance bullet, already judged in round 1.

What I read: all 11 artifacts, whole body, via `aep plan artifact show <id>`; `aep plan artifact kinds`; `aep plan artifact lifecycle story|task|epic`; `review-result:next-waves-acceptance-r1`.

```findings
[]
```
