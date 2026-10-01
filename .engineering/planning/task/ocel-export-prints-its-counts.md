---
format: aep.planning-md/3
id: task:ocel-export-prints-its-counts
kind: task
status: draft
title: ekr ocel prints the counts of its export
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o5
revision: 1
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
