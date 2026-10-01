---
format: aep.planning-md/3
id: task:read-only-open-passes-under-load
kind: task
status: draft
title: A read-only SQLite open beside a writer passes under load
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o5
revision: 1
---
## What is wrong

`crates/ekr-store/tests/adversary_read_only_open.rs`,
`a_sqlite_store_opened_read_only_while_a_writer_writes_holds_every_acknowledged_object`, failed once
in the full workspace run of the 0.0.25 gate (2026-10-01, load average 34–43): the read-only open
itself returned an error (`:51`, the `open().unwrap_or_else` panic). Run alone five times at the
same load it passed 5 of 5. The error text was not kept.

## Build

Reproduce under load (the full workspace run, or the test looped with parallel builds), keep the
error, and decide whether the read-only open of a SQLite store being written may fail transiently
(a lock or a WAL checkpoint) and should retry, or whether the test's expectation is wrong.

## Acceptance

- The error is named in this task; the case passes 50 runs in a row under load above 30, or the
  open retries a named transient error and the case says so.
