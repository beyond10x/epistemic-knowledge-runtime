---
format: aep.planning-md/3
id: task:resolver-queue-drops-shared-alias-answers
kind: task
status: implemented
title: Queuing a node drops cached answers that share its aliases
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o5
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T18:15:38Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-09-30T18:15:39Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-01T18:09:54Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## What is wrong

A consumer's adversary found this on 2026-09-30, on synthetic data, with ekr-sdk 0.0.24. The
`Resolver` does not drop a cached answer when it queues a new node of the same type that shares an
alias with that answer. A later resolve can then be answered `Resolved(M)` from the cache while
`ekr resolve` against the store would answer `Ambiguous` (M and the queued node N). The consumer
then asserts a fact about the wrong node, and nothing reports it.

Where: `crates/ekr-sdk/src/resolve.rs:130-171` (`resolve_with`). A `ProposeNew` answer inserts its
own key and queues the node (`:159-166`), and removes no other key. Only `observe` (`:183-205`)
invalidates by alias, and only for committed operations.

Repro, confirmed by the consumer (its adversary test
`a_cached_answer_is_dropped_when_a_node_sharing_its_alias_is_created`, which drives the consumer's own
apply; the four steps are what the SDK test lifts):

1. resolve `["X"]` → `Resolved(M)`;
2. resolve `["Ada","X"]` → `Resolved(M)`, cached under `{Ada, X}`;
3. resolve `["Ada"]` → `ProposeNew`, N is queued;
4. resolve `["Ada","X"]` → `Resolved(M)` from the cache, where the store answers `Ambiguous`.

## Build

When `resolve_with` queues a node, it drops every cached key of the same type id that shares an
alias with the queued node, other than the queued node's own key. This is the same rule `observe`
applies to a committed `CreateNode`.

## Acceptance

- A failing SDK test of the four steps passes: after N is queued, resolving `["Ada","X"]` answers
  `Ambiguous` with M and N, the answer `ekr resolve` gives against the flushed store.
- A cached key of another type, or one sharing no alias with N, is still answered from the cache
  (no extra `ekr resolve` call, counted by the test transport).
- The SDK's existing resolver tests pass.
