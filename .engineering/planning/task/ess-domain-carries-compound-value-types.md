---
format: aep.planning-md/1
id: task:ess-domain-carries-compound-value-types
kind: task
status: active
title: The ESS domain has no representation for a List or Record property type
relations:
- informed_by: review-result:adversary-ontology-pass-2
- derived_from: story:ontology-types-and-values
- serves: vision:o2
revision: 5
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

Derived 2026-09-23 by `story-scoper` (wave p1-14). Verdict: binding-case-needed.

- premise resolved at HEAD 014c901: `systems/ekr/domains/ontology.yaml` declares `ekr.ontology.ValueTypeProjection` with recursive `element` and `fields`; `ValueKind` lists `List` and `Record` — cited
- `crates/ekr-ontology/tests/domain_projection.rs::the_domain_and_current_codec_retain_all_recursive_compound_parameters` checks the YAML against a hard-coded field list and never derives kinds from the crate, so a new `ValueKind` variant passes it — cited
- closing change: one case in `crates/ekr-ontology/tests/domain_projection.rs` that equates the YAML `ValueKind` set with the crate set and maps each kind to its carrier through an exhaustive `match` — inferred
- `crates/ekr-ontology/src/value.rs` read, not changed — inferred
- Confidence: high

## Wave p1-14 rescope

Rescoped in wave p1-14 (2026-09-23): the plan review of 72779f9 found this task's premise already
resolved in `systems/` at b2b64f8. The only remaining obligation is a binding case in the owning
crate's `tests/domain_projection.rs` that fails if the declaration and the Rust type drift. If that
case already exists, the wave cites it and archives this task.
