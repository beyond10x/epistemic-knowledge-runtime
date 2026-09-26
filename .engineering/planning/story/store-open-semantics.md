---
format: aep.planning-md/2
id: story:store-open-semantics
kind: story
status: implemented
title: Read verbs open only an existing store, and SQLite open waits for its lock
relations:
- serves: vision:o5
- derived_from: task:no-store-at-a-mistyped-path
- derived_from: task:sqlite-provider-open-contention
scope:
- confidence: inferred
  path: crates/ekr-kernel/src/commit.rs
- confidence: cited
  path: crates/ekr-kernel/src/runtime.rs
- confidence: cited
  path: crates/ekr-kernel/src/seed.rs
- confidence: cited
  path: crates/ekr-store/src/eventlog.rs
- confidence: inferred
  path: crates/ekr-store/tests/providers.rs
- confidence: cited
  path: crates/ekr/src/cli/mod.rs
- confidence: cited
  path: crates/ekr/src/cli/seed.rs
- confidence: inferred
  path: crates/ekr/src/conformance.rs
- confidence: cited
  path: crates/ekr/tests/adversary_p1_13_cli_exit_contract.rs
- confidence: inferred
  path: crates/ekr/tests/agent_cli.rs
revision: 16
---
## Context

Two draft tasks: `no-store-at-a-mistyped-path` (a read verb, or a refused seed, leaves a new store at the path it was given) and `sqlite-provider-open-contention` (six concurrent identical `seed` calls on SQLite failed at open with "database is locked" in 8 of 8 runs).

## Acceptance

- Read verbs open an existing provider store only and refuse a path with no store, creating nothing; a refused seed leaves no store behind. Cases on both providers.
- Opening a SQLite store waits for a held write lock with a bounded timeout, so concurrent exact Seed and Commit invocations each exit 0 with the one retained result; the SQLite branch in `adversary_p1_13_cli_exit_contract.rs` `assert_one_result` is removed.

## Scope

Derived 2026-09-25 by `story-scoper` (wave p1-15). Confidence: high for the files; medium on whether the SQLite lock fix stays inside EKR.

- `crates/ekr-store/src/eventlog.rs:97-119`: `EventlogStore::sqlite` calls `SqliteEventStore::open` (open-or-create); `EventlogStore::file` runs `create_dir_all` (:117) then `FileEventStore::open` — cited
- `crates/ekr-kernel/src/runtime.rs:105-137`: `Runtime::file` / `Runtime::sqlite`, the only constructors — cited
- `crates/ekr/src/cli/mod.rs:248-297,377-389`: `Store::open`; read verbs `snapshot`, `explain`, `head`, `transactions`, `ontology` — cited
- `crates/ekr/src/cli/seed.rs:13-27` opens the store before seed admission; `crates/ekr-kernel/src/seed.rs:198-204` refuses `seed-evidence-payload-mismatch` after open — cited
- `crates/ekr/tests/adversary_p1_13_cli_exit_contract.rs:403-418` `assert_one_result` SQLite branch — cited
- eventlog pinned rev `28e5785`: both providers already have `open_existing`; the SQLite provider sets no busy timeout, rusqlite defaults to 5000 ms — cited
- `crates/ekr-kernel/src/commit.rs:170`, `crates/ekr/src/conformance.rs:222-232`, new cases in `crates/ekr/tests/agent_cli.rs` or `crates/ekr-store/tests/providers.rs` — inferred
- not established: which statement returns "database is locked" despite the 5 s default (candidates: `PRAGMA journal_mode=WAL` on a new database, or a lock taken later in open/replay), so the fix may land in the eventlog provider

## Scope as landed (wave p5-01)

Confirmed by the implementor and the merge `51d4497`:

- the lock is `PRAGMA journal_mode=WAL` on open (eventlog-sqlite `70096af` `lib.rs:283`); rusqlite's busy timeout does not cover it. The fix stayed in EKR (`crates/ekr-store/src/eventlog.rs`, a bounded retry) — the "may land in the eventlog provider" line resolved to no
- `crates/ekr-kernel/src/commit.rs` and `seed.rs` were not changed; admission-before-create went into `crates/ekr-kernel/src/runtime.rs` (`admit_seed`, `check_anchor`) — two inferred lines were wrong
- `crates/ekr/tests/agent_cli.rs` was not needed; the CLI cases are in `crates/ekr/tests/store_open.rs` (new) — inferred line wrong
- `crates/ekr/src/conformance.rs` admits a seed as the CLI does and still opens unseeded stores, documented — cited
