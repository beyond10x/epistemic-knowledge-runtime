---
format: aep.planning-md/3
id: review-result:roadmap-2026-09-28-acceptance-r2
kind: review-result
status: active
title: Acceptance critic, round 2, roadmap 2026-09-28 drafts
relations:
- reviews: story:add-evidence-operation
- reviews: story:observations-are-retained
- reviews: story:v1-chat-raw-becomes-observations
- reviews: story:mcp-read-tools
- reviews: story:seed-envelope-v3-references-payloads
- reviews: release-plan:roadmap-2026-09-28
revision: 1
---
approve/needs-revision verdict below, per the acceptance-critic rubric.

**needs-revision**

story:seed-envelope-v3-references-payloads — the first acceptance bullet joins two independent outcomes with "and" (envelope blob under 1 MB for 10 MB of evidence; no retained blob carries the payload bytes twice), so one can hold while the other fails and the bullet is neither met nor unmet — .engineering/planning/story/seed-envelope-v3-references-payloads.md:47

What I read: all five current story bodies in full via `aep plan artifact show` (story:add-evidence-operation, story:observations-are-retained, story:v1-chat-raw-becomes-observations, story:mcp-read-tools, story:seed-envelope-v3-references-payloads), the round-1 record `review-result:roadmap-2026-09-28-acceptance-r1`, `aep plan artifact lifecycle story`, `aep plan artifact validate` (reports `valid`, no unrelated errors on these five), and `git diff` on the one tracked file to isolate exactly what changed since round 1. I also confirmed every symbol/path the acceptance sections rely on exists in the tree (`crates/ekr-kernel/src/validate/reference.rs`, `docs/cli.md`, `systems/ekr/domains/{kernel,graph,observe,store}.yaml`, `crates/ekr-store/src/eventlog.rs`, `crates/ekr-kernel/src/seed.rs`) and that every referenced decision-blocker/story/task id resolves (`decision-blocker:evidence-entry-after-seed`, `decision-blocker:observation-retention-path`, `decision-blocker:source-unit-granularity`, `decision-blocker:checkpoint-unit-cardinality`, `story:fixture-records-become-observations`, `story:seed-envelope-decoded-once`, `story:history-loaded-once-per-process`, `task:object-payloads-belong-in-provider-blobs`).

The round-1 finding is fixed: `story:seed-envelope-v3-references-payloads` no longer carries a deferral note, and its five new bullets are each observable and transition-implying, except the first, which is new text introduced by this revision's fix and bundles two distinct, separately-failable facts (envelope size vs. store-wide blob deduplication) under one "and." The other four stories are unchanged from round 1 in every acceptance clause I judged, and I found the same semicolon-joined jointly-tested-mechanism pattern in them that round 1 reasoned through and declined to flag (`story:add-evidence-operation`'s replay clause, `story:observations-are-retained`'s cited/reclaimable clause, `story:v1-chat-raw-becomes-observations`'s vocabulary/no-customer-data clause, and this same story's migration-snapshot/retained-object clause at line 51) — I kept that reasoning rather than re-litigate settled ground, since none of that text moved.

Out of my lane, not set into my verdict: `story:add-evidence-operation` and `story:observations-are-retained` both wait on decision-blockers still `open` (`decision-blocker:evidence-entry-after-seed`, `decision-blocker:observation-retention-path`) — a scope/sequencing question, not an acceptance defect.

What I could not establish: none — every acceptance clause I judged either cited an existing path/symbol or was checkable by description alone.

```findings
- file: .engineering/planning/story/seed-envelope-v3-references-payloads.md
  line: 47
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: the first acceptance bullet joins two independent outcomes with "and" (envelope blob under 1 MB for 10 MB of evidence; no retained blob carries the payload bytes twice), so one can hold while the other fails and the bullet is neither met nor unmet
```
