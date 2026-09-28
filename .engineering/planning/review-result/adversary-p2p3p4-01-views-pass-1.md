---
format: aep.planning-md/3
id: review-result:adversary-p2p3p4-01-views-pass-1
kind: review-result
status: active
title: Adversary pass 1 on unit V (views domain)
relations:
- reviews: story:graph-projection-specification
revision: 1
---
## Adversary pass 1 — unit V (`story:graph-projection-specification`), HEAD `09491147`

Verdict: NEEDS-CHANGE. Cases executed 0→2 (two scratch checks under `views/scratch/adversary/`), red 2:
the schema generated from `views.yaml` rejects a document written by its own determinism rules, and 4 of
7 wrong renderers pass all five authored scenarios. Origin: introduced 8.

Coordinator routing (2026-09-27): F1–F7 back to the implementor. F6 decided by the coordinator: a
scenario may only require a store the kernel can build today; evidence whose bytes were never retained
cannot be built (the seed refuses it, no post-seed operation adds evidence — open blocker
`decision-blocker:evidence-entry-after-seed`), so the unretained case leaves the scenario and is named in
an `UNMAPPED:` marker under that blocker. F8 is the coordinator's (suites regenerated at the wave close).

```findings
- file: systems/ekr/domains/views.yaml
  line: 19
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: determinism rules (3) integer-millisecond Timestamp and (4) null for absent Optional are rejected by the schema ess 0.33.0 generates from the same types
- file: crates/ekr-views/tests/fixtures/conformance/scenarios/evidence-reports-whether-its-bytes-are-retained.yaml
  line: 23
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: retained_evidence 1 of 2 cannot distinguish a renderer that inverts the retained flag
- file: systems/ekr/domains/views.yaml
  line: 389
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: GraphProjected carries no ontology count, so a renderer projecting the head ontology at a past revision passes every scenario
- file: systems/ekr/domains/views.yaml
  line: 380
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: not-found is external and no authored step requests at beyond head, so a renderer clamping to head passes
- file: crates/ekr-views/tests/fixtures/conformance/scenarios/a-seed-only-store-projects-revision-zero.yaml
  line: 5
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the header claims the seed revision carries no transaction id but nothing asserts it
- file: crates/ekr-views/tests/fixtures/conformance/scenarios/evidence-reports-whether-its-bytes-are-retained.yaml
  line: 1
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the fixture needs evidence with unretained bytes, which the seed refuses and no post-seed operation can create, so the renderer story cannot pass it without skipping
- file: systems/ekr/domains/views.yaml
  line: 15
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: rule (1) orders arrays by id and leaves id-less arrays (props value lists, ListValue.items) unordered by any rule
- file: systems/ekr/conformance/views-suite.json
  line: 980
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: suite digests cover the whole system and the ekr-views block text, so this branch's suite must be synthesized again on the integration branch
```
