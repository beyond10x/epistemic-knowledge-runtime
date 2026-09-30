---
format: aep.planning-md/3
id: task:divergence-is-a-typed-store-error
kind: task
status: draft
title: Store divergence reaches the CLI as a typed error, not a message
relations:
- serves: vision:o5
- decomposes: epic:p1-kernel-ontology-core
revision: 2
---
## Context

Wave sdk-01 unit H (2026-09-29) reopens a file store when a read fails as diverged by matching the
provider's message text, `history diverged from this handle's observed history` (eventlog-file
`fe8a0a7`). A reworded message in a later eventlog release would silently turn the reopen off.
A SQLite database overwritten in place under the same inode (`cp` over the file) is not covered and
not measured.

## Build

A typed divergence error from `ekr-store` (mapping the provider's condition once, in the store crate)
that the CLI matches instead of message text; a case for a SQLite database overwritten in place.

## Acceptance

- No CLI code matches a provider message string for divergence (a source guard).
- A SQLite store overwritten in place is followed or refused by name, never answered from the old
  state.

## Surface

- **Files:** `crates/ekr-store/src/eventlog.rs` (where the provider's divergence condition is mapped
  once, to a typed error) and `crates/ekr/src/cli/session.rs:622` (the `DIVERGED` message match and
  the reopen path, also used by `view` and `mcp`) — found by grep for the message text, inferred
- **Collides with:** `story:extraction-verb-shares-the-sdk-path` (`session.rs`) and
  `story:preparation-blobs-are-reclaimed` (`eventlog.rs`) in wave extract-07; the coordinator merges
  them (`release-plan:next-waves-2026-09-30`)
