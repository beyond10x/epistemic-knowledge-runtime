---
format: aep.planning-md/1
id: story:p1-exit-properties
kind: story
status: draft
title: 'P1 exit properties: no dangling reference commits; the membrane holds for every reference'
relations:
- decomposes: epic:p1-kernel-ontology-core
- serves: vision:o2
revision: 1
---
## Acceptance

Property tests, on both providers through the real kernel authority:

- no transaction whose operations reference an absent node, edge, assertion or evidence commits;
  the refusal is named and nothing is published;
- every canonical reference kind (`Subject::Edge`, `Assertion.evidence`, `CanonicalRef<Evidence>`)
  is typed so a transient identity cannot inhabit it, held by compile-fail cases (AGENTS.md
  invariant 2); today only node references are typed;
- replay from the seed reproduces the head root for generated lineages.

Then a `verification-report` maps each P1 exit clause in `docs/roadmap.md` to the case that
executes it.
