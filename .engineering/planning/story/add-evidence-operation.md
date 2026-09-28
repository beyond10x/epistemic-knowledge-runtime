---
format: aep.planning-md/3
id: story:add-evidence-operation
kind: story
status: draft
title: A transaction adds evidence after the seed
relations:
- serves: vision:o2
- decomposes: epic:p2-observation-layer
scope:
- confidence: inferred
  path: crates/ekr-kernel/src/transaction.rs
- confidence: inferred
  path: crates/ekr-kernel/src/validate/reference.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/add_evidence.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/domains/graph.yaml
- confidence: inferred
  path: systems/ekr/domains/kernel.yaml
revision: 7
---
## Context

Evidence enters canonical state only through the seed (`crates/ekr-kernel/src/validate/reference.rs`:
"P1 has no operation that introduces evidence"; README "not in 0.0.11"). A consumer that ingests
chat therefore seeds one evidence entry per corpus file, cites that file from every assertion, and
must reseed a new store for every ingest. `decision-blocker:evidence-entry-after-seed` holds the
choice of mechanism; this story is option 1 and waits for that decision.

## Build

- `ekr.kernel`: an `AddEvidence` operation in `ekr.transaction-document/2` that carries one
  `ekr.graph.Evidence` and its payload bytes (or the address of a retained payload), declared in
  `systems/ekr/domains/kernel.yaml` and `graph.yaml` with its refusals and scenarios.
- The validators check that the payload's content hash matches the evidence entry, that the id is
  new, and that an assertion in the same or a later transaction may cite it (provenance validator).
- Replay applies it; the payload is retained in the Provenance class.
- `docs/cli.md` documents the operation and its refusals; `ekr example` prints one.

## Acceptance

- A transaction with `AddEvidence` and an `AddAssertion` citing it validates and commits on both
  providers; full replay reproduces the root.
- A payload whose hash does not match, a reused evidence id, and an assertion citing evidence that
  no revision holds are refused by name.
- The kernel conformance suite gains the authored scenarios and passes on both providers.
