---
format: aep.planning-md/3
id: story:store-equivalence-check
kind: story
status: implemented
title: Two stores can be compared for equal content, ignoring commit times
relations:
- serves: vision:o2
- decomposes: epic:p7-migration-cutover
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T07:25:56Z", actor: "agent:claude-ekr-controller", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T07:25:56Z", actor: "agent:claude-ekr-controller", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-14T23:12:22Z", actor: "agent:claude-ekr-controller", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Context

Asked by a consumer instance (2026-09-28): after a rebuild or a provider move there is no way to
check that two stores hold the same knowledge. Root hashes differ whenever commit times differ,
so they cannot answer it.

## Build

A read verb that compares two stores (any providers) at their heads, or at given revisions, and
reports whether they hold the same content, ignoring commit times and transaction ids: the
ontology (types, properties by id), node and edge ids with their types, properties and aliases,
assertions (subject, predicate, object, evidence ids, valid time, assessment, lifecycle), and
evidence entries with their payload hashes. The output is one JSON document: equal or not, and
per category the counts on each side and the first differing ids.

## Acceptance

- Two stores built from the same seed and the same transaction documents at different times
  compare equal, on the same and on different providers.
- A store that differs by one assertion's object, one alias, one evidence payload hash or one
  extra node compares unequal and names that id.
- The verb opens both stores read-only and writes nothing.
