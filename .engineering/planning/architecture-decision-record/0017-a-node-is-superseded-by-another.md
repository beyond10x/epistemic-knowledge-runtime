---
format: aep.planning-md/3
id: architecture-decision-record:0017-a-node-is-superseded-by-another
kind: architecture-decision-record
status: accepted
title: ADR 0017 — A duplicate node is superseded by the node that replaces it; MergeEntity builds on it
relations:
- decides: story:supersede-node
revision: 3
transitions:
- {from: "proposed", to: "accepted", at: "2026-10-19T11:14:42Z", actor: "agent:claude-ekr-session", revision: 2}
---
## Status

Accepted on 2026-10-19. The operator delegated this choice in the session that opened wave
2026-10-19a: decide whether the need below is met fastest by finishing `MergeEntity` or by a
smaller operation `MergeEntity` can build on, and record the decision. It is option A below.
`story:supersede-node` builds it, spec first.

## Context

A consumer's ingestion had a defect that wrote duplicate nodes: a second node for an entity the
store already held, under another type or with a differently shaped alias (for example a `Tag`
created beside an adopted `Tag`, or a `MergeRequest` beside a `WorkItem` for the same tracker key
`group/app!18`). Such defects will recur, and rebuilding a store to remove them replaces its
revision history (invariant 5).

Wanted: one write that retires a node in favour of another, so that readers stop showing the
retired node, its history stays readable at earlier revisions, the retirement is its own revision
with evidence and a reason, the retired node points at the node that replaces it, and its aliases
resolve to that node afterwards, so a later ingestion run that looks the old key up lands on the
kept node.

What the store already holds:

- `MergeEntity { absorbed, into }` parses and is refused at commit as `unsupported-operation`
  (`crates/ekr-kernel/src/apply.rs:486` at `2dd094077`). `story:merge-entity-applies` (draft)
  builds it under three rules cleared on 2026-10-14: the absorbed node stays in canonical state
  pointing at `into`; every edge and assertion naming it is rewritten to `into`; a merge across
  types is admitted only when one type conforms to the other through `parents`, so two unrelated
  types (`MergeRequest`, `WorkItem`) are refused until a retype or retraction fixes them.
- That fix is `story:retype-node` under ADR 0016, which is proposed, not accepted, and depends on
  the merge story.
- `story:evidence-erasure` (ADR 0015, accepted 2026-10-17) depends on both.

So finishing `MergeEntity` does not meet the need for the cross-type case without RetypeNode as
well: two stories, one of them behind a proposed record, plus the merge and split ADR the merge
story still has to write.

What the read side holds (surveyed at `2dd094077`): every view and MCP read, search, `resolve`,
`describe_node`, `expand`, the overview counts, the timeline, the projections and the graph UI's
four routes, builds from `CanonicalGraph.nodes` and `CanonicalGraph.edges`, most of them through
`ekr_views::Index::build` (`crates/ekr-views/src/index.rs:125`); `resolve` reads
`ekr_graph::AliasIndex::of` (`crates/ekr-graph/src/aliases.rs:23`), keyed by exact type and
alias. No provider keeps a node table or a search index; every index is derived in memory per
revision, and a read at an earlier revision replays to it (`crates/ekr-kernel/src/replay.rs:890`).

## Decision

Option A: a `SupersedeNode { node, by, reason, evidence }` operation, committed as an ordinary
validated transaction through the kernel, on which `MergeEntity` is later built.

1. **The retired node leaves the head's node map and enters a supersession record.** The applying
   revision removes `node` from `CanonicalGraph.nodes` and holds it, whole, in a new canonical
   collection of superseded nodes, keyed by its id: the node as it was, the edges retired with it,
   `by`, the revision, the reason and the evidence. Every reader that builds from `nodes` and
   `edges` stops showing it without a filter of its own, so a reader nobody updated fails closed:
   it omits the node rather than showing it.
2. **Its edges are retired with it, not moved.** Every edge with `node` at either end leaves
   `CanonicalGraph.edges` and is held in the supersession record. Moving them to `by` would have to
   revalidate each edge's endpoint types against `by`'s type, which for two unrelated types is the
   refusal the merge rule exists to make, and would have to settle cardinality and duplicate edges:
   that is `MergeEntity`'s work, under its rules. A proposer that wants an edge on the kept node
   writes a `CreateEdge` in the same transaction, which passes every ordinary validator. Nothing is
   lost: the edges are in the record and at every earlier revision.
3. **Assertions are not rewritten.** An assertion that names the retired node keeps its bytes,
   evidence and lifecycle; no read reaches it through the retired node, and a read that reaches it
   from elsewhere names the retired id with its pointer. Rewriting assertions to the survivor is
   `MergeEntity`'s.
