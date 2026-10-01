---
format: aep.planning-md/3
id: task:sqlite-store-replaced-in-place
kind: task
status: draft
title: A SQLite store replaced in place is not answered from its old state
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o2
- derived_from: task:divergence-is-a-typed-store-error
revision: 2
---
## What is wrong

Wave extract-07 unit G (2026-10-01) made divergence a typed store error (`StoreError::Diverged`),
mapped from the file provider's message. A SQLite store overwritten in place under a live handle is
still answered from the old state, at the store and through `ekr session`: the eventlog SQLite
provider has no divergence check, and the live connection keeps serving its cached pages. Nothing
simple tells an overwrite from a normal checkpoint, and dropping the connection writes its WAL into
the new file. The unit's two red cases are kept outside the repository as
`~/.cache/ekr-x7-g-keep/sqlite-overwritten-in-place-cases.patch`.

Who reaches it: an operator who replaces a store file while a session, MCP or view host holds it,
for example by restoring a backup over it.

## Build

Decide in the ESS store domain what a live SQLite handle does when its file is replaced in place:
refuse by name, follow the new file, or document the operation as unsupported (with `ekr` refusing
to open a store whose file identity changed). Implement it, with the provider change in
`beyond10x/eventlog` if one is needed.

## Acceptance

- The two pinned cases pass, or are replaced by cases asserting the decided behaviour.
- No live handle answers from a SQLite file that is no longer at its path.
