---
format: aep.planning-md/3
id: architecture-decision-record:0018-merge-and-split-identity
kind: architecture-decision-record
status: accepted
title: ADR 0018 — A merge is a supersession whose references are rewritten in place, with the lineage a split needs
relations:
- decides: story:merge-entity-applies
revision: 3
transitions:
- {from: "proposed", to: "accepted", at: "2026-10-19T14:19:13Z", actor: "agent:claude-ekr-session", revision: 3}
---
## Status

Accepted by the operator on 2026-10-19 with option A and the default of each of the six open
questions below. Proposed the same day with `story:merge-entity-applies`. It records the three rules
the operator decided on 2026-10-14 (`decision-blocker:merge-absorbed-node-fate`,
`decision-blocker:merge-across-node-types`, `decision-blocker:split-reverting-merge-identity`) and
builds them on `architecture-decision-record:0017-a-node-is-superseded-by-another` (accepted).
`story:merge-entity-applies` builds it, spec first, after `story:supersede-node`. Drafted read-only
at `5b358cfaa`, while `story:supersede-node` was being implemented; the record field and refusal
names below are aligned with that story's once it merges.

## Context

`MergeEntity { absorbed, into }` parses. A structurally sound one whose nodes both exist is refused
as `unsupported-operation` ("MergeEntity is not supported in P1",
`crates/ekr-kernel/src/validate/structural.rs:1108-1121`), and apply refuses it again
(`crates/ekr-kernel/src/apply.rs:486`). The only merge rule that exists is `merge-into-itself`
(`structural.rs:110-111`, `:947-958`). The reference validator checks that both nodes exist
(`crates/ekr-kernel/src/validate/reference.rs:171-174`). The payload is two ids, encoded in order
(`crates/ekr-kernel/src/transaction.rs:132-137`, `:852-859`; `systems/ekr/domains/kernel.yaml:632-638`).
`ekr.graph.Node` has one lifecycle state, `Present` (`systems/ekr/domains/graph.yaml:472-475`).

ADR 0017 gives a superseded node a record: the node whole, the edges retired with it, `by`, the
revision, the reason and the evidence. The node leaves the head's node map, records are repointed
so there is no chain, and aliases resolve through the record. Its last paragraph says a merge is a
supersession whose edges and assertions are rewritten to `into` instead of retired.

Three facts limit how the rewrite can be done:

- Retained rejections replay to identical issues. The base-era fixture keeps a rejected proposal
  carrying a `MergeEntity` (`crates/ekr-kernel/tests/base_era_v1_replay.rs:229-231`). Replay checks
  issue count, code and message (`crates/ekr-kernel/src/replay.rs:1187-1211`), and a store never
  changes profile (`crates/ekr-kernel/src/authority.rs:33-39`).
