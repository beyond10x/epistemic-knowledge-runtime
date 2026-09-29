---
format: aep.planning-md/3
id: story:reads-share-verified-state
kind: story
status: implemented
title: Read verbs share the verified state instead of copying it
relations:
- serves: vision:o5
- decomposes: epic:ingestion-throughput
scope:
- confidence: cited
  path: crates/ekr-kernel/src/commands.rs
- confidence: cited
  path: crates/ekr-kernel/src/read.rs
- confidence: inferred
  path: crates/ekr-kernel/src/replay.rs
- confidence: cited
  path: crates/ekr-views/src/lib.rs
- confidence: cited
  path: crates/ekr/src/cli/resolve.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T07:27:54Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-09-29T07:27:54Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-09-29T11:07:35Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Context

Performance audit of 2026-09-29 (release build with frame pointers, `perf`; synthetic stores of a
consumer's shape: 1× = 4,127 nodes, 14.4k edges, ~67k assertions, 57 commits, 522 MB SQLite; 3×;
10×; load average 20–33, so CPU time and perf shares are the reliable figures).

`Runtime::read` builds a `VerifiedRead` with `Arc::unwrap_or_clone`, which always clones because the
replay cache holds a reference (`crates/ekr-kernel/src/read.rs:173`, `:178`). Every read verb copies
every transaction record (each proposal's document bytes twice, ~100 MB at 1×), the head graph and
the retained objects. `resolve` at 1×: 102 ms CPU, of which the lookup is 0.5%; copying the replay
state 29%, the graph 23%, loading history 25%, freeing 12.6%; ~47.6k page faults (~186 MB) per call.
Wall: 89–205 ms (1×), 192–324 ms (3×), 850 ms (10×). `ontology`, `snapshot`, `explain` and
`transactions` (`crates/ekr-kernel/src/commands.rs:219`) share the path. A consumer batch spends
about 10 s of ~15 s here.

## Build

- `VerifiedRead` shares the verified state (`Arc` views of the records, the head graph and the
  objects) instead of copying it; consumers in `ekr-views` and the CLI read through the shared view.
- A per-head index from (node type, alias) to node, built once per head and shared, so `resolve` is a
  lookup.

## Acceptance

- At the 1× shape, `resolve` inside `ekr session` costs at most 5 ms CPU median, on both providers
  (a measurement recorded with the command and the machine load).
- A counting test shows a read verb clones no transaction record and no graph.
- Every existing read answers byte-identically (the kernel and views suites, `docs_cli`).

## Scope (cited from the audit)

`crates/ekr-kernel/src/read.rs`, `crates/ekr-kernel/src/commands.rs`, `crates/ekr-kernel/src/replay.rs`
(the cache's shared state), `crates/ekr-views/src/lib.rs` (the `VerifiedRead` consumer),
`crates/ekr/src/cli/resolve.rs`.
