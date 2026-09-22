---
format: aep.planning-md/1
id: story:version-persisted-contracts
kind: story
status: draft
title: Version assertion history and revision occurrences before durable application
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:kernel-validated-seed
- serves: vision:o2
- implements: executable-system-specification:ekr-v1
scope:
- confidence: inferred
  path: crates/ekr-core/src/identity.rs
- confidence: inferred
  path: crates/ekr-core/src/lib.rs
- confidence: cited
  path: crates/ekr-graph/src/assertion.rs
- confidence: cited
  path: crates/ekr-graph/src/events.rs
- confidence: cited
  path: crates/ekr-graph/src/snapshot.rs
- confidence: cited
  path: crates/ekr-graph/tests
- confidence: cited
  path: crates/ekr-kernel/src/commit.rs
- confidence: inferred
  path: crates/ekr-kernel/src/migration.rs
- confidence: inferred
  path: crates/ekr-kernel/src/seed.rs
- confidence: cited
  path: crates/ekr-kernel/src/transaction.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/provenance.rs
- confidence: cited
  path: crates/ekr-kernel/tests
- confidence: inferred
  path: crates/ekr-kernel/tests/migration_inventory.rs
- confidence: cited
  path: crates/ekr-store/src/eventlog.rs
- confidence: cited
  path: crates/ekr-store/src/log.rs
- confidence: cited
  path: crates/ekr-store/src/snapshot.rs
- confidence: cited
  path: crates/ekr-store/tests
- confidence: inferred
  path: crates/ekr-store/tests/read_only_inventory.rs
- confidence: cited
  path: crates/ekr/tests/graph_assertion_serde.rs
- confidence: cited
  path: crates/ekr/tests/graph_events_serde.rs
- confidence: cited
  path: docs/epistemic-knowledge-runtime-design.md
- confidence: cited
  path: systems/ekr/domains/graph.yaml
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
revision: 16
---
## Context

The approved completion plan places persisted-contract corrections after seed admission and before durable application. The read-only scope report in .engineering/waves/p1-persisted-contract-scope.md cites the current shapes and preservation constraints. No format change is implemented by this record.

## Acceptance

Retraction and supersession retain exact validation attribution and evidence. Independent lifecycle state and validation assessment are encoded explicitly, with proposal admission refusing caller verdicts or withdrawal state. A dated design amendment settles superseded-valid-time behavior before tests are changed.

Each revision fact has an occurrence identity: separate equal-content occurrences persist separately; retries of the same record do not duplicate it; reusing the same identity with different content refuses. This covers Proposed and Validated as well as Rejected and Stale. Preserve recorded backend identity, schema version and stream position for replay and migration.

Validation receipts use the canonical value domain with equality and sensitivity regressions. Format dispatch distinguishes old/new encoders without changing global hash labels. Unknown versions and unsupported semantic fields refuse.

Legacy verification is read-only, using frozen original vectors and their original encoders. Preserve source bytes and identities. Missing operation payloads, governing ontology, validation receipts or prior acceptance history produce a named migration refusal. Do not invent metadata or convert a seed snapshot into missing history. Complete successful history conversion remains coupled to the durable writer and its acceptance; this story alone cannot close that obligation.

## Scope

Cited by the story-scoper: graph assertion and revision-event types, canonical encoders and snapshot queries; store event metadata, append identity and fold; kernel transaction encoding, proposal admission and event publication; serialization, projection and provider regressions. Coordinator owns the additive design amendment, ESS graph/kernel/core changes and planning. New occurrence identity in core is inferred. Seed conversion paths must be re-read after the seed story lands.

## Decisions before dispatch

Adopt separate validation/lifecycle fields and an occurrence envelope covering every repeatable fact, with a versioned graph/seed envelope coordinated with the seed implementor. Record exact shapes in the additive amendment and ESS contracts before source work. The suggested format strings in the scope report are recommendations until those documents land.

## Verification

Test before implementation. Run both provider lanes for duplicate occurrence, retry and changed-payload identity conflict, including repeated same-content proposal/validation/rejection. Preserve legacy vectors and test corrupt/unknown format and missing-history refusals. Full gate and independent adversary on the integrated format precede writer dispatch.
