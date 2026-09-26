---
format: aep.planning-md/2
id: review-result:p1-14-bindings-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p1-14 bindings unit
relations:
- reviews: task:store-snapshot-and-its-id-are-declared-not-implemented
- reviews: task:ess-domain-carries-compound-value-types
- reviews: task:graph-properties-keyed-by-string
revision: 1
---
Adversary pass 2 on unit p1-14-bindings (wave p1-14), `aep-drive:adversary`, against `cfa7ad3`.

Owners: 4 findings, 2 implementor (introduced), 2 pre-existing.

Verdict: CONFIRMED (4 notes). cases: executed 334→337, red 0 on the tree.

Pass-1 check: findings 1–5 fixed (re-run on mirrors f1, f1b, f2, f4, f5a, f5b); finding 6 carried.

Cases added, all green on the tree and red on a mirror where the unit's case stayed green:
`crates/ekr-store/tests/adversary_p1_14_r2_store_carriers.rs`
(`every_store_field_is_carried_with_the_type_the_domain_declares`,
`every_store_carrier_is_written_under_the_member_names_the_binding_compares`) and
`crates/ekr-graph/tests/adversary_p1_14_r2_graph_declaration_scan.rs`
(`every_graph_declaration_the_line_scans_must_see_opens_with_its_name`).

Routing (coordinator): findings 1–3 are held by the adversary's own green cases, which the unit
keeps, so no correction round; finding 4 is pass-1 finding 6, no-op. The unit merges.

```findings
[
  {"file": "crates/ekr-store/tests/domain_projection.rs", "line": 908, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The store binding compares member names only, so a field type changed in store.yaml alone (StoredObject.byte_len Integer to String) leaves it green; adversary_p1_14_r2_store_carriers.rs holds the types."},
  {"file": "crates/ekr-store/tests/domain_projection.rs", "line": 819, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "rust_members strips attributes and compares the Rust identifier, so a serde rename that moves a carrier's wire name away from store.yaml stays green."},
  {"file": "crates/ekr-graph/tests/domain_projection.rs", "line": 68, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "The graph line scans see a declaration only when its mapping opens with name:, so a reordered graph.yaml declaration with fields needs no carrier and the field-for-field case stays green."},
  {"file": "crates/ekr-ontology/tests/domain_projection.rs", "line": 253, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "The kind-to-carrier table exists only in the test because no Rust type implements ValueTypeProjection, so a swapped List/Record carrier row stays green."}
]
```
