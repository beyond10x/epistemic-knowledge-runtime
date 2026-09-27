---
format: aep.planning-md/1
id: epic:p6-maintenance-observability
kind: epic
status: draft
title: P6 — Maintenance and observability
relations:
- decomposes: initiative:epistemic-knowledge-runtime
- depends_on: epic:p3-incubation-integration
- serves: vision:o6
revision: 1
---
## Context

Design § 35–43, § 60–61, § 75. Growth without maintenance is an invalid long-term operating mode
(§ 43). Forgetting is a first-class subsystem with three distinct forms: semantic removal, retention
removal, physical reclamation.

## Outcome

Crates `ekr-maintain`, `ekr-metrics`:

- retention by storage class (§ 37), consolidation (§ 41–42), decay scoring over `RootUtility`
  (§ 40), GC by reachability with quarantine and a grace period (§ 38–39);
- deletion requests through eventlog redaction with dependent assertions moved to `Disputed` or
  retracted (§ 60);
- epistemic health metrics (§ 61, § 75); lessons and gates as ordinary canonical nodes with
  recurrence counted (A9).

## Acceptance

The § 66 expiry and § 67 consolidation fixtures run; an erasure request leaves dependents
`Disputed` or retracted and the bytes gone; the metrics surface lists the § 61 counters.

## Carries

A9.

## Depends on

P3 only; runs in parallel with P5.
