---
format: aep.planning-md/3
id: task:projection-carries-per-type-property-definitions
kind: task
status: implemented
title: The graph projection carries each type's own property definitions
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T18:15:39Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-09-30T18:15:41Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-01T18:09:55Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Context

Found by the adversary on unit R of wave p2p3p4-02
(`review-result:adversary-p2p3p4-02-views-pass-1`, F1). `ekr-ontology` admits a child type that
redeclares an inherited property with another value kind
(`crates/ekr-ontology/tests/inheritance_and_declaration_coherence.rs:40`, cited by the adversary).
`ekr.graph-projection/1` (`systems/ekr/domains/views.yaml`) carries one `ProjectedProperty` list
keyed by property id, so the format cannot hold two definitions of one id. Until this task lands,
the renderer refuses such a revision with `ProjectError::Inconsistent` naming the id.

No shipped seed or example redeclares a property down a hierarchy (adversary report, § 4).

## Acceptance

A revision whose ontology has a child type redeclaring an inherited property renders, and the
projection gives each declaring type its own definition of that property, with the conformance
suite resynthesised.

## Also closes

Until this lands, `render` refuses a revision whose types disagree on a property's name or value kind with `ProjectError::Inconsistent`, an outcome `ekr.views` `ProjectGraph` does not declare (`systems/ekr/domains/views.yaml:388`, `review-result:adversary-p2p3p4-02-views-pass-2` F3). Per-type definitions remove the refusal.
