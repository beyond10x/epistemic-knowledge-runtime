---
format: aep.planning-md/1
id: story:p1-transaction-membrane-repair
kind: story
status: active
title: Repair transaction validation before durable application
relations:
- decomposes: epic:p1-kernel-ontology-core
- serves: vision:o2
- implements: executable-system-specification:ekr-v1
scope:
- confidence: inferred
  path: crates/ekr-kernel/src/validate/candidate.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/cardinality.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/mod.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/ontology.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/reference.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/structural.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/types.rs
- confidence: cited
  path: crates/ekr-kernel/tests/adversary_membrane.rs
- confidence: cited
  path: crates/ekr-kernel/tests/validation.rs
revision: 7
---
## Context

The independent review of 209dd5e reproduced seven invalid transactions accepted by the deterministic pipeline. Source and observed output are retained under `.engineering/reviews/p1-baseline-209dd5e/`. The approved completion plan repairs these before seed and durable application.

## Acceptance

Each archived membrane probe receives a named refusal: deleting an edge referenced by a new assertion; a relation with a scalar object; a String property with a Type object; conflicting updates to one property; a type with an absent parent; modification of an undeclared property; Invoke with unsupported preconditions. Valid controls still validate. Operation permutations preserve validity and resulting reference semantics. A deleted edge referenced by a retained assertion is also refused.

Schema evolution and entity merge remain later-phase operations and must refuse explicitly in P1. Ordinary declared lifecycle transitions without unsupported semantics remain accepted. Existing positive tests that accepted schema definition or MergeEntity must be replaced by explicit refusal tests; their prior acceptance contradicts the approved P1 boundary. Preserve separate malformed-operation and identity-collision regressions. Value properties cannot refer to edges in the current type system, so no such unreachable acceptance case is required.

## Implementation

Validators must agree on the resulting state of an unordered atomic operation set. Reject conflicting writes before application. Enforce declared assertion predicates and endpoint semantics for each subject/object form. Refuse unsupported constraints instead of ignoring them. Keep each validator responsible for its own issue class; unresolved references do not cause unrelated type refusals.

## Scope

Derived 2026-09-22 by the story-scoper charter; confidence high.

- Cited: `crates/ekr-kernel/src/validate/reference.rs`, `types.rs`, `structural.rs`, `ontology.rs`, `cardinality.rs` and `mod.rs` — the inspected validators and shared identity helper.
- Inferred: `crates/ekr-kernel/src/validate/candidate.rs` — optional shared post-state index.
- Cited: `crates/ekr-kernel/tests/validation.rs` and `adversary_membrane.rs` — fixture, named refusal cases, and permutation controls.
- Source contracts: no store, graph wire-format, dependency or ontology-schema changes. Coordinator owns planning, ESS changes and CI.
- Existing fresh-type and valid-merge positive controls conflict with the approved phase boundary; replace with unsupported-operation refusals, preserving malformed-operation coverage.
- Retained properties cannot refer to edges in this model; retained assertion subjects can. The acceptance has been corrected accordingly.

## Verification

Write failing cases before implementation and retain red runner output. Run kernel tests, format check and package clippy. An adversary adds boundary cases before integration. The coordinator runs the full gate and an independent review after the membrane repair.
