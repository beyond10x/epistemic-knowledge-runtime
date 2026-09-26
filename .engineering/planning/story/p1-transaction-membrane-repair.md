---
format: aep.planning-md/2
id: story:p1-transaction-membrane-repair
kind: story
status: implemented
title: Repair transaction validation before durable application
relations:
- decomposes: epic:p1-kernel-ontology-core
- serves: vision:o2
- implements: executable-system-specification:ekr-v1
scope:
- confidence: cited
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
  path: crates/ekr-kernel/tests/adversary_p1_07.rs
- confidence: cited
  path: crates/ekr-kernel/tests/validation.rs
revision: 11
---
## Context

The independent review of 209dd5e reproduced seven invalid transactions accepted by the deterministic pipeline. Source and observed output are retained under `.engineering/reviews/p1-baseline-209dd5e/`. The approved completion plan repairs these before seed and durable application.

## Acceptance

Each archived membrane probe receives a named refusal: deleting an edge referenced by a new assertion; a relation with a scalar object; a String property with a Type object; conflicting updates to one property; a type with an absent parent; modification of an undeclared property; Invoke with unsupported preconditions. Valid controls still validate. Operation permutations preserve validity and resulting reference semantics. A deleted edge referenced by a retained assertion is also refused.

Schema evolution and entity merge remain later-phase operations and must refuse explicitly in P1. Ordinary declared lifecycle transitions without unsupported semantics remain accepted. Existing positive tests that accepted schema definition or MergeEntity must be replaced by explicit refusal tests; their prior acceptance contradicts the approved P1 boundary. Preserve separate malformed-operation and identity-collision regressions. Value properties cannot refer to edges in the current type system, so no such unreachable acceptance case is required.

## Implementation

Validators must agree on the resulting state of an unordered atomic operation set. Reject conflicting writes before application. Enforce declared assertion predicates and endpoint semantics for each subject/object form. Refuse unsupported constraints instead of ignoring them. Keep each validator responsible for its own issue class; unresolved references do not cause unrelated type refusals.

## Scope

Confirmed against implementation commit 6328031 and integrated helper visibility correction.

Cited: crates/ekr-kernel/src/validate/candidate.rs (originally inferred, now implemented), cardinality.rs, mod.rs, ontology.rs, reference.rs, structural.rs and types.rs; tests/validation.rs and adversary_membrane.rs. The adversary added tests/adversary_p1_07.rs. These are the complete changed kernel paths for the membrane unit.

The shared candidate indexes drive reference and cardinality checks. Unsupported schema/merge positives were replaced by explicit refusals under the approved P1 phase boundary, retaining malformed-operation and identity cases. Retained assertions participate in edge-reference validation. Coordinator-owned ontology comments now cite the executable opaque-constraint regressions. The separate decoder story owns its ontology source changes.

No store, graph wire-format, dependency or schema-shape change belongs to this membrane unit. Seed, durable writer and serialized-ontology admission remain separately recorded. All executable source paths added by this unit are runtime-local or contain no checkout lookup; the pre-existing compile-time lookup debt remains scheduled.

## Verification

Write failing cases before implementation and retain red runner output. Run kernel tests, format check and package clippy. An adversary adds boundary cases before integration. The coordinator runs the full gate and an independent review after the membrane repair.
