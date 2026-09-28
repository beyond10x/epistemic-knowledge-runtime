---
format: aep.planning-md/3
id: review-result:roadmap-2026-09-28-design-r1
kind: review-result
status: active
title: Design critic, round 1, roadmap 2026-09-28 drafts
relations:
- reviews: story:add-evidence-operation
- reviews: story:observations-are-retained
- reviews: story:v1-chat-raw-becomes-observations
- reviews: story:mcp-read-tools
- reviews: story:seed-envelope-v3-references-payloads
- reviews: release-plan:roadmap-2026-09-28
revision: 1
---
**approve**

**What I read:** `aep plan artifact show` on all 5 drafted stories, the roadmap record `release-plan:roadmap-2026-09-28`, the three named parent epics (`epic:p2-observation-layer`, `epic:p4-operator-surface`, `epic:ingestion-throughput`), the five decision-blockers the set's edges point at (`evidence-entry-after-seed`, `observation-retention-path`, `source-unit-granularity`, `checkpoint-unit-cardinality`, `adapter-unit-ownership`), `task:object-payloads-belong-in-provider-blobs`, and the two already-implemented predecessor stories the new ones build on (`story:observe-domain-model`, `story:fixture-records-become-observations`). Also `aep plan artifact relations`, `aep plan artifact graph --format json` (full 236-artifact graph, inspected the five items' own edge lists and their parents' up to `initiative`/`vision`, which are terminal), `aep plan artifact validate` (clean, findings unrelated to this set), `git diff` on the modified decision-blocker files and the seed-envelope story, and `crates/ekr-kernel/src/seed.rs` plus a grep of `docs/epistemic-knowledge-runtime-design.md` § 47, § 62, § 70, § 86 to confirm the cited sections exist and say what the stories claim.

Walked: every declared edge from the 5 items (each `decomposes`/`serves`/`depends_on`/`informed_by`) and the 5 `blocks` edges landing on them from decision-blockers, up to the terminal `initiative`/`vision` nodes. All edges in this vocabulary point upward (toward parent/vision) or forward (`blocks`, `depends_on`); none of the parents' or blockers' own edges point back into the five, so no cycle is reachable from this set.

Findings: none.

- The one `depends_on` edge in the set (`story:v1-chat-raw-becomes-observations` → `story:observations-are-retained`) is recorded, and `observations-are-retained` has its own demonstrable acceptance against the existing fixture source (`crates/ekr-observe/tests/retained.rs`), so it isn't a disguised horizontal slice.
- `story:add-evidence-operation` and `story:seed-envelope-v3-references-payloads` both touch payload retention but at different layers (transaction-document evidence vs. seed-envelope format) and different files (`transaction.rs`/`validate/reference.rs` vs. `seed.rs`/`eventlog.rs`); both build on the already-shipped provider-blob capability that `task:object-payloads-belong-in-provider-blobs` records as done, so neither is half of an abstraction the other completes.
- `story:mcp-read-tools`'s body is self-contained (reads existing `ekr.views`/explain output over stdio) and names no internals of `epic:ingestion-throughput`'s stories; the roadmap's "M1 released" wave gate is a scheduling choice stated in `release-plan:roadmap-2026-09-28`, not a dependency the story's outcome requires.
- Each of the four decision-blockers gating these stories is correctly attached by a `blocks` edge added in this same diff (confirmed via `git diff`), matching the roadmap's wave gates exactly.

What I could not establish: whether the five items' file scopes actually collide in a way that matters for concurrent work (parallel-safety's lane, not mine). Whether `story:seed-envelope-v3-references-payloads`'s deferred acceptance ("to be written with the ESS change") is adequate (acceptance critic's lane). Whether the set covers everything the three parent epics still owe (scope critic's lane) — noting only, out of lane, that `epic:p4-operator-surface` formally `depends_on epic:p3-incubation-integration` while `story:mcp-read-tools` decomposes it as a slice that doesn't touch P3; the roadmap already documents this as a deliberate, pre-existing divergence, not something this drafted set introduced.

```findings
[]
```
