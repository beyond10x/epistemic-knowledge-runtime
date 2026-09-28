---
format: aep.planning-md/3
id: review-result:roadmap-2026-09-28-scope-r2
kind: review-result
status: active
title: Scope critic, round 2, roadmap 2026-09-28 drafts
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

## What I read

`aep plan artifact show` on: `release-plan:roadmap-2026-09-28`, `epic:p2-observation-layer`, `epic:p4-operator-surface`, `epic:ingestion-throughput`, all 5 drafted stories, and `decision-blocker:adapter-unit-ownership`. `aep plan artifact graph`, `kinds`, `relations`; `git status`/`git diff` on the uncommitted planning-store files to see this round's delta over round 1 (release-plan revision 1→2, `story:v1-chat-raw-becomes-observations` revised, 5 decision-blockers gained `blocks` edges to the drafted stories). `docs/roadmap.md` §4 (P2, P4) and §7 (out of scope). `.engineering/planning/review-result/roadmap-2026-09-28-scope-r1.md` (my lane's prior round). Same 19 promise-clauses as round 1 (8 from p2's Outcome+Acceptance, 9 from p4's, 2 from ingestion-throughput's Outcome + closing note) since the three epics are byte-identical to round 1 (`git diff HEAD` on them is empty). Same 9 trace to an item in the store; the remaining 10 (adapters, poll health/coverage-report, redaction gates in p2; attention queue, approvals/obligations, rendered views in p4) are unclaimed by anything in the set, and the roadmap record now names each of them explicitly as deferred after `obs-01` — matching the task's "next waves only" framing.

All three round-1 findings are resolved in this revision:

- `story:v1-chat-raw-becomes-observations` no longer silently narrows the parent's v1+v2 promise: its Context now states "This story covers v1 raw chat only. The v2 `raw/` import and its reconciliation, the other half of that P2 bullet, are not claimed by this story; they stay open under `epic:p2-observation-layer` and are listed as unclaimed in `release-plan:roadmap-2026-09-28`" (`.engineering/planning/story/v1-chat-raw-becomes-observations.md:14-16`), and the release-plan's closing paragraph after the Milestones table does list "v2 `raw/` import and its reconciliation" as claimed by nothing yet (`.engineering/planning/release-plan/roadmap-2026-09-28.md:63-64`).
- The Milestones table no longer says M3 closes "P2 exit" outright; it now reads "part of the P2 exit: the twice-safe and v1-reconciliation clauses" (`.engineering/planning/release-plan/roadmap-2026-09-28.md:53`).
- The `epic:ingestion-throughput` figure is now quoted correctly: "on a 112 MB store one `ekr resolve` costs 2.94 s and `ekr head` 0.57 s" (`.engineering/planning/release-plan/roadmap-2026-09-28.md:45`), matching `epic:ingestion-throughput.md:17-18` exactly instead of the earlier fabricated 2.3–2.9 s range.

I checked the revision for new defects in both directions and found none: no new gap (the same 10 clauses stay honestly unclaimed, now more explicitly than in round 1), no reach beyond the parents or `docs/roadmap.md` §7's exclusions, no two items claiming the same outcome, and no new silent narrowing.

## What I could not establish

- Whether `decision-blocker:adapter-unit-ownership`'s new `blocks: story:v1-chat-raw-becomes-observations` edge is itself well-founded — that blocker is framed around a polling `SourceAdapter`'s unit discovery, and the v1 story is a static file importer with no discovery. This is a blocking-edge/dependency question, not a coverage question, so it is out of my lane (design or parallel-safety), and I did not let it affect my verdict.
- Why `story:add-evidence-operation` serves `vision:o2` while its parent `epic:p2-observation-layer` serves `vision:o5` — unchanged from round 1, still a design-lane question, not mine to set the verdict on.

```findings
[]
```

Relevant paths: `<worktree>/.engineering/planning/story/v1-chat-raw-becomes-observations.md`, `<worktree>/.engineering/planning/release-plan/roadmap-2026-09-28.md`, `<worktree>/.engineering/planning/epic/p2-observation-layer.md`, `<worktree>/.engineering/planning/epic/p4-operator-surface.md`, `<worktree>/.engineering/planning/epic/ingestion-throughput.md`, `<worktree>/.engineering/planning/review-result/roadmap-2026-09-28-scope-r1.md`.
