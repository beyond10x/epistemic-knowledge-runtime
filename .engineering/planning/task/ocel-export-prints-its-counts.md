---
format: aep.planning-md/3
id: task:ocel-export-prints-its-counts
kind: task
status: active
title: ekr ocel prints the counts of its export
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o5
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T10:34:31Z", actor: "agent:codex-ekr-x7b", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-02T10:34:31Z", actor: "agent:codex-ekr-x7b", revision: 4}
---
## What is wrong

`ekr ocel` prints only the `ekr.ocel/1` document. A consumer (2026-10-01) recounts the export's
events, objects and relationships itself. Those counts are what `ekr.views.OcelExported` already
carries (`crates/ekr-views/src/ocel.rs`).

## Build

The `OcelExported` counts appear in the document's `meta`, in the format its spec names, or on
stderr. The spec says which, and the one-shot and session lanes agree.

## Acceptance

- The counts printed equal `ekr_views::export_ocel`'s `OcelExported` for each OCEL fixture.

## Resume scope (2026-10-02)

Read-only story-scoper inspected main 4832d892. Primary paths, cited unless explicitly new:

- `crates/ekr/src/cli/ocel.rs`
- `systems/ekr/domains/views.yaml`
- `docs/cli.md`

The consumer group shares views.yaml, CLI dispatch, typed SDK exports and documentation;
its artifacts are implemented serially in one managed consumer unit. Generated conformance
suites and planning writes belong to the coordinator. New SDK modules are inferred.

## Resume decision

Use the existing OcelExported summary on stderr, preserving the export document's bytes.
The session reply carries the same counts in its per-request stderr field; a successful
session request must not leak counts to process-global stderr. SDK OCEL reads expose both.
The named-time task preserves default stdout relative to the released 0.0.26 document.
