---
format: aep.planning-md/3
id: review-result:adversary-p2p3p4-01-integrate-pass-2
kind: review-result
status: active
title: Adversary pass 2 on unit I (integrate domain)
relations:
- reviews: story:integrate-domain-typed-reference
revision: 1
---
## Adversary pass 2 — unit I (`story:integrate-domain-typed-reference`), HEAD `9509f5e7`

Verdict: NEEDS-CHANGE. Cases executed 101→101 (ekr-core), plus scratch check
`integrate/scratch/adv-I-2/unmapped_markers.py`: A green (5 blocker relations, one marker each, right
cardinality), B red (no marker for Split → the node it splits). Origin: introduced 1.

Ledger against pass 1: carried 0, new 1, resolved 3 (the id doc count, the split-to-merge marker, the
survivor/NodeType markers); the suite-digest and union findings were routed no-op in pass 1.

Coordinator routing (2026-09-27): the relation "Split → source Node, exactly one" is filed under
`decision-blocker:split-reverting-merge-identity` (its relation section extended by the coordinator) and
goes back to the implementor as a one-marker fix. Attack budget is two passes; the coordinator verifies
the correction by reading the diff.

```findings
- file: systems/ekr/domains/integrate.yaml
  line: 100
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the correction removed the only marker for Split to the node it splits, a relation that is still undeclared and that the entity's own comment calls undecided, so the acceptance clause "every merge and split relation left as UNMAPPED" no longer holds for it
```
