---
format: aep.planning-md/3
id: story:seed-if-absent
kind: story
status: implemented
title: A seed can be written only if the lineage has none
relations:
- serves: vision:o5
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T18:39:06Z", actor: "agent:claude", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-05T18:39:06Z", actor: "agent:claude", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-06T06:08:55Z", actor: "agent:claude", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

A caller can seed a store only if the lineage has no seed: when another caller seeded it first, `ekr seed` refuses with `AlreadySeeded`, even if the two seeds are identical.

## Why

cortex `story:store-backend-per-instance` (wave 20261005c) creates PostgreSQL-backed instances. Re-seeding an identical seed exits 0 with the first seed's result (`commit.rs`: `retained_seed` before `now()`). So two hosts creating the same tenant at once can both believe they created it. cortex narrows this by refusing when the result's `committed_at` predates its own call. Two windows stay open: a seed committed by another caller during this call, and clock skew between hosts. Only a conditional seed in EKR closes them. Found by the cortex adversary, `review-result:adversary-store-backend-per-instance-pass-2` (cortex store), finding A1.

## Work

An `ekr seed --if-absent` mode (or an equivalent SDK option). It refuses with the existing `AlreadySeeded` refusal whenever a seed already exists, decided inside the transaction that would write the seed, on both providers.

## Acceptance

Two concurrent `ekr seed --if-absent` calls with an identical seed on one PostgreSQL tenant: exactly one exits 0, and the other exits non-zero with `AlreadySeeded`. On SQLite, a second call after the first refuses the same way.
