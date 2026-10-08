---
format: aep.planning-md/3
id: epic:p2-observation-layer
kind: epic
status: active
title: P2 — Observation layer and adapters
relations:
- decomposes: initiative:epistemic-knowledge-runtime
- depends_on: epic:p1-kernel-ontology-core
- serves: vision:o5
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T18:16:19Z", actor: "agent:claude-coordinator", revision: 3}
- {from: "proposed", to: "active", at: "2026-09-28T18:16:20Z", actor: "agent:claude-coordinator", revision: 4}
---
## Context

Design § 15–16, § 54–57. Sources must enter as immutable, content-addressed observations that are
safe to ingest twice, with health the operator can read. v2's incident of 2026-09-07 (replies missed
despite successful polls) sets the bar: a completed poll proves only its own window.

## Outcome

The engine owns the source-adapter trait, immutable content-addressed observations, per-unit
checkpoints, idempotency, poll health and coverage, and redaction before model input. Consumers
implement third-party connectors and own credentials and predecessor-specific raw importers.
A fixture adapter exercises the engine contract. This replaces the earlier bundled-adapter and
raw-import promises under accepted architecture-decision-record:0012-source-adapters-are-a-contract,
Decision and Consequences. Existing observation, checkpoint and retention decision blockers remain.

## Acceptance

A fixture adapter ingested twice produces no new observations. Coverage reports declare their
denominators and checked-through cutoff per unit. Poll results distinguish attempt, complete,
partial and failed; privacy projection precedes model input. The source-adapter contract and its
named conformance scenarios own the executable acceptance as its open decisions are settled.
Consumer connectors and predecessor raw-import reconciliation are outside this engine epic,
as decided by architecture-decision-record:0012-source-adapters-are-a-contract.

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
