---
format: aep.planning-md/1
id: epic:p2-observation-layer
kind: epic
status: draft
title: P2 — Observation layer and adapters
relations:
- decomposes: initiative:epistemic-knowledge-runtime
- depends_on: epic:p1-kernel-ontology-core
- serves: vision:o5
revision: 2
---
## Context

Design § 15–16, § 54–57. Sources must enter as immutable, content-addressed observations that are
safe to ingest twice, with health the operator can read. v2's incident of 2026-09-07 (replies missed
despite successful polls) sets the bar: a completed poll proves only its own window.

## Outcome

Crates `ekr-observe`, `ekr-adapters`, `ekr-import`:

- `Observation` with `ContentHash`, `SourceRef`, `captured_at`; `ObservationContent` including
  `Blob` (amendment 86, A12); `Evidence` (§ 16);
- `SourceAdapter` over Connectors; per-unit checkpoints (§ 55); idempotency at the observation and
  integration layers (§ 56); poll health with `checked_through` and
  `attempt | complete | partial | failed` (A8);
- credential redaction and privacy projection before any model input; PII and secret gates (A6);
- adapters for Slack, GitLab, Jira, Confluence and GitHub, declarations lifted from
  `org-brain-successor/adapters/*.yaml`;
- raw importers: v1 `knowledge/raw/**/*.jsonl` and v2 `raw/` become observations.

## Acceptance

The same Slack delta ingested twice produces zero new observations; a coverage report prints
declared denominators and the checked-through cutoff per unit; the v1 and v2 raw imports
reconcile against source file counts.

## Carries

A6, A8, A12.

## Before decomposition

Recorded in wave p1-14 (2026-09-23) from the plan review of 72779f9.

1. Model first: an `ekr.observe` ESS domain and component for SourceAdapter, SourceUnit,
   Checkpoint, PollHealth (`checked_through`, attempt/complete/partial/failed, A8), the idempotency
   key (§ 56), the redaction gate (A6), the coverage report and the Blob payload (§ 86).
2. Decide the Observation owner: `components.yaml` gives it to `ekr-graph`, roadmap § 3 to
   `ekr-observe`. Coordinator default: `ekr-observe`, with `ekr-graph` holding only the canonical
   reference.
3. Restate acceptance clause 3: raw v1/v2 imports reconcile on synthetic fixtures in the suite; the
   real reconcile is an operator step, because predecessor data holds personal data.
4. Lift source for adapters: the roadmap says v2 `adapters/*.yaml`, this epic says
   `org-brain-successor/adapters/*.yaml`. Coordinator default: the v2 path the roadmap names.
