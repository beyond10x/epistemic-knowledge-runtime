---
format: aep.planning-md/1
id: review-result:p1-14-bindings-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p1-14 bindings unit
relations:
- reviews: task:store-snapshot-and-its-id-are-declared-not-implemented
- reviews: task:ess-domain-carries-compound-value-types
- reviews: task:graph-properties-keyed-by-string
revision: 1
---
Adversary pass 1 on unit p1-14-bindings (wave p1-14), `aep-drive:adversary`, against `4956f27`.

Owners: 6 findings, 4 implementor (introduced), 2 pre-existing.

Verdict: NEEDS-CHANGE (1 warning), INFEASIBLE (2 notes), CONFIRMED (3 notes). cases: executed
332→334, red 0 on the tree.

Cases in `crates/ekr-store/tests/adversary_p1_14_store_bindings.rs`:
`every_store_declaration_agrees_member_for_member_with_its_carrier` and
`every_store_declaration_names_a_carrier_whatever_key_its_mapping_opens_with`, both green on the
tree and red on scratch mirrors where a store field was added in YAML (s1), removed from Rust (s2),
or the removed Snapshot declarations were restored with a non-`name` first key (s4). The unit's
own store case stayed green on all three mirrors.

Routing (coordinator): findings 1, 2, 4, 5 back to the implementor; finding 3 folded into the same
correction because the unit extended that scanner; finding 6 no-op (the carrier table exists only
in the test because no Rust type implements `ValueTypeProjection`; recorded on the task).

```findings
[
  {"file": "crates/ekr-store/tests/domain_projection.rs", "line": 561, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The store binding checks only that a same-named Rust type exists, so a field added in store.yaml or removed from the Rust carrier passes all 8 domain_projection tests."},
  {"file": "crates/ekr-store/tests/domain_projection.rs", "line": 548, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "A declaration whose YAML mapping does not start with name is invisible to the scanner, so re-adding SnapshotId and Snapshot with kind or identity first stays green."},
  {"file": "crates/ekr-core/tests/identity_serde.rs", "line": 134, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "pre-existing", "message": "The newtype scanner the unit extended to store.yaml misses a reordered ekr.store.SnapshotId declaration."},
  {"file": "crates/ekr-ontology/tests/domain_projection.rs", "line": 280, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "A closing brace in a doc comment inside pub enum ValueKind cuts the variant scan short and fails the case with a misleading domain-disagreement message."},
  {"file": "crates/ekr-graph/tests/domain_projection.rs", "line": 771, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The PropertyId key check compares one exact line, so a path-qualified PropertyId fails it, and a serde attribute that changes the serialised key would not be seen."},
  {"file": "crates/ekr-ontology/tests/domain_projection.rs", "line": 253, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "The kind-to-carrier table exists only in the test because no Rust type implements ValueTypeProjection, so a swapped List/Record carrier row stays green."}
]
```
