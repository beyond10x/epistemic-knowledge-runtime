---
format: aep.planning-md/3
id: story:merge-entity-applies
kind: story
status: draft
title: MergeEntity joins duplicate nodes of one type or of related types, and rewrites every reference to the survivor
relations:
- decomposes: epic:p3-incubation-integration
- serves: vision:o2
- depends_on: story:supersede-node
revision: 2
---
## Context

`MergeEntity { absorbed, into }` is declared (`systems/ekr/domains/kernel.yaml`, `ekr.kernel.EntityMerge`)
and refused at commit as `unsupported-operation`. A consumer holds duplicate nodes, of the same type and
of related types, and needs to repair them before it resumes two suspended ingestion sources.

The rules were decided on 2026-10-14 and are recorded on three cleared decision-blockers:

- `decision-blocker:merge-absorbed-node-fate`: after the merge, `absorbed` stays in canonical state in
  a new `Absorbed` lifecycle state pointing at `into`; every edge and assertion naming it is rewritten
  to `into` in the merging revision; a later merge of the survivor repoints earlier absorbed nodes to
  the new survivor, so a reader never follows a chain.
- `decision-blocker:merge-across-node-types`: a merge may join two nodes whose types are related
  through `parents` (one conforms to the other); `into`'s type survives; the merge is refused while
  `absorbed` holds a property the surviving type does not declare, and the proposer retracts it
  first. Two unrelated types (siblings included: neither conforms to the other) are refused; they are
  fixed first by an explicit retype or retraction.
- `decision-blocker:split-reverting-merge-identity`: a split that reverts a recorded merge restores
  the absorbed node's original id. This story records the merge lineage that rule needs; the split
  itself is not in scope.

The decision-blockers name "the merge and split architecture decision record"; none is in the store
yet. This story writes it.

## Build (spec first)

1. An architecture decision record, merge and split identity, carrying the three rules above.
2. `systems/ekr/domains/graph.yaml` and `kernel.yaml` first, green under `ess specify validate --path
   systems/ekr` (the pinned `ess` and the newest) before any code: the `Absorbed` lifecycle state of
   `ekr.graph.Node` and its `merged_into` reference; the merge lineage record; `EntityMerge`'s
   validation rules and named refusals (same node, absorbed node not present, types not related
   through `parents`, a property the surviving type does not declare, `absorbed` already absorbed);
   the rewrite of edges and assertions and the repoint of earlier absorbed nodes in the merging
   revision; how reads answer an absorbed id.
3. `ekr-kernel` validates and applies `MergeEntity`; replay applies it identically; both providers.
4. Reads: a read of an absorbed id answers its survivor (or a refusal naming it), never stale edges.
   Search and the views count only Present nodes.
5. `docs/cli.md` (the operation, its refusals and a worked example), `CHANGELOG.md`.

## Acceptance

Conformance scenarios and kernel tests, on both providers:

- `a-merge-of-same-type-duplicates-rewrites-every-reference`: edges and assertions naming `absorbed`
  name `into` after the merging revision; `absorbed` reads as Absorbed, pointing at `into`.
- `a-merge-into-a-parent-type-keeps-the-survivors-type`.
- `a-merge-across-sibling-types-is-refused`: neither type conforms to the other.
- `a-merge-is-refused-while-absorbed-holds-an-undeclared-property`; after the retraction it commits.
- `a-merge-of-a-survivor-repoints-earlier-absorbed-nodes`: no chain is left to follow.
- `a-merge-replays-to-identical-roots`: reopen and full replay give the same roots.
- `a-merge-of-an-absorbed-node-is-refused`.

## Out of scope

`SplitEntity` (its identity rule is recorded here only as the lineage it needs); a retype operation;
fuzzy duplicate detection.

## Builds on SupersedeNode

Added 2026-10-19 with `architecture-decision-record:0017-a-node-is-superseded-by-another`
(accepted): this story depends on `story:supersede-node`, which removes a duplicate from every head
read, records it with a pointer to the kept node and retires its edges with it. A merge builds on
that record: it is a supersession whose edges and assertions are rewritten to `into` instead of
retired, under the parent-type rule above. The `Absorbed` state named on
`decision-blocker:merge-absorbed-node-fate` is that supersession record. Build steps 2–4 below are
narrowed accordingly when this story is scoped.
