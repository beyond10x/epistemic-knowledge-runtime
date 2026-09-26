---
format: aep.planning-md/2
id: review-result:p1-14-exit-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p1-14 exit unit
relations:
- reviews: story:p1-exit-properties
revision: 1
---
Adversary pass 2 on unit p1-14-exit (wave p1-14), `aep-drive:adversary`, against `8a22a2e` (tree also carried the coordinator systems/ copies of 74e7fb0).

Owners: 4 findings, 3 implementor (introduced), 1 pre-existing.

Verdict: NEEDS-CHANGE (1 warning), CONFIRMED (3 notes). cases: executed 563→569, red 0.

Pass-1 check: finding 1 resolved (three assertion references are CanonicalRef<Assertion>, trybuild cases pass); finding 2 resolved as worded, replaced by finding A; finding 3 coverage resolved, residue carried as finding B.

Cases in `crates/ekr-kernel/tests/adversary2_p1_14_exit_readback_and_assertion_refs.rs` (6, green): the physical event counter sees publications; a refused commit publishes nothing a fresh runtime reads, both providers; typed dispute, supersession and graph-assertion source keep frozen bytes and wire shape; a transient identity minted into every reference kind compiles through CanonicalRef::new (finding D).

Routing (coordinator): A and B back to the implementor for a last correction the coordinator verifies (no third attack); C no-op (bytes held by vectors and the pass-2 cases); D no-op, accepted design (ADR 0008).

```findings
[
  {"file": "verification-report.md", "line": 20, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The report describes ba0f59d plus an uncommitted patch and makes the assertion-reference type guarantee conditional on it, while 8a22a2e has the patch committed and lib.rs states the opposite of what the report quotes."},
  {"file": "crates/ekr-kernel/tests/p1_exit_properties.rs", "line": 510, "category": "property", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The cross-kind borrowed-id variant is a coin flip over 3 unseeded cases per shape, so in about 94 percent of runs at least one shape never tests it, and a failure is not reproducible."},
  {"file": "crates/ekr-kernel/tests/p1_exit_properties.rs", "line": 705, "category": "property", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The replay generator produces no supersession, dispute or edge-subject assertion, so none of the retyped fields appear in a generated lineage."},
  {"file": "crates/ekr-graph/src/canonical.rs", "line": 318, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "The public CanonicalRef::new mints a canonical reference of every kind from a LocalRef's id, so the compile-fail cases rule out a bare id coercing but not a transient identity."}
]
```
