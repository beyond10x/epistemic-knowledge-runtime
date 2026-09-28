---
format: aep.planning-md/3
id: story:extraction-document-applies-to-a-store
kind: story
status: draft
title: An extracting agent's document applies to a store through one verb
relations:
- serves: vision:o5
- decomposes: epic:p3-incubation-integration
- depends_on: story:add-evidence-operation
revision: 1
---
## Context

A consumer instance carries 1,686 lines that turn an extracting agent's output (named things and
facts, each citing one source item) into transactions: resolve before every `CreateNode`, attach
per-item evidence, batch, and grow the ontology (reported 2026-09-29). Any store fed by an
extracting agent needs the same path; design § 24 and § 63 place interpretation and integration in
the runtime.

## Build

A documented extraction document format (types and relations by name, references with aliases,
facts citing an evidence item) and a verb that applies it through propose/validate/commit only:
resolve each distinct reference once, create the missing nodes, add assertions citing their
evidence (needs `story:add-evidence-operation`), and report ambiguities and rejections. The verb
name must not collide with the `ekr import` planned for raw observations
(`story:v1-chat-raw-becomes-observations`); declared in ESS first.

## Acceptance

- A fixture extraction applies to a fresh store with the same nodes, edges and assertions the
  consumer's current path produces, on both providers.
- An ambiguous reference is reported and its fact skipped; nothing is written outside
  propose/validate/commit.
