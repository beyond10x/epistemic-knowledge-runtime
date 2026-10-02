---
format: aep.planning-md/3
id: story:preparation-blobs-are-reclaimed
kind: story
status: proposed
title: Preparation blobs are reclaimed after publication
relations:
- serves: vision:o5
- decomposes: epic:read-and-storage-cost
- depends_on: story:explain-reads-an-index
- depends_on: task:migrate-reads-a-current-store
- depends_on: task:sqlite-store-replaced-in-place
scope:
- confidence: inferred
  path: crates/ekr-kernel/src/checkpoint.rs
- confidence: cited
  path: crates/ekr-kernel/src/commands.rs
- confidence: inferred
  path: crates/ekr-kernel/src/commit.rs
- confidence: inferred
  path: crates/ekr-kernel/src/explain.rs
- confidence: inferred
  path: crates/ekr-kernel/src/migrate.rs
- confidence: cited
  path: crates/ekr-kernel/src/records.rs
- confidence: cited
  path: crates/ekr-kernel/src/replay.rs
- confidence: inferred
  path: crates/ekr-store/src/eventlog.rs
- confidence: inferred
  path: crates/ekr-store/src/inventory.rs
- confidence: inferred
  path: crates/ekr-store/src/log.rs
- confidence: cited
  path: crates/ekr-store/src/preparation.rs
- confidence: inferred
  path: crates/ekr-views/src/changes.rs
- confidence: inferred
  path: crates/ekr/src/conformance.rs
- confidence: inferred
  path: docs/epistemic-knowledge-runtime-design.md
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
- confidence: inferred
  path: systems/ekr/domains/store.yaml
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T12:26:23Z", actor: "human:timo", revision: 5}
---
## Context

Performance audit 2026-09-29: preparation blobs are never reclaimed and each proposal is stored about
four times (`crates/ekr-store/src/preparation.rs:1162`, `:1166`, `crates/ekr-kernel/src/commands.rs:578`).
Preparations are about half the store; about 196 MB is written per batch at 1×. Estimated store at
1×: 522 → about 200 MB.

## Build

- After a revision is published, the preparation blobs it no longer needs are reclaimed.
- New commit receipts reference the proposal by content hash instead of carrying another copy; old
  receipts are read as before and keep their bytes (invariant 5). A format change: specify it and
  version the receipt.
- A replay of a store written before the change and after it gives the same roots.

## Acceptance

- The consumer-shaped 1× ingest leaves a store under half its size at 0.0.24, on both providers.
- Replay equality and every retained record intact on stores written before and after the change.
- No preparation blob is removed while its proposal is open, meaning the proposal holds a
  preparation that was elected and not yet published (a test that elects a preparation, runs the
  reclaim, and then publishes it).

## Scope

Derived 2026-09-30 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr-store` (preparation lifecycle) and `crates/ekr-kernel` (commit receipt format, commit decision, replay linkage) — cited
- **Files:** `crates/ekr-store/src/preparation.rs:1162` (`read_preparation`), `:1166` — cited
- **Files:** `crates/ekr-kernel/src/commands.rs:543` (`commit_decision`; the receipt is built at `:599`–`:611`, `proposal: tx.proposal.clone()`) — cited
- **Files:** `crates/ekr-kernel/src/records.rs:332` (`CommitReceiptV1`, `FORMAT` `ekr.commit-receipt/3` at `:360`, `check_format` `:381`) — cited
- **Files:** `systems/ekr/domains/kernel.yaml:151`, `:429` (`CommitReceiptV1`, `proposal: ProposalRecordV1`) — cited
- **Files:** `crates/ekr-kernel/src/replay.rs:1077`–`:1085` (`commit-record-linkage`) — cited
- **Files:** `crates/ekr-store/src/inventory.rs:153`–`:180` (`prepared_occurrence` reads the preparation blob; a reclaimed blob breaks it) — inferred
- **Files:** `crates/ekr-kernel/src/migrate.rs:155`–`:172`, `:535`–`:562` — inferred
- **Files:** `systems/ekr/domains/store.yaml:161`–`:235` (`PublicationPreparationV1`) — inferred
- **Also likely:** `crates/ekr-store/src/eventlog.rs:1361`–`:1378` (reclaim hook; `delete_blob` precedent `:1687`), `crates/ekr-store/src/log.rs:232` — inferred
- **Also likely (readers of `receipt.proposal`):** `crates/ekr-kernel/src/commit.rs:161`, `crates/ekr-kernel/src/explain.rs:227,381,401-406`, `crates/ekr-kernel/src/checkpoint.rs:209,537`, `crates/ekr-views/src/changes.rs:575`, `crates/ekr/src/conformance.rs:1876` — inferred
- **Documents:** `docs/epistemic-knowledge-runtime-design.md` (§ 94, § 99.5, § 100.2, § 100.3) — inferred
- **Confidence:** medium — the three named sites exist; the reclaim hook and the receipt change's fan-out are read from the tree
- **Would collide with:** `story:explain-reads-an-index` (`replay.rs`, `explain.rs`, `kernel.yaml`); anything touching the kernel's receipts or replay linkage, `ekr-store`'s preparation/inventory/`RevisionLog`, `migrate.rs` or the design document
- **Safety fact:** reclaim deletes only a preparation's private blob `ekr.private.preparation.{hash}` (`preparation.rs:585`); evidence payloads under their own addresses are published too (`preparation.rs:236`–`:247`, `:1326`–`:1332`) and must stay — inferred
- **Not established:** a harness for the 1× measurement; which four copies the audit counted; whether the receipt drops the field or resolves it on read (the fan-out depends on it); whether `migrate` must keep working on reclaimed stores (as the code stands, `inventory.rs` breaks)
