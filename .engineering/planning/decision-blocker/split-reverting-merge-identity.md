---
format: aep.planning-md/3
id: decision-blocker:split-reverting-merge-identity
kind: decision-blocker
status: open
title: Nobody has decided whether a split that reverts a merge restores the absorbed node id or mints new nodes
relations:
- blocks: epic:p3-incubation-integration
revision: 2
---
## Question

The epic exit is "a wrong merge is reverted by a split with provenance intact". Design § 46 says a split "creates distinct entities, rewrites affected references, preserves historical provenance, flags uncertain assignments". When the split undoes a recorded merge, does the absorbed node come back under its original id, or does the split mint new nodes and link them to the old ids by lineage? Who assigns each edge and assertion to a product, and what does "flags uncertain assignments" record?

## The relation

`Split → produced Node`, one-to-many (at least two), `Split → source Node` it splits, exactly one (added 2026-09-27 by the wave p2p3p4-01 coordinator: design § 46 splits "a previously merged entity", so every split has one source node; pass 2 of the adversary on `story:integrate-domain-typed-reference` found no blocker naming it), and `Split → MergeLineage` it reverts, zero-or-one. Lifecycle coupling with the merge record is open. **requires-stakeholder-input**: no `ess/1` document declares a split or a merge lineage entity; no code implements either.

## Depends on

`decision-blocker:merge-absorbed-node-fate`: if the absorbed node is retained, restoring its id is possible; if it is rewritten away, it is not.

## What it stops

The split transaction and the exit fixture (wrong merge reverted with provenance intact). Not drafted.
