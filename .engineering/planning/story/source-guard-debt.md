---
format: aep.planning-md/2
id: story:source-guard-debt
kind: story
status: implemented
title: Make repository guards check the invoking tree and declared surface
relations:
- serves: vision:o2
- decomposes: epic:p1-kernel-ontology-core
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/ekr-core/tests/adversary2_public_surface.rs
- confidence: cited
  path: crates/ekr-core/tests/identity_serde.rs
- confidence: cited
  path: crates/ekr-core/tests/public_surface.rs
- confidence: cited
  path: crates/ekr-graph/tests/adversary2_guard_bounds_and_ranges.rs
- confidence: cited
  path: crates/ekr-graph/tests/adversary2_membrane_and_addresses.rs
- confidence: cited
  path: crates/ekr-graph/tests/adversary_canonical_value_reach.rs
- confidence: cited
  path: crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs
- confidence: cited
  path: crates/ekr-graph/tests/canonical_value_and_assertion.rs
- confidence: cited
  path: crates/ekr-graph/tests/domain_projection.rs
- confidence: cited
  path: crates/ekr-graph/tests/revision_events.rs
- confidence: cited
  path: crates/ekr-kernel/tests/validation.rs
- confidence: cited
  path: crates/ekr-ontology/tests/domain_projection.rs
- confidence: cited
  path: crates/ekr-ontology/tests/inheritance_and_declaration_coherence.rs
- confidence: cited
  path: crates/ekr-ontology/tests/value_type_checking.rs
- confidence: cited
  path: crates/ekr-store/tests/adversary2_retention_event_contract.rs
- confidence: cited
  path: crates/ekr-store/tests/document_refusals.rs
- confidence: cited
  path: crates/ekr-store/tests/domain_projection.rs
- confidence: cited
  path: crates/ekr-store/tests/providers.rs
- confidence: cited
  path: crates/ekr/tests/adversary_docs_contract.rs
- confidence: cited
  path: crates/ekr/tests/msrv_contract.rs
- confidence: cited
  path: crates/ekr/tests/public_surface.rs
- confidence: cited
  path: crates/ekr/tests/support/rust_source.rs
- confidence: cited
  path: crates/ekr/tests/support/workspace_manifest.rs
- confidence: cited
  path: crates/ekr/tests/temporal_reads.rs
- confidence: cited
  path: xtask/src/main.rs
revision: 36
---
## Outcome

Close the existing runtime source-path, cross-crate temporal-read, public-type use,
and README membership guard tasks. This bounded unit gives those existing tasks
one source owner; their historical implemented origin stories remain implemented.
It introduces no new domain entity or product behavior.

## Acceptance

Source readers resolve the invoking checkout at runtime, including xtask doctor.
Use runtime CARGO_MANIFEST_DIR for Cargo invocation; a standalone xtask may walk
from its current directory to Cargo.lock, or refuse clearly when no workspace
can be located. A distinguishing mutation in a second checkout must be observed
by the same guard binary while the original checkout remains unchanged.

A workspace guard recursively checks product Rust source for the forbidden
open-ended valid-time filter, covering graph, kernel, store and nested modules.
It leaves legitimate transaction-time operations and negative test witnesses
alone. This is a policy tripwire, not proof against every equivalent expression.

The public-surface guard includes exported structs, enums, traits and type
aliases, without treating restricted visibility, comments, strings or partial
identifiers as evidence of use. Demonstrate an unused public type turns it red
and an actual typed use turns it green. Keep existing function/constant checks.

README Status explicitly accounts for every product crate declared by Cargo.
The xtask utility is identified separately and does not become a product domain.
Missing Status, omitted product members and invented members are refusals.
The currently documented behavior remains truthful while later work is open.

Preserve all current and frozen legacy canonical-family checks and substantive
assertions. Any uncovered API receives meaningful behavior coverage or a scoped
visibility correction; no no-op mentions solely to satisfy the guard. Retain
mutation evidence, independent adversarial review and the complete integration
gate. Existing exact compiler pin and diagnostic churn are recorded separately;
no snapshot edit is required for this guard unit. Runtime nesting and ESS/blob
representation remain owned by subsequent runtime/activation work.

## Scope

The inspected scope is .engineering/waves/p1-guard-debt-scope.md. It includes
source-reading helpers across core, ontology, graph, store, ekr and xtask,
the ekr public-surface and README guards, and a new recursive temporal guard.
Coordinator alone owns AGENTS/README policy prose and all planning writes.

This may execute beside upstream Eventlog atomic implementation because it
shares no repository bytes. Its EKR test paths collide with persisted-contract
activation, so finish this unit before dispatching that source implementation.
The original-format freeze correction precedes this unit's starting commit.
