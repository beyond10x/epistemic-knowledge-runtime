---
format: aep.planning-md/1
id: review-result:p5-01-ontology-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p5-01 ontology unit
relations:
- reviews: story:schema-evolution-transactions
revision: 1
---
Adversary pass 1 on unit p5-01-ontology (wave p5-01), `aep-drive:adversary`, against `194761a`.

Owners: 5 findings, 5 implementor (introduced).

Verdict: NEEDS-CHANGE (1 warning), CONFIRMED (1 warning, 2 notes), INFEASIBLE (1 note). cases: executed 104→107, red 3.

Cases in `crates/ekr-ontology/tests/adversary_p5_01_ontology.rs` (3; 3 red): a constraint added to a held property, a change list that changes nothing, a parent dropped by an abstract type.

Routing (coordinator): all five back to the implementor. The adversary hit ENOSPC on `/` after its suite run (another session's build filled the disk; `/` recovered to 37G without action from this wave).

```findings
- file: crates/ekr-ontology/src/evolve.rs
  line: 311
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a ModifyProperty that adds an opaque constraint to a property instances hold passes incompatibilities empty, while the kernel then refuses every write to that type
- file: crates/ekr-ontology/src/evolve.rs
  line: 135
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: an identical property redeclaration derives a new version that changes nothing, which the EmptyChanges doc says is not a next version
- file: crates/ekr-ontology/src/evolve.rs
  line: 243
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a parent change on a zero-instance abstract type moves concrete children's conformance unseen, contradicting the module claim that any second route to next is checked
- file: crates/ekr-ontology/src/evolve.rs
  line: 67
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: evolve refuses only the prior's own id and accepts the prior's parent id, so lineage-wide id uniqueness falls to the kernel without being stated
- file: crates/ekr-ontology/src/evolve.rs
  line: 208
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: InstanceState does not say whether max_values and value_kinds count top-level values or list elements, and incompatibilities does not check next's parent is prior
```
