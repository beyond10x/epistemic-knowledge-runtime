---
format: aep.planning-md/1
id: task:two-of-the-five-revision-sub-roots-are-placeholders
kind: task
status: implemented
title: ontology_root and agent_root have no type to hash and stay placeholders in P1
relations:
- serves: vision:o2
revision: 5
---
## What is wrong

`ekr_graph::Root` carries five content addresses (`crates/ekr-graph/src/root.rs:54-63`):
`ontology_root`, `knowledge_root`, `evidence_root`, `agent_root` and `transaction`. Wave p1-05's
unit 0 makes three of the five computable — graph state, evidence state and the transaction —
because it gives `Node`, `Edge`, `Assertion`, `Evidence`, `Observation` and `Support` a `Canonical`
implementation.

Two are not computable and will not be in P1:

- **`ontology_root`.** `ekr_ontology::Ontology` has no `Canonical` implementation. It cannot be
  written in `ekr-graph`: `Canonical` belongs to `ekr-core` and `Ontology` to `ekr-ontology`, so the
  orphan rule puts the implementation in `ekr-ontology`. It is a substantial surface — node types,
  edge types, property definitions, lifecycles, value types — and wave p1-05's unit 0 declined to
  widen into it on its last correction round with no review pass left to follow.
- **`agent_root`.** Design § 18 gives `Agent`, and no crate declares one. It is not in the P1
  roster.

## What this means for P1

`story:eventlog-store`'s acceptance — a reopened store folds to the same head `Root` hash — is
satisfied with both of those as fixed placeholder values. The property it tests, that a fold is
reproducible across a close and reopen, does not depend on which sub-roots are real. `docs/roadmap.md`
§ 4's P1 exit criterion that replay reproduces the root hash is the same.

Say so in the code rather than letting a reader assume five real addresses. A placeholder that looks
like a hash is the kind of thing a later wave builds on without noticing.

## What closes this

`Canonical for Ontology` in `ekr-ontology`, with cases, which makes `ontology_root` real. `agent_root`
closes when an `Agent` type exists, which is not P1.

Until then: both fields documented as placeholders, and a case asserting they are the placeholder
rather than something derived, so the day they become real is a day a test changes.

## Boundary correction, 2026-09-22

The approved completion plan supersedes the earlier claim that populated ontology content can remain a placeholder through P1. Implement canonical ontology addressing and persist the actual governing ontology before claiming replay. The agent root may represent an explicitly empty registry only while no such registry exists; it must bind any populated registry. The task now blocks the commit story so the stronger acceptance cannot be scheduled past this omission.
