---
format: aep.planning-md/3
id: story:copy-free-stage
kind: story
status: draft
title: A stage begins without copying the store
relations:
- serves: vision:o5
revision: 1
---
## Outcome

A stage begins without copying the store: `ekr stage begin` records the stage and its base head,
and the stage's reads resolve the base from the store's own tenant and only its suffix from the
stage tenant, so a begin costs the same on an empty store as on a large one.

## Why

Every stage begin today copies the whole store into the stage's tenant (design § 107.2). The copy's
time and memory grow with the store, and on PostgreSQL a long copy is what turned a committed
begin into an unknown outcome and a stage left Begun. A run usually adds a small suffix to a large
store; the copy is the whole cost of staging.

## Acceptance

- `ekr stage begin` on a store with N revisions writes a number of events and objects that does
  not grow with N (counted, not timed).
- Every verb joined to a stage reads the base's state and the stage's commits together, as today;
  the existing stage suites pass unchanged on SQLite and PostgreSQL.
- `ekr stage publish` and `ekr stage abandon` keep their contract; abandon forgets only the
  suffix.
- The design amendment and the specification (`systems/ekr/domains/store.yaml`) say how a stage
  references its base before any code.
