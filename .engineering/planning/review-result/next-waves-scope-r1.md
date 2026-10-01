---
format: aep.planning-md/3
id: review-result:next-waves-scope-r1
kind: review-result
status: active
title: Scope critic, next waves 2026-09-30, round 1
relations:
- reviews: epic:read-and-storage-cost
- reviews: release-plan:next-waves-2026-09-30
- reviews: task:held-bytes-notice-deleted-blobs
- reviews: story:explain-reads-an-index
- reviews: task:divergence-is-a-typed-store-error
revision: 1
---
needs-revision

- `task:held-bytes-notice-deleted-blobs` — this task decomposes `epic:read-and-storage-cost` but serves `vision:o2` (decisions/evidence integrity) while the epic and every other child serve `vision:o5`, and its outcome — deciding whether a live handle re-verifies retained bytes on each request — is not traceable to the epic's outcome ("reads and storage cost what the work needs... not what the history's length dictates"); re-verifying presence per request would add cost, not reduce it. It is a correctness/staleness decision, not a cost-scaling one. — `.engineering/planning/task/held-bytes-notice-deleted-blobs.md:8` (serves vision:o2) vs `.engineering/planning/epic/read-and-storage-cost.md:8` (serves vision:o5)
- `story:explain-reads-an-index` — the epic's acceptance "`explain` at 1× answers in under 0.5 s and under 200 KB" names no lane, but the audit this story itself cites measured MCP `explain` larger than session (MCP 6.0/15.4 MB vs session 3.0/7.7 MB); the story's acceptance binds the 0.5 s/200 KB number to "(session)" only and leaves MCP with "answers the new format," no bound. The epic's promise is narrowed to session with nothing recording that MCP was dropped. — `.engineering/planning/epic/read-and-storage-cost.md:27` vs `.engineering/planning/story/explain-reads-an-index.md:71,74` (numbers at `:57`)
- `task:divergence-is-a-typed-store-error` (release-plan item, wave extract-07) — decomposes `epic:p4-operator-surface` but its outcome (a typed divergence error from `ekr-store`, matched by the CLI instead of a message string) does not trace to any of p4's outcome bullets (attention queue, approvals/obligations, rendered views, query scopes, `ekr explain`, MCP read tools) and touches neither crate the epic names (`ekr-views`, `ekr-mcp`). — `.engineering/planning/epic/p4-operator-surface.md:27-34` vs `.engineering/planning/task/divergence-is-a-typed-store-error.md:9`

What I read: `epic:read-and-storage-cost` and all 7 drafted children, `task:perf-audit-2026-09-29-remaining`'s eight audit bullets, `release-plan:next-waves-2026-09-30`, `epic:consumer-sdk`, `epic:p4-operator-surface`, `epic:p6-maintenance-observability`, plus `aep plan artifact kinds`/`relations`, and `relations`/`decomposes` grep checks for all 10 release-plan items and several candidate-overlap artifacts (`task:view-load-replays-once`, `story:viewer-3d-draws-in-batches`, `task:mcp-serves-the-head`, `story:seed-envelope-v3-references-payloads`, `task:viewer-compact-follow-ups`) to see whether unscheduled audit bullets were already covered elsewhere. 5 promise-sentences extracted from `epic:read-and-storage-cost`'s Outcome/Acceptance/Context; all 5 traced to at least one item, one (explain's bound) traced only partially (finding above). Of the release plan's 10 items, 6 decompose from the three named epics; 5 of those trace cleanly to an explicit Outcome/Stories-table/Shape sentence, 1 does not (finding above).

What I could not establish: whether the "Views" and "Viewer" bullets in `task:perf-audit-2026-09-29-remaining` (uncached `/changes`, superlinear timeline, stream-batch redraw) are genuinely still open or already resolved by `task:view-load-replays-once`/`story:viewer-3d-draws-in-batches` (implemented, but citing different numbers and a different measurement date) — I did not call this a gap because the task itself names these bullets as not yet scheduled, which the rubric treats as an honest omission, not a gap. Whether `story:reads-served-from-a-persisted-read-model`'s "consumer instance" measurement is the same audit the epic's Context names via `task:perf-audit-2026-09-29-remaining`, or a second 2026-09-29 report — I treated it as legitimate since two other stories (`story:reads-share-verified-state`, `story:viewer-3d-draws-in-batches`) cite the identical numbers. Whether `task:one-event-type-rule` and `task:projection-carries-per-type-property-definitions` genuinely belong to p4 (they touch `ekr-views`, the crate p4's Outcome names, so I did not flag them) is a judgment call at the edge of my lane; a design critic may see it differently.

```findings
- file: .engineering/planning/epic/read-and-storage-cost.md
  line: 8
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: task:held-bytes-notice-deleted-blobs decomposes epic:read-and-storage-cost but serves vision:o2 while the epic and its other children serve vision:o5, and its outcome (deciding whether a live handle re-verifies retained bytes) is not traceable to the epic's cost-scaling outcome — re-verifying on each request would add cost, not reduce it
- file: .engineering/planning/epic/read-and-storage-cost.md
  line: 27
  category: scope
  severity: warning
  verdict: needs-revision
  origin: undecided
  message: the epic's acceptance "explain at 1x answers in under 0.5 s and under 200 KB" names no lane, but story:explain-reads-an-index binds that number to "(session)" only and leaves MCP explain (measured larger in the same audit, 6.0/15.4 MB) with no numeric acceptance, silently narrowing the promise
- file: .engineering/planning/epic/p4-operator-surface.md
  line: 27
  category: scope
  severity: warning
  verdict: needs-revision
  origin: undecided
  message: task:divergence-is-a-typed-store-error (release-plan wave extract-07) decomposes epic:p4-operator-surface but its typed-error fix in ekr-store/CLI does not trace to any of p4's outcome bullets or its named crates (ekr-views, ekr-mcp)
```
