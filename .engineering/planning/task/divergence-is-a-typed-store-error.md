---
format: aep.planning-md/3
id: task:divergence-is-a-typed-store-error
kind: task
status: active
title: Store divergence reaches the CLI as a typed error, not a message
relations:
- serves: vision:o5
- decomposes: epic:p6-maintenance-observability
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T11:10:06Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-01T11:10:06Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":2}}}
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
- A store whose file provider detects divergence reaches the CLI as `StoreError::Diverged`, and the
  session, MCP and view hosts reopen on it, with every printed text unchanged.
- Out of this task, revised by the coordinator on 2026-10-01: a SQLite store overwritten in place
  under a live handle is `task:sqlite-store-replaced-in-place`; it needs a decision on what a live
  handle does and possibly an eventlog change, which this task's typed error does not supply.

## Surface

- **Files:** `crates/ekr-store/src/eventlog.rs` (where the provider's divergence condition is mapped
  once, to a typed error) and `crates/ekr/src/cli/session.rs:622` (the `DIVERGED` message match and
  the reopen path, also used by `view` and `mcp`) — found by grep for the message text, inferred
- **Collides with:** `story:extraction-verb-shares-the-sdk-path` (`session.rs`) and
  `story:preparation-blobs-are-reclaimed` (`eventlog.rs`) in wave extract-07; the coordinator merges
  them (`release-plan:next-waves-2026-09-30`)
