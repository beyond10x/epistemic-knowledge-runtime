---
format: aep.planning-md/3
id: story:types-merge-and-split
kind: story
status: draft
title: A schema change merges two types or splits one
relations:
- serves: vision:o6
- decomposes: epic:p5-frontier-schema-scheduler
- depends_on: story:facts-migrate-across-schema-versions
revision: 1
---
## Context

A store improves by restructuring what it holds as well as by ingesting more (asked by a consumer
instance, 2026-09-28). Schema changes today can add a node or edge type and add or redeclare a
property (`story:schema-evolution-transactions`); they cannot merge two types, split one, or
remove one (README "not in 0.0.12"; design § 95 "a later milestone").

## Build

Schema transactions that merge two node types (or edge types) into one and split one into two,
each a new schema version, with the facts of the affected types carried to the new version by the
migration of `story:facts-migrate-across-schema-versions`. Declared in `ekr.ontology` and
`ekr.kernel` first.

## Acceptance

- Merging two types yields a schema version where the merged type holds every node of both, their
  history stays readable at the earlier versions, and replay reproduces the root, on both
  providers.
- A split assigns every node of the split type to exactly one new type by a rule the transaction
  states; a node the rule does not place is refused by name.
- Refusals for a merge across incompatible property kinds and a split that loses a property are
  named and tested.
