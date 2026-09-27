---
format: aep.planning-md/1
id: epic:p3-incubation-integration
kind: epic
status: draft
title: P3 — Incubation, interpretation, integration
relations:
- decomposes: initiative:epistemic-knowledge-runtime
- depends_on: epic:p2-observation-layer
- serves: vision:o5
revision: 1
---
## Context

Design § 22–29, § 44–46, § 52–53. Interpretation is separated from integration; what cannot be
represented canonically is parked, not discarded and not forced. Entity resolution is the
highest-risk semantic operation and is therefore an explicit transaction.

## Outcome

Crates `ekr-incubate`, `ekr-interpret`, `ekr-integrate`:

- transient roots with their own stable schemas (§ 22, § 53), `RootUtility` metadata (§ 40);
- the interpretation session contract (amendment 85, A5): immutable request bytes with digests, one
  disposition per signal including no-ops with a `basis`, receipts, a bounded number of visits;
- the resolver (A10): typed references, explicit merge and split transactions with lineage
  (§ 45–46), no string matching;
- integration plans with versioned, conditional mappings (§ 27–28, § 52); contradiction produces
  `Disputed` (§ 44), never a silent loser.

## Acceptance

The § 63 Slack-to-canonical fixture commits the proposal and, on the confirming message, the fact;
a parked root integrates after a mapping is added; a wrong merge is reverted by a split with
provenance intact.

## Carries

A5, A10.
