---
format: aep.planning-md/3
id: epic:type-packs
kind: epic
status: draft
title: Shared type packs with a sensitivity class per property
relations:
- serves: vision:o5
- decomposes: initiative:epistemic-knowledge-runtime
revision: 1
---
## Outcome

Stores seeded from one shared base schema hold equal type and property ids, and a property carries a
sensitivity class the observation layer acts on, so consumers share types instead of each growing
their own, and redaction is decided once per class.

## Context

A consumer asked on 2026-09-29 for shared type packs (base schemas) with a per-property sensitivity
class; its operator decided on the consumer side that packs are authored as ESS domains, projected
into EKR ontologies and shipped as crates.

- Deterministic ids: pack types and properties get fixed ids (for example UUIDv5 of pack, type,
  property); pack versions only add, which fits evolve's no-removal rule. The generator can do this
  without a kernel change.
- Two first packs above `ekr-graph` (invariant 8), each exposing an `OntologySpec` for
  `Ontology::ensure`: an engineering pack (repositories, merge requests, pipelines, deployments,
  releases, incidents, services, tickets) and an organisation pack (people, teams, organisations,
  contact details, roles). The consumer's store names are input, not a specification.
- A sensitivity class per property (`pii.phone`, `pii.email`, `pii.name`, `pii.address`,
  `credential`), a detector per class shipped with the pack that owns it, and one enforcement point
  at the observation layer (invariant 9) taking a class → action policy (drop, mask, hash, keep).
- ESS will carry the class in its compiled IR (ess/19); EKR owns the generator from the IR to an
  ontology. Relations across packs wait for ESS's cross-system relation target.

## Blocked

`decision-blocker:property-sensitivity-home`: whether the class lives in the store's ontology (a
field on `PropertyDefinition`, versioned, a schema change to set it) or only in pack metadata.

## Acceptance

- Two stores seeded from one pack version hold byte-equal type and property ids.
- `Ontology::ensure` against a pack adds only what a store lacks.
- A property of class `pii.email` is handled by the observation layer per the caller's policy, with
  the detector's findings counted.
