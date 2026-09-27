---
format: aep.planning-md/1
id: review-result:p1-15-docs-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p1-15 docs unit
relations:
- reviews: story:cli-user-documentation
revision: 1
---
Adversary pass 2 on unit p1-15-docs (wave p1-15), `aep-drive:adversary`, against `17f8f2d`.

Owners: 5 findings, 5 implementor (introduced).

Verdict: NEEDS-CHANGE (1 warning), CONFIRMED (1 warning, 3 notes). cases: executed 12→14, red 1.

Pass-1 check: findings 1-5 resolved; 6 and 7 partly carried (A, B).

Purpose test: a bridge-inspection register authored from docs/cli.md alone seeded and committed first time, revisions 0-3, including an Invoke, a supersession and a retraction; bitemporal reads matched.

Cases in `crates/ekr/tests/adversary_p1_15_docs_r2.rs` (2; 1 red: three reachable refusals missing from the table; 1 green that goes red on a relabelled row).

Routing (coordinator): last correction, verified by the coordinator, no third attack.

```findings
[
  {"file": "docs/cli.md", "line": 1073, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "the refusal table omits invalid-supersession, unresolved-graph-root and assertion-lifecycle-state, which the page own retraction and supersession rules lead a reader to"},
  {"file": "crates/ekr/tests/docs_cli.rs", "line": 969, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "relabelling a named refusal as a validation issue, or changing the where column of any row, leaves the refusal-table guard green"},
  {"file": "crates/ekr/tests/docs_cli.rs", "line": 985, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the InvalidSeed stderr check is a bare prefix, so seed-ontology accepts a seed-ontology-lineage refusal"},
  {"file": "docs/cli.md", "line": 493, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the page does not state that supersession closes the old assertion valid time at effective_from or that the replacement from must equal it exactly"},
  {"file": "docs/cli.md", "line": 181, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "explain links carry the assertion id as id on Assertion links and assertion_id on Lifecycle links, and the page names neither field"}
]
```
