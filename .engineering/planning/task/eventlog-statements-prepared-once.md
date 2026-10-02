---
format: aep.planning-md/3
id: task:eventlog-statements-prepared-once
kind: task
status: draft
title: The SQLite event log prepares each read statement once
relations:
- decomposes: epic:read-and-storage-cost
- serves: vision:o5
revision: 1
---
## What is wrong

The 2026-09-30 frame-pointer profile of a consumer's 80,000-fact delta
(`story:commit-cost-flat-with-store-size` § Probe result) put 49.8% of the `ekr session` process in
`sqlite3LockAndPrepare`: `eventlog-sqlite`'s `read_stream` (`crates/eventlog-sqlite/src/lib.rs:1869`,
eventlog at `fe8a0a7e`) calls `prepare`, not `prepare_cached`, so every call parses the same SELECT
again (`sqlite3RunParser` 48.0%, `sqlite3Reprepare` 25.7%). The same holds at `:2605` and `:2713`.
Unit C of wave extract-06 cut the number of calls; each call still pays the parse.

## Build

In `beyond10x/eventlog`: the SQLite provider's per-call statements use `prepare_cached` (or a
statement held per connection), with eventlog's own conformance unchanged. Then EKR moves its
eventlog pin to that release.

## Acceptance

- The eventlog change ships in an eventlog release; EKR's `Cargo.toml` pins it and EKR's gate passes.
- `crates/ekr-sdk/tests/commit_scaling.rs` on SQLite at full size reports its per-transaction times
  before and after the pin, in this task.

## Not established

- Why SQLite re-prepares inside one step (`sqlite3Reprepare` 25.7%); a schema change between
  statements would cause it.
