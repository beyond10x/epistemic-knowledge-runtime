---
format: aep.planning-md/3
id: review-result:roadmap-2026-09-28-acceptance-r1
kind: review-result
status: active
title: Acceptance critic, round 1, roadmap 2026-09-28 drafts
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

story:seed-envelope-v3-references-payloads — the acceptance section holds no criteria at all, only a deferral note ("To be written with the ESS change; drafted here so the format change is not lost."), so there is nothing to check the story against and it can only be asserted closed — .engineering/planning/story/seed-envelope-v3-references-payloads.md:37

What I read: all five drafted artifacts in full via `aep plan artifact show` (story:add-evidence-operation, story:observations-are-retained, story:v1-chat-raw-becomes-observations, story:mcp-read-tools, story:seed-envelope-v3-references-payloads), plus `release-plan:roadmap-2026-09-28`, the three parent epics (`epic:p2-observation-layer`, `epic:p4-operator-surface`, `epic:ingestion-throughput`), and the two decision-blockers that ground two of the acceptance sections (`decision-blocker:evidence-entry-after-seed`, `decision-blocker:observation-retention-path`). Ran `aep plan artifact kinds` and `aep plan artifact lifecycle story`, and grepped the tree (`docs/cli.md`, `systems/ekr/domains/views.yaml`, `AGENTS.md`, and every other occurrence of "the runtime's own vocabulary") to confirm the symbols and the fixture-language convention the other four stories' acceptances rely on actually exist and mean what the acceptance text assumes.

The other four stories' acceptance sections are each one-per-bullet, observable (idempotency counts, refusal-by-name, output equality to an existing read, documentation presence), and each states or clearly implies a before/after transition (no `AddEvidence` operation → validates/commits/replays; first ingest → second ingest yields zero-new; no MCP tools → read tools match canonical reads). Several bullets bundle two related sub-facts with a semicolon (e.g. `story:add-evidence-operation`'s replay clause, `story:observations-are-retained`'s cited-vs-uncited clause, `story:v1-chat-raw-becomes-observations`'s vocabulary/no-customer-data clause) — I judged these as one coherent, jointly-tested mechanism rather than two independently-failing outcomes, consistent with the same pattern already present in the approved parent epics' own acceptance sections, so I did not raise them as findings.

What I could not establish: none — every acceptance clause I judged either cited an existing path/symbol or was checkable by description alone, and I found nothing outside my lane worth flagging (no acceptance-adjacent coupling, coverage or parallel-safety issue surfaced while reading).

```findings
- file: .engineering/planning/story/seed-envelope-v3-references-payloads.md
  line: 37
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: the acceptance section holds no criteria at all, only a deferral note ("To be written with the ESS change; drafted here so the format change is not lost."), so there is nothing to check the story against and it can only be asserted closed
```
