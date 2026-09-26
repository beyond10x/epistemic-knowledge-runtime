---
format: aep.planning-md/2
id: story:ontology-types-and-values
kind: story
status: implemented
title: 'Ontology: types, typed values, lifecycles, and the type checker'
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:kernel-identity-and-hashing
- implements: executable-system-specification:ekr-v1
- serves: vision:o2
scope:
- confidence: cited
  path: crates/ekr-ontology/src/check.rs
- confidence: cited
  path: crates/ekr-ontology/src/lib.rs
- confidence: cited
  path: crates/ekr-ontology/src/lifecycle.rs
- confidence: cited
  path: crates/ekr-ontology/src/schema.rs
- confidence: cited
  path: crates/ekr-ontology/src/types.rs
- confidence: cited
  path: crates/ekr-ontology/src/value.rs
- confidence: cited
  path: crates/ekr-ontology/tests/domain_projection.rs
- confidence: cited
  path: crates/ekr-ontology/tests/hierarchy_specificity.rs
- confidence: cited
  path: crates/ekr-ontology/tests/inheritance_and_declaration_coherence.rs
- confidence: cited
  path: crates/ekr-ontology/tests/lifecycle_transitions.rs
- confidence: cited
  path: crates/ekr-ontology/tests/ontology_load.rs
- confidence: cited
  path: crates/ekr-ontology/tests/type_hierarchy.rs
- confidence: cited
  path: crates/ekr-ontology/tests/value_type_checking.rs
revision: 11
---
## Context

Design § 11–12 give the ontology: node types with parents and properties, edge types with source
and target constraints and cardinality, property definitions with typed values, and a `Value`
that mirrors its `ValueType` so that `employer = NodeRef(...)` is checkable and `employer =
"OpenAI"` is not. Amendment 87 adds per-type lifecycles and named operations. The type checker
this story delivers is what validators 3–5 of design § 20 call.

## Acceptance

A value generated to violate exactly one property of its declared type is refused.

## Tests the story ships

- A refused check names the property and the reason.
- `NodeRef.allowed_types` and `Enum.variants` are enforced by the checker.
- `Lifecycle::transition(from, operation)` refuses a move the type does not declare and accepts
  every declared one.
- An `Ontology` with a property whose `value_kind` is `NodeRef` and an empty `allowed_types` is
  refused at load.

## Scope

- `crates/ekr-ontology/src/value.rs` — `ValueType`, `Value` (design § 11.3), `Cardinality`
- `crates/ekr-ontology/src/types.rs` — `NodeType`, `EdgeType`, `PropertyDefinition`
  (`systems/ekr/domains/ontology.yaml`)
- `crates/ekr-ontology/src/lifecycle.rs` — `Lifecycle`, `Transition`, `OperationDefinition`
  (amendment 87)
- `crates/ekr-ontology/src/schema.rs` — `SchemaVersion`, `Ontology` (the registry of one
  version's types; design § 69 `SchemaRegistry`)
- `crates/ekr-ontology/src/check.rs` — the type checker
- `crates/ekr-ontology/src/lib.rs`

Test files, written during wave p1-03 and read from the merged tree:

- `crates/ekr-ontology/tests/value_type_checking.rs` — the acceptance, the two `NodeRef`/`Enum`
  enforcement cases, and the property that a value broken in exactly one place is refused
- `crates/ekr-ontology/tests/ontology_load.rs` — what a document must satisfy to load at all
- `crates/ekr-ontology/tests/type_hierarchy.rs` — parents, ancestry and `conforms_to`
- `crates/ekr-ontology/tests/hierarchy_specificity.rs` — property resolution by the specialisation
  order, and the cases where `AmbiguousProperty` may and may not fire
- `crates/ekr-ontology/tests/inheritance_and_declaration_coherence.rs` — declaration coherence, and
  which fault a two-fault document reports
- `crates/ekr-ontology/tests/lifecycle_transitions.rs` — declared and undeclared moves
- `crates/ekr-ontology/tests/domain_projection.rs` — the crate's citations of
  `systems/ekr/domains/ontology.yaml` stay true in both directions

## Notes

Depends on `story:kernel-identity-and-hashing` for the id types. `Constraint` (design § 11.2) has
no defined language; carry it as an opaque string and refuse nothing on it yet, matching the
`UNMAPPED:` marker in `ontology.yaml`. Schema versions are data in this story; the transactions
that change them are P5. Uses only dependencies `story:workspace-crate-skeleton` declared for
`ekr-ontology`.

The acceptance was restated on 2026-09-21, before wave p1-03 dispatched. It read "for every
`(Value, ValueType)` pair the property suite generates, `Ontology::check` returns `Ok` exactly when
the value satisfies the declared type" — and "satisfies the declared type" is what `check` decides,
so the case had no oracle independent of the thing it tests. Generating a value *from* a type, then
breaking exactly one of that type's properties, gives the generator as the oracle; a checker that
answers `Ok` to everything now dies. The opposite failure — a checker that refuses everything — is
killed by the shipped case that a well-typed value checks `Ok`.

## Constraint boundary correction, 2026-09-22

The ontology crate still carries opaque constraint and precondition text; its loader and lifecycle helper do not evaluate that language. The earlier note to refuse nothing does not apply to transaction admission: story:p1-transaction-membrane-repair refuses writes affected by nonempty uninterpreted constraints and invokes carrying preconditions or emitted effects with no implementation. The kernel cases applicable_opaque_property_constraints_refuse_all_node_write_paths and opaque_preconditions_and_emissions_are_not_silently_accepted execute that boundary. The pure ontology fixture remains a carrier test, not transaction acceptance.
