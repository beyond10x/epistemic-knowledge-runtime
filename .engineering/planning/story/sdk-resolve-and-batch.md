---
format: aep.planning-md/3
id: story:sdk-resolve-and-batch
kind: story
status: implemented
title: The SDK resolves-or-creates through a cache and bisects a rejected batch
relations:
- depends_on: story:sdk-session-transport
- depends_on: story:sdk-typed-documents
- serves: vision:o5
- decomposes: epic:consumer-sdk
scope:
- confidence: inferred
  path: crates/ekr-sdk/src/batch.rs
- confidence: inferred
  path: crates/ekr-sdk/src/resolve.rs
- confidence: inferred
  path: crates/ekr-sdk/tests/batch_bisect.rs
- confidence: inferred
  path: crates/ekr-sdk/tests/resolve_cache.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T18:41:42Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-09-29T18:41:42Z", actor: "human:timo", revision: 8}
- {from: "active", to: "implemented", at: "2026-09-29T20:35:57Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

A consumer instance's resolve cache and batch bisection (~550 lines) took its apply of a
5,000-message run from 18,791 resolves to 821. They are generic to any store fed from references.

## Build

- `Resolver`, keyed by type id and sorted aliases:
  - a `ProposeNew` answer mints an id and queues a `CreateNode` carrying the aliases;
  - queued nodes are flushed before any resolve sharing an alias with them;
  - invalidation is per alias;
  - the cache is dropped when `head` shows a commit the SDK did not make (checked at each flush);
  - `Ambiguous` answers are returned as values.
- `Batcher`:
  - units are atomic dependency groups;
  - chunks respect the 10,000-operation and 8 MiB caps;
  - `Stale` retries under a newly minted transaction id;
  - a `Rejected` batch is bisected down to a single group;
  - a `BatchReport` lists every committed transaction with its id, and every rejected operation
    with its validator issues and the batch and group it came from. A consumer acts on the
    rejected operation, so a per-group-only report is not enough (consumer input, 2026-09-29).

## Surface (inferred)

`crates/ekr-sdk/src/{resolve.rs,batch.rs}`, `crates/ekr-sdk/tests/{resolve_cache.rs,batch_bisect.rs}`.

The cache key is affected by `decision-blocker:typed-reference-subtype-matching` (open).

## Acceptance

- Resolve requests equal the number of distinct references in a fixture with repeats.
- One planted invalid operation among 2,000 is the only rejection reported, named as that
  operation with its issues, its batch and its group; every other group is committed and listed
  with its transaction id.
- An alias committed by a second process between two resolves produces no duplicate node and no
  `alias-already-exists`.
- A forced `Stale` is committed on retry under a new id.
