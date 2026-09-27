---
format: aep.planning-md/1
id: story:transaction-and-validators
kind: story
status: implemented
title: Transactions and deterministic validators 1–7
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:graph-model-and-assertions
- implements: executable-system-specification:ekr-v1
- serves: vision:o2
scope:
- confidence: cited
  path: crates/ekr-kernel/src/issue.rs
- confidence: cited
  path: crates/ekr-kernel/src/lib.rs
- confidence: cited
  path: crates/ekr-kernel/src/transaction.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate
- confidence: cited
  path: crates/ekr-kernel/src/validate/authorization.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/cardinality.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/mod.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/ontology.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/provenance.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/reference.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/structural.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/types.rs
- confidence: cited
  path: crates/ekr-kernel/tests/adversary_membrane.rs
- confidence: cited
  path: crates/ekr-kernel/tests/adversary_membrane_pass_two.rs
- confidence: cited
  path: crates/ekr-kernel/tests/compile_fail
- confidence: cited
  path: crates/ekr-kernel/tests/compile_fail/validated_transaction_cannot_be_deserialised.rs
- confidence: cited
  path: crates/ekr-kernel/tests/compile_fail/validated_transaction_cannot_be_deserialised.stderr
- confidence: cited
  path: crates/ekr-kernel/tests/compile_fail/validated_transaction_has_no_constructor_outside_the_kernel.rs
- confidence: cited
  path: crates/ekr-kernel/tests/compile_fail/validated_transaction_has_no_constructor_outside_the_kernel.stderr
- confidence: cited
  path: crates/ekr-kernel/tests/encoding_field_order.rs
- confidence: cited
  path: crates/ekr-kernel/tests/validate_properties.proptest-regressions
- confidence: cited
  path: crates/ekr-kernel/tests/validate_properties.rs
- confidence: cited
  path: crates/ekr-kernel/tests/validation.rs
revision: 14
---
## Context

Design § 19–20: agents propose a `GraphTransaction`; a pipeline of deterministic validators turns
it into a `ValidatedTransaction`, and only that type commits. Design § 6.10: the proposer is never
the sole basis of validation. Design § 73: identity format, reference existence, type checking,
cardinality and transaction integrity are deterministic and rerunnable without a model. This
story is the integrity membrane.

## Acceptance

A transaction carrying exactly one of the five defects is refused, and the issue names that defect.

## Tests the story ships

- `ValidatedTransaction` has no public constructor outside `ekr-kernel`: a `trybuild` compile-fail
  test constructs one from an integration test and fails to compile.
- A transaction whose validator identity equals its proposer is refused by the authorization
  validator (design § 6.10).
- A valid transaction validates to the same `validation_hash` on two runs against the same
  snapshot.

## Scope

- `crates/ekr-kernel/src/transaction.rs` — `GraphTransaction`, `GraphOperation` (all § 19
  variants plus `Invoke`), `ValidatedTransaction` (private constructor, `validated_against`,
  `validation_hash`)
- `crates/ekr-kernel/src/validate/mod.rs` — the `Validator` trait (design § 20) and the pipeline
- `crates/ekr-kernel/src/validate/{structural,reference,types,cardinality,ontology,provenance,authorization}.rs`
  — validators 1–7, one file each, named as in `ekr.kernel.ValidatorName`
- `crates/ekr-kernel/src/issue.rs` — `ValidationIssue`
- `crates/ekr-kernel/src/lib.rs`
- `crates/ekr-kernel/tests/validate_properties.rs` (proptest)
- `crates/ekr-kernel/tests/compile_fail/` (trybuild)

## Notes

Depends on `story:graph-model-and-assertions`. Crate dependencies, as the skeleton declared them:
`ekr-kernel` → `ekr-core`, `ekr-ontology`, `ekr-graph`, `ekr-store`; the validators read a
`GraphSnapshot` from `ekr-graph` and call the type checker in `ekr-ontology`. The provenance
validator's policy in P1 is the minimum design § 6.5 states: a canonical assertion needs at least
one `Evidence` and `proposed_by` alone is not evidence. The authorization validator in P1 checks
only that the validating actor differs from the proposer (§ 6.10); the outward-write check of
amendment 83 is P4. Validators 8–10 arrive with P3 and P5. `proptest` and `trybuild` are declared
for `ekr-kernel` by `story:workspace-crate-skeleton`; this story adds no dependency and does not
touch `Cargo.lock`, so it can run beside `story:eventlog-store`.

The acceptance was restated on 2026-09-21, before wave p1-03 dispatched. It read "`validate` never
returns `Ok` when any operation carries a reference that does not resolve …" — which a `validate`
that answers `Err` to everything satisfies completely. Naming the five defects one at a time, and
requiring the issue to name the one it found, cannot be satisfied that way. The shipped case that a
valid transaction validates `Ok` is what closes the other direction. The five defects are unchanged:
an unresolvable reference, a value that fails its type, a cardinality the edge type forbids, an
`Invoke` the lifecycle does not declare, and a canonical assertion with no evidence.
