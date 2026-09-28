---
format: aep.planning-md/3
id: story:store-provider-migration
kind: story
status: draft
title: A store moves between the file and SQLite providers with its identities and roots
relations:
- serves: vision:o2
- decomposes: epic:p7-migration-cutover
revision: 1
---
## Context

Asked by a consumer instance (2026-09-28): a store written with the file provider cannot be moved to
SQLite. The routes today are rebuilding from the consumer's own inputs or replaying the retained
transaction documents; both mint new commit times, so every revision root differs from the
original (`docs/cli.md` has no migrate, export or import verb; `crates/ekr/src/cli` has none).

## Build

A verb that copies an existing store to a new store on another provider (file to SQLite and back)
occurrence by occurrence and object by object, preserving event and revision identities, commit
times, retained objects and their storage classes, so the target replays to the same roots.
The target must not exist; the source is read only. The copy is admitted by replaying it through
the kernel authority like any reopen (invariant 1).

## Acceptance

- A file store with schema changes, retractions, supersessions and evidence copies to SQLite, and
  `ekr head` prints the same revision root on both.
- The same holds from SQLite to file.
- A target path that holds a store is refused by name; the source is unchanged byte for byte.
- A source whose history fails replay is refused and no target is left behind.
