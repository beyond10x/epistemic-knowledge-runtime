---
format: aep.planning-md/3
id: story:store-provider-migration
kind: story
status: draft
title: A store moves between the file and SQLite providers with its identities and roots
relations:
- serves: vision:o2
- decomposes: epic:p7-migration-cutover
revision: 2
---
## Context

The released CLI already provides preserving migration to the same provider
(crates/ekr/src/cli/migrate.rs, module documentation; docs/cli.md, ekr migrate).
The remaining outcome is a preserving move between the file and SQLite providers.
Extend the existing migration path and preserve its source-read-only, destination-empty,
kernel-replay and incomplete-migration guarantees. The earlier claim that no migrate verb
existed described an obsolete baseline; it is superseded by this source inspection.

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
