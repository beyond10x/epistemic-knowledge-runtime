---
format: aep.planning-md/1
id: story:eventlog-0-4-batched-reads
kind: story
status: implemented
title: Eventlog 0.4.0 and batched history reads, so file-provider commands stop growing quadratically
relations:
- serves: vision:o2
- derived_from: task:store-reads-rehash-the-whole-log
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: inferred
  path: crates/ekr-kernel/tests
- confidence: cited
  path: crates/ekr-store/src/eventlog.rs
- confidence: inferred
  path: crates/ekr-store/src/lib.rs
- confidence: inferred
  path: crates/ekr-store/src/preparation.rs
- confidence: cited
  path: crates/ekr/tests/story_contract.rs
revision: 13
---
## Context

`task:store-reads-rehash-the-whole-log`: on the file provider each command re-reads and re-hashes the whole event log once per object and blob it reads; propose took 344 ms at revision 1 and 2,487 ms at revision 40 (release build). Eventlog 0.4.0 stops the per-operation re-hash while the journal is unchanged and at least two seconds old, and adds `read_many`, which answers a batch in one transaction.

## Acceptance

- EKR pins eventlog 0.4.0 (all provider crates, one rev).
- `EventlogStore::load_history` reads a command's objects and blobs through `read_many` (or the fewest transactions 0.4.0 allows), both providers.
- A benchmark case, not part of the default test run, measures propose, commit and head at 3 revision counts on both providers and records them; on the file provider, propose at revision 40 is at most 3 times revision 1.
- Every existing store, replay and durable-command case stays green.

## Scope

Derived 2026-09-25 by `story-scoper` (wave p1-15). Confidence: high for store, pin and pin-test surfaces; low for the benchmark location.

- eventlog 0.4.0 = `70096af8c231fedf6d2206c97ce2940b99aecdb8`; `read_many(&[Read]) -> Vec<ReadResult>` is a default `EventStore` method (`eventlog-core/src/lib.rs:1157`); the file provider batches it in one transaction (`eventlog-file/src/lib.rs:1383`), SQLite uses the per-item default — cited
- `crates/ekr-store/src/eventlog.rs:386` `load_history` (with `load_object` :373, `object_versioned` :315, `read_until` :238): one read per object stream plus one per blob today; target 3 transactions per command — cited
- `Cargo.toml:31-33` (the only eventlog rev declaration), `Cargo.lock` — cited
- `crates/ekr/tests/story_contract.rs:660` `QUALIFIED` requires rev `28e57856` verbatim — cited
- `crates/ekr-store/src/preparation.rs:217` `native_expected` matches `Expected` exhaustively; 0.4.0 makes it `#[non_exhaustive]` with `Merge` — inferred, not built
- benchmark home unset (`crates/ekr-kernel/tests` guessed) — inferred
- not addressed by `read_many`: `history_at`/`replay` read the revision stream one event per call (`read_until` limit 1)

## Coordinator decision: the bench bound

Decided 2026-09-25 by the coordinator after adversary pass 1 (`review-result:p1-15-eventlog-adversary-r1`): the acceptance "file-provider propose at revision 40 is at most 3 times revision 1" is unreachable on a quiet machine, because each command still opens the store and verifies its whole log once, a linear cost that dominates when the fixed cost is small (quiet run: 42 → 277 ms). What the story fixes is the quadratic term. The acceptance becomes: a history load costs a constant number of provider calls (held by a unit test), and the bench asserts linear growth: the revision 20 → 40 increase is at most twice the revision 1 → 20 increase, on both providers. The per-open verification is eventlog's (`story:incremental-history-digest-for-a-resumed-handle`, draft).
