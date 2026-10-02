---
format: aep.planning-md/3
id: task:replaced-store-close-keeps-the-restored-file
kind: task
status: draft
title: Closing a handle on a replaced SQLite store does not write its old log into the new file
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o2
- derived_from: task:sqlite-store-replaced-in-place
revision: 1
---
## What is wrong

Wave correct-07 unit S (2026-10-02, `a704f2af`) refuses a SQLite store replaced in place under a
live handle as `store-replaced`, and the hosts reopen. Its implementor observed, by probe and not yet
by a committed test, that when the replaced handle's connection closes, SQLite checkpoints the old
write-ahead log into the file now at the path: the old history comes back into the restored copy.
An operator who restores a backup over a store while a session, MCP or view host holds it can have
the backup overwritten by the old log when that host reopens or exits.

## Build

In `beyond10x/eventlog`: the SQLite provider can close a connection without checkpointing its WAL
(SQLite's `SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE`), and EKR closes a handle it found replaced that way.
EKR moves its eventlog pin to that release.

## Acceptance

- A test copies a database over a live store, lets the host refuse and reopen, closes it, and
  reads the restored file: it holds exactly the copied history, on the pinned eventlog.
- A store that was not replaced still checkpoints on close as today.
