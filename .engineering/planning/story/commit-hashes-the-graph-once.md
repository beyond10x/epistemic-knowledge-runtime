---
format: aep.planning-md/3
id: story:commit-hashes-the-graph-once
kind: story
status: active
title: A commit hashes the graph once, as a stream
relations:
- serves: vision:o5
- decomposes: epic:ingestion-throughput
scope:
- confidence: cited
  path: crates/ekr-graph/src/lib.rs
- confidence: cited
  path: crates/ekr-kernel/src/apply.rs
- confidence: cited
  path: crates/ekr-kernel/src/commit.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T11:07:35Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-09-29T11:07:35Z", actor: "human:timo", revision: 5}
---
## Context

Performance audit of 2026-09-29 (see `story:reads-share-verified-state`). A commit grows from 0.48 s
(batch 1) to 2.8 s (batch 57) at 1×; 4.5 s at 3×; 11–25 s at 10×.

- The whole graph is hashed twice: `apply` computes `knowledge_root` in `commit_decision`, and again
  when `publish` verifies and replays (`crates/ekr-kernel/src/apply.rs:20`, `:177`); 35.8% of commit
  CPU at 1×, 31.1% at 3×. 12% of that is buffer reallocation from building the full encoding before
  hashing it.
- Fsync is not material: 18 per batch, 25 ms.

## Build

- The verify step reuses the kernel's own `apply` result for the candidate instead of applying and
  hashing it again; the root is still computed by the kernel (invariant 1).
- The encoding is hashed as a stream instead of being built in memory first.
- An incremental (Merkle) root changes the root format and is out of scope.

## Acceptance

- At the 1× shape, commit of a 1,561-operation batch at batch 57 takes at most 1.5 s median (measured,
  with the checkpoint cadence change of `task:checkpoint-cadence-by-size`, at most 1 s).
- Roots are byte-identical to today's for the same history (replay equality, the kernel suite).

## Scope (cited from the audit)

`crates/ekr-kernel/src/apply.rs`, `crates/ekr-kernel/src/commit.rs`, `crates/ekr-graph` (streaming
root hash).
