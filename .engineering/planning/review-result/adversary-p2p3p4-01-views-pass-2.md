---
format: aep.planning-md/2
id: review-result:adversary-p2p3p4-01-views-pass-2
kind: review-result
status: active
title: Adversary pass 2 on unit V (views domain)
relations:
- reviews: story:graph-projection-specification
revision: 1
---
## Adversary pass 2 — unit V (`story:graph-projection-specification`), HEAD `c548b271`

Verdict: NEEDS-CHANGE. Scripted checks executed 5→7, red 3 (one new: `views/scratch/adversary-2/check_more_mutants.py`,
3 of 3 new wrong renderers survive; two are the pass-1 originals, red because the rules legitimately changed — the
adversary judged the implementor's r1 revisions legitimate, no assertion weakened). Origin: introduced 4.
Ledger against pass 1: carried 0, new 4, resolved 7 (F1–F7; F8 routed to the coordinator).

Coordinator routing (2026-09-27): attack budget spent (two passes). A1, A2, A3 and a rewording for A4 go to the
implementor as correction round 2; the coordinator verifies that diff.

```findings
- file: crates/ekr-views/tests/fixtures/conformance/scenarios/a-retracted-assertion-is-projected-with-its-lifecycle.yaml
  line: 13
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: no authored step projects a store with graph state at a past revision, so renderers reading lifecycle or assertions from the head pass every scenario
- file: crates/ekr-views/tests/fixtures/conformance/scenarios/a-revision-beyond-the-head-is-refused.yaml
  line: 17
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: no step requests at equal to the head, so a renderer refusing at >= head instead of at > head survives
- file: systems/ekr/domains/views.yaml
  line: 355
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: ProjectGraph has no outcome for an unseeded store and RevisionNotFound requires a head such a store lacks
- file: systems/ekr/domains/views.yaml
  line: 36
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the UNMAPPED marker blames the evidence-entry blocker, but the verified read already refuses missing evidence bytes, so retained false may stay unreachable whatever the blocker decides
```
