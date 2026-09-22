---
format: aep.planning-md/1
id: task:qualify-released-eventlog
kind: task
status: active
title: Qualify released Eventlog before durable activation
relations:
- derived_from: story:version-persisted-contracts
- serves: vision:o2
revision: 3
---
## Context

The operator reported an upstream upgrade during the approved completion run.
The published Eventlog release incorporates the earlier atomic-content and strict
inspection work plus provider changes. Current EKR selectors still use the
earlier source. Entity Runtime remains outside the product dependency graph under
roadmap D1.

## Acceptance

Verify the published immutable source and assess every currently consumed port.
Qualify existing kernel/store and dependency-contract checks against one candidate
revision selected consistently by core, File, SQLite and Cargo.lock. Exercise
atomic publication and retry through both providers before the durable writer
depends on them, using the portable AtomicBlobEventStore capability explicitly.
Preserve source stores and frozen EKR codecs. Do not silently migrate populated
legacy blob bindings or substitute another provider publication path.

Probe any measured source-path concern before calling it a defect. The upstream
assessment flags possible reuse of a corrupt existing SQLite blob binding by
atomic publication; an executed control must establish whether the released
provider admits it. Record the result and fix an actual upstream defect through
its owning repository before declaring that behavior qualified.

## Scope

Cited consumer selectors: Cargo.toml and Cargo.lock.
Cited qualifier guard: crates/ekr/tests/story_contract.rs.
Cited consumer behavior: crates/ekr-store/src/eventlog.rs and its existing tests.
Reports live under .engineering/reviews and the completion scratch root.
This task adds no domain entity or product storage format. Root owns pin changes,
planning and publication. The upgrade assessment and temporary isolated Rust
probe do not write to the live stores or either primary checkout.
