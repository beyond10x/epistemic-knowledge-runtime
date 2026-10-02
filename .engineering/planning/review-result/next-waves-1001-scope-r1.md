---
format: aep.planning-md/3
id: review-result:next-waves-1001-scope-r1
kind: review-result
status: active
title: Scope critic, next waves 2026-10-01, round 1
relations:
- reviews: release-plan:next-waves-2026-10-01
revision: 1
---
`needs-revision`

- `release-plan:next-waves-2026-10-01` — the "Not scheduled" section calls the decision blockers "unchanged" and names nine, but two open blockers that also block `epic:p3-incubation-integration` — `decision-blocker:typed-reference-identifying-keys` and `decision-blocker:typed-reference-subtype-matching` — are named nowhere in the plan, so the claim of completeness is false and P3's wait is understated — `.engineering/planning/release-plan/next-waves-2026-10-01.md:77`

What I read: the plan and the superseded plan (whole bodies), kinds, relations, graph, the 17 draft and 4 active tasks, all 13 blockers (11 open), the three epics and the scheduled stories, validate (391 artifacts). Q1: five state-damaging defects among the open tasks, all in wave correct-07. Q2: all seven active schedulable items placed. Q3: nothing scheduled beyond its epic. The omission is carried forward from the superseded plan (both blockers date from 2026-09-27).

```findings
- file: .engineering/planning/release-plan/next-waves-2026-10-01.md
  line: 77
  category: scope
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: "the Not scheduled section calls the decision blockers unchanged and names nine, but decision-blocker:typed-reference-identifying-keys and decision-blocker:typed-reference-subtype-matching, which also block epic:p3-incubation-integration, are named nowhere in the plan"
```