4. **No chain.** `by` must be a node the head holds. Superseding a node that others were superseded
   by repoints those records to the new `by` in the same revision, so a reader follows at most one
   pointer, as the merge rule decided for absorbed nodes.
5. **Names resolve to the kept node.** The superseded node keeps its type and aliases in the
   record. Typed-reference resolution (`ekr_integrate::resolve` and `resolve_indexed`) takes a
   superseded node matching the reference's exact type and alias as the node `by` names, so a
   reference to the old key resolves to the kept node even when the two types differ. The kernel's
   alias holders keep counting the superseded node, so a later `CreateNode` or `AddAlias` of that
   type and alias is refused as `alias-already-exists` rather than recreating the duplicate.
6. **Reads.** A read of the superseded id at the head (`describe_node`, `expand`, the `/node/` and
   `/expand` routes) answers a named refusal, `node-superseded`, naming `by`; a read at an earlier
   revision answers the node, its edges, assertions and evidence as they were.
7. **The retirement is its own revision.** `reason` is required and non-empty; `evidence` names at
   least one evidence entry canonical state holds or the same transaction adds. Named refusals cover
   the rest: the node is unknown, already superseded (naming its `by`), the same as `by`, `by` is
   unknown or superseded, another operation of the transaction names the node or one of its edges,
   and any later operation naming a superseded node (`node-superseded`, naming `by`).
8. **Roots of stores that never supersede do not move.** The new collection enters the knowledge
   root only when it is non-empty, after the attachments, the way `attachments` did
   (`crates/ekr-store/src/log.rs:32`), and the replay checkpoint carries it only when non-empty.

`MergeEntity` builds on this: a merge is a supersession whose edges and assertions are rewritten to
`into` instead of retired, under the parent-type rule; the absorbed node's record is the
supersession record, and the `Absorbed` state the merge decision named is that record.

## Options

- **A. `SupersedeNode` now; `MergeEntity` built on it later (accepted).** One kernel operation, one
  canonical collection, one refusal on the reads; readers fail closed. Cost: edges are retired and
  any edge the kept node needs is recreated explicitly; assertions about the retired node stay
  attached to its id until a merge rewrites them.
- **B. Finish `MergeEntity` first.** Rewrites every reference, so nothing is retired. Cost: the
  merge and split ADR, the merge story, and for the cross-type case `RetypeNode` under ADR 0016,
  which is not accepted: three pieces of work before the first duplicate across two unrelated types
  can be repaired.
- **C. Retract and recreate, with `DeleteEdge` for the edges.** No new operation. Cost: the
  duplicate stays in every read, nothing points at the kept node, and its aliases still resolve to
  it, so the next ingestion run lands on the duplicate again.

## Consequences

`story:supersede-node` builds A, spec first, in `systems/ekr/domains/kernel.yaml`, `graph.yaml`,
`integrate.yaml` and `views.yaml`; `docs/cli.md`, the design (a dated amendment) and `CHANGELOG.md`
gain the operation. `story:merge-entity-applies` depends on it and is narrowed to the rewrite and
the type rule. `story:retype-node` refuses a superseded node as it refuses an absorbed one. No
existing store's roots, checkpoints or formats change until it commits a supersession.

## Clarifications

### Clarification to decision 3 (2026-10-20; story:supersede-node-review-fixes, D4)

A `NodeRef` value inside an assertion's `Object::Value` — a property whose value type is a `NodeRef`,
or such a value nested in a `List` or `Record` — keeps naming the superseded id, as every other
byte of the assertion keeps its value. Reads render no pointer for it: `object_superseded_by` is
written only for a `!Node` object (`views.yaml` `ekr.views.ProjectedAssertion`, `kernel.yaml`
`ekr.kernel` explanation link), so "a read that reaches it from elsewhere names the retired id with
its pointer" holds for a `!Node` object and for the subject. A reader that follows such a value to
the superseded id gets `node-superseded` (`ekr.views.NodeSuperseded`), which names `by`; and a
later operation writing such a value is refused as `node-superseded` by the reference validator,
naming `by`, so no new value names a superseded node.

### Clarification to decision 4 (2026-10-20; story:supersede-node-review-fixes, R2)

"`by` must be a node the head holds" admits a `by` that a `!CreateNode` of the same transaction
creates: creations apply before supersessions (design § 110.2), so the head the commit produces
holds it, and the reference validator accepts it (`supersede-by-unknown-node` only when `by` is
neither held nor created by the transaction). Two nodes may be superseded in one transaction, each
by a node the head holds or the transaction creates, and an edge between them is retired once, into
the record of its source. Pinned by `crates/ekr-kernel/tests/adversary_w20261019a_supersede_node.rs`
(`adv_two_nodes_superseded_together_with_an_edge_between_them`) and
`crates/ekr-kernel/tests/supersede_node.rs`
(`an_edge_between_two_nodes_superseded_together_is_held_by_its_sources_record`).
