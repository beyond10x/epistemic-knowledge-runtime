---
format: aep.planning-md/3
id: task:sqlite-store-replaced-in-place
kind: task
status: implemented
title: A SQLite store replaced in place is not answered from its old state
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o2
- derived_from: task:divergence-is-a-typed-store-error
- informed_by: task:held-bytes-notice-deleted-blobs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T18:42:14Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-01T19:15:31Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-02T10:29:25Z", actor: "agent:codex-ekr-x7b", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
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

## Decision

Coordinator decision, 2026-10-01 (store audit), for this task and `task:held-bytes-notice-deleted-blobs`
together, to be written into `systems/ekr/domains/store.yaml` in one section:

- A live handle never answers from bytes a fresh handle would refuse.
- A store file replaced in place (its file identity or its log header no longer what the handle
  opened) is refused by name, `store-replaced`, and the session, MCP and view hosts reopen on it,
  as they do on `StoreError::Diverged`.
- Any operation that withdraws retained bytes (redaction, deletion, retention lowering) appends an
  event to the object's stream, so a holding handle sees it through the memo's stream check;
  nothing redacts in place without that event.
- The two pinned SQLite cases (`~/.cache/ekr-x7-g-keep/sqlite-overwritten-in-place-cases.patch`) are
  committed into the repository as ignored tests carrying this task's id before the fix starts.

The operator can override this; until then it is the rule the fix implements.
