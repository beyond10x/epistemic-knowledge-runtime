---
format: aep.planning-md/3
id: review-result:adversary-checks-01-q-pass-1
kind: review-result
status: active
title: Adversary, wave checks-01 unit Q, pass 1
relations:
- reviews: story:store-quality-report
revision: 1
---
## Verdict

CONFIRMED on `03ad1cfc`: no defect. Two rules the unit's suite never exercised (rounding a share down,
omitting a share of a whole of 0) are pinned by the adversary's cases in `07ad96e6`. The
`with_item_evidence` row of `docs/cli.md` was reworded at the merge. Held under attack: retracted and
superseded assertions, evidence cited twice, constrained edge-type properties, inherited properties,
aliases equal to names, revisions past the head, file and SQLite bytes, session and one-shot at one
revision.

```findings
- file: crates/ekr-views/src/quality.rs
  line: 94
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "every share the unit's suite checked divided exactly, so a rounding mutant passed"
- file: crates/ekr-views/src/quality.rs
  line: 59
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "no store in the unit's suite had a whole of 0, so emitting its share passed"
- file: docs/cli.md
  line: 333
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "with_item_evidence counts evidence added after the seed; the page called it evidence for one source item"
- file: docs/cli.md
  line: 341
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "the documented ekr.views.NotSeeded refusal has no CLI path; an unseeded store answers store-not-found"
- file: systems/ekr/domains/views.yaml
  line: 1801
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "shared_names is unbounded by design, about 39 bytes per sharing node"
```
