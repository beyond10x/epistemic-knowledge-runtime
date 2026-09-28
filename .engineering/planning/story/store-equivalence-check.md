---
format: aep.planning-md/3
id: story:store-equivalence-check
kind: story
status: draft
title: Two stores can be compared for equal content, ignoring commit times
relations:
- serves: vision:o2
- decomposes: epic:p7-migration-cutover
revision: 1
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
