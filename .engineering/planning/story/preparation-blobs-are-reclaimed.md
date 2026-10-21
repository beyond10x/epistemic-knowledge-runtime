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
- confidence: inferred
  path: crates/ekr-store/src/objects.rs
- confidence: cited
  path: crates/ekr-store/src/preparation.rs
- confidence: cited
  path: crates/ekr-store/tests/eventlog_object_memo.rs
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
revision: 10
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

Re-derived 2026-10-21 by `story-scoper` against the tree of wave 2026-10-21b U1 (profile v4), read-only. Each line is **cited** (read in the tree) or **inferred**.

- **What a commit writes today:** four copies of the proposal: the proposal record object; propose's private preparation record; the receipt, which embeds the full proposal (`crates/ekr-kernel/src/commands.rs:658-670`, `:662`); commit's private preparation record. The two private preparation records are what this story reclaims. Private blob key: `crates/ekr-store/src/preparation.rs:590`. cited
- **Readers that break when a preparation blob is gone:** inventory (`crates/ekr-store/src/inventory.rs:137-148`, `:370-373`), `migrate` through inventory (`crates/ekr-store/src/migrate.rs:261`), and `read_preparation` (`crates/ekr-store/src/preparation.rs:1172-1197`, which reads every selection's blob). cited
- **Readers that hold:** replay, explain and checkpoint do not read the private blob. cited
- **Store rule 2** (`systems/ekr/domains/store.yaml:738-762`): retained bytes are withdrawn only with an event on the object's stream. Guard: `crates/ekr-store/tests/eventlog_object_memo.rs`, `no_source_withdraws_retained_bytes_without_an_event_on_the_object_stream`. cited
- **No harness measures store bytes.** `crates/ekr/tests/explain_by_reference.rs:70-78` (`ONE_X`) is the closest pattern. cited
- **Receipt format:** `ekr.commit-receipt/3` (`crates/ekr-store/src/records.rs:360`). cited
- **Files:** `crates/ekr-store/src/preparation.rs` cited; `inventory.rs`, `eventlog.rs`, `objects.rs`, `tests/eventlog_object_memo.rs`, `crates/ekr-store/tests/recovery/support.rs`, `systems/ekr/domains/store.yaml`, `docs/epistemic-knowledge-runtime-design.md` inferred.
- **Confidence:** medium.
- **Would collide with:** wave 2026-10-21b U2 (`story:validation-profile-move`) if it adds a log event kind in `eventlog.rs`. inferred

**Decided by the coordinator on 2026-10-21** (each the option the code favours):

1. A reclaimed preparation blob is withdrawn with an event on the preparation stream, and the guard's allow-list names that event.
2. `read_preparation` reads only the latest selection's blob.
3. Stage `/4` preparations are excluded.
4. The receipt format does not change: the receipt keeps embedding the proposal, so no reader of `ekr.commit-receipt/3` breaks.
5. Inventory skips the blob for a published selection.
