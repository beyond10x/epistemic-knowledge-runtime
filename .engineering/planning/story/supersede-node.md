---
format: aep.planning-md/3
id: story:supersede-node
kind: story
status: active
title: SupersedeNode retires a duplicate node in favour of the node that replaces it, and its aliases resolve there
relations:
- decomposes: epic:p3-incubation-integration
- serves: vision:o2
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/ekr-graph/src/aliases.rs
- confidence: cited
  path: crates/ekr-graph/src/canonical.rs
- confidence: cited
  path: crates/ekr-graph/src/lib.rs
- confidence: cited
  path: crates/ekr-graph/tests
- confidence: cited
  path: crates/ekr-integrate/src/lib.rs
- confidence: cited
  path: crates/ekr-integrate/tests
- confidence: cited
  path: crates/ekr-kernel/src/apply.rs
- confidence: cited
  path: crates/ekr-kernel/src/checkpoint.rs
- confidence: cited
  path: crates/ekr-kernel/src/lib.rs
- confidence: cited
  path: crates/ekr-kernel/src/replay.rs
- confidence: cited
  path: crates/ekr-kernel/src/seed.rs
- confidence: cited
  path: crates/ekr-kernel/src/transaction.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate
- confidence: cited
  path: crates/ekr-kernel/tests
- confidence: cited
  path: crates/ekr-sdk/src/document/transaction.rs
- confidence: inferred
  path: crates/ekr-sdk/tests/document_drift.rs
- confidence: cited
  path: crates/ekr-store/src/log.rs
- confidence: cited
  path: crates/ekr-store/src/snapshot.rs
- confidence: cited
  path: crates/ekr-store/tests
- confidence: cited
  path: crates/ekr-views/src/changes.rs
- confidence: cited
  path: crates/ekr-views/src/detail.rs
- confidence: cited
  path: crates/ekr-views/src/document.rs
- confidence: cited
  path: crates/ekr-views/src/index.rs
- confidence: cited
  path: crates/ekr-views/src/query.rs
- confidence: cited
  path: crates/ekr-views/tests/conformance.rs
- confidence: cited
  path: crates/ekr-views/tests/fixtures/conformance
- confidence: inferred
  path: crates/ekr-views/tests/support/fixtures.rs
- confidence: cited
  path: crates/ekr-views/tests/support/suite.rs
- confidence: cited
  path: crates/ekr/src/cli/agent.rs
- confidence: cited
  path: crates/ekr/src/cli/mcp.rs
- confidence: cited
  path: crates/ekr/src/cli/session/views.rs
- confidence: cited
  path: crates/ekr/src/cli/view.rs
- confidence: inferred
  path: crates/ekr/tests/agent_cli.rs
- confidence: cited
  path: crates/ekr/tests/conformance.rs
- confidence: cited
  path: crates/ekr/tests/conformance_integrate.rs
- confidence: inferred
  path: crates/ekr/tests/docs_cli.rs
- confidence: cited
  path: crates/ekr/tests/fixtures/conformance
- confidence: inferred
  path: crates/ekr/tests/schema_cli.rs
- confidence: cited
  path: docs/cli.md
- confidence: cited
  path: docs/epistemic-knowledge-runtime-design.md
- confidence: inferred
  path: docs/sdk.md
- confidence: cited
  path: systems/ekr/conformance
- confidence: cited
  path: systems/ekr/domains/graph.yaml
- confidence: cited
  path: systems/ekr/domains/integrate.yaml
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
- confidence: cited
  path: systems/ekr/domains/views.yaml
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-10-19T11:23:55Z", actor: "agent:claude-ekr-session", revision: 12}
- {from: "proposed", to: "active", at: "2026-10-19T11:23:55Z", actor: "agent:claude-ekr-session", revision: 13}
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

## Scope

Derived 2026-10-19 by `aep:story-scoper` at `f05aa3326`. Every line is **cited** (read from the
story or the tree) or **inferred**.

- **Spec:** `systems/ekr/domains/{kernel,graph,integrate,views}.yaml` — cited.
- **Graph field:** `crates/ekr-graph/src/canonical.rs:441` `CanonicalGraph`;
  `crates/ekr-graph/src/aliases.rs:23` `AliasIndex::of` — cited.
- **`CanonicalGraph { … }` literals, 49 sites plus one destructure** — cited (rg, by file): src 4
  (`ekr-kernel/src/checkpoint.rs` 1, `seed.rs` 2, `validate/candidate.rs` 1); the destructure
  `ekr-store/src/snapshot.rs:162`; doc tests 3 (`ekr-graph/src/lib.rs:112`,
  `ekr-integrate/src/lib.rs:72`, `ekr-kernel/src/lib.rs:57`); `ekr-graph/tests` 10, one of them the
  trybuild case `review_p1_compile_fail/a_canonical_edge_may_target_a_candidate_node.rs` with a
  pinned `.stderr`; `ekr-integrate/tests` 2; `ekr-kernel/tests` 15; `ekr-store/tests` 15.
