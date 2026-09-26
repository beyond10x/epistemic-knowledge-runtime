---
format: aep.planning-md/2
id: review-result:p1-13-explain-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p1-13 explain unit
relations:
- reviews: story:seed-and-explain
revision: 1
---
Adversary pass 1 on unit p1-13-explain (wave p1-13), `aep-drive:adversary`.

Owners: 2 findings, 2 implementor (introduced).

Verdict: INFEASIBLE (both red cases are built by mutating a capture's public fields; nothing found
that reaches them). cases: executed 10→15, red 2.

Cases in `crates/ekr-kernel/tests/adversary_p1_13_explain_1.rs`: four consistent forgeries of a
captured graph returned as Ok on both providers (red); a snapshot pairing a root with a graph it
does not commit to (red); a two-step supersession chain at head and earlier revision (green); a
retraction transaction's own evidence set (green); carrier names and `kind` tags against
`kernel.yaml` (green).

The `systems/ekr/domains/kernel.yaml` change the adversary saw in the tree is the coordinator's
`ekr.kernel.ProposalAttribution` declaration, synced from the integration branch.

```findings
- file: crates/ekr-kernel/src/explain.rs
  line: 188
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: explain never binds the captured graph or seed_input to the root it reports, so a consistent forgery of assertion, evidence or seed-origin records is returned as Ok despite the module's claim that every named record is verified.
- file: crates/ekr-kernel/src/explain.rs
  line: 152
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: snapshot pairs self.root with self.graph without recomputing knowledge_root/evidence_root, so it can return a root that does not commit to the graph it carries.
```
