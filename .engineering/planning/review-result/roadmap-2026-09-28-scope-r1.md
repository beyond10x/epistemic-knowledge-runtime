---
format: aep.planning-md/3
id: review-result:roadmap-2026-09-28-scope-r1
kind: review-result
status: active
title: Scope critic, round 1, roadmap 2026-09-28 drafts
relations:
- reviews: story:add-evidence-operation
- reviews: story:observations-are-retained
- reviews: story:v1-chat-raw-becomes-observations
- reviews: story:mcp-read-tools
- reviews: story:seed-envelope-v3-references-payloads
- reviews: release-plan:roadmap-2026-09-28
revision: 1
---
approve or needs-revision — **needs-revision**

Below: what I read, findings, and the fenced block.

## What I read

`aep plan artifact show` on: `release-plan:roadmap-2026-09-28`, `epic:p2-observation-layer`, `epic:p4-operator-surface`, `epic:ingestion-throughput`, all 5 drafted stories, the 5 decision-blockers named in the graph as blocking those items, and `task:object-payloads-belong-in-provider-blobs` / `task:selected-revision-loads-read-one-event-per-call`. Also `aep plan artifact graph`, `kinds`, `relations`, and `docs/roadmap.md` §4 (P2, P4). I extracted 19 promise-clauses across the three parent epics (8 from p2's Outcome+Acceptance, 9 from p4's, 2 from ingestion-throughput's Outcome + its own closing note). 9 trace to an item already in the store (existing or drafted); of those, 2 (p2's v1/v2-raw-import outcome bullet and its acceptance-mirror) trace only to a narrowed claim, flagged below. The remaining 10 (adapters, poll health/coverage-report, redaction gates in p2; attention queue, approvals/obligations, rendered views in p4) are unclaimed by anything in the store — per the task's explicit framing that this set is a next-wave slice, not a full decomposition, and given the roadmap record's own milestone table names most of these as later work (M6 etc.), I did not count these as gaps.

## Findings

```
story:v1-chat-raw-becomes-observations — quotes the parent's full promise ("v1 `knowledge/raw/**/*.jsonl` and v2 `raw/` become observations", epic:p2-observation-layer.md:35-36) in its own Context, then Build and Acceptance cover v1 only, with nothing saying v2 was dropped from this story or where it goes — .engineering/planning/story/v1-chat-raw-becomes-observations.md:31
```
```
release-plan:roadmap-2026-09-28 — the Milestones table lists M3 ("retained observations, raw v1 chat import, twice-safe") as closing "P2 exit", but P2's Acceptance also requires the Slack-delta twice-ingest test, a coverage report printing declared denominators, and v2 reconciliation, none of which anything in the store claims — .engineering/planning/release-plan/roadmap-2026-09-28.md:65 (parent acceptance at .engineering/planning/epic/p2-observation-layer.md:33-36)
```
```
release-plan:roadmap-2026-09-28 — attributes "a 112 MB store costs 2.3–2.9 s per call" to `epic:ingestion-throughput`, but that epic states `ekr resolve` at 2.94 s and `ekr head` at 0.57 s, not a 2.3–2.9 s range — .engineering/planning/release-plan/roadmap-2026-09-28.md:45 (epic:ingestion-throughput.md:17-18)
```

## What I could not establish

- Whether the ~10 untraced p2/p4 promise-clauses (adapters, poll health/coverage report, redaction gates, attention queue, approvals, rendered views) are deliberately deferred beyond "next four waves" — the roadmap record doesn't enumerate them by name as deferred, only implies it through the wave/milestone structure; treated as expected given the task's explicit "next waves only" framing, not as gaps.
- Why `story:add-evidence-operation` serves `vision:o2` while its parent `epic:p2-observation-layer` serves `vision:o5` — plausible (O2 is "decisions as data, with evidence") but undocumented; a design-lane question, not mine to set the verdict on.
- The source of "a 66.5 MB publication-preparation blob carries it again" in `story:seed-envelope-v3-references-payloads.md:25` (cited as "store of epic:ingestion-throughput") — I could not find that figure anywhere else in the store or tree; noted here rather than as a finding since the underlying claim (the 39 MB envelope, doubled decode) is otherwise well-grounded in the epic.
- `story:seed-envelope-v3-references-payloads`'s empty Acceptance ("to be written with the ESS change") is a real thinness but belongs to `plan-critic-acceptance`, not this lane — flagged here as out of my lane, not counted toward my verdict.

```findings
- file: .engineering/planning/story/v1-chat-raw-becomes-observations.md
  line: 31
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: quotes the parent's full promise ("v1 knowledge/raw/**/*.jsonl and v2 raw/ become observations", epic:p2-observation-layer.md:35-36) in its own Context, then Build and Acceptance cover v1 only, with nothing saying v2 was dropped from this story or where it goes
- file: .engineering/planning/release-plan/roadmap-2026-09-28.md
  line: 65
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: the Milestones table lists M3 ("retained observations, raw v1 chat import, twice-safe") as closing "P2 exit", but P2's Acceptance also requires the Slack-delta twice-ingest test, a coverage report printing declared denominators, and v2 reconciliation, none of which anything in the store claims
- file: .engineering/planning/release-plan/roadmap-2026-09-28.md
  line: 45
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: attributes "a 112 MB store costs 2.3–2.9 s per call" to epic:ingestion-throughput, but that epic states ekr resolve at 2.94 s and ekr head at 0.57 s, not a 2.3–2.9 s range
```

Relevant paths: `<worktree>/.engineering/planning/story/v1-chat-raw-becomes-observations.md`, `<worktree>/.engineering/planning/release-plan/roadmap-2026-09-28.md`, `<worktree>/.engineering/planning/epic/p2-observation-layer.md`, `<worktree>/.engineering/planning/epic/ingestion-throughput.md`.
