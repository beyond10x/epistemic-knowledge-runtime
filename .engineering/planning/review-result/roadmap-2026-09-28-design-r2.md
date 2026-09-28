---
format: aep.planning-md/3
id: review-result:roadmap-2026-09-28-design-r2
kind: review-result
status: active
title: Design critic, round 2, roadmap 2026-09-28 drafts
relations:
- reviews: story:add-evidence-operation
- reviews: story:observations-are-retained
- reviews: story:v1-chat-raw-becomes-observations
- reviews: story:mcp-read-tools
- reviews: story:seed-envelope-v3-references-payloads
- reviews: release-plan:roadmap-2026-09-28
revision: 1
---
approve

**What I read:** `aep plan artifact show` on all 5 drafted stories, `release-plan:roadmap-2026-09-28`, the three parent epics, the four decision-blockers with `blocks` edges into the set, `task:object-payloads-belong-in-provider-blobs` (verified `status draft` via the CLI, not inferred from title), and the two wave-`ingest-01` stories the new edges point at (`story:seed-envelope-decoded-once`, `story:history-loaded-once-per-process`, both `status active`). Also `aep plan artifact relations`, `aep plan artifact graph --format json` (walked edges for the 5 items plus one hop out to their new dependency targets and back — none return), `aep plan artifact validate` (clean; the one flagged item, `review-result:roadmap-2026-09-28-design-r1`'s findings-format note, is the validator's own output, not mine), `git diff` on `story:seed-envelope-v3-references-payloads.md` (the only body change in this round), and a source check (`grep put_blob/get_blob crates/ekr-store/src/eventlog.rs`, `ls crates/ekr-store/tests/durable_objects.rs`) to confirm the blob API the v3 story's build leans on is real, not just claimed by the draft task. I also read the round-1 `parallel-r1`, `scope-r1` and `acceptance-r1` review results to see what this revision was answering.

**What changed since r1, from my lane:** `story:seed-envelope-v3-references-payloads` gained two `depends_on` edges (`story:seed-envelope-decoded-once`, `story:history-loaded-once-per-process`) plus `informed_by task:object-payloads-belong-in-provider-blobs`. These are exactly the edges `parallel-r1` asked for — the story's scope names `crates/ekr-kernel/src/seed.rs` and `crates/ekr-store/src/eventlog.rs`, the same files those two active wave-`ingest-01` stories rewrite — so per the rubric this is the file-collision trade-off already resolved, not a new chain to flag. Both targets are outside the 5-item set and have no edges back into it, so no cycle. Inside the set, the shape is unchanged from r1: one `depends_on` (`v1-chat-raw-becomes-observations` → `observations-are-retained`), no other internal ordering. `informed_by task:object-payloads-belong-in-provider-blobs` is the correct weaker edge: the story's own Context now states the task is `(draft)` and names it as recording "the same defect" for a different surface (object payloads in event bodies vs. the seed envelope), and the code check confirms the capability v3 actually needs (`put_blob`/`get_blob`, exercised by `durable_objects.rs`) is already merged — so nothing about v3 needs the task's still-open item (legacy schema-1 migration) to land first, and no hidden dependency is disguised as `informed_by`.

Findings: none.

**What I could not establish:** whether the `docs/cli.md` four-way collision `parallel-r1` also found is now adequately covered — the release-plan's new merge-ownership note is written, but whether that note is the right mechanism for four stories that may run concurrently is parallel-safety's lane, not mine. Whether v2 `raw/` import being carved out of `v1-chat-raw-becomes-observations` is fully resolved is scope's lane. Out of lane, unchanged from r1: `epic:p4-operator-surface` still formally `depends_on epic:p3-incubation-integration` while `story:mcp-read-tools` decomposes it as a slice that doesn't touch P3 — pre-existing, not introduced by this diff, and not something this round's revision touched.

```findings
[]
```
