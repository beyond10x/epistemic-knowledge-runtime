---
format: aep.planning-md/3
id: story:v1-chat-raw-becomes-observations
kind: story
status: archived
title: v1 raw chat files become observations, twice-safe, reconciled against source counts
relations:
- serves: vision:o5
- decomposes: epic:p2-observation-layer
- depends_on: story:observations-are-retained
scope:
- confidence: inferred
  path: crates/ekr-import/src/lib.rs
- confidence: inferred
  path: crates/ekr/src/cli/import.rs
- confidence: inferred
  path: crates/ekr/tests/import.rs
- confidence: inferred
  path: docs/cli.md
revision: 7
transitions:
- {from: "draft", to: "archived", at: "2026-09-28T23:00:43Z", actor: "agent:claude-coordinator", revision: 7, decided_on: {"recorded":{"review_outcome":1}}}
---
## Context

Roadmap P2: "Raw importers: v1 `knowledge/raw/**/*.jsonl` and v2 `raw/` become observations", with
the exit "v1 and v2 raw imports reconcile against source file counts". The first consumer already
reads v1's raw chat cache (one JSONL file per channel and month) through its own harness; the
runtime has no importer.

This story covers v1 raw chat only. The v2 `raw/` import and its reconciliation, the other half
of that P2 bullet, are not claimed by this story; they stay open under `epic:p2-observation-layer`
and are listed as unclaimed in `release-plan:roadmap-2026-09-28`.

## Build

An `ekr import` path that reads v1 raw chat JSONL for declared channels and records each message
as an observation through the retained observation store (`story:observations-are-retained`),
with its checkpoint per channel. No interpretation: observations only. Redaction before any model
input stays the observation layer's (invariant 9).

## Acceptance

- Importing a fixture tree twice records each message once; the second run reports zero new.
- Counts reconcile: observations per channel and month equal the lines in the source files, less
  the lines the importer reports as skipped, each with its reason.
- Fixtures use the runtime's own vocabulary; no customer or personal data (AGENTS.md).
