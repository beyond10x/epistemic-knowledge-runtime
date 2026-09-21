---
format: aep.planning-md/1
id: epic:p1-kernel-ontology-core
kind: epic
status: draft
title: P1 — Kernel, ontology, canonical core
relations:
- decomposes: initiative:epistemic-knowledge-runtime
- depends_on: epic:p0-bootstrap
revision: 1
---
## Context

Design § 8–21, § 34, § 70–72. Everything above the kernel depends on stable identity, typed values,
a validation membrane and an immutable revision lineage. Decision D1 (ADR 0001): the kernel is this
workspace's own code, persisted through eventlog.

## Outcome

Crates `ekr-kernel`, `ekr-ontology`, `ekr-graph`, `ekr-store` and the `ekr` binary:

- identity classes (§ 10), typed values (§ 11.3), node and edge types with cardinality (§ 11–12),
  per-type lifecycles and named operations (amendment 87, A13);
- assertions with valid time and transaction time (§ 13–14), `ValidationState` (§ 17), evidence
  and observation types (§ 15–16);
- `GraphTransaction` → deterministic validators 1–7 of § 20 → `ValidatedTransaction` → commit →
  revision root hash (§ 19, § 34); snapshot reads and optimistic concurrency (§ 71–72);
- eventlog-backed store with the SQLite and file providers; storage classes (§ 37); content
  addressing (§ 57);
- `ekr seed | propose | validate | commit | snapshot | explain`.

## Acceptance

Property tests show that no dangling reference can commit and that a `Canonical → Transient`
reference is unrepresentable at the type level; replay from the seed reproduces the root hash; the
§ 65 retraction example runs through the CLI.

## Carries

A13.

## Notes

ESS domains under `systems/ekr/` for each crate; `ess specify validate` joins `task check` with
the first. No crate in this epic names a domain concept (invariant 8).
