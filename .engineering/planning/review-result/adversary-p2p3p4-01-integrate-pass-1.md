---
format: aep.planning-md/2
id: review-result:adversary-p2p3p4-01-integrate-pass-1
kind: review-result
status: active
title: Adversary pass 1 on unit I (integrate domain)
relations:
- reviews: story:integrate-domain-typed-reference
revision: 1
---
## Adversary pass 1 — unit I (`story:integrate-domain-typed-reference`), HEAD `3e8be368`

Verdict: NEEDS-CHANGE. Cases executed 100→101, red 1 (`adversary_i_macro_doc_counts_the_id_newtypes_it_declares`
in `crates/ekr-core/tests/identity_serde.rs`), plus one scratch check (`conform-freshness.sh`), red.
Origin: introduced 5, pre-existing 0, undecided 0.

Coordinator routing (2026-09-27): the identity doc count and the two UNMAPPED marker findings go back to
the implementor; the `suite.json` digest finding is the coordinator's (regenerated at the wave close, no-op
for the unit); the union-for-enum finding is accepted as designed — an ESS enum cannot carry a payload, so
`ResolutionOutcome` is a tagged union (no-op).

```findings
- file: crates/ekr-core/src/identity.rs
  line: 78
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the id_newtype! doc still says fifteen types share the macro's shape, but the unit's MergeId and SplitId make it seventeen
- file: systems/ekr/conformance/suite.json
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: registering ekr.integrate changes spec_digest and contract_digest, so conform-check fails at cmp unless the coordinator regenerates the suite
- file: systems/ekr/domains/integrate.yaml
  line: 71
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: ResolutionOutcome is a union with one Refused variant nesting both refusal codes, while the story declares an enum whose variants include the two named refusals
- file: systems/ekr/domains/integrate.yaml
  line: 102
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the Split marker leaves out the zero-or-one relation from a split to the merge it reverts, which split-reverting-merge-identity names
- file: systems/ekr/domains/integrate.yaml
  line: 87
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the marker gives the merge-to-survivor relation to merge-across-node-types, whose relation is Merge to NodeType of each side, and that relation is never marked
```