- **`GraphDocument { … }` literals, 9 in tests**, only if the checkpoint document gains the field —
  cited.
- **Exhaustive `GraphOperation` matches** — cited: `ekr-kernel/src/apply.rs:96` and `:407`;
  `replay.rs:162`; `transaction.rs:494` (enum; the new variant follows `CiteEvidence`), `:732`
  (`encode`), `:896`; `validate/aliases.rs:74`, `cardinality.rs:69`, `reference.rs:95`,
  `structural.rs:901`, `types.rs:108` and `:420`; `crates/ekr/src/cli/agent.rs:288` and its matches;
  `crates/ekr-sdk/src/document/transaction.rs` (`Operation`, `OperationKind`, the list, names, the
  macro at `:629`).
- **Wildcard matches that need a semantic look** — cited: `apply.rs:334`; `structural.rs:340, 351,
  699, 801, 1060, 1102`; `validate/{schema,lifecycle,attachment,candidate}.rs`; `explain.rs:307`;
  `records.rs:311`; `ekr-views/src/changes.rs:488` (a supersession is invisible to changes unless an
  arm is added).
- **Checkpoint serde:** `ekr-store/src/snapshot.rs` — `GraphDocument` `:25`, `GraphFields` `:52`,
  `BorrowedGraphFields` `:111`, `serialize_graph` `:162`, `of` `:193` — cited.
- **Knowledge root:** `ekr-store/src/log.rs:32`, the empty-skip at `:39` — cited.
- **Alias holders:** `ekr-kernel/src/validate/aliases.rs:74`, `validate/structural.rs:215` — cited.
- **Resolution:** `ekr-integrate/src/lib.rs` `resolve` `:207` and `resolve_indexed` `:228` — cited.
- **Views that refuse a whole revision when an assertion's subject is not held** — cited:
  `ekr-views/src/index.rs:201` (`Subject::Node`) and `:206` (`Subject::Edge`) return
  `ProjectError::Inconsistent`, so `Index::build` fails; `ekr-views/src/document.rs:411–432` (the
  projection) does the same. Every read built on `Index::load` inherits it. ADR 0017 decision 3 keeps
  those assertions, so both must skip an assertion whose subject is a superseded node or an edge
  retired with one, and keep `Inconsistent` for anything else (fix inferred).
- **Views paths that are safe** — cited: the indexing `graph.nodes[&id]` at `detail.rs:649, 692`,
  `query.rs:1471, 1515`, `search.rs:553, 718`, `timeline.rs:499, 596`, `ocel.rs:367` take ids from
  `Index.node_ids`; `Object::Node` lookups already tolerate a missing node (`index.rs:219, 530–536,
  678`, `detail.rs:600, 612`, `query.rs:1360–1367`, `sample.rs:197, 326`, `cli/view.rs:1501`).
- **The `node-superseded` refusal:** `QueryError` in `ekr-views/src/query.rs:70`, raised near
  `query.rs:1203, 1336` and `detail.rs:499–513`; mapped in `crates/ekr/src/cli/mcp.rs:320`,
  `cli/view.rs:1324`, `cli/session/views.rs:445` — cited.
- **Conformance:** `systems/ekr/conformance/{suite,baseline,provenance}.json` and the views and
  integrate triples; `crates/ekr/tests/fixtures/conformance` (manifest and scenarios),
  `crates/ekr-views/tests/fixtures/conformance/scenarios`, runners `crates/ekr/tests/conformance.rs`,
  `conformance_integrate.rs`, `crates/ekr-views/tests/conformance.rs`; freshness `Taskfile.yml:101–112`
  — cited.
- **Also likely:** `crates/ekr-sdk/tests/document_drift.rs`, `crates/ekr/tests/{agent_cli,schema_cli,
  docs_cli}.rs`, `crates/ekr-views/tests/support/fixtures.rs`, `docs/sdk.md` — inferred from the
  `AttachEvidence` change `68ef2ecfa`.
- **Documents:** `docs/cli.md`, the design amendment, `CHANGELOG.md` — cited.
- **Confidence:** high; every list was read from rg output.
- **Would collide with:** any unit touching `CanonicalGraph`'s fields, `GraphOperation` or
  `OperationKind`, `ekr-views/src/index.rs`, the store's snapshot or root encoding, or a committed
  conformance suite.
- **Not established:** whether the views and integrate conformance runners need new fixture shapes;
  whether `quality.rs`, `code_names.rs` and `process_map.rs` look subjects up themselves; whether
  `ekr-sdk`'s `SnapshotGraph` (`crates/ekr-sdk/src/read/kernel.rs:75`) must carry the field.
