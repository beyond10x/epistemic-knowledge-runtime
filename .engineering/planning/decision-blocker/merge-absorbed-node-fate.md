---
format: aep.planning-md/3
id: decision-blocker:merge-absorbed-node-fate
kind: decision-blocker
status: cleared
title: Nobody has decided whether a merged-away node stays in canonical state, or whether a merge rewrites the references to it
relations:
- blocks: epic:p3-incubation-integration
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-14T12:20:57Z", actor: "agent:claude-ekr-controller", revision: 3}
---
## Question

When `MergeEntity { absorbed, into }` commits, what happens to `absorbed`: does it stay in canonical state as a retained record pointing at `into`, and are the edges and assertions that name it rewritten to `into` in the merging revision or resolved through the pointer at read time? Can a node that absorbed another later be absorbed itself (a chain)?

## The relation

Absorbed `Node → surviving Node`, many-to-one (one node may absorb several). Ownership and lifecycle coupling are both open: which outlives which, and what the merge does to everything that references the absorbed id. **requires-stakeholder-input**: `systems/ekr/domains/kernel.yaml` `ekr.kernel.EntityMerge` is a struct of `absorbed` and `into` with no relation; `systems/ekr/domains/graph.yaml` `ekr.graph.Node` has lifecycle states `[Present]` only, so no absorbed state exists. Design § 46 says "preserve alias and historical lineage rather than rewriting identity blindly" and does not say which. A code comment says a merge "leaves an alias behind" (inferred from `crates/ekr-kernel/src/validate/structural.rs:283-296`, the self-merge refusal); nothing implements it.

## What it stops

Applying `MergeEntity` (today refused as `unsupported-operation`), the merge lineage record, and reading a node through its merge. Not drafted.

## Related

`decision-blocker:split-reverting-merge-identity` depends on this answer.

## Answer

Decided 2026-10-14 by the operator: after `MergeEntity { absorbed, into }`, `absorbed` stays in canonical state in a new `Absorbed` lifecycle state pointing at `into`, and every edge and assertion naming it is rewritten to `into` in the merging revision; a later merge repoints earlier absorbed nodes to the new survivor, so readers never follow chains. This is option A; one architecture decision record carries it together with merge-across-node-types and split-reverting-merge-identity.
