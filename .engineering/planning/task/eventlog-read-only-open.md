---
format: aep.planning-md/3
id: task:eventlog-read-only-open
kind: task
status: draft
title: The store opens read-only natively, without a private copy
relations:
- serves: vision:o5
revision: 1
---
## Context

Wave reads-04 made read verbs answer on a store this process may not write
(`task:read-verbs-open-a-read-only-store`). Neither eventlog provider at the pinned revision can open
without write access (`FileEventStore::open_existing` opens `writer.lock` for writing;
`SqliteEventStore` opens read-write), so `ekr-store` reads a private copy: the whole file store is
copied into TMPDIR per open, and a SQLite store is read into an in-memory image. Each open therefore
costs the store's size in time and space.

## Build

- In the eventlog repository: a read-only open for the file provider (a shared lock, no write to
  `writer.lock`) and the SQLite provider (`mode=ro`, no `-shm` created), then pin that revision here.
- `ekr-store` uses the native read-only opens and drops the copy and the image.

## Acceptance

- The read-only lanes (`read_only_store`, `adversary_read_only_store`, `read_only_open`) pass without
  a copy in TMPDIR.
- A read-only open of a 100 MB store costs no more time than a writable one.
