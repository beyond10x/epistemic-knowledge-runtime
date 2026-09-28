---
format: aep.planning-md/3
id: story:seed-envelope-decoded-once
kind: story
status: active
title: The seed envelope is decoded once per process
relations:
- serves: vision:o5
- decomposes: epic:ingestion-throughput
scope:
- confidence: cited
  path: crates/ekr-kernel/src/checkpoint.rs
- confidence: cited
  path: crates/ekr-kernel/src/commit.rs
- confidence: cited
  path: crates/ekr-kernel/src/read.rs
- confidence: cited
  path: crates/ekr-kernel/src/replay.rs
- confidence: cited
  path: crates/ekr-kernel/src/seed.rs
- confidence: cited
  path: crates/ekr-kernel/tests/seed_envelope_once.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T09:14:10Z", actor: "agent:claude-coordinator", revision: 8}
- {from: "proposed", to: "active", at: "2026-09-28T09:14:10Z", actor: "agent:claude-coordinator", revision: 9}
---
## Context

`seed::envelope` (`crates/ekr-kernel/src/seed.rs`) decodes the retained seed envelope twice, first
as `serde_json::Value` to recognise a legacy envelope and then typed. On the store of
`epic:ingestion-throughput` the envelope is 39 MB, because evidence payloads are JSON integer
arrays. It is decoded again by checkpoint restore (`checkpoint.rs`), the verified read (`read.rs`)
and the commit authority (`commit.rs`), each re-hashing and re-parsing it.

## Build

- `seed::envelope` decodes typed first; the `Value` pass runs only after a failed typed decode, to
  name `SeedMigrationRequired` as today.
- The kernel authority keeps the decoded envelope per `seed_hash` for the life of the process,
  next to the payload keys `ReplayCache` already keeps (`crates/ekr-kernel/src/replay.rs`), and
  checkpoint restore, verified read and commit use it.
- A path that needs only the payload keys and the graph decodes payload values as
  `serde::de::IgnoredAny`.

## Acceptance

- Every existing refusal is unchanged: `seed-decode`, `SeedMigrationRequired`,
  `unsupported-seed-envelope`, `checkpoint-seed-payloads`, `checkpoint-graph-root-identity`.
- A counting test shows one full envelope decode per process across restore, read and commit.
- `crates/ekr-kernel/tests/seed.rs` and `replay_checkpoint.rs` pass on both providers.
- Measured: one-shot `ekr resolve` on the store of the epic, before and after.
