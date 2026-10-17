---
format: aep.planning-md/3
id: architecture-decision-record:0016-retype-keeps-node-identity
kind: architecture-decision-record
status: proposed
title: ADR 0016 — A node is retyped by its own operation and keeps its id
relations:
- decides: story:retype-node
revision: 2
---
## Status

Proposed on 2026-10-17 with `story:retype-node`. The operator decided on 2026-10-17 that a node
may be retyped by its own operation, after `MergeEntity`, to a conforming type or to an explicitly
named target type; this record holds that rule and the details below for acceptance. No
implementing wave starts on the story until it is accepted.

## Context

A node's type is fixed when the node is created. `story:merge-entity-applies` refuses a merge of
two nodes whose types are unrelated through `parents` (siblings included), and says the proposer
fixes them first "by an explicit retype or retraction". No retype operation exists: a node created
under the wrong type can only be retracted and recreated, which mints a new id and loses the
node's identity, against invariant 3 (names are not identities; ids are stable).

## Decision (proposed)

A `RetypeNode { node, to, reason }` operation, committed as an ordinary validated transaction
through the kernel.

1. **Conforming target.** `to` conforms to the node's current type or the current type conforms to
   `to` (related through `parents`): the retype commits when every property the node holds is
   declared by `to`; otherwise it is refused naming the undeclared properties, and the proposer
   retracts them first, as the merge rule does.
2. **Explicitly named target.** `to` is unrelated to the current type: the retype commits only when
   the transaction names `to` explicitly as an unrelated target (an `unrelated: true` flag) and the
   node holds no property `to` does not declare. Without the flag an unrelated target is refused.
3. **Identity.** The node keeps its id. Its type history is recorded in canonical state (the
   revision, the previous type, the actor and the reason), so a read at an earlier revision answers
   the type the node had then, and replay applies the retype identically.
4. **Edges.** Every edge the node takes part in is revalidated against the edge type's endpoint
   types in the retyping revision; an edge that no longer conforms refuses the retype, naming it,
   so the proposer retracts or replaces it first. No edge is rewritten or dropped silently.
5. **Absorbed nodes.** A node in the `Absorbed` state is not retyped; the survivor is.

## Options

- **A. A RetypeNode operation, id kept, refusals as above (proposed).** Cost: one more kernel
  operation, its replay rule and its views; edge revalidation in the kernel.
- **B. Retract and recreate.** No new operation. Cost: a new id for the same thing, and every
  reference and alias to the old id has to be moved by hand.
- **C. Retype as part of MergeEntity across unrelated types.** Cost: merges a type change into an
  identity change and contradicts the merge rule decided on 2026-10-14.

## Consequences

If A is accepted: `story:retype-node` builds it, spec first, after `story:merge-entity-applies`;
`docs/cli.md` gains the operation and its refusals; the merge story's refusal of unrelated types
then names `RetypeNode` as the fix.
