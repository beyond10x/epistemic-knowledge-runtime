---
format: aep.planning-md/3
id: story:retained-bytes-shared-not-copied
kind: story
status: active
title: A request neither copies retained bytes nor reloads an admitted seed
relations:
- serves: vision:o5
- decomposes: epic:ingestion-throughput
- informed_by: task:session-request-cost-grows-with-evidence
scope:
- confidence: cited
  path: crates/ekr-kernel/src/commit.rs
- confidence: cited
  path: crates/ekr-kernel/src/seed.rs
- confidence: cited
  path: crates/ekr-store/src/eventlog.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T07:27:54Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-09-29T07:27:54Z", actor: "human:timo", revision: 4}
---
## Context

Performance audit of 2026-09-29 (see `story:reads-share-verified-state` for the method). This is the
root cause of `task:session-request-cost-grows-with-evidence`: `HeldObject::retained` does a
`Vec::clone` of every retained object on every request (`crates/ekr-store/src/eventlog.rs:133`), and
the seed envelope is always required (`eventlog.rs:1033`, `crates/ekr-kernel/src/seed.rs:44`). Copying
objects is 23% of `resolve` at 1× and comparing the envelope bytes another 7%. `resolve` costs
5 / 7 / 35 / 133 ms at 0.1 / 1 / 11 / 44 MB of seed evidence: about 2.9 ms per MB per request.

New seeds already write `ekr-seed-envelope/3` (0.0.16), which shrinks the envelope; stores seeded
before keep `/2` until `ekr migrate`.

## Build

- Retained bytes are held as `Arc<[u8]>` (or `Bytes`); equality checks take a pointer-equality fast
  path.
- Once this handle has admitted the seed, a request does not load the envelope and payload bytes
  again unless the read needs them (`explain`, `snapshot` of evidence).
- No format change.

## Acceptance

- Per-request `resolve` cost no longer grows with seed evidence: at 0.1 MB and 44 MB of evidence the
  medians differ by at most 5 ms, on both providers (measured on `/2` and `/3` stores).
- A counting test shows a request copies no retained object.
- The kernel suite passes on both providers.

## Scope (cited from the audit)

`crates/ekr-store/src/eventlog.rs`, `crates/ekr-kernel/src/seed.rs`, `crates/ekr-kernel/src/commit.rs`
(`required_objects`).
