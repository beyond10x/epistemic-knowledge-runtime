---
format: aep.planning-md/2
id: architecture-decision-record:0006-ekr-store-bridges-the-async-port
kind: architecture-decision-record
status: accepted
title: ADR 0006 — ekr-store owns the tokio runtime and exposes a synchronous log
relations:
- decides: story:eventlog-store
revision: 3
---
## Status

Proposed and accepted 2026-09-21, before units 1 and 2 of wave p1-05 dispatched. Taken by the
coordinator on the pre-dispatch read, for the same reason ADR 0005 was: it decides a surface no
story owns and both would otherwise decide in passing.

## What the read found

`eventlog-core`'s `EventStore` trait is **async**. Every method returns a `BoxFuture`
(`eventlog-core/src/lib.rs:43`, `:668-684`), and its own doc says why: "the callers are async
servers". `eventlog-sqlite` and `eventlog-file` both depend on `tokio` with the `rt` feature, and
both of the eventlog repository's own test suites drive them with `#[tokio::test]`.

**This workspace has no async runtime.** `Cargo.toml`'s `[workspace.dependencies]` lists no
`tokio`, no `futures`, no `pollster`, no `smol`. `ekr-store` declares `eventlog-core`,
`eventlog-file`, `eventlog-sqlite`, `serde`, `serde_json`, `thiserror` and dev-dependency
`tempfile`, and nothing that can drive a future.

`story:eventlog-store`'s Notes say "this story adds no dependency and does not touch `Cargo.lock`,
so it can run beside `story:transaction-and-validators`". **That is false**, and the parallel-safety
argument it supports has to be re-made rather than assumed. `pollster` or any bare executor is not
a way out: `eventlog-sqlite` wraps a synchronous `rusqlite` and needs a tokio runtime context, so a
future driven outside one panics rather than failing.

## Decision

**`ekr-store` owns a `tokio` current-thread runtime and exposes a synchronous `RevisionLog`.**

- `tokio` joins `[workspace.dependencies]` with the `rt` feature, and `ekr-store` takes it. Tests
  add `macros` where they need `#[tokio::test]`.
- `ekr-store`'s public API — `append`, `fold`, `head`, `replay` — is synchronous. The runtime lives
  inside the crate and no type above it is async.
- `ekr-kernel`, `ekr` and everything P1 builds later stay synchronous.

`Cargo.toml` and `Cargo.lock` are in `story:eventlog-store`'s scope for this change and this change
only. `story:transaction-and-validators` still touches neither, so the two units remain disjoint on
every file — but on a corrected reading rather than on the story's own wrong sentence.

## Why synchronous, given the port is async

The eventlog port is async because its intended callers are servers. This runtime's only P1 consumer
is a command-line program (`story:ekr-cli`), and async buys it nothing. Colouring `ekr-store`,
`ekr-kernel` and the CLI async to satisfy a dependency's calling convention is a large change with
no P1 benefit, and `AGENTS.md` invariant 7 wants the validators plain and rerunnable.

The minimal working path is a bridge in the one crate that touches the port. The larger change is
available later and is not foreclosed: making `RevisionLog` async is a mechanical widening of one
trait.

## The cost, stated rather than hidden

**A synchronous wrapper around an async port panics when it is called from inside a runtime.**
`Runtime::block_on` refuses to nest. Nothing in P1 does that — the CLI is the only caller — but the
first async consumer of `ekr-store` will meet it, and it will meet it as a panic rather than a type
error. Recorded as `task:ekr-store-block-on-cannot-nest`, so it is a known cost rather than a
discovery.

## What this does not settle

- Which runtime flavour beyond `rt`. Nothing here needs `rt-multi-thread`, `net`, `time` or
  `signal`; adding a feature later is not a decision this record has to take now.
- Tenancy. One tenant per store in P1, as `story:eventlog-store` already says.
- Whether `ekr-store`'s API should be async in P2 or later. It is a widening, and a later ADR
  supersedes this one if it is wanted.


## Amendment, 2026-09-21, after unit 1 reported

**The runtime was not the whole of the cost.** This record's decision bullet said "`tokio` joins
`[workspace.dependencies]` with the `rt` feature". Two more declarations are needed and the
pre-dispatch read missed both.

- **`time`.** `eventlog_core::CommandMeta.occurred_at` is a `time::OffsetDateTime`, and the struct
  has no constructor, no `Default` and no builder. `eventlog-core` re-exports nothing from `time`,
  so no consumer can build a command envelope without naming the crate. This is the same class of
  finding as the one that produced this record — a calling convention nobody checked — and it was
  found by the implementor rather than by the read that was supposed to catch it. It adds no package
  to `Cargo.lock`: `time 0.3.55` was already resolved transitively.
- **`ekr-ontology`.** `RevisionLog::fold` returns a `CanonicalGraph`, whose `ontology` field is an
  `ekr_ontology::Ontology`. The edge sits inside the acyclic order `docs/roadmap.md` § 3 gives —
  `ekr-core <- ekr-ontology <- ekr-graph <- ekr-store` — and adds no cycle.

`story:workspace-crate-skeleton`'s tables and `crates/ekr/tests/story_contract.rs`, which is the
tree's copy of them, move with this amendment.

The decision itself is unchanged: `ekr-store` owns the runtime and its `RevisionLog` is synchronous.
