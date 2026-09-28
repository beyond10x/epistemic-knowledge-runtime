---
format: aep.planning-md/3
id: story:seed-envelope-v3-references-payloads
kind: story
status: draft
title: The seed envelope names evidence payloads by hash instead of embedding them
relations:
- serves: vision:o5
- decomposes: epic:ingestion-throughput
revision: 1
---
## Context

The seed envelope embeds every evidence payload as a JSON integer array, so 11 MB of evidence
becomes a 39 MB envelope, and a 66.5 MB publication-preparation blob carries it again (store of
`epic:ingestion-throughput`). The payloads are also retained as their own content-addressed blobs.

## Build

`ekr-seed-envelope/3`: the envelope names each payload by its content hash instead of embedding
it; replay reads the payload blobs it names. A migration verb rewrites an `/2` store's envelope.
This is a persisted-format change: its ESS declaration, the migration, and the replay of both
formats are part of the story.

## Acceptance

To be written with the ESS change; drafted here so the format change is not lost.
