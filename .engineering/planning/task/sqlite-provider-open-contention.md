---
format: aep.planning-md/1
id: task:sqlite-provider-open-contention
kind: task
status: draft
title: Concurrent processes cannot open one SQLite store
relations:
- serves: vision:o2
- derived_from: epic:p4-operator-surface
revision: 1
---
## What is wrong

Concurrent `ekr` invocations against one SQLite store can fail to open the provider:
`ekr: opening the provider: the store is unavailable: event store is unavailable: database is
locked`, exit 1. Measured by wave p1-13's CLI adversary
(`crates/ekr/tests/adversary_p1_13_cli_exit_contract.rs::concurrent_exact_seed_and_commit_invocations_return_one_retained_result`):
six concurrent identical `seed` calls on SQLite failed at open in 8 of 8 standalone runs; the File
provider passed. The open path is `EventlogStore::sqlite` (`crates/ekr-store/src/eventlog.rs:73-79`).

Exit 1 is correct under the CLI exit contract, but an exact retry that happens to run concurrently
should return the one retained result, as it does on the File provider.

## What closes this

The SQLite provider waits for, or retries, a held write lock at open with a bounded timeout, so
concurrent exact Seed and Commit invocations each exit 0 with the one retained result. Then delete
the SQLite branch in that test's `assert_one_result`, which currently pins today's behaviour.

What reaches it: two processes on one SQLite store at once. No documented workflow does this yet
(inferred from the CLI and runbook sources); the P4 operator surface will.
