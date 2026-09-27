---
format: aep.planning-md/1
id: review-result:p5-01-ontology-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p5-01 ontology unit
relations:
- reviews: story:schema-evolution-transactions
revision: 1
---
Adversary pass 2 on unit p5-01-ontology (wave p5-01), `aep-drive:adversary`, against `36a00eb`.

Owners: 2 findings, 2 implementor (introduced).

Verdict: CONFIRMED (1 note), NEEDS-CHANGE (1 warning). cases: executed 112→115, red 1.

Cases in `crates/ekr-ontology/tests/adversary2_p5_01_ontology.rs` (3; 1 red): a reference narrowed from an abstract type to its only concrete subtype; a 20,000-draw soundness property over redeclarations (green); a check that the property catches a state blind to value kinds (green).

Routing (coordinator): both back to the implementor; finding 2 also goes into the kernel unit brief.

```findings
- file: crates/ekr-ontology/src/evolve.rs
  line: 472
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a NodeRef narrowed from an abstract type to its only concrete subtype admits every value it did and is still refused as value-type-narrowed, because admits_all_of compares declared types rather than the concrete types that can be referenced
- file: crates/ekr-ontology/src/evolve.rs
  line: 226
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: InstanceState defines values as what cardinality bounds, which leaves out property-assertion objects the kernel type-checks, so a faithful kernel passes a value-type change that invalidates existing assertions
```