- An assertion's assessment and lifecycle point at other assertions by id: `Disputed {
  competing_assertions }` and `Superseded { by }` (`crates/ekr-graph/src/assertion.rs:447-451`,
  `:534-555`). `SupersedeAssertion` closes the valid time at `effective_from` (`apply.rs:480`), which
  claims the world changed.
- Edge cardinality is `One` or `Many` per source and edge type (`crates/ekr-ontology/src/value.rs:30-36`;
  `crates/ekr-kernel/src/validate/cardinality.rs:107-127`). No validator refuses two parallel edges.
  Endpoints are checked by `conforms_to` (`crates/ekr-kernel/src/validate/types.rs:511-540`,
  `crates/ekr-ontology/src/schema.rs:430`), so an edge end that admits only the child type stops
  conforming once it points at a parent-type node.

## Decision (proposed)

1. **The absorbed node's record is a supersession record.** A committed merge does what
   `SupersedeNode { node: absorbed, by: into }` does: the node leaves the head's node map, the record
   is created, earlier records whose `by` is `absorbed` are repointed to `into`, aliases resolve
   through the record, and alias holders keep counting it. Reads of `absorbed` at the head refuse as
   `node-superseded`, naming `into`. No `Absorbed` lifecycle state is added to `ekr.graph.Node`; a
   node "in the Absorbed state" (ADR 0016 decision 5) is a node held in a record that carries merge
   lineage.
2. **Cleared rule: the absorbed node's fate** (merge-absorbed-node-fate, option A). `absorbed` stays
   in canonical state pointing at `into`. Every edge and assertion naming it is rewritten to `into`
   in the merging revision. A later merge of the survivor repoints earlier absorbed nodes to the new
   survivor, so a reader never follows a chain.
3. **Cleared rule: merges across types** (merge-across-node-types, option B). A merge may join two
   nodes whose types are related through `parents` (`Ontology::conforms_to`). `into`'s type
   survives. The merge is refused while `absorbed` holds a property the surviving type does not
   declare, and the proposer clears it first. Unrelated types, siblings included, are refused
   (`merge-types-unrelated`); the refusal names `SupersedeNode` as the fix available today, and
   `RetypeNode` if ADR 0016 is accepted.
4. **Cleared rule: a split that reverts a merge** (split-reverting-merge-identity, option C). A split
   that reverts a recorded merge restores the absorbed node's original id; any other split mints new
   nodes. The proposer assigns every edge and assertion explicitly, and an assignment it marks
   uncertain is recorded as flagged on the split record. A split has exactly one source node and
   reverts zero or one merge lineage.
5. **The record holds merge lineage.** A merge adds a `merge` field to the record, encoded only when
   present, so a supersession record without one hashes as ADR 0017 wrote it. It holds: (a) each
   rewritten edge, by id, with the end or ends that named `absorbed`; (b) each collapsed edge, whole,
   with the id of the edge it collapsed onto; (c) each edge joining `absorbed` and `into`, retired
   whole as ADR 0017 retires edges; (d) each rewritten assertion, by id, with the field (`subject` or
   `object`) and the reference it held before; (e) each property moved from `absorbed` to `into`. The
   node itself is already in the record. Restoring the id is the split's own operation, not a
   `CreateNode`, so `identity-previously-held` (`structural.rs:25-30`) does not apply to it.
6. **Shape and refusals.** `EntityMerge` gains `reason` (required, non-empty) and `evidence` (at
   least one entry held or added), as ADR 0017 decision 7 requires of any retirement, both encoded
   after `into` only when present. The bare `{absorbed, into}` shape keeps P1's verdict and message
   (`unsupported-operation`, "MergeEntity is not supported in P1") under every profile, and its
   bytes do not change, so retained rejections replay; this follows the ownerless `ModifyProperty`
   P1 shape (`kernel.yaml:640-642`, `structural.rs:130-131`). The new shape is refused by name as
   `merge-into-itself`; an unknown node as `unresolved-node` (`reference.rs:171-174`); `absorbed` or
   `into` already superseded as `node-superseded` naming its `by`; `merge-types-unrelated`;
   `merge-undeclared-property` and `merge-property-conflict`, naming the properties;
   `merge-duplicate-edge-conflict`; `edge-endpoint-type` and `edge-cardinality`, the existing codes,
   naming the edges; another operation of the transaction naming `absorbed` or one of its edges, as
   ADR 0017 decision 7 refuses it. Preparation (clearing a property with an empty `UpdateProperty`,
   `apply.rs:408-419`, or deleting an edge) goes in an earlier revision.
7. **Edges keep their ids and are rewritten in place**, endpoint by endpoint, in this order: (a) an
   edge joining `absorbed` and `into` is retired into the record; (b) an edge from `absorbed` with
   the same type, direction and other end as one `into` holds collapses onto it when their
   properties are equal (retired into the record, mapped to the kept edge, assertions about it
   rewritten to the kept edge) and refuses the merge as `merge-duplicate-edge-conflict` when they
   differ; (c) each remaining rewritten edge is revalidated against its edge type's endpoint types;
   (d) outgoing counts are taken after the collapse, and an edge type of cardinality `One` that
   `into` would hold twice refuses the merge, naming both edges.
8. **Assertions are rewritten in place, not replaced by new ids.** Each `Subject::Node(absorbed)` and
   `Object::Node(absorbed)` becomes `into` in the merging revision; id, evidence, proposer,
   assessment, lifecycle, valid time and transaction time are unchanged. Relation assertions are
   endpoint-checked as edges are. Every earlier revision replays to the assertion as it was
   (invariant 5), and the lineage names the prior reference. A held assertion already changes in a
   later revision under `RetractAssertion` and `SupersedeAssertion` (`apply.rs:454-485`).
9. **Properties.** A property `absorbed` holds and `into` does not moves to `into`; one both hold with
   equal values stays; one both hold with different values refuses the merge
   (`merge-property-conflict`). The absorbed node keeps every value in the record.

## Options

- **A. Merge as supersession plus in-place rewrite and lineage (proposed).** Ids stay stable
  (invariant 3); assessment, disputes and supersession chains keep their meaning; a split can restore
  the absorbed id from one record. Cost: the head's copy of a rewritten assertion differs from its
  bytes at earlier revisions; endpoint and cardinality revalidation in the kernel; one payload grows
  two optional fields.
- **B. Merge as supersession plus new assertion and edge ids.** Every assertion naming `absorbed` is
  superseded by a copy naming `into`, every edge deleted and recreated. Cost: `SupersedeAssertion`
  closes valid time at `effective_from`, a world change that did not happen; a copied `Accepted {
  validators }` claims a review no validator made, and resetting it loses acceptance; `Disputed` and
  `Superseded` references need a cascading rewrite; the assertion count doubles; a reverting split
  must mint a third set.
- **C. No merge apply: the proposer writes `SupersedeNode` plus explicit `CreateEdge` and
  `AddAssertion`.** Nothing new in the kernel. Cost: contradicts the 2026-10-14 rewrite rule; facts
  are duplicated under new ids; nothing records what moved, so a split cannot restore identity.
- **D. As A, but a new operation kind after `SupersedeNode`, leaving `MergeEntity` refused for
  good.** Cost: two names for one operation and a dead variant; it buys nothing over decision 6's
  shape gate.

## Open questions, with the default each takes

| # | question | options | default |
|---|---|---|---|
| 1 | how properties transfer | a: absent ones move, conflicting ones refuse; b: `into` wins, `absorbed`'s values only in the record; c: nothing moves, the rule is a guard | a: the only reading in which the cleared undeclared-property refusal has a purpose |
| 2 | the wire shape | a: extend `EntityMerge`, the bare shape keeps its refusal; b: a new operation kind | a: admitting the bare shape would break replay of retained rejections |
| 3 | where preparation goes | a: an earlier revision; b: earlier operations of the same transaction | a: keeps ADR 0017's same-transaction refusal |
| 4 | an edge joining `absorbed` and `into` | a: retired into the record; b: the merge is refused; c: a self-loop where the edge type admits it | a |
| 5 | which profiles admit the new merge | a: the ones that admit `SupersedeNode`; b: only v2 and v3 | a |
| 6 | ADR 0016 | a: `RetypeNode` stays proposed, after this story; b: archived, since `SupersedeNode` repairs the unrelated-type case | a: the operator decided on 2026-10-17 that retyping is wanted in its own right |

## Consequences

If A is accepted: `story:merge-entity-applies` builds it spec first, after `story:supersede-node`.
ADR 0016 decision 5 then reads "a node held in a supersession record". The split operation, its
record and its read of the lineage are a later story under decision 4; a split that reverts a merge
whose `into` was itself merged later reverts the later merge first (the ordering rule is fixed
there). Stores that never merge keep byte-identical roots and checkpoints.
