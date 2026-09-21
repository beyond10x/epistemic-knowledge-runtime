---
format: aep.planning-md/1
id: story:ontology-types-and-values
kind: story
status: draft
title: 'Ontology: types, typed values, lifecycles, and the type checker'
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:kernel-identity-and-hashing
- implements: executable-system-specification:ekr-v1
scope:
- confidence: inferred
  path: crates/ekr-ontology/src/check.rs
- confidence: inferred
  path: crates/ekr-ontology/src/lib.rs
- confidence: inferred
  path: crates/ekr-ontology/src/lifecycle.rs
- confidence: inferred
  path: crates/ekr-ontology/src/schema.rs
- confidence: inferred
  path: crates/ekr-ontology/src/types.rs
- confidence: inferred
  path: crates/ekr-ontology/src/value.rs
revision: 4
---
## Context

Design § 11–12 give the ontology: node types with parents and properties, edge types with source
and target constraints and cardinality, property definitions with typed values, and a `Value`
that mirrors its `ValueType` so that `employer = NodeRef(...)` is checkable and `employer =
"OpenAI"` is not. Amendment 87 adds per-type lifecycles and named operations. The type checker
this story delivers is what validators 3–5 of design § 20 call.

## Acceptance

For every `(Value, ValueType)` pair the property suite generates, `Ontology::check` returns `Ok`
exactly when the value satisfies the declared type.

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

## Notes

Depends on `story:kernel-identity-and-hashing` for the id types. `Constraint` (design § 11.2) has
no defined language; carry it as an opaque string and refuse nothing on it yet, matching the
`UNMAPPED:` marker in `ontology.yaml`. Schema versions are data in this story; the transactions
that change them are P5. Uses only dependencies `story:workspace-crate-skeleton` declared for
`ekr-ontology`.
