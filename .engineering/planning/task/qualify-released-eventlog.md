---
format: aep.planning-md/1
id: task:qualify-released-eventlog
kind: task
status: implemented
title: Qualify released Eventlog before durable activation
relations:
- derived_from: story:version-persisted-contracts
- serves: vision:o2
revision: 6
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

## Released provider qualification in progress

The candidate selectors and dependency guard now name the published tag's exact
commit, verified against its remote annotated tag and release. Current kernel/
store tests and dependency checks passed; the retained cross-version probe wrote
synthetic evidence-backed seeds with the previous provider pin and reopened them
in a fresh process through the candidate's real kernel authority. Roots, graph
state, replay and exact evidence agree on both backends. This proves bounded seed
compatibility, not transaction application or live-store migration.

Candidate commit: ac6b1731654329d32f1e3c9cf164fefad6a5b46a.

Executed kernel/store results: 197 passed, 0 failed, 0 ignored across 29 runner summaries; command status 0

The separate provider probe reproduced acceptance of a corrupt existing SQLite
blob binding by fresh atomic publication. Its full report is
.engineering/reviews/eventlog-030-integrity-probe.md; the source assessment is
.engineering/reviews/eventlog-030-assessment.md. Exact prior-receipt retry is a
passing control, not the defect. The existing inline consumer does not use the
affected publication path.

The upstream repair is active as Eventlog task:validate-atomic-blob-reuse.
Do not claim the future atomic publication path qualified until its correction,
review and required source CI pass and EKR selects that repaired source.
The candidate pin remains unpublished while qualification proceeds. Entity
Runtime remains outside the product dependency graph under roadmap D1.

## Correction qualification completed

The earlier candidate above is superseded by the reviewed repair recorded in
.engineering/reviews/eventlog-repair-adoption.md. Eventlog PR #15 is merged with
required backend, comparative and restart CI green. All three selectors, the
lock and qualifier guard select its exact published source commit in this
branch. The complete EKR gate and a new direct old-writer/repaired-reader
fresh-process proof pass on both providers.

The original report and probe remain retained. The changed SQLite path refuses
fresh corrupt binding reuse while preserving original receipt retries. The
source qualification is complete; consumer branch publication and durable
writer acceptance remain separate work and are not claimed by this task.
