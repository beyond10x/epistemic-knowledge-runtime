---
format: aep.planning-md/3
id: story:stage-copy-bounded-locks
kind: story
status: implemented
title: A stage begin takes a bounded number of locks per transaction
relations:
- serves: vision:o5
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T21:34:48Z", actor: "agent:claude-ekr-controller", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-16T21:34:49Z", actor: "agent:claude-ekr-controller", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-17T03:09:01Z", actor: "agent:claude-ekr-controller", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

`ekr stage begin` (and `ekr migrate`) copies a store into a PostgreSQL destination with a bounded
number of locks per transaction, whatever the number of evidence payloads one seed or commit of the
store carries: a store whose largest occurrence carries more payloads than the server's lock table
holds is staged, on a server with `max_locks_per_transaction=64`.

## Why

Issue #93. The copy publishes each occurrence as one atomic group that carries every payload
(`crates/ekr-kernel/src/migrate.rs` `publish_into`); each payload is its own object stream
(`crates/ekr-store/src/eventlog.rs` `object_append`), and the PostgreSQL event store takes one
advisory transaction lock per distinct stream of a group. A seed of 8,192 payloads is refused
`stage-copy-failed` (SQLSTATE 53200) on a stock-sized lock table; 4,096 stages.

## Acceptance

- `crates/ekr-kernel/tests/stage_hosted_limits.rs`
  `a_store_with_more_seed_payloads_than_the_lock_table_is_staged` passes on a server with
  `max_locks_per_transaction=64` and `max_connections=20`, where it fails on 0.0.35.
- The staged store replays to the same head, roots and retained bytes as its source
  (`migrate-verification-disagrees` never fires on the existing stage and migrate suites, SQLite
  and PostgreSQL).
- A copy that fails part-way still leaves the stage Abandoned and its tenant empty, and the store
  unchanged (`stage_begin_copy_failure.rs` stays green).
- The number of streams in any one transaction of the copy is bounded by a constant the
  specification names, counted through the store's group calls, not timed.
- `docs/cli.md` names the PostgreSQL lock-table setting that bounds the original seed and commit
  paths, which this story does not change.
