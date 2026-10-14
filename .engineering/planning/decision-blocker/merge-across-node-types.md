---
format: aep.planning-md/3
id: decision-blocker:merge-across-node-types
kind: decision-blocker
status: cleared
title: Nobody has decided whether a merge may join two nodes of different types, and which type survives
relations:
- blocks: epic:p3-incubation-integration
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-14T12:20:57Z", actor: "agent:claude-ekr-controller", revision: 3}
---
## Question

The defect behind the resolver slice is one real-world entity held as two nodes of different types. May `MergeEntity` join nodes whose `type_id` differ? If so, does `into` keep its type, must the two types be related through `parents`, and what happens to properties the surviving type does not declare?

## The relation

`EntityMerge → NodeType` of each side, two many-to-one references; whether they must be equal (or conform) is open. **requires-stakeholder-input**: `systems/ekr/domains/graph.yaml` `ekr.graph.Node` relation `type` fixes one type per node; `systems/ekr/domains/kernel.yaml` `ekr.kernel.EntityMerge` says nothing about types. The kernel today checks only that both nodes exist and differ (inferred from `crates/ekr-kernel/src/validate/reference.rs:158-161` and `validate/structural.rs:283-296`).

## What it stops

Validation rules for `MergeEntity` and any repair of the existing cross-type duplicates. Not drafted.

## Answer

Decided 2026-10-14 by the operator: a merge may join two nodes whose types are related through `parents` (one conforms to the other); `into`'s type survives; the merge is refused while `absorbed` holds a property the surviving type does not declare, so the proposer retracts it first. Unrelated types are fixed first by an explicit retype or retraction. This is option B, recorded in the merge and split architecture decision record.
