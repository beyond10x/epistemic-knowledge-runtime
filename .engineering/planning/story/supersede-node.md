---
format: aep.planning-md/3
id: story:supersede-node
kind: story
status: draft
title: SupersedeNode retires a duplicate node in favour of the node that replaces it, and its aliases resolve there
relations:
- decomposes: epic:p3-incubation-integration
- serves: vision:o2
revision: 1
---
## Context

A consumer's ingestion wrote duplicate nodes: a second node for an entity the store already held,
under another type or with a differently shaped alias (a `Tag` beside an adopted `Tag`; a
`MergeRequest` beside a `WorkItem` for the same tracker key `group/app!18`). Such defects recur, and
rebuilding a store replaces its history. The design is
`architecture-decision-record:0017-a-node-is-superseded-by-another` (accepted 2026-10-19, option
A): `SupersedeNode { node, by, reason, evidence }`. This story builds it; `story:merge-entity-applies`
builds on it.

Surfaces read at `2dd094077`: every head read builds from `CanonicalGraph.nodes` and `.edges`, most
through `ekr_views::Index::build` (`crates/ekr-views/src/index.rs:125`); `resolve` reads
`ekr_graph::AliasIndex::of` (`crates/ekr-graph/src/aliases.rs:23`); the kernel's alias holders are
`crates/ekr-kernel/src/validate/aliases.rs` and `validate/structural.rs:215`; the knowledge root is
`crates/ekr-store/src/log.rs:32`; the checkpoint graph serde is `crates/ekr-store/src/snapshot.rs:50`
(`GraphFields`) and `:109` (`BorrowedGraphFields`).

## Build (spec first)

1. `systems/ekr/domains/kernel.yaml`, `graph.yaml`, `integrate.yaml` and `views.yaml` first, green
   under `ess specify validate --path systems/ekr` before any code, and the conformance suites
   regenerated with their red scenarios:
   - `kernel.yaml`: `ekr.kernel.NodeSupersession` (`node`, `by`, `reason`, `evidence`), the
     operation kind `SupersedeNode` appended after the existing seventeen, so no variant number
     moves; its validation rules and named refusals (ADR 0017 decision 7).
   - `graph.yaml`: the superseded-node record (the node, the edges retired with it, `by`, the
     revision, the reason, the evidence), its relation to `ekr.graph.Node` (superseded node to kept
     node, many-to-one, referenced not owned), the graph's collection of them, and the knowledge-root
     rule: encoded only when non-empty, after the attachments.
   - `integrate.yaml`: resolution takes a superseded node matching the exact type and alias as the
     node its record names.
   - `views.yaml`: the `node-superseded` refusal of `describe_node`, `expand` and their routes,
     naming `by`.
2. `ekr-graph`: the collection on `CanonicalGraph`; `AliasIndex::of` maps a superseded node's
   aliases, under its own type, to its `by`.
3. `ekr-kernel`: validation and apply of `SupersedeNode` (remove the node and its edges from the
   head, record them, repoint earlier records whose `by` is this node); the alias holders count
   superseded nodes; the reference validator answers `node-superseded` for any later operation
   naming one; replay applies it identically; both providers.
4. `ekr-store`: the knowledge root and the checkpoint carry the collection only when non-empty; a
   graph with none hashes and serialises byte-identical to before.
5. `ekr-integrate`: `resolve` and `resolve_indexed` follow the record's pointer and still answer
   alike.
6. `ekr-views` and `crates/ekr`: `describe_node` and `expand` of a superseded id answer
   `node-superseded`; any read that names a superseded id (an assertion object, a reference) answers
   without failing; `ekr-sdk` and the agent help list the operation as applied.
7. `docs/cli.md` (the operation, its refusals, a worked example that `crates/ekr/tests/docs_cli.rs`
   runs on both providers), a dated design amendment, `CHANGELOG.md` under Unreleased.

## Acceptance

Kernel tests and conformance scenarios, on both providers, fixtures in the runtime's own
vocabulary only:

- `a-superseded-node-leaves-every-head-read`: after `SupersedeNode { node: R, by: K }`, search (v1,
  v2 and v3) for R's name and aliases returns no R; the overview's node count is one lower and its
  per-type count of R's type one lower; `expand` from R's former neighbour returns neither R nor an
  edge to it; the projection, timeline and graph UI routes (`/overview`, `/expand`, `/search`)
  hold no R.
- `a-read-of-a-superseded-id-names-its-successor`: `describe_node` and `expand` of R at the head
  refuse as `node-superseded` naming K.
- `a-superseded-node-reads-at-an-earlier-revision`: at the revision before the supersession,
  `describe_node` of R answers R with its edges, assertions and evidence.
- `a-superseded-alias-resolves-to-the-kept-node`: `ekr resolve` of R's type and alias answers K,
  with K of a different type than R; `ekr_integrate::resolve` and `resolve_indexed` agree.
- `a-superseded-alias-is-not-taken-again`: a later `CreateNode` of R's type with R's alias is
  refused as `alias-already-exists`.
- `a-supersession-is-its-own-revision`: the head moves by one revision; its record names K, the
  reason and the evidence.
- `a-superseded-nodes-edges-are-retired-with-it`: no head edge names R; a `CreateEdge` from K in the
  same transaction commits.
- `superseding-the-kept-node-repoints-earlier-records`: after K is superseded by L, R's record names
  L.
- `a-supersession-is-refused-by-name`: one case per refusal of ADR 0017 decision 7, each asserting
  its code.
- `a-supersession-replays-to-identical-roots`: reopen, with and without `--full-replay`, gives the
  same roots; a store that never supersedes keeps byte-identical roots and checkpoint bytes (the
  existing root fixtures stay green unedited).

## Out of scope

Rewriting edges or assertions to the kept node (`story:merge-entity-applies`); retyping; bulk
supersession by predicate; duplicate detection; undoing a supersession.
