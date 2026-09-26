---
format: aep.planning-md/2
id: review-result:p1-15-docs-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p1-15 docs unit
relations:
- reviews: story:cli-user-documentation
revision: 1
---
Adversary pass 1 on unit p1-15-docs (wave p1-15), `aep-drive:adversary`, against `3dd52ab`.

Owners: 7 findings, 7 implementor (introduced).

Verdict: NEEDS-CHANGE (1 blocker, 1 warning), CONFIRMED (1 warning, 4 notes). cases: executed 9→11, red 2.

Method: the adversary wrote a lab-inventory schema from docs/cli.md alone and seeded it on the first try; 25 of 25 validate-level refusal rows and 13 of 13 seed and ontology rules matched the page.

Cases in `crates/ekr/tests/adversary_p1_15_docs.rs` (2, red): Invoke resolves by map key, not name; `assessment: Accepted` is refused at propose as StructurallyInvalid.

Routing (coordinator): all seven back to the implementor.

```findings
[
  {"file": "docs/cli.md", "line": 368, "category": "contract-drift", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "!Invoke resolves an operation by its map key, not by `name` as the page states, and the seed accepts a name that differs from its key"},
  {"file": "docs/cli.md", "line": 495, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "assessment: Accepted is refused at propose as StructurallyInvalid exit 2, not rejected at validation as assertion-states-its-own-verdict"},
  {"file": "docs/cli.md", "line": 313, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Decimal is called an exact number but any string is accepted"},
  {"file": "docs/cli.md", "line": 165, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "explain also returns Seed, Lifecycle and repeated Assertion links for seeded or superseded assertions"},
  {"file": "docs/cli.md", "line": 1071, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "seed-authority-profile exits 1 as opening the provider: invalid seed, not in the documented InvalidSeed exit-2 form"},
  {"file": "docs/cli.md", "line": 1054, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the refusal table omits seed-unsupported-source, seed-root-lineage, seed-ontology-lineage, TransactionNotFound and bootstrap-authority-mismatch"},
  {"file": "crates/ekr/tests/docs_cli.rs", "line": 739, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "the refusal-table guard is a prefix substring search over all source, so a truncated code passes and the where, exit and trigger columns are never checked"}
]
```
