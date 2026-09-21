---
format: aep.planning-md/1
id: task:ess-domain-carries-compound-value-types
kind: task
status: draft
title: The ESS domain has no representation for a List or Record property type
relations:
- informed_by: review-result:adversary-ontology-pass-2
- derived_from: story:ontology-types-and-values
- serves: vision:o2
revision: 1
---
## Context

Found by the adversary in wave p1-03 (`review-result:adversary-ontology-pass-2`, finding 4). `ekr.ontology.PropertyDefinition` carries `ref_allowed_types` for a `NodeRef` and `enum_variants` for an `Enum`, and nothing for a `List` or a `Record`. The document's own comment claims it carries the parameters of all four compound kinds; it does not.

The flattening exists because `ess/1` types are not recursive, so a `ValueType` containing another `ValueType` cannot be declared there. `NodeRef` and `Enum` flatten to a list of ids and a list of names. `List` and `Record` do not flatten: their parameter *is* another `ValueType`.

## Why it matters

`ekr-ontology` constructs and loads `ValueType::List` and `ValueType::Record`, and `systems/ekr/domains/ontology.yaml` is the contract `ess conform` holds an implementation to. A property type the crate accepts and the domain cannot express is a property type no conformance scenario can cover, and one the P3 store has no declared shape to persist.

## Acceptance

A `List` or `Record` property type declared in `ekr-ontology` has a declared representation in `systems/ekr/domains/ontology.yaml`, and `crates/ekr-ontology/tests/domain_projection.rs` asserts the carriers the domain declares match the compound kinds the crate admits.

## Notes

Three routes, none obviously right, which is why this is a task rather than a fix. The domain could carry a flattened textual encoding of the nested type, the way `graph.yaml` carries `TypedValue` as a kind plus a canonical string. It could declare a recursive shape if `ess/2` admits one — worth checking before inventing a projection. Or the crate could refuse `List` and `Record` property types until the domain can hold them, which is the smallest change and the largest loss.

Settle it no later than P3, which is the first wave whose store must persist a `PropertyDefinition`.

## Scope

- `systems/ekr/domains/ontology.yaml`
- `crates/ekr-ontology/src/value.rs`
- `crates/ekr-ontology/tests/domain_projection.rs`
